//! Build automation tasks for the device-envoy-esp32 project.
//!
//! Run with: `cargo xtask <command>`

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TARGET: &str = "riscv32imac-unknown-none-elf";

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for device-envoy-esp32 project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all checks: lib + all examples + docs
    CheckAll,
    /// Check all examples
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

    println!("{}", "==> cargo check-all: device-envoy-esp32".cyan().bold());

    // Check the library itself.
    println!("{}", "--> check lib".cyan());
    if !run(Command::new("cargo").current_dir(&root).args([
        "check",
        "--lib",
        "--release",
        "--target",
        TARGET,
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    if check_examples() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    if check_host_tests() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    // Generate docs.
    println!("{}", "--> doc".cyan());
    if !run(Command::new("cargo").current_dir(&root).args([
        "doc",
        "--no-deps",
        "--release",
        "--target",
        TARGET,
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> All checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_examples() -> ExitCode {
    let root = workspace_root();
    println!("{}", "--> check examples".cyan());

    let examples_dir = root.join("examples");
    let mut examples: Vec<String> = std::fs::read_dir(&examples_dir)
        .expect("examples/ directory not found")
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.ends_with(".rs").then(|| name.trim_end_matches(".rs").to_owned())
        })
        .collect();
    examples.sort();

    for example in &examples {
        println!("    check example: {example}");
        if !run(Command::new("cargo").current_dir(&root).args([
            "check",
            "--example",
            example,
            "--release",
            "--target",
            TARGET,
            "--no-default-features",
        ])) {
            return ExitCode::FAILURE;
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
            name.ends_with(".rs").then(|| name.trim_end_matches(".rs").to_owned())
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

    let status = cmd.status().unwrap_or_else(|e| panic!("failed to run command: {e}"));
    if !status.success() {
        eprintln!("{}", format!("    FAILED: {display}").red().bold());
    }
    status.success()
}
