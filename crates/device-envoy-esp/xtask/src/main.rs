//! Build automation tasks for the device-envoy-esp project.
//!
//! Run with: `cargo xtask <command>`

mod ir_generated;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TARGET_C6: &str = "riscv32imac-unknown-none-elf";
const TARGET_S3: &str = "xtensa-esp32s3-none-elf";

/// Locates the `xtensa-esp32s3-elf-gcc` linker and returns its parent directory.
///
/// `espup install` places the linker inside the `esp` rustup toolchain directory,
/// which is not automatically on PATH in every shell.  This function checks PATH
/// first, then falls back to searching the toolchain tree so that S3 builds work
/// after `espup install` without requiring `source ~/export-esp.sh`.
///
/// Returns `None` with a descriptive error printed if the linker is not found.
fn find_s3_linker_dir() -> Option<PathBuf> {
    const LINKER: &str = "xtensa-esp32s3-elf-gcc";

    // Check PATH first.
    if Command::new(LINKER)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Already on PATH — no extra dir needed (use empty PathBuf as sentinel).
        return Some(PathBuf::new());
    }

    // espup installs the linker inside ~/.rustup/toolchains/esp/; search there.
    let home = std::env::var_os("HOME").unwrap_or_default();
    let esp_toolchain = Path::new(&home).join(".rustup/toolchains/esp");

    fn find_in(dir: &Path, linker: &str, depth: u32) -> Option<PathBuf> {
        if depth == 0 {
            return None;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return None;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join(linker).exists() {
                    return Some(path);
                }
                if let Some(found) = find_in(&path, linker, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    if let Some(dir) = find_in(&esp_toolchain, LINKER, 6) {
        return Some(dir);
    }

    eprintln!(
        "{}",
        "error: xtensa-esp32s3-elf-gcc not found.\n\
         Run `espup install` to install the Xtensa GCC linker, then re-run check-all."
            .red()
            .bold()
    );
    None
}

/// Returns `Some(linker_dir)` when the full S3 toolchain is ready, `None` otherwise.
///
/// Requires the `esp` rustup toolchain with `rust-src` *and* the Xtensa GCC linker.
/// Prints a clear error if anything is missing.
fn require_s3_toolchain() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").unwrap_or_default();
    let rust_src = Path::new(&home).join(".rustup/toolchains/esp/lib/rustlib/src/rust");

    if !rust_src.exists() {
        eprintln!(
            "{}",
            "error: `esp` rustup toolchain with rust-src not found.\n\
             Run `espup install` to install it, then re-run check-all."
                .red()
                .bold()
        );
        return None;
    }

    find_s3_linker_dir()
}

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for device-envoy-esp project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all checks: lib + all examples + docs
    CheckAll,
    /// Build all examples (catches linker errors)
    CheckExamples,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::CheckAll => check_all(),
        Commands::CheckExamples => check_examples(),
    }
}

