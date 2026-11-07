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
        (Some("check-docs-rs"), None) => check_docs_rs(&workspace_root()),
        (Some("check-clippy"), None) => check_clippy(&workspace_root()),
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
         <check-all | check-core | check-rp | check-esp [riscv | xtensa] | check-docs-rs | \
         check-clippy>\n\
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
        spawn_check("clippy", &workspace_root, check_clippy),
        spawn_check("docs.rs", &workspace_root, check_docs_rs),
    ];

    let mut failed = false;
    for (name, handle) in handles {
        match handle.join() {
            Ok(true) => {}
            Ok(false) => {
                // The checks run in parallel, so name the failing one; its errors are above.
                eprintln!("{name} check failed");
                failed = true;
            }
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

/// The host clippy runs. The `Check Clippy` workflow runs this function (`check-clippy`),
/// and so does `check-all`, so the two cannot drift apart.
fn check_clippy(workspace_root: &Path) -> bool {
    const CLIPPY_RUNS: [&[&str]; 6] = [
        &[
            "-p",
            "device-envoy-core",
            "--all-targets",
            "--no-default-features",
        ],
        &[
            "-p",
            "device-envoy-core",
            "--all-targets",
            "--no-default-features",
            "--features",
            "host",
        ],
        &[
            "-p",
            "device-envoy-core",
            "--all-targets",
            "--no-default-features",
            "--features",
            "wifi",
        ],
        &[
            "-p",
            "device-envoy",
            "--all-targets",
            "--no-default-features",
        ],
        &[
            "-p",
            "device-envoy-examples-core",
            "--all-targets",
            "--no-default-features",
        ],
        &[
            "-p",
            "device-envoy-examples-core",
            "--all-targets",
            "--features",
            "host",
        ],
    ];
    CLIPPY_RUNS.iter().all(|clippy_args| {
        run_command(
            Command::new("cargo")
                .current_dir(workspace_root)
                .arg("clippy")
                .args(*clippy_args)
                .args(["--", "-D", "warnings"]),
        )
    })
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

/// The published crates, each documented on docs.rs.
const DOCS_RS_CRATES: [&str; 3] = ["device-envoy-core", "device-envoy-rp", "device-envoy-esp"];

/// Builds each published crate's documentation the way docs.rs does: nightly rustdoc,
/// `--cfg docsrs`, and the features, target, and rustdoc arguments from the crate's
/// `[package.metadata.docs.rs]`. Rustdoc warnings are errors for every crate.
///
/// docs.rs uses the newest nightly, and RP 0.2.0's docs failed there on lints that older
/// toolchains do not report. Run `rustup update nightly` first to match it. Only the
/// default target is built; it is the one docs.rs's build status depends on.
fn check_docs_rs(workspace_root: &Path) -> bool {
    if !run_command(
        Command::new("cargo")
            .args(["+nightly", "--version"])
            .stdout(std::process::Stdio::null()),
    ) {
        eprintln!("docs.rs check needs a nightly toolchain: rustup toolchain install nightly");
        return false;
    }
    DOCS_RS_CRATES.iter().all(|package| {
        let manifest_path = workspace_root
            .join("crates")
            .join(package)
            .join("Cargo.toml");
        let metadata = match DocsRsMetadata::read(&manifest_path) {
            Ok(metadata) => metadata,
            Err(message) => {
                eprintln!("{}: {message}", manifest_path.display());
                return false;
            }
        };
        println!("==> docs.rs build of {package}");
        let mut rustdoc_flags = vec!["--cfg", "docsrs", "-D", "warnings"];
        rustdoc_flags.extend(metadata.rustdoc_args.iter().map(String::as_str));
        let mut command = Command::new("cargo");
        command
            .current_dir(workspace_root)
            .env("DOCS_RS", "1")
            .env("RUSTDOCFLAGS", rustdoc_flags.join(" "))
            .args(["+nightly", "rustdoc", "--lib", "-p", package])
            .args(["--target-dir", "target/docs-rs-check"]);
        if metadata.no_default_features {
            command.arg("--no-default-features");
        }
        if !metadata.features.is_empty() {
            command.args(["--features", &metadata.features.join(",")]);
        }
        if let Some(target) = &metadata.default_target {
            command.args(["--target", target]);
        }
        run_command(&mut command)
    })
}

/// The `[package.metadata.docs.rs]` settings this check understands.
#[derive(Default)]
struct DocsRsMetadata {
    no_default_features: bool,
    features: Vec<String>,
    default_target: Option<String>,
    rustdoc_args: Vec<String>,
}

impl DocsRsMetadata {
    /// Reads the table from a manifest. Any key this check does not understand is an error,
    /// so a docs.rs setting cannot be silently ignored.
    fn read(manifest_path: &Path) -> Result<Self, String> {
        let manifest = fs::read_to_string(manifest_path).map_err(|error| error.to_string())?;
        let mut lines = manifest
            .lines()
            .skip_while(|line| line.trim() != "[package.metadata.docs.rs]");
        if lines.next().is_none() {
            return Err("no [package.metadata.docs.rs] table".to_owned());
        }
        let mut metadata = Self::default();
        for line in lines
            .map(str::trim)
            .take_while(|line| !line.starts_with('['))
        {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("unexpected docs.rs metadata line: {line}"))?;
            let value = value.trim();
            match key.trim() {
                "all-features" if value == "false" => {}
                "no-default-features" => metadata.no_default_features = value == "true",
                "features" => metadata.features = parse_string_array(value)?,
                "default-target" => metadata.default_target = Some(parse_string(value)?),
                "targets" => {}
                "rustdoc-args" => metadata.rustdoc_args = parse_string_array(value)?,
                _ => return Err(format!("docs.rs check does not understand: {line}")),
            }
        }
        Ok(metadata)
    }
}

fn parse_string(value: &str) -> Result<String, String> {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| format!("expected a one-line string: {value}"))
}

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let items = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("expected a one-line array: {value}"))?;
    items
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(parse_string)
        .collect()
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
