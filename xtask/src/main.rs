use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Instant;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let succeeded = match (args.next().as_deref(), args.next().as_deref()) {
        (Some("check-all"), None) => return check_all(),
        (Some("check-core"), None) => check_core_job(&workspace_root()),
        (Some("check-rp"), None) => check_rp(&workspace_root()),
        (Some("check-esp"), arch) => match EspArch::parse(arch) {
            Some(arch) => check_esp(&workspace_root(), arch),
            None => return usage(),
        },
        _ => return usage(),
    };
    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "Usage: cargo run --manifest-path xtask/Cargo.toml -- \
         <check-all | check-core | check-rp | check-esp [riscv | xtensa]>\n\
         check-all runs everything, formatting the workspace first; the others are the \
         parts CI runs as separate jobs."
    );
    ExitCode::FAILURE
}

/// Which ESP chips a check covers.
#[derive(Clone, Copy)]
enum EspArch {
    All,
    Riscv,
    Xtensa,
}

impl EspArch {
    fn parse(arg: Option<&str>) -> Option<Self> {
        match arg {
            None => Some(Self::All),
            Some("riscv") => Some(Self::Riscv),
            Some("xtensa") => Some(Self::Xtensa),
            Some(_) => None,
        }
    }

    /// The ESP xtask's `--arch` argument, if any.
    fn as_arg(self) -> &'static [&'static str] {
        match self {
            Self::All => &[],
            Self::Riscv => &["--arch", "riscv"],
            Self::Xtensa => &["--arch", "xtensa"],
        }
    }
}

fn check_all() -> ExitCode {
    let workspace_root = workspace_root();
    let start = Instant::now();

    println!("==> Checking documentation self-links...");
    if !reject_absolute_core_self_links(&workspace_root) {
        return ExitCode::FAILURE;
    }

    println!("==> Formatting workspace...");
    if !run_command(
        Command::new("cargo")
            .current_dir(&workspace_root)
            .args(["fmt", "--all"]),
    ) {
        return ExitCode::FAILURE;
    }

    println!("==> Running top-level checks in parallel...");
    let handles = [
        spawn_check("core", &workspace_root, check_core),
        spawn_check("device-envoy", &workspace_root, check_device_envoy_crate),
        spawn_check("esp", &workspace_root, |workspace_root| {
            check_esp(workspace_root, EspArch::All)
        }),
        spawn_check("rp", &workspace_root, check_rp),
    ];

    let mut failed = false;
    for (name, handle) in handles {
        match handle.join() {
            Ok(true) => {}
            Ok(false) => failed = true,
            Err(_) => {
                eprintln!("{name} check thread panicked");
                failed = true;
            }
        }
    }

    let elapsed = start.elapsed();
    if failed {
        eprintln!("check-all failed in {:.1}s", elapsed.as_secs_f64());
        ExitCode::FAILURE
    } else {
        println!("check-all passed in {:.1}s", elapsed.as_secs_f64());
        ExitCode::SUCCESS
    }
}

/// Runs `check` on its own thread, labeled `name` for error messages.
fn spawn_check(
    name: &'static str,
    workspace_root: &Path,
    check: fn(&Path) -> bool,
) -> (&'static str, thread::JoinHandle<bool>) {
    let workspace_root = workspace_root.to_path_buf();
    (name, thread::spawn(move || check(&workspace_root)))
}

/// CI's core job: formatting (checked, not applied), documentation self-links, Core, and
/// the `device-envoy` landing crate.
fn check_core_job(workspace_root: &Path) -> bool {
    run_command(
        Command::new("cargo")
            .current_dir(workspace_root)
            .args(["fmt", "--all", "--", "--check"]),
    ) && reject_absolute_core_self_links(workspace_root)
        && check_core(workspace_root)
        && check_device_envoy_crate(workspace_root)
}

