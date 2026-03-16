use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;

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
                .args(["doc", "--no-deps", "--features", "host"]),
        ) && run_command(
            Command::new("cargo")
                .current_dir(core_workspace_root.join("crates/device-envoy-core"))
                .args(["check", "--features", "host", "--examples"]),
        )
    });

    let esp_workspace_root = workspace_root.clone();
    let esp_handle = thread::spawn(move || -> bool {
        run_command(
            Command::new("cargo")
                .current_dir(esp_workspace_root.join("crates/device-envoy-esp"))
                .args(["check-all"]),
        )
    });

    let rp_workspace_root = workspace_root;
    let rp_handle = thread::spawn(move || -> bool {
        run_command(
            Command::new("cargo")
                .current_dir(rp_workspace_root.join("crates/device-envoy-rp"))
                .args(["check-all"]),
        )
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

    if failed {
        eprintln!("check-all failed");
        ExitCode::FAILURE
    } else {
        println!("check-all passed");
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
