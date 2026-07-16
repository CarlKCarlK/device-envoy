//! Build automation tasks for the device-envoy project.
//!
//! Run with: `cargo xtask <command>`

mod adpcm_clip_generated;
mod audio_player_generated;
mod ir_generated;
mod lcd_text_generated;
mod led2d_generated;
mod led_generated;
mod led_strip_generated;
mod pcm_clip_generated;
mod servo_player_generated;
mod video_frames_gen;

use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use tar::Archive;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for device-envoy project", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all checks: build lib, examples, run tests, generate docs
    CheckAll,
    /// Run compile-only validation (no test execution)
    CheckQuick,
    /// Check compile-only tests (tests-compile-only)
    CheckCompileOnly,
    /// Check all examples (pico1 + pico2, with and without wifi)
    CheckExamples,
    /// Check all demos (pico1 + pico2, with and without wifi)
    CheckDemos,
    /// Check documentation: run doc tests and generate docs
    CheckDocs,
    /// Verify README Rust example extraction + compile
    CheckReadmeExample,
    /// Generate video frames from PNG files (santa video)
    VideoFramesGen,
    /// Generate cat video frames from video file
    CatFramesGen,
    /// Generate hand video frames from video file
    HandFramesGen,
    /// Generate clock video frames from video file
    ClockFramesGen,
    /// Build library with specified features
    Build {
        #[arg(long, default_value = "pico1")]
        board: Board,
        #[arg(long, default_value = "arm")]
        arch: Arch,
        #[arg(long)]
        wifi: bool,
    },
    /// Build an example
    Example {
        /// Example name (e.g., blinky, clock_lcd)
        name: String,
        #[arg(long, default_value = "pico1")]
        board: Board,
        #[arg(long, default_value = "arm")]
        arch: Arch,
        #[arg(long)]
        wifi: bool,
    },
    /// Build UF2 firmware file for flashing to Pico
    Uf2 {
        /// Example name (e.g., blinky, clock_lcd)
        name: String,
        #[arg(long, default_value = "pico1")]
        board: Board,
        #[arg(long, default_value = "arm")]
        arch: Arch,
        #[arg(long)]
        wifi: bool,
    },
    /// Run all checks for a single board (fast developer iteration)
    CheckBoard {
        /// Board to build and check
        #[arg(long, default_value = "pico2")]
        board: Board,
        /// Architecture
        #[arg(long, default_value = "arm")]
        arch: Arch,
        /// Include Wi-Fi examples
        #[arg(long)]
        wifi: bool,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Board {
    Pico1,
    Pico2,
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Board::Pico1 => write!(f, "pico1"),
            Board::Pico2 => write!(f, "pico2"),
        }
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Arch {
    Arm,
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::Arm => write!(f, "arm"),
        }
    }
}

impl Arch {
    fn target(&self, board: Board) -> &'static str {
        match (board, self) {
            (Board::Pico1, Arch::Arm) => "thumbv6m-none-eabi",
            (Board::Pico2, Arch::Arm) => "thumbv8m.main-none-eabihf",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Capability {
    Wifi,
}

#[derive(Debug, Clone, Copy)]
struct CapabilitySet {
    bits: u16,
}

impl Capability {
    const fn bit(self) -> u16 {
        match self {
            Capability::Wifi => 1 << 0,
        }
    }

    const fn feature_name(self) -> &'static str {
        match self {
            Capability::Wifi => "wifi",
        }
    }
}

impl CapabilitySet {
    const fn empty() -> Self {
        Self { bits: 0 }
    }

    fn with_capabilities(capabilities: &[Capability]) -> Self {
        let mut capability_set = Self::empty();
        for capability in capabilities {
            capability_set.insert(*capability);
        }
        capability_set
    }

    fn insert(&mut self, capability: Capability) {
        self.bits |= capability.bit();
    }

