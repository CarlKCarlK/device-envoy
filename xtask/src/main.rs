use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Instant;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check-all") => check_all(),
        _ => {
            eprintln!("Usage: cargo run --manifest-path xtask/Cargo.toml -- check-all");
            ExitCode::FAILURE
        }
    }
}

fn check_all() -> ExitCode {
    let workspace_root = workspace_root();
    let start = Instant::now();

    println!("==> Formatting workspace...");
    if !run_command(
        Command::new("cargo")
            .current_dir(&workspace_root)
            .args(["fmt", "--all"]),
    ) {
        return ExitCode::FAILURE;
    }

    println!("==> Running top-level checks in parallel...");
    let core_workspace_root = workspace_root.clone();
    let core_handle = thread::spawn(move || -> bool {
        run_command(
            Command::new("cargo")
                .current_dir(core_workspace_root.join("crates/device-envoy-core"))
                .args(["test", "--features", "host", "--lib", "--tests"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(core_workspace_root.join("crates/device-envoy-core"))
                .env("RUSTDOCFLAGS", "-D warnings")
                .args(["test", "--doc", "--features", "host"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(core_workspace_root.join("crates/device-envoy-core"))
                .env("RUSTDOCFLAGS", "-D warnings")
                // `wasm` is added here (docs-only) so intra-doc links to
                // `CydWasm` resolve; it must stay out of the test/check steps
                // above and below, which mirror real firmware feature sets.
                .args(["doc", "--no-deps", "--features", "host,wasm"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(core_workspace_root.join("crates/device-envoy-core"))
                .args(["check", "--features", "host", "--examples"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&core_workspace_root)
                .args([
                    "test",
                    "-p",
                    "device-envoy-examples-core",
                    "--features",
                    "host",
                ]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&core_workspace_root)
                .args([
                    "check",
                    "-p",
                    "device-envoy-examples-core",
                    "--no-default-features",
                ]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&core_workspace_root)
                .args(["check", "-p", "device-envoy-dns-tester-wasm", "--tests"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&core_workspace_root)
                .args(["check", "-p", "device-envoy-conway-wasm"]),
        )
    });

    let device_envoy_workspace_root = workspace_root.clone();
    let device_envoy_handle = thread::spawn(move || -> bool {
        run_command(
            Command::new("cargo")
                .current_dir(&device_envoy_workspace_root)
                .args(["check", "-p", "device-envoy"]),
        )
    });

    let esp_workspace_root = workspace_root.clone();
    let esp_handle = thread::spawn(move || -> bool {
        run_command(
            Command::new("cargo")
                .current_dir(&esp_workspace_root)
                .args(["check", "-p", "device-envoy-esp", "--lib"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&esp_workspace_root)
                .args([
                    "run",
                    "--manifest-path",
                    "crates/device-envoy-examples-esp/xtask/Cargo.toml",
                    "--",
                    "check-board-examples",
                ]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&esp_workspace_root)
                .args([
                    "run",
                    "--manifest-path",
                    "crates/device-envoy-examples-esp/xtask/Cargo.toml",
                    "--",
                    "check-examples-all-processors",
                ]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(&esp_workspace_root)
                .args([
                    "check",
                    "-p",
                    "device-envoy-examples-esp",
                    "--example",
                    "conway_esp32c6_generic",
                    "--target",
                    "riscv32imac-unknown-none-elf",
                    "--no-default-features",
                    "--features",
                    "esp32c6",
                ]),
        )
    });

    let rp_workspace_root = workspace_root;
    let rp_handle = thread::spawn(move || -> bool {
        run_command(Command::new("cargo").current_dir(&rp_workspace_root).args([
            "check",
            "-p",
            "device-envoy-rp",
            "--lib",
        ])) && run_command(Command::new("cargo").current_dir(&rp_workspace_root).args([
            "check",
            "-p",
            "device-envoy-examples-rp",
            "--examples",
            "--target",
            "thumbv6m-none-eabi",
            "--no-default-features",
            "--features",
            "embedded,pico1,wifi",
        ])) && run_command(Command::new("cargo").current_dir(&rp_workspace_root).args([
            "check",
            "-p",
            "device-envoy-examples-rp",
            "--examples",
            "--target",
            "thumbv8m.main-none-eabihf",
            "--no-default-features",
            "--features",
            "embedded-pico2,wifi",
        ])) && run_command(Command::new("cargo").current_dir(&rp_workspace_root).args([
            "check",
            "-p",
            "device-envoy-examples-rp",
            "--example",
            "conway",
            "--target",
            "thumbv6m-none-eabi",
            "--no-default-features",
            "--features",
            "embedded,pico1",
        ])) && run_command(Command::new("cargo").current_dir(&rp_workspace_root).args([
            "check",
            "-p",
            "device-envoy-examples-rp",
            "--example",
            "dns_tester",
            "--target",
            "thumbv6m-none-eabi",
            "--no-default-features",
            "--features",
            "embedded,pico1,wifi",
        ]))
    });

    let mut failed = false;
    match core_handle.join() {
        Ok(core_ok) => {
            if !core_ok {
                failed = true;
            }
        }
        Err(_) => {
            eprintln!("core check thread panicked");
            failed = true;
        }
    }
    match esp_handle.join() {
        Ok(esp_ok) => {
            if !esp_ok {
                failed = true;
            }
        }
        Err(_) => {
            eprintln!("esp check thread panicked");
            failed = true;
        }
    }
    match rp_handle.join() {
        Ok(rp_ok) => {
            if !rp_ok {
                failed = true;
            }
        }
        Err(_) => {
            eprintln!("rp check thread panicked");
            failed = true;
        }
    }
    match device_envoy_handle.join() {
        Ok(device_envoy_ok) => {
            if !device_envoy_ok {
                failed = true;
            }
        }
        Err(_) => {
            eprintln!("device-envoy check thread panicked");
            failed = true;
        }
    }

    if failed {
        let elapsed = start.elapsed();
        eprintln!("check-all failed in {:.1}s", elapsed.as_secs_f64());
        ExitCode::FAILURE
    } else {
        let elapsed = start.elapsed();
        println!("check-all passed in {:.1}s", elapsed.as_secs_f64());
        ExitCode::SUCCESS
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should be in workspace_root/xtask")
        .to_path_buf()
}

fn run_command(command: &mut Command) -> bool {
    match command.status() {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("Failed to execute command: {error}");
            false
        }
    }
}