fn check_core(workspace_root: &Path) -> bool {
    run_command(
        Command::new("cargo")
            .current_dir(workspace_root.join("crates/device-envoy-core"))
            .args(["test", "--features", "host", "--lib", "--tests"]),
    ) && run_command(
        Command::new("cargo")
            .current_dir(workspace_root.join("crates/device-envoy-core"))
            .env("RUSTDOCFLAGS", "-D warnings")
            .args(["test", "--doc", "--features", "host"]),
    ) && run_command(
        Command::new("cargo")
            .current_dir(workspace_root.join("crates/device-envoy-core"))
            .env("RUSTDOCFLAGS", "-D warnings")
            // `wasm` is added here (docs-only) so intra-doc links to
            // `CydWasm` resolve; it must stay out of the test/check steps
            // above and below, which mirror real firmware feature sets.
            .args(["doc", "--no-deps", "--features", "host,wasm"]),
    ) && run_command(
        Command::new("cargo")
            .current_dir(workspace_root.join("crates/device-envoy-core"))
            .args(["check", "--features", "host", "--examples"]),
    ) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "test",
        "-p",
        "device-envoy-examples-core",
        "--features",
        "host",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-examples-core",
        "--no-default-features",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-dns-tester-wasm",
        "--tests",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-conway-wasm",
    ]))
}

fn check_device_envoy_crate(workspace_root: &Path) -> bool {
    run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy",
    ]))
}

fn check_esp(workspace_root: &Path, arch: EspArch) -> bool {
    let esp_xtask = |command: &str| {
        run_command(
            Command::new("cargo")
                .current_dir(workspace_root)
                .args([
                    "run",
                    "--manifest-path",
                    "crates/device-envoy-examples-esp/xtask/Cargo.toml",
                    "--",
                    command,
                ])
                .args(arch.as_arg()),
        )
    };
    // These don't depend on the chip architecture, so the Xtensa CI job skips them.
    let chip_independent = || {
        run_command(Command::new("cargo").current_dir(workspace_root).args([
            "check",
            "-p",
            "device-envoy-esp",
            "--lib",
        ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
            "run",
            "--manifest-path",
            "crates/device-envoy-examples-esp/xtask/Cargo.toml",
            "--",
            "check-board-examples",
        ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
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
        ]))
    };
    (matches!(arch, EspArch::Xtensa) || chip_independent())
        && esp_xtask("check-examples-all-processors")
        && esp_xtask("check-embedded-tests")
}

fn check_rp(workspace_root: &Path) -> bool {
    run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-rp",
        "--lib",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "run",
        "--manifest-path",
        "crates/device-envoy-rp/xtask/Cargo.toml",
        "--",
        "check-compile-only",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-examples-rp",
        "--examples",
        "--target",
        "thumbv6m-none-eabi",
        "--no-default-features",
        "--features",
        "embedded,pico1,wifi",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
        "check",
        "-p",
        "device-envoy-examples-rp",
        "--examples",
        "--target",
        "thumbv8m.main-none-eabihf",
        "--no-default-features",
        "--features",
        "embedded-pico2,wifi",
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
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
    ])) && run_command(Command::new("cargo").current_dir(workspace_root).args([
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
}

fn reject_absolute_core_self_links(workspace_root: &Path) -> bool {
    let core_root = workspace_root.join("crates/device-envoy-core");
    let mut matches = Vec::new();
    let result = ["src", "docs"].into_iter().try_for_each(|directory| {
        find_text_matches(
            &core_root.join(directory),
            &core_root,
            "https://docs.rs/device-envoy-core/",
            &mut matches,
        )
    });

    match result {
        Ok(()) if matches.is_empty() => true,
        Ok(()) => {
            eprintln!(
                "Core documentation must use checked intra-doc links for Core items:\n{}",
                matches.join("\n")
            );
            false
        }
        Err(error) => {
            eprintln!("failed to scan Core documentation self-links: {error}");
            false
        }
    }
}

fn find_text_matches(
    directory: &Path,
    display_root: &Path,
    needle: &str,
    matches: &mut Vec<String>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            find_text_matches(&path, display_root, needle, matches)?;
            continue;
        }

        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("md" | "rs")
        ) {
            continue;
        }

        let contents = fs::read_to_string(&path)?;
        for (line_index, line) in contents.lines().enumerate() {
            if line.contains(needle) {
                let display_path = path.strip_prefix(display_root).unwrap_or(&path);
                matches.push(format!(
                    "{}:{}:{}",
                    display_path.display(),
                    line_index + 1,
                    line.trim()
                ));
            }
        }
    }
    Ok(())
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