    const fn contains(self, capability: Capability) -> bool {
        (self.bits & capability.bit()) != 0
    }
}

fn capability_set_from_wifi_enabled(wifi_enabled: bool) -> CapabilitySet {
    if wifi_enabled {
        CapabilitySet::with_capabilities(&[Capability::Wifi])
    } else {
        CapabilitySet::empty()
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::CheckAll => check_all(),
        Commands::CheckQuick => check_quick(),
        Commands::CheckCompileOnly => check_compile_only(),
        Commands::CheckExamples => check_examples(),
        Commands::CheckDemos => check_demos(),
        Commands::CheckDocs => check_docs(),
        Commands::CheckReadmeExample => check_readme_example(),
        Commands::VideoFramesGen => {
            if let Err(e) = video_frames_gen::generate_frames() {
                eprintln!("Error generating video frames: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Commands::CatFramesGen => {
            if let Err(e) = video_frames_gen::generate_cat_frames() {
                eprintln!("Error generating cat video frames: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Commands::HandFramesGen => {
            if let Err(e) = video_frames_gen::generate_hand_frames() {
                eprintln!("Error generating hand video frames: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Commands::ClockFramesGen => {
            if let Err(e) = video_frames_gen::generate_clock_frames() {
                eprintln!("Error generating clock video frames: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Commands::Build { board, arch, wifi } => {
            let capability_set = capability_set_from_wifi_enabled(wifi);
            build_lib(board, arch, capability_set)
        }
        Commands::Example {
            name,
            board,
            arch,
            wifi,
        } => {
            let capability_set = capability_set_from_wifi_enabled(wifi);
            build_example(&name, board, arch, capability_set)
        }
        Commands::Uf2 {
            name,
            board,
            arch,
            wifi,
        } => {
            let capability_set = capability_set_from_wifi_enabled(wifi);
            build_uf2(&name, board, arch, capability_set)
        }
        Commands::CheckBoard { board, arch, wifi } => {
            let capability_set = capability_set_from_wifi_enabled(wifi);
            check_board(board, arch, capability_set)
        }
    }
}

fn check_board(board: Board, arch: Arch, caps: CapabilitySet) -> ExitCode {
    let workspace_root = workspace_root();
    let wifi = caps.contains(Capability::Wifi);
    let target = arch.target(board);
    let features = build_features(board, arch, caps);
    let features_no_wifi = build_features(board, arch, CapabilitySet::empty());

    println!(
        "{}",
        format!(
            "==> cargo check-board: {board} ({arch}{})",
            if wifi { ", wifi" } else { "" }
        )
        .cyan()
        .bold()
    );

    if let Err(err) = regenerate_generated_sources(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }

    // Library build (no wifi — validates the non-wifi feature set)
    if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "build",
        "--lib",
        "--target",
        target,
        "--features",
        features_no_wifi.as_str(),
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    let examples = discover_examples(&workspace_root);
    let failures = Mutex::new(Vec::new());

    // No-wifi examples (parallel)
    let no_wifi_examples: Vec<_> = examples
        .iter()
        .filter(|e| !e.required_capabilities.contains(Capability::Wifi))
        .collect();
    no_wifi_examples.par_iter().for_each(|example| {
        if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
            "build",
            "--example",
            &example.name,
            "--target",
            target,
            "--features",
            features_no_wifi.as_str(),
            "--no-default-features",
        ])) {
            failures
                .lock()
                .unwrap()
                .push(format!("example {}", example.name));
        }
    });

    // All examples with wifi (parallel, only when wifi flag is set)
    if wifi {
        examples.par_iter().for_each(|example| {
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "build",
                "--example",
                &example.name,
                "--target",
                target,
                "--features",
                features.as_str(),
                "--no-default-features",
            ])) {
                failures
                    .lock()
                    .unwrap()
                    .push(format!("example (wifi) {}", example.name));
            }
        });
    }

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        eprintln!("\n{}", "Failed checks:".red().bold());
        for failure in failures.iter() {
            eprintln!("  - {}", failure.red());
        }
        return ExitCode::FAILURE;
    }

    println!(
        "\n{}",
        format!("==> check-board {board} passed! \u{1F389}")
            .green()
            .bold()
    );
    ExitCode::SUCCESS
}

fn check_quick() -> ExitCode {
    let workspace_root = workspace_root();
    if let Err(err) = regenerate_generated_sources(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }

    let arch = Arch::Arm;
    let board_pico2 = Board::Pico2;
    let target_pico2 = arch.target(board_pico2);
    let no_capabilities = CapabilitySet::empty();
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features_no_wifi = build_features(board_pico2, arch, no_capabilities);
    let features_wifi_pico2 = build_features(board_pico2, arch, wifi_capabilities);

    println!(
        "{}",
        "==> Running quick compile checks (no test execution)...".cyan()
    );

    if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "build",
        "--lib",
        "--target",
        target_pico2,
        "--features",
        features_no_wifi.as_str(),
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    if check_examples() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }
    if check_demos() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }
    if check_compile_only() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "doc",
        "--target",
        target_pico2,
        "--no-deps",
        "--features",
        features_wifi_pico2.as_str(),
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }
    if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "doc",
        "--target",
        arch.target(Board::Pico1),
        "--no-deps",
        "--features",
        "embedded,wifi,doc-images",
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> Quick checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_compile_only() -> ExitCode {
    let workspace_root = workspace_root();
    let arch = Arch::Arm;
    let board = Board::Pico1;
    let target = arch.target(board);

    println!("{}", "==> Checking compile-only tests...".cyan());

    let compile_tests_dir = workspace_root.join("tests-compile-only");
    if !compile_tests_dir.exists() {
        eprintln!("{}", "No tests-compile-only directory found.".red());
        return ExitCode::FAILURE;
    }

    let mut compile_tests = Vec::new();
    if let Ok(entries) = fs::read_dir(&compile_tests_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".rs") {
                    let test_name = filename.trim_end_matches(".rs");
                    compile_tests.push(test_name.to_string());
                }
            }
        }
    }
    compile_tests.sort();

    let failures = Mutex::new(Vec::new());
    compile_tests.par_iter().for_each(|test| {
        let is_should_fail_test = test.ends_with("_should_fail");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&workspace_root).args([
            "check",
            "-p",
            "device-envoy-compile-only",
            "--bin",
            test,
            "--target",
            target,
            "--features",
            "pico1,arm,wifi",
            "--no-default-features",
        ]);
        let passed = if is_should_fail_test {
            run_command_quiet(&mut cmd)
        } else {
            run_command(&mut cmd)
        };
        if (is_should_fail_test && passed) || (!is_should_fail_test && !passed) {
            failures.lock().unwrap().push(test.clone());
        }
    });

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        eprintln!("\n{}", "Failed compile-only tests:".red().bold());
        for failure in failures.iter() {
            eprintln!("  - {}", failure.red());
        }
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> Compile-only tests passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_all() -> ExitCode {
    let workspace_root = workspace_root();
    if let Err(err) = check_shared_markdown_sync(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = regenerate_generated_sources(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if check_readme_example() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }
    let examples = discover_examples(&workspace_root);
    let demos = discover_demo_bins(&workspace_root);
    let no_wifi_examples: Vec<_> = examples
        .iter()
        .filter(|example| !example.required_capabilities.contains(Capability::Wifi))
        .collect();
    let arch = Arch::Arm;
    let board_pico2 = Board::Pico2;
    let board_pico1 = Board::Pico1;
    let target_pico2 = arch.target(board_pico2);
    let target_pico1 = arch.target(board_pico1);
    let no_capabilities = CapabilitySet::empty();
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features_no_wifi = build_features(board_pico2, arch, no_capabilities);
    let features_no_wifi_pico1 = build_features(board_pico1, arch, no_capabilities);
    let features_wifi_pico2 = build_features(board_pico2, arch, wifi_capabilities);
    let features_wifi_pico1 = build_features(board_pico1, arch, wifi_capabilities);

    println!("{}", "==> Running all checks in parallel...".cyan());

    let failures = Mutex::new(Vec::new());

    rayon::scope(|s| {
        // 1. Doc tests
        s.spawn(|_| {
            println!("{}", "  [1/10] Doc tests...".bright_black());
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "test",
                "--doc",
                "--target",
                target_pico2,
                "--features",
                features_wifi_pico2.as_str(),
                "--no-default-features",
            ])) {
                failures.lock().unwrap().push("doc tests");
            }
        });

        // 2. Host tests (unit + integration)
        s.spawn(|_| {
            println!(
                "{}",
                "  [2/10] Host tests (unit + integration)...".bright_black()
            );
            let host_target = host_target();
            let mut host_test_cmd = Command::new("cargo");
            host_test_cmd.current_dir(&workspace_root).args(["test"]);

            if let Some(target) = host_target {
                host_test_cmd.arg("--target").arg(target);
            }

            host_test_cmd.args([
                "--no-default-features",
                "--features",
                "defmt,host",
                "--lib",
                "--tests",
            ]);

            if !run_command(&mut host_test_cmd) {
                failures.lock().unwrap().push("host tests");
            }
        });

        // 3. Library build
        s.spawn(|_| {
            println!("{}", "  [3/10] Library build...".bright_black());
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "build",
                "--lib",
                "--target",
                target_pico2,
                "--features",
                features_no_wifi.as_str(),
                "--no-default-features",
            ])) {
                failures.lock().unwrap().push("library build");
            }
        });

        // 4. Examples (pico2, no wifi)
        s.spawn(|_| {
            println!("{}", "  [4/10] Examples (pico2, no wifi)...".bright_black());
            no_wifi_examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico2,
                    "--features",
                    features_no_wifi.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico2 no-wifi examples");
                }
            });
        });

        // 5. Demos (pico2 + pico1, with and without wifi)
        s.spawn(|_| {
            println!(
                "{}",
                "  [5/10] Demos (pico2 + pico1, with and without wifi)...".bright_black()
            );
            demos.par_iter().for_each(|demo| {
                let features_pico2 = if demo.required_capabilities.contains(Capability::Wifi) {
                    features_wifi_pico2.as_str()
                } else {
                    features_no_wifi.as_str()
                };
                let features_pico1 = if demo.required_capabilities.contains(Capability::Wifi) {
                    features_wifi_pico1.as_str()
                } else {
                    features_no_wifi_pico1.as_str()
                };
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "-p",
                    "device-envoy-demos",
                    "--bin",
                    &demo.name,
                    "--target",
                    target_pico2,
                    "--features",
                    features_pico2,
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("demos (pico2)");
                }
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "-p",
                    "device-envoy-demos",
                    "--bin",
                    &demo.name,
                    "--target",
                    target_pico1,
                    "--features",
                    features_pico1,
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("demos (pico1)");
                }
            });
        });

        // 6. Examples (pico2, with wifi)
        s.spawn(|_| {
            println!(
                "{}",
                "  [6/10] Examples (pico2, with wifi)...".bright_black()
            );
            examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico2,
                    "--features",
                    features_wifi_pico2.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico2 wifi examples");
                }
            });
        });

        // 7. Examples (pico1, with wifi)
        s.spawn(|_| {
            println!(
                "{}",
                "  [7/10] Examples (pico1, with wifi)...".bright_black()
            );
            examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico1,
                    "--features",
                    features_wifi_pico1.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico1 wifi examples");
                }
            });
        });

        // 8. Compile-only tests
        s.spawn(|_| {
            println!("{}", "  [8/10] Compile-only tests...".bright_black());
            let compile_tests_dir = workspace_root.join("tests-compile-only");
            if compile_tests_dir.exists() {
                let mut compile_tests = Vec::new();
                if let Ok(entries) = fs::read_dir(&compile_tests_dir) {
                    for entry in entries.flatten() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".rs") {
                                let test_name = filename.trim_end_matches(".rs");
                                compile_tests.push(test_name.to_string());
                            }
                        }
                    }
                }
                compile_tests.sort();
                compile_tests.par_iter().for_each(|test| {
                    let is_should_fail_test = test.ends_with("_should_fail");
                    let mut cmd = Command::new("cargo");
                    cmd.current_dir(&workspace_root).args([
                        "check",
                        "-p",
                        "device-envoy-compile-only",
                        "--bin",
                        test,
                        "--target",
                        target_pico1,
                        "--features",
                        "pico1,arm,wifi",
                        "--no-default-features",
                    ]);
                    let passed = if is_should_fail_test {
                        run_command_quiet(&mut cmd)
                    } else {
                        run_command(&mut cmd)
                    };
                    if (is_should_fail_test && passed) || (!is_should_fail_test && !passed) {
                        failures.lock().unwrap().push("compile-only tests");
                    }
                });
            }
        });

        // 9. Documentation
        s.spawn(|_| {
            println!("{}", "  [9/10] Documentation...".bright_black());
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "doc",
                "--target",
                target_pico2,
                "--no-deps",
                "--features",
                features_wifi_pico2.as_str(),
                "--no-default-features",
            ])) {
                failures.lock().unwrap().push("documentation");
            }
        });

        // 10. docs.rs documentation (embedded target)
        s.spawn(|_| {
            println!(
                "{}",
                "  [10/10] docs.rs documentation (embedded target)...".bright_black()
            );
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "doc",
                "--target",
                target_pico1,
                "--no-deps",
                "--features",
                "embedded,wifi,doc-images",
                "--no-default-features",
            ])) {
                failures
                    .lock()
                    .unwrap()
                    .push("docs.rs documentation (embedded target)");
            }
        });
    });

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        eprintln!("\n{}", "Failed checks:".red().bold());
        for failure in failures.iter() {
            eprintln!("  - {}", failure.red());
        }
        return ExitCode::FAILURE;
    }

    println!(
        "{}",
        "==> Verifying packaged embedded build (pico2)...".cyan()
    );

    let device_envoy_core_path = workspace_root.join("../device-envoy-core");
    let device_envoy_core_patch = format!(
        "patch.crates-io.device-envoy-core.path=\"{}\"",
        device_envoy_core_path.display()
    );

    let mut package_command = Command::new("cargo");
    package_command
        .current_dir(&workspace_root)
        .args(["package", "--allow-dirty", "--no-verify"]);
    package_command
        .arg("--config")
        .arg(&device_envoy_core_patch);
    if !run_command(&mut package_command) {
        return ExitCode::FAILURE;
    }

    let packaged_manifest_path = match packaged_manifest_path(&workspace_root, "device-envoy-rp") {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{}", error.red().bold());
            return ExitCode::FAILURE;
        }
    };

    let mut packaged_check_command = Command::new("cargo");
    packaged_check_command
        .current_dir(&workspace_root)
        .arg("check")
        .arg("--manifest-path")
        .arg(&packaged_manifest_path)
        .args([
            "--target",
            target_pico2,
            "--features",
            features_no_wifi.as_str(),
            "--no-default-features",
        ]);
    packaged_check_command
        .arg("--config")
        .arg(&device_envoy_core_patch);
    if !run_command(&mut packaged_check_command) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> All checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_readme_example() -> ExitCode {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the crate root")
        .to_path_buf();
    let readme_path = crate_root.join("README.md");
    println!("{}", "==> Checking README example...".cyan());

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
    let generated_example_path = crate_root.join("examples/__readme_example_generated.rs");
    let generated_example_cleanup = TemporaryFileCleanup::new(generated_example_path.clone());
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

    let arch = Arch::Arm;
    let board = Board::Pico2;
    let target = arch.target(board);
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features = build_features(board, arch, wifi_capabilities);

    if !run_command(Command::new("cargo").current_dir(&crate_root).args([
        "check",
        "--example",
        "__readme_example_generated",
        "--target",
        target,
        "--features",
        features.as_str(),
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    core::hint::black_box(&generated_example_cleanup);

    println!("{}", "README example compile check passed.".green());
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

fn check_docs() -> ExitCode {
    let workspace_root = workspace_root();
    if let Err(err) = check_shared_markdown_sync(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = regenerate_generated_sources(&workspace_root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    let arch = Arch::Arm;
    let board = Board::Pico2;
    let target = arch.target(board);
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features = build_features(board, arch, wifi_capabilities);

    println!("{}", "==> Checking documentation...".cyan());

    let failures = Mutex::new(Vec::new());

    rayon::scope(|s| {
        // 1. Doc tests
        s.spawn(|_| {
            println!("{}", "  [1/3] Doc tests...".bright_black());
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "test",
                "--doc",
                "--target",
                target,
                "--features",
                features.as_str(),
                "--no-default-features",
            ])) {
                failures.lock().unwrap().push("doc tests");
            }
        });

        // 2. Documentation generation
        s.spawn(|_| {
            println!("{}", "  [2/3] Documentation generation...".bright_black());
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "doc",
                "--target",
                target,
                "--no-deps",
                "--features",
                features.as_str(),
                "--no-default-features",
            ])) {
                failures.lock().unwrap().push("documentation");
            }
        });

        // 3. docs.rs documentation (embedded target)
        s.spawn(|_| {
            println!(
                "{}",
                "  [3/3] docs.rs documentation (embedded target)...".bright_black()
            );
            if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                "doc",
                "--target",
                arch.target(Board::Pico1),
                "--no-deps",
                "--features",
                "embedded,wifi,doc-images",
                "--no-default-features",
            ])) {
                failures
                    .lock()
                    .unwrap()
                    .push("docs.rs documentation (embedded target)");
            }
        });
    });

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        eprintln!("\n{}", "Failed checks:".red().bold());
        for failure in failures.iter() {
            eprintln!("  - {}", failure.red());
        }
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> Documentation checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_examples() -> ExitCode {
    let workspace_root = workspace_root();
    let examples = discover_examples(&workspace_root);
    let no_wifi_examples: Vec<_> = examples
        .iter()
        .filter(|example| !example.required_capabilities.contains(Capability::Wifi))
        .collect();
    let arch = Arch::Arm;
    let board_pico2 = Board::Pico2;
    let board_pico1 = Board::Pico1;
    let target_pico2 = arch.target(board_pico2);
    let target_pico1 = arch.target(board_pico1);
    let no_capabilities = CapabilitySet::empty();
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features_no_wifi = build_features(board_pico2, arch, no_capabilities);
    let features_wifi_pico2 = build_features(board_pico2, arch, wifi_capabilities);
    let features_wifi_pico1 = build_features(board_pico1, arch, wifi_capabilities);

    println!("{}", "==> Checking all examples in parallel...".cyan());

    let failures = Mutex::new(Vec::new());

    rayon::scope(|s| {
        // 1. Examples (pico2, no wifi)
        s.spawn(|_| {
            println!("{}", "  [1/3] Examples (pico2, no wifi)...".bright_black());
            no_wifi_examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico2,
                    "--features",
                    features_no_wifi.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico2 no-wifi examples");
                }
            });
        });

        // 2. Examples (pico2, with wifi)
        s.spawn(|_| {
            println!(
                "{}",
                "  [2/3] Examples (pico2, with wifi)...".bright_black()
            );
            examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico2,
                    "--features",
                    features_wifi_pico2.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico2 wifi examples");
                }
            });
        });

        // 3. Examples (pico1, with wifi)
        s.spawn(|_| {
            println!(
                "{}",
                "  [3/3] Examples (pico1, with wifi)...".bright_black()
            );
            examples.par_iter().for_each(|example| {
                if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
                    "build",
                    "--example",
                    &example.name,
                    "--target",
                    target_pico1,
                    "--features",
                    features_wifi_pico1.as_str(),
                    "--no-default-features",
                ])) {
                    failures.lock().unwrap().push("pico1 wifi examples");
                }
            });
        });
    });

    let failures = failures.lock().unwrap();
    if !failures.is_empty() {
        eprintln!("\n{}", "Failed checks:".red().bold());
        for failure in failures.iter() {
            eprintln!("  - {}", failure.red());
        }
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> All examples passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_demos() -> ExitCode {
    let workspace_root = workspace_root();
    let demos = discover_demo_bins(&workspace_root);
    if demos.is_empty() {
        println!("{}", "No demos found.".yellow());
        return ExitCode::SUCCESS;
    }

    let arch = Arch::Arm;
    let board_pico2 = Board::Pico2;
    let board_pico1 = Board::Pico1;
    let target_pico2 = arch.target(board_pico2);
    let target_pico1 = arch.target(board_pico1);
    let no_capabilities = CapabilitySet::empty();
    let wifi_capabilities = CapabilitySet::with_capabilities(&[Capability::Wifi]);
    let features_no_wifi_pico2 = build_features(board_pico2, arch, no_capabilities);
    let features_no_wifi_pico1 = build_features(board_pico1, arch, no_capabilities);
    let features_wifi_pico2 = build_features(board_pico2, arch, wifi_capabilities);
    let features_wifi_pico1 = build_features(board_pico1, arch, wifi_capabilities);

    println!("{}", "==> Checking demos...".cyan());

    for demo in &demos {
        let features_pico2 = if demo.required_capabilities.contains(Capability::Wifi) {
            features_wifi_pico2.as_str()
        } else {
            features_no_wifi_pico2.as_str()
        };
        let features_pico1 = if demo.required_capabilities.contains(Capability::Wifi) {
            features_wifi_pico1.as_str()
        } else {
            features_no_wifi_pico1.as_str()
        };
        if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
            "build",
            "-p",
            "device-envoy-demos",
            "--bin",
            &demo.name,
            "--target",
            target_pico2,
            "--features",
            features_pico2,
            "--no-default-features",
        ])) {
            return ExitCode::FAILURE;
        }
        if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
            "build",
            "-p",
            "device-envoy-demos",
            "--bin",
            &demo.name,
            "--target",
            target_pico1,
            "--features",
            features_pico1,
            "--no-default-features",
        ])) {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn build_lib(board: Board, arch: Arch, capability_set: CapabilitySet) -> ExitCode {
    let workspace_root = workspace_root();
    let target = arch.target(board);
    let features = build_features(board, arch, capability_set);
    println!(
        "{}",
        format!("Building library with features: {features}").cyan()
    );

    if run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "build",
        "--lib",
        "--target",
        target,
        "--features",
        &features,
        "--no-default-features",
    ])) {
        println!("{}", "Build successful! ✨".green());
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn build_example(name: &str, board: Board, arch: Arch, capability_set: CapabilitySet) -> ExitCode {
    let workspace_root = workspace_root();
    let target = arch.target(board);
    let features = build_features(board, arch, capability_set);
    println!(
        "{}",
        format!("Building example '{name}' with features: {features}").cyan()
    );

    if run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "build",
        "--example",
        name,
        "--target",
        target,
        "--features",
        &features,
        "--no-default-features",
    ])) {
        println!("{}", "Build successful! ✨".green());
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn build_uf2(name: &str, board: Board, arch: Arch, capability_set: CapabilitySet) -> ExitCode {
    let workspace_root = workspace_root();
    let target = arch.target(board);
    let features = build_features(board, arch, capability_set);

    println!(
        "{}",
        format!("Building UF2 for example '{name}' ({board}/{arch})").cyan()
    );
    println!("  Features: {}", features.bright_black());
    println!("  Target: {}", target.bright_black());

    // Build in release mode for UF2
    if !run_command(Command::new("cargo").current_dir(&workspace_root).args([
        "build",
        "--example",
        name,
        "--release",
        "--target",
        target,
        "--features",
        &features,
        "--no-default-features",
    ])) {
        return ExitCode::FAILURE;
    }

    // Convert to UF2 using elf2uf2-rs
    let elf_path = format!("target/{target}/release/examples/{name}");
    let uf2_path = format!("{name}.uf2");

    println!("\n{}", "Converting to UF2 format...".cyan());

    if run_command(
        Command::new("elf2uf2-rs")
            .current_dir(&workspace_root)
            .args([&elf_path, &uf2_path]),
    ) {
        println!("{}", format!("UF2 created: {uf2_path} 🚀").green().bold());
        println!("{}", "Ready to drag-and-drop to your Pico!".bright_black());
        ExitCode::SUCCESS
    } else {
        println!(
            "{}",
            "Note: Install elf2uf2-rs with: cargo install elf2uf2-rs".yellow()
        );
        ExitCode::FAILURE
    }
}

#[derive(Debug, Clone)]
struct ExampleInfo {
    name: String,
    required_capabilities: CapabilitySet,
}

fn discover_examples(workspace_root: &Path) -> Vec<ExampleInfo> {
    let examples_dir = workspace_root.join("examples");
    let mut examples = Vec::new();
    for entry in fs::read_dir(&examples_dir).expect("Failed to read examples directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("Example file must have valid UTF-8 name")
            .to_string();
        let source = fs::read_to_string(&path).expect("Failed to read example source");
        if source.contains("check-all: skip") {
            println!(
                "{}",
                format!("Skipping example '{}' (opt-out)", name).bright_black()
            );
            continue;
        }
        let required_capabilities = if source.contains("#![cfg(feature = \"wifi\")]") {
            CapabilitySet::with_capabilities(&[Capability::Wifi])
        } else {
            CapabilitySet::empty()
        };
        examples.push(ExampleInfo {
            name,
            required_capabilities,
        });
    }
    examples.sort_by(|a, b| a.name.cmp(&b.name));
    examples
}

#[derive(Debug, Clone)]
struct DemoInfo {
    name: String,
    required_capabilities: CapabilitySet,
}

fn discover_demo_bins(workspace_root: &Path) -> Vec<DemoInfo> {
    let cargo_toml = workspace_root.join("demos/Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");
    let mut demos = Vec::new();

    let mut in_bin = false;
    let mut current_name: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut current_requires_wifi = false;

    let finalize = |current_name: &mut Option<String>,
                    current_path: &mut Option<String>,
                    current_requires_wifi: &mut bool,
                    demos: &mut Vec<DemoInfo>| {
        if let (Some(name), Some(path)) = (current_name.take(), current_path.take()) {
            let demo_path = workspace_root.join("demos").join(&path);
            let mut requires_wifi = *current_requires_wifi;
            if !requires_wifi {
                let source = fs::read_to_string(&demo_path)
                    .unwrap_or_else(|_| panic!("Failed to read {}", demo_path.display()));
                requires_wifi = source.contains("#![cfg(feature = \"wifi\")]");
            }
            let required_capabilities = if requires_wifi {
                CapabilitySet::with_capabilities(&[Capability::Wifi])
            } else {
                CapabilitySet::empty()
            };
            demos.push(DemoInfo {
                name,
                required_capabilities,
            });
        }
        *current_requires_wifi = false;
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == "[[bin]]" {
            if in_bin {
                finalize(
                    &mut current_name,
                    &mut current_path,
                    &mut current_requires_wifi,
                    &mut demos,
                );
            }
            in_bin = true;
            continue;
        }

        if in_bin && trimmed.starts_with('[') && trimmed != "[[bin]]" {
            finalize(
                &mut current_name,
                &mut current_path,
                &mut current_requires_wifi,
                &mut demos,
            );
            in_bin = false;
            continue;
        }

        if !in_bin {
            continue;
        }

        if let Some(value) = parse_toml_string(trimmed, "name") {
            current_name = Some(value);
        } else if let Some(value) = parse_toml_string(trimmed, "path") {
            current_path = Some(value);
        } else if trimmed.starts_with("required-features") && trimmed.contains("wifi") {
            current_requires_wifi = true;
        }
    }

    if in_bin {
        finalize(
            &mut current_name,
            &mut current_path,
            &mut current_requires_wifi,
            &mut demos,
        );
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

fn build_features(board: Board, arch: Arch, capability_set: CapabilitySet) -> String {
    let mut features = vec![board.to_string(), arch.to_string()];
    if capability_set.contains(Capability::Wifi) {
        features.push(Capability::Wifi.feature_name().to_string());
    }
    features.join(",")
}

struct GeneratedDocStubExpectation {
    relative_path: &'static str,
    required_fragments: &'static [&'static str],
}

fn check_generated_doc_stubs(workspace_root: &Path) -> Result<(), String> {
    let generated_doc_stub_expectations = [
        GeneratedDocStubExpectation {
            relative_path: "src/audio_player/pcm_clip_generated.rs",
            required_fragments: &[
                "pub const SAMPLE_RATE_HZ: u32",
                "pub const PCM_SAMPLE_COUNT: usize",
                "pub const fn pcm_clip() -> PcmClipBuf<",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/audio_player/adpcm_clip_generated.rs",
            required_fragments: &[
                "pub const SAMPLE_RATE_HZ: u32",
                "pub const PCM_SAMPLE_COUNT: usize",
                "pub const ADPCM_DATA_LEN: usize",
                "pub const fn adpcm_clip() -> AdpcmClipBuf<",
                "pub const fn pcm_clip() -> PcmClipBuf<",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/audio_player/audio_player_generated.rs",
            required_fragments: &[
                "pub const SAMPLE_RATE_HZ: u32",
                "pub const MAX_CLIPS: usize",
                "pub const INITIAL_VOLUME: Volume",
                "pub const MAX_VOLUME: Volume",
                "impl AudioPlayer<VOICE_22050_HZ> for AudioPlayerGenerated",
                "const SAMPLE_RATE_HZ: u32",
                "const MAX_CLIPS: usize",
                "const INITIAL_VOLUME: Volume",
                "const MAX_VOLUME: Volume",
                "fn play<I>(&self, audio_clips: I, at_end: AtEnd)",
                "async fn wait_until_stopped(&self)",
                "fn set_volume(&self, volume: Volume)",
                "fn volume(&self) -> Volume",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led2d/led2d_generated.rs",
            required_fragments: &[
                "pub struct Led2dGenerated",
                "pub const MAX_FRAMES: usize",
                "pub const MAX_BRIGHTNESS: u8",
                "pub const FONT: Led2dFont",
                "pub const WIDTH: usize",
                "pub const HEIGHT: usize",
                "pub const LEN: usize",
                "pub const SIZE: Size",
                "pub const TOP_LEFT: Point",
                "pub const TOP_RIGHT: Point",
                "pub const BOTTOM_LEFT: Point",
                "pub const BOTTOM_RIGHT: Point",
                "pub fn new(",
                "impl Led2d<12, 4> for Led2dGenerated",
                "fn write_frame(&self, frame2d: Frame2d<12, 4>)",
                "fn animate<I>(&self, frames: I)",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/lcd_text/lcd_text_generated.rs",
            required_fragments: &[
                "pub struct LcdTextGenerated",
                "pub const WIDTH: usize",
                "pub const HEIGHT: usize",
                "pub fn new(",
                "impl crate::lcd_text::LcdText<16, 2> for LcdTextGenerated",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led_strip/led_strip_generated.rs",
            required_fragments: &[
                "pub struct LedStripGenerated",
                "pub const LEN: usize",
                "pub const MAX_FRAMES: usize",
                "pub const MAX_BRIGHTNESS: u8",
                "pub fn new(",
                "impl LedStrip<8> for LedStripGenerated",
                "fn write_frame(&self, frame: Frame1d<8>)",
                "fn animate<I>(&self, frames: I)",
            ],
        },
        GeneratedDocStubExpectation {
            relative_path: "src/led/led_generated.rs",
            required_fragments: &[
                "pub struct LedGenerated",
                "pub const MAX_STEPS: usize",
                "pub fn new(",
                "impl Led for LedGenerated",
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
            relative_path: "src/servo_player/servo_player_generated.rs",
            required_fragments: &[
                "pub const MAX_STEPS: usize",
                "impl Servo for ServoPlayerGenerated",
                "impl ServoPlayer<16> for ServoPlayerGenerated",
                "fn set_degrees(&self, degrees: u16)",
                "fn hold(&self)",
                "fn relax(&self)",
                "fn animate<I>(&self, steps: I, at_end: AtEnd)",
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
    let rp_markdown_path = workspace_root.join("src/docs/current_limiting_and_gamma.md");
    let esp_markdown_path =
        workspace_root.join("../device-envoy-esp/src/docs/current_limiting_and_gamma.md");

    let rp_markdown_source = fs::read_to_string(&rp_markdown_path)
        .map_err(|read_error| format!("{}: {}", rp_markdown_path.display(), read_error))?;
    let esp_markdown_source = fs::read_to_string(&esp_markdown_path)
        .map_err(|read_error| format!("{}: {}", esp_markdown_path.display(), read_error))?;

    let rp_markdown_source = rp_markdown_source.replace("\r\n", "\n");
    let esp_markdown_source = esp_markdown_source.replace("\r\n", "\n");

    if rp_markdown_source == esp_markdown_source {
        Ok(())
    } else {
        Err(format!(
            "Shared markdown mismatch:\n  {}\n  {}\nKeep these files identical.",
            rp_markdown_path.display(),
            esp_markdown_path.display()
        ))
    }
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

fn workspace_root() -> PathBuf {
    // The xtask binary is in target/x86_64-pc-windows-msvc/debug/ or similar
    // We need to find the workspace root (parent of xtask directory)
    std::env::current_dir().expect("Failed to get current directory")
}

fn package_version_from_cargo_toml(crate_root: &Path) -> Result<String, String> {
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let cargo_toml_source = fs::read_to_string(&cargo_toml_path)
        .map_err(|read_error| format!("{}: {}", cargo_toml_path.display(), read_error))?;

    let mut in_package_section = false;
    for cargo_toml_line in cargo_toml_source.lines() {
        let trimmed_line = cargo_toml_line.trim();
        if trimmed_line.starts_with("[") {
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

const GENERATED_RUST_FILE_PATHS: [&str; 9] = [
    "src/audio_player/pcm_clip_generated.rs",
    "src/audio_player/adpcm_clip_generated.rs",
    "src/audio_player/audio_player_generated.rs",
    "src/led2d/led2d_generated.rs",
    "src/lcd_text/lcd_text_generated.rs",
    "src/led_strip/led_strip_generated.rs",
    "src/ir/ir_generated.rs",
    "src/led/led_generated.rs",
    "src/servo_player/servo_player_generated.rs",
];

fn regenerate_generated_sources(workspace_root: &Path) -> Result<(), String> {
    pcm_clip_generated::generate_pcm_clip_generated(workspace_root)
        .map_err(|error| format!("Error generating pcm_clip_generated.rs: {error}"))?;
    adpcm_clip_generated::generate_adpcm_clip_generated(workspace_root)
        .map_err(|error| format!("Error generating adpcm_clip_generated.rs: {error}"))?;
    audio_player_generated::generate_audio_player_generated(workspace_root)
        .map_err(|error| format!("Error generating audio_player_generated.rs: {error}"))?;
    led2d_generated::generate_led2d_generated(workspace_root)
        .map_err(|error| format!("Error generating led2d_generated.rs: {error}"))?;
    lcd_text_generated::generate_lcd_text_generated(workspace_root)
        .map_err(|error| format!("Error generating lcd_text_generated.rs: {error}"))?;
    led_strip_generated::generate_led_strip_generated(workspace_root)
        .map_err(|error| format!("Error generating led_strip_generated.rs: {error}"))?;
    ir_generated::generate_ir_generated(workspace_root)
        .map_err(|error| format!("Error generating ir_generated.rs: {error}"))?;
    led_generated::generate_led_generated(workspace_root)
        .map_err(|error| format!("Error generating led_generated.rs: {error}"))?;
    servo_player_generated::generate_servo_player_generated(workspace_root)
        .map_err(|error| format!("Error generating servo_player_generated.rs: {error}"))?;
    format_generated_rust_sources(workspace_root)?;
    check_generated_doc_stubs(workspace_root)
        .map_err(|error| format!("Generated doc stub consistency check failed:\n{error}"))?;
    Ok(())
}

fn format_generated_rust_sources(workspace_root: &Path) -> Result<(), String> {
    let mut format_generated_sources_command = Command::new("cargo");
    format_generated_sources_command
        .current_dir(workspace_root)
        .arg("fmt")
        .arg("--");
    for generated_rust_file_path in GENERATED_RUST_FILE_PATHS {
        format_generated_sources_command.arg(generated_rust_file_path);
    }

    match format_generated_sources_command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!(
            "Formatting generated Rust files failed with exit code {:?}. Install rustfmt with `rustup component add rustfmt`.",
            status.code()
        )),
        Err(error) => Err(format!(
            "Failed to run `cargo fmt` for generated Rust files: {error}. Install rustfmt with `rustup component add rustfmt`."
        )),
    }
}

fn host_target() -> Option<String> {
    let output = Command::new("rustc").arg("-vV").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(host) = line.strip_prefix("host: ") {
            return Some(host.trim().to_string());
        }
    }
    None
}

fn run_command(cmd: &mut Command) -> bool {
    match cmd.status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("{}", format!("Failed to execute command: {e}").red());
            false
        }
    }
}

fn run_command_quiet(cmd: &mut Command) -> bool {
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    match cmd.status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("{}", format!("Failed to execute command: {e}").red());
            false
        }
    }
}
