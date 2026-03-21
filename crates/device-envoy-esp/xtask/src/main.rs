//! Build automation tasks for the device-envoy-esp project.
//!
//! Run with: `cargo xtask <command>`

mod audio_player_generated;
mod ir_generated;
mod led2d_generated;
mod led_generated;
mod led_strip_generated;
mod servo_player_generated;

use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use owo_colors::OwoColorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use tar::Archive;

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
    /// Check documentation workflows and build docs
    CheckDocs,
    /// Build all examples (catches linker errors)
    CheckExamples,
    /// Build all demos (catches linker errors)
    CheckDemos,
    /// Verify README Rust example extraction + compile
    CheckReadmeExample,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::CheckAll => check_all(),
        Commands::CheckDocs => check_docs(),
        Commands::CheckExamples => check_examples(),
        Commands::CheckDemos => check_demos(),
        Commands::CheckReadmeExample => check_readme_example(),
    }
}

fn check_docs() -> ExitCode {
    let root = workspace_root();
    println!("{}", "==> cargo check-docs: device-envoy-esp".cyan().bold());

    if let Err(err) = check_shared_markdown_sync(&root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = ir_generated::generate_ir_generated(&root) {
        eprintln!("Error generating ir_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = audio_player_generated::generate_audio_player_generated(&root) {
        eprintln!("Error generating audio_player_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led2d_generated::generate_led2d_generated(&root) {
        eprintln!("Error generating led2d_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led_generated::generate_led_generated(&root) {
        eprintln!("Error generating led_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led_strip_generated::generate_led_strip_generated(&root) {
        eprintln!("Error generating led_strip_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = servo_player_generated::generate_servo_player_generated(&root) {
        eprintln!("Error generating servo_player_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = check_generated_doc_stubs(&root) {
        eprintln!("Generated doc stub consistency check failed:\n{err}");
        return ExitCode::FAILURE;
    }
    if check_readme_example() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    println!("{}", "--> doc".cyan());
    if !run(Command::new("cargo").current_dir(&root).args([
        "doc",
        "--no-deps",
        "--release",
        "--target",
        TARGET_C6,
        "--features",
        "doc-images",
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> Docs check passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_all() -> ExitCode {
    let root = workspace_root();

    println!("{}", "==> cargo check-all: device-envoy-esp".cyan().bold());

    if let Err(err) = check_shared_markdown_sync(&root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = ir_generated::generate_ir_generated(&root) {
        eprintln!("Error generating ir_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = audio_player_generated::generate_audio_player_generated(&root) {
        eprintln!("Error generating audio_player_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led2d_generated::generate_led2d_generated(&root) {
        eprintln!("Error generating led2d_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led_generated::generate_led_generated(&root) {
        eprintln!("Error generating led_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = led_strip_generated::generate_led_strip_generated(&root) {
        eprintln!("Error generating led_strip_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = servo_player_generated::generate_servo_player_generated(&root) {
        eprintln!("Error generating servo_player_generated.rs: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = check_generated_doc_stubs(&root) {
        eprintln!("Generated doc stub consistency check failed:\n{err}");
        return ExitCode::FAILURE;
    }
    if check_readme_example() != ExitCode::SUCCESS {
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

    if check_demos() != ExitCode::SUCCESS {
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
        "--features",
        "doc-images",
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    println!("{}", "--> packaged embedded verify (c6)".cyan());
    let device_envoy_core_path = root.join("../device-envoy-core");
    let device_envoy_core_patch = format!(
        "patch.crates-io.device-envoy-core.path=\"{}\"",
        device_envoy_core_path.display()
    );
    let mut package_command = Command::new("cargo");
    package_command
        .current_dir(&root)
        .args(["package", "--allow-dirty", "--no-verify"]);
    package_command
        .arg("--config")
        .arg(&device_envoy_core_patch);
    if !run(&mut package_command) {
        return ExitCode::FAILURE;
    }

    let packaged_manifest_path = match packaged_manifest_path(&root, "device-envoy-esp") {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{}", error.red().bold());
            return ExitCode::FAILURE;
        }
    };

    let mut packaged_check_command = Command::new("cargo");
    packaged_check_command
        .current_dir(&root)
        .arg("check")
        .arg("--release")
        .arg("--manifest-path")
        .arg(&packaged_manifest_path)
        .args(["--target", TARGET_C6, "--no-default-features"])
        .arg("--config")
        .arg(&device_envoy_core_patch);
    if !run(&mut packaged_check_command) {
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

#[derive(Debug, Clone)]
struct DemoInfo {
    name: String,
}

fn check_demos() -> ExitCode {
    let root = workspace_root();
    let demos = discover_demo_bins(&root);
    if demos.is_empty() {
        println!("{}", "No demos found.".yellow());
        return ExitCode::SUCCESS;
    }

    let Some(s3_linker_dir) = require_s3_toolchain() else {
        return ExitCode::FAILURE;
    };
    // (label, target, toolchain_override, build_std)
    let targets: &[(&str, &str, Option<&str>, bool)] = &[
        ("c6", TARGET_C6, None, false),
        ("s3", TARGET_S3, Some("+esp"), true),
    ];
    for (label, target, toolchain, build_std) in targets {
        println!("{}", format!("--> build demos ({label})").cyan());
        for demo in &demos {
            println!("    build demo: {}", demo.name);
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
                "--package",
                "device-envoy-esp-demos",
                "--bin",
                &demo.name,
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
        "led_five_compile",
        "led_strip_two_strips_compile",
        "led_strip_spi_two_strips_compile",
        "led2d_two_panels_compile",
        "led4_two_displays_compile",
        "lcd_text_four_addresses_compile",
        "clock_sync_two_compile",
        "button_five_compile",
        "button_watch_five_compile",
        "rfid_one_compile",
        "ir_four_receivers_compile",
        "ir_mapping_four_receivers_compile",
        "ir_kepler_four_receivers_compile",
        "ir_visibility_compile",
    ];
    let compile_fail_tests = [
        "ir_duplicate_channel_compile_fail",
        "ir_colon_form_compile_fail",
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

fn check_readme_example() -> ExitCode {
    let root = workspace_root();
    let readme_path = root.join("README.md");
    println!("{}", "--> README example compile check".cyan());

    let readme_source = match fs::read_to_string(&readme_path) {
        Ok(readme_source) => readme_source,
        Err(readme_read_error) => {
            eprintln!(
                "{}",
                format!(
                    "error: failed to read {} ({readme_read_error})",
                    readme_path.display()
                )
                .red()
                .bold()
            );
            return ExitCode::FAILURE;
        }
    };

    let extracted_example = match extract_single_rust_example(&readme_source, &readme_path) {
        Ok(extracted_example) => extracted_example,
        Err(extract_error) => {
            eprintln!("{}", extract_error.red().bold());
            return ExitCode::FAILURE;
        }
    };

    let generated_example_source = format!(
        "// @generated by xtask check-readme-example. Do not edit.\n#![allow(dead_code, missing_docs)]\n\n{}",
        extracted_example
    );
    let generated_example_path = root.join("examples/__readme_example_generated.rs");
    let _generated_example_cleanup = TemporaryFileCleanup::new(generated_example_path.clone());
    if let Err(write_error) = fs::write(&generated_example_path, generated_example_source) {
        eprintln!(
            "{}",
            format!(
                "error: failed to write {} ({write_error})",
                generated_example_path.display()
            )
            .red()
            .bold()
        );
        return ExitCode::FAILURE;
    }

    let Some(s3_linker_dir) = require_s3_toolchain() else {
        return ExitCode::FAILURE;
    };
    let targets: &[(&str, &str, Option<&str>, bool)] = &[
        ("c6", TARGET_C6, None, false),
        ("s3", TARGET_S3, Some("+esp"), true),
    ];
    for (label, target, toolchain, build_std) in targets {
        println!("{}", format!("    README example target: {label}").cyan());
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        if *build_std {
            prepend_path(&mut cmd, &s3_linker_dir);
        }
        if let Some(tc) = toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            "check",
            "--release",
            "--example",
            "__readme_example_generated",
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

    ExitCode::SUCCESS
}

fn extract_single_rust_example(readme_source: &str, readme_path: &Path) -> Result<String, String> {
    let mut extracted_blocks = Vec::new();
    let mut in_rust_block = false;
    let mut current_block_lines = Vec::new();

    for readme_line in readme_source.lines() {
        let trimmed_line = readme_line.trim();
        if !in_rust_block && trimmed_line.starts_with("```") {
            let fence_info = trimmed_line.trim_start_matches("```").trim();
            if fence_info.starts_with("rust") {
                in_rust_block = true;
                current_block_lines.clear();
            }
            continue;
        }

        if in_rust_block && trimmed_line == "```" {
            extracted_blocks.push(current_block_lines.join("\n"));
            in_rust_block = false;
            current_block_lines.clear();
            continue;
        }

        if in_rust_block {
            current_block_lines.push(readme_line.to_string());
        }
    }

    if extracted_blocks.len() != 1 {
        return Err(format!(
            "error: {} must contain exactly one Rust fenced example block; found {}",
            readme_path.display(),
            extracted_blocks.len()
        ));
    }

    let extracted_block = extracted_blocks.remove(0);
    let mut normalized_lines = Vec::new();
    for extracted_line in extracted_block.lines() {
        if let Some(unhidden_line) = extracted_line.strip_prefix("# ") {
            normalized_lines.push(unhidden_line.to_string());
            continue;
        }
        if extracted_line.trim() == "#" {
            normalized_lines.push(String::new());
            continue;
        }
        normalized_lines.push(extracted_line.to_string());
    }
    Ok(normalized_lines.join("\n"))
}

struct TemporaryFileCleanup {
    file_path: PathBuf,
}

impl TemporaryFileCleanup {
    fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }
}

impl Drop for TemporaryFileCleanup {
    fn drop(&mut self) {
        if let Err(remove_error) = fs::remove_file(&self.file_path) {
            if remove_error.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "{}",
                    format!(
                        "warning: failed to remove temporary file {} ({remove_error})",
                        self.file_path.display()
                    )
                    .yellow()
                );
            }
        }
    }
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

fn discover_demo_bins(workspace_root: &Path) -> Vec<DemoInfo> {
    let cargo_toml = workspace_root.join("demos/Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml).expect("Failed to read demos/Cargo.toml");
    let mut demos = Vec::new();

    let mut in_bin = false;
    let mut current_name: Option<String> = None;

    let finalize = |current_name: &mut Option<String>, demos: &mut Vec<DemoInfo>| {
        if let Some(name) = current_name.take() {
            demos.push(DemoInfo { name });
        }
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == "[[bin]]" {
            if in_bin {
                finalize(&mut current_name, &mut demos);
            }
            in_bin = true;
            continue;
        }

        if in_bin && trimmed.starts_with('[') && trimmed != "[[bin]]" {
            finalize(&mut current_name, &mut demos);
            in_bin = false;
            continue;
        }

        if !in_bin {
            continue;
        }

        if let Some(value) = parse_toml_string(trimmed, "name") {
            current_name = Some(value);
        }
    }

    if in_bin {
        finalize(&mut current_name, &mut demos);
    }

    demos.sort_by(|a, b| a.name.cmp(&b.name));
    demos
}

fn parse_toml_string(line: &str, key: &str) -> Option<String> {
    let line = line.split('#').next()?.trim();
    let prefix = format!("{key} =");
    if !line.starts_with(&prefix) {
        return None;
    }
    let value = line[prefix.len()..].trim();
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(value.to_string())
}

fn package_version_from_cargo_toml(crate_root: &Path) -> Result<String, String> {
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let cargo_toml_source = fs::read_to_string(&cargo_toml_path)
        .map_err(|read_error| format!("{}: {}", cargo_toml_path.display(), read_error))?;

    let mut in_package_section = false;
    for cargo_toml_line in cargo_toml_source.lines() {
        let trimmed_line = cargo_toml_line.trim();
        if trimmed_line.starts_with('[') {
            in_package_section = trimmed_line == "[package]";
            continue;
        }

        if !in_package_section {
            continue;
        }

        if let Some(version_fragment) = trimmed_line.strip_prefix("version") {
            let Some(equals_fragment) = version_fragment.strip_prefix(" = ") else {
                continue;
            };
            let version = equals_fragment.trim_matches('"').trim().to_string();
            if version.is_empty() {
                continue;
            }
            return Ok(version);
        }
    }

    Err(format!(
        "Failed to find [package] version in {}",
        cargo_toml_path.display()
    ))
}

fn packaged_manifest_path(crate_root: &Path, package_name: &str) -> Result<PathBuf, String> {
    let package_version = package_version_from_cargo_toml(crate_root)?;
    let package_dir_name = format!("{package_name}-{package_version}");
    let target_package_dir = crate_root.join("../../target/package");
    let package_archive_path = target_package_dir.join(format!("{package_dir_name}.crate"));

    if !package_archive_path.exists() {
        return Err(format!(
            "Packaged archive not found at {}. Run `cargo package --allow-dirty` first.",
            package_archive_path.display()
        ));
    }

    let unpack_root = target_package_dir.join("tmp-crate-unpacked");
    let unpacked_package_dir = unpack_root.join(&package_dir_name);
    if unpacked_package_dir.exists() {
        fs::remove_dir_all(&unpacked_package_dir).map_err(|error| {
            format!(
                "Failed to remove previous unpacked package at {}: {error}",
                unpacked_package_dir.display()
            )
        })?;
    }

    fs::create_dir_all(&unpack_root).map_err(|error| {
        format!(
            "Failed to create unpack directory at {}: {error}",
            unpack_root.display()
        )
    })?;

    let archive_file = fs::File::open(&package_archive_path).map_err(|error| {
        format!(
            "Failed to open packaged archive at {}: {error}",
            package_archive_path.display()
        )
    })?;
    let archive_decoder = GzDecoder::new(archive_file);
    let mut archive = Archive::new(archive_decoder);
    archive.unpack(&unpack_root).map_err(|error| {
        format!(
            "Failed to unpack packaged archive {} into {}: {error}",
            package_archive_path.display(),
            unpack_root.display()
        )
    })?;

    let target_package_manifest_path = unpacked_package_dir.join("Cargo.toml");
    if !target_package_manifest_path.exists() {
        return Err(format!(
            "Unpacked packaged manifest not found at {} after extracting {}.",
            target_package_manifest_path.display(),
            package_archive_path.display()
        ));
    }

    Ok(target_package_manifest_path)
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
    let generated_doc_stub_expectations = [
        GeneratedDocStubExpectation {
            relative_path: "src/audio_player/audio_player_generated.rs",
            required_fragments: &[
                "pub struct AudioPlayerGenerated",
                "pub type AudioPlayerGeneratedPlayable = dyn Playable<VOICE_22050_HZ>;",
                "impl AudioPlayerGenerated",
                "impl AudioPlayer<VOICE_22050_HZ> for AudioPlayerGenerated",
                "pub fn new(",
            ],
        },
        GeneratedDocStubExpectation {
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
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led2d/led2d_generated.rs",
            required_fragments: &[
                "pub struct Led2dGenerated",
                "impl Led2dGenerated",
                "impl Led2d<12, 4> for &'static Led2dGenerated",
                "pub const MAX_FRAMES: usize",
                "pub fn new(",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led/led_generated.rs",
            required_fragments: &[
                "pub struct LedGenerated",
                "impl LedGenerated",
                "pub const MAX_STEPS: usize",
                "impl Led for LedGenerated",
                "pub fn new(",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led_strip/led_strip_generated.rs",
            required_fragments: &[
                "pub struct LedStripGenerated",
                "impl LedStripGenerated",
                "impl LedStrip<8> for LedStripGenerated",
                "pub const MAX_FRAMES: usize",
                "pub fn new(",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/servo_player/servo_player_generated.rs",
            required_fragments: &[
                "pub struct ServoPlayerGenerated",
                "impl ServoPlayerGenerated",
                "impl Servo for ServoPlayerGenerated",
                "impl ServoPlayer<16> for ServoPlayerGenerated",
                "pub fn new(",
            ],
        },
    ];

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

fn check_shared_markdown_sync(workspace_root: &Path) -> Result<(), String> {
    let esp_markdown_path = workspace_root.join("src/docs/current_limiting_and_gamma.md");
    let rp_markdown_path =
        workspace_root.join("../device-envoy-rp/src/docs/current_limiting_and_gamma.md");

    let esp_markdown_source = fs::read_to_string(&esp_markdown_path)
        .map_err(|read_error| format!("{}: {}", esp_markdown_path.display(), read_error))?;
    let rp_markdown_source = fs::read_to_string(&rp_markdown_path)
        .map_err(|read_error| format!("{}: {}", rp_markdown_path.display(), read_error))?;

    let esp_markdown_source = esp_markdown_source.replace("\r\n", "\n");
    let rp_markdown_source = rp_markdown_source.replace("\r\n", "\n");

    if esp_markdown_source == rp_markdown_source {
        Ok(())
    } else {
        Err(format!(
            "Shared markdown mismatch:\n  {}\n  {}\nKeep these files identical.",
            esp_markdown_path.display(),
            rp_markdown_path.display()
        ))
    }
}