fn check_all() -> ExitCode {
    let root = workspace_root();

    println!("{}", "==> cargo check-all: device-envoy-esp".cyan().bold());

    if let Err(err) = ir_generated::generate_ir_generated(&root) {
        eprintln!("Error generating ir_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = check_generated_doc_stubs(&root) {
        eprintln!("Generated doc stub consistency check failed:\n{err}");
        return ExitCode::FAILURE;
    }

    // Build the library itself for both supported chips.
    // S3 uses -Zbuild-std because the `esp` toolchain ships no prebuilt Xtensa sysroot.
    let Some(s3_linker_dir) = require_s3_toolchain() else {
        return ExitCode::FAILURE;
    };
    // (label, target, toolchain_override, build_std)
    let targets: &[(&str, &str, Option<&str>, bool)] = &[
        ("c6", TARGET_C6, None, false),
        ("s3", TARGET_S3, Some("+esp"), true),
    ];
    for (label, target, toolchain, build_std) in targets {
        println!("{}", format!("--> build lib ({label})").cyan());
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        if *build_std {
            prepend_path(&mut cmd, &s3_linker_dir);
        }
        if let Some(tc) = toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            "build",
            "--lib",
            "--release",
            "--target",
            target,
            "--no-default-features",
        ]);
        if *build_std {
            cmd.arg("-Zbuild-std=core,alloc");
        }
        if !run(&mut cmd) {
            return ExitCode::FAILURE;
        }
    }

    if check_examples() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    if check_embedded_tests() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    if check_host_tests() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    // Generate docs (docs only need to be generated once; C6 is the primary target).
    println!("{}", "--> doc".cyan());
    if !run(Command::new("cargo").current_dir(&root).args([
        "doc",
        "--no-deps",
        "--release",
        "--target",
        TARGET_C6,
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> All checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_examples() -> ExitCode {
    let root = workspace_root();

    let examples_dir = root.join("examples");
    let mut examples: Vec<String> = std::fs::read_dir(&examples_dir)
        .expect("examples/ directory not found")
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.ends_with(".rs")
                .then(|| name.trim_end_matches(".rs").to_owned())
        })
        .collect();
    examples.sort();

    let Some(s3_linker_dir) = require_s3_toolchain() else {
        return ExitCode::FAILURE;
    };
    // (label, target, toolchain_override, build_std)
    let targets: &[(&str, &str, Option<&str>, bool)] = &[
        ("c6", TARGET_C6, None, false),
        ("s3", TARGET_S3, Some("+esp"), true),
    ];
    for (label, target, toolchain, build_std) in targets {
        println!("{}", format!("--> build examples ({label})").cyan());
        for example in &examples {
            println!("    build example: {example}");
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&root);
            if *build_std {
                prepend_path(&mut cmd, &s3_linker_dir);
            }
            if let Some(tc) = toolchain {
                cmd.arg(tc);
            }
            cmd.args([
                "build",
                "--example",
                example,
                "--release",
                "--target",
                target,
                "--no-default-features",
            ]);
            if *build_std {
                cmd.arg("-Zbuild-std=core,alloc");
            }
            if !run(&mut cmd) {
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn check_embedded_tests() -> ExitCode {
    let root = workspace_root();
    println!(
        "{}",
        "--> embedded tests (compile-pass + expected compile-fail)".cyan()
    );

    let compile_pass_tests = [
        "ir_two_receivers_compile",
        "led_strip_two_strips_compile",
        "led_strip_spi_two_strips_compile",
        "led2d_two_panels_compile",
        "led4_two_displays_compile",
        "lcd_text_four_addresses_compile",
        "clock_sync_two_compile",
        "button_five_compile",
        "button_watch_five_compile",
        "ir_four_receivers_compile",
        "ir_mapping_four_receivers_compile",
        "ir_kepler_four_receivers_compile",
    ];
    let compile_fail_tests = [
        "ir_duplicate_channel_compile_fail",
        "lcd_text_duplicate_address_compile_fail",
    ];
    let Some(s3_linker_dir) = require_s3_toolchain() else {
        return ExitCode::FAILURE;
    };
    // (label, target, toolchain_override, build_std)
    let targets: &[(&str, &str, Option<&str>, bool)] = &[
        ("c6", TARGET_C6, None, false),
        ("s3", TARGET_S3, Some("+esp"), true),
    ];
    for (label, target, toolchain, build_std) in targets {
        println!("{}", format!("    target: {label}").cyan());
        for embedded_test in &compile_pass_tests {
            println!("      compile-pass test: {embedded_test}");
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&root);
            if *build_std {
                prepend_path(&mut cmd, &s3_linker_dir);
            }
            if let Some(tc) = toolchain {
                cmd.arg(tc);
            }
            cmd.args([
                "build",
                "--test",
                embedded_test,
                "--release",
                "--target",
                target,
                "--no-default-features",
            ]);
            if *build_std {
                cmd.arg("-Zbuild-std=core,alloc");
            }
            if !run(&mut cmd) {
                return ExitCode::FAILURE;
            }
        }

        for embedded_test in &compile_fail_tests {
            println!("      compile-fail test: {embedded_test}");
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&root);
            // Compile-fail tests are expected to fail. Keep their output non-colored so
            // expected rustc errors do not show as alarming red blocks in the check log.
            cmd.env("CARGO_TERM_COLOR", "never");
            if *build_std {
                prepend_path(&mut cmd, &s3_linker_dir);
            }
            if let Some(tc) = toolchain {
                cmd.arg(tc);
            }
            cmd.args([
                "build",
                "--test",
                embedded_test,
                "--release",
                "--target",
                target,
                "--no-default-features",
                "--features",
                "compile-fail-tests",
            ]);
            if *build_std {
                cmd.arg("-Zbuild-std=core,alloc");
            }
            if run_expect_failure(&mut cmd) {
                eprintln!(
                    "{}",
                    format!(
                        "error: compile-fail test `{embedded_test}` unexpectedly compiled for target `{target}`"
                    )
                    .red()
                    .bold()
                );
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn check_host_tests() -> ExitCode {
    let root = workspace_root();
    println!("{}", "--> host tests (including compile-fail UI)".cyan());

    let tests_dir = root.join("tests");
    let mut tests: Vec<String> = std::fs::read_dir(&tests_dir)
        .expect("tests/ directory not found")
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.ends_with(".rs")
                .then(|| name.trim_end_matches(".rs").to_owned())
        })
        .collect();
    tests.sort();

    for test in &tests {
        println!("    test target: {test}");
        if !run(Command::new("cargo").current_dir(&root).args([
            "test",
            "--test",
            test,
            "--target",
            "x86_64-unknown-linux-gnu",
            "--features",
            "host",
        ])) {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

// ============================================================================
// Helpers
// ============================================================================

fn workspace_root() -> PathBuf {
    // The xtask binary lives at <root>/xtask/; go up one level.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be a subdirectory of the workspace root")
        .to_owned()
}

/// Prepends `dir` to the `PATH` environment variable for `cmd`.
///
/// When `dir` is empty (the linker was already found on PATH), this is a no-op.
fn prepend_path(cmd: &mut Command, dir: &Path) {
    if dir.as_os_str().is_empty() {
        return;
    }
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = std::env::split_paths(&current).collect::<Vec<_>>();
    paths.insert(0, dir.to_owned());
    cmd.env(
        "PATH",
        std::env::join_paths(paths).expect("PATH join failed"),
    );
}

fn run(cmd: &mut Command) -> bool {
    let display = format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("    $ {}", display.dimmed());

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run command: {e}"));
    if !status.success() {
        eprintln!("{}", format!("    FAILED: {display}").red().bold());
    }
    status.success()
}

fn run_expect_failure(cmd: &mut Command) -> bool {
    let display = format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("    $ {}", display.dimmed());

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run command: {e}"));
    if status.success() {
        eprintln!(
            "{}",
            format!("    UNEXPECTED SUCCESS: {display}").red().bold()
        );
        true
    } else {
        println!("{}", "    expected failure (pass)".green());
        false
    }
}

struct GeneratedDocStubExpectation {
    relative_path: &'static str,
    required_fragments: &'static [&'static str],
}

fn check_generated_doc_stubs(workspace_root: &Path) -> Result<(), String> {
    let generated_doc_stub_expectations = [GeneratedDocStubExpectation {
        relative_path: "src/ir/ir_generated.rs",
        required_fragments: &[
            "pub struct IrGenerated",
            "impl Ir for IrGenerated",
            "pub struct IrMappingGenerated",
            "impl IrMapping<RemoteKeysGenerated> for IrMappingGenerated",
            "pub struct IrKeplerGenerated",
            "impl IrKepler for IrKeplerGenerated",
            "pub fn new(",
        ],
    }];

    let mut failure_messages = Vec::new();
    for generated_doc_stub_expectation in generated_doc_stub_expectations {
        let generated_doc_stub_path =
            workspace_root.join(generated_doc_stub_expectation.relative_path);
        let generated_doc_stub_source = match fs::read_to_string(&generated_doc_stub_path) {
            Ok(source) => source,
            Err(read_error) => {
                failure_messages.push(format!(
                    "{}: failed to read ({})",
                    generated_doc_stub_path.display(),
                    read_error
                ));
                continue;
            }
        };

        for required_fragment in generated_doc_stub_expectation.required_fragments {
            if !generated_doc_stub_source.contains(required_fragment) {
                failure_messages.push(format!(
                    "{}: missing required fragment `{}`",
                    generated_doc_stub_path.display(),
                    required_fragment
                ));
            }
        }
    }

    if failure_messages.is_empty() {
        Ok(())
    } else {
        Err(failure_messages.join("\n"))
    }
}
