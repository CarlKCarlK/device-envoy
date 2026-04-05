//! Build automation tasks for the device-envoy-esp project.
//!
//! Run with: `cargo xtask <command>`

mod audio_player_generated;
mod board_examples_generated;
mod ir_generated;
mod led2d_generated;
mod led_generated;
mod led_strip_generated;
mod servo_player_generated;

use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use tar::Archive;

const TARGET_RISCV32IMC: &str = "riscv32imc-unknown-none-elf";
const TARGET_RISCV32IMAC: &str = "riscv32imac-unknown-none-elf";
const TARGET_XTENSA_ESP32: &str = "xtensa-esp32-none-elf";
const TARGET_XTENSA_ESP32S2: &str = "xtensa-esp32s2-none-elf";
const TARGET_XTENSA_ESP32S3: &str = "xtensa-esp32s3-none-elf";

const CHIP_FEATURE_ESP32: &str = "esp32";
const CHIP_FEATURE_ESP32C2: &str = "esp32c2";
const CHIP_FEATURE_ESP32C3: &str = "esp32c3";
const CHIP_FEATURE_ESP32C6: &str = "esp32c6";
const CHIP_FEATURE_ESP32H2: &str = "esp32h2";
const CHIP_FEATURE_ESP32S2: &str = "esp32s2";
const CHIP_FEATURE_ESP32S3: &str = "esp32s3";

#[derive(Clone, Copy)]
struct BuildTarget {
    label: &'static str,
    target: &'static str,
    chip_feature: &'static str,
    toolchain: Option<&'static str>,
    build_std: bool,
}

// TODO000 use bools for capabilities and requirements, or bitflags if we want to get fancy. The current struct+vec approach is a bit verbose. (may no longer apply)
// TODO000 could also have a single `Capability` enum and then have Vec<Capability> for both chip capabilities and example/demo requirements, which would simplify the logic but be less explicit about which capabilities are relevant to which examples/demos. (may no longer apply)
// TOOD000 or some enum+struct approach where the enum variants are the capabilities and the struct has bools for which ones are present/required, which would be more concise but less flexible/extensible if we want to have parameters for capabilities in the future (for example, "has_audio_gpio" could become "audio_gpio_pin_count: u8" or something like that). (may no longer apply)
#[derive(Clone, Copy)]
enum Capability {
    Rmt,
    I2s,
    AudioGpio,
    ButtonGpio,
    HighGpioPins,
    Wifi,
    ExtendedGpio,
}

#[derive(Clone, Copy)]
struct CapabilitySet {
    bits: u16,
}

const ALL_CAPABILITIES: [Capability; 7] = [
    Capability::Rmt,
    Capability::I2s,
    Capability::AudioGpio,
    Capability::ButtonGpio,
    Capability::HighGpioPins,
    Capability::Wifi,
    Capability::ExtendedGpio,
];

impl Capability {
    const fn bit(self) -> u16 {
        match self {
            Capability::Rmt => 1 << 0,
            Capability::I2s => 1 << 1,
            Capability::AudioGpio => 1 << 2,
            Capability::ButtonGpio => 1 << 3,
            Capability::HighGpioPins => 1 << 4,
            Capability::Wifi => 1 << 5,
            Capability::ExtendedGpio => 1 << 6,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Capability::Rmt => "RMT",
            Capability::I2s => "I2S",
            Capability::AudioGpio => "audio GPIO mapping",
            Capability::ButtonGpio => "button GPIO mapping",
            Capability::HighGpioPins => "high GPIO pin set",
            Capability::Wifi => "Wi-Fi",
            Capability::ExtendedGpio => "extended GPIO set",
        }
    }
}

impl CapabilitySet {
    const fn empty() -> Self {
        Self { bits: 0 }
    }

    fn from_capabilities(capabilities: &[Capability]) -> Self {
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

fn chip_capabilities(chip_feature: &str) -> CapabilitySet {
    match chip_feature {
        CHIP_FEATURE_ESP32 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::AudioGpio,
            Capability::ButtonGpio,
            Capability::HighGpioPins,
            Capability::Wifi,
            Capability::ExtendedGpio,
        ]),
        CHIP_FEATURE_ESP32C2 => {
            CapabilitySet::from_capabilities(&[Capability::ButtonGpio, Capability::Wifi])
        }
        CHIP_FEATURE_ESP32C3 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::AudioGpio,
            Capability::ButtonGpio,
            Capability::HighGpioPins,
            Capability::Wifi,
        ]),
        CHIP_FEATURE_ESP32C6 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::AudioGpio,
            Capability::ButtonGpio,
            Capability::HighGpioPins,
            Capability::Wifi,
            Capability::ExtendedGpio,
        ]),
        CHIP_FEATURE_ESP32H2 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::ExtendedGpio,
        ]),
        CHIP_FEATURE_ESP32S2 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::AudioGpio,
            Capability::ButtonGpio,
            Capability::HighGpioPins,
            Capability::Wifi,
            Capability::ExtendedGpio,
        ]),
        CHIP_FEATURE_ESP32S3 => CapabilitySet::from_capabilities(&[
            Capability::Rmt,
            Capability::I2s,
            Capability::AudioGpio,
            Capability::ButtonGpio,
            Capability::HighGpioPins,
            Capability::Wifi,
            Capability::ExtendedGpio,
        ]),
        _ => panic!("unknown chip feature: {chip_feature}"),
    }
}

fn example_requirements(example: &str) -> CapabilitySet {
    let mut capability_set = CapabilitySet::empty();

    if example.starts_with("audio") {
        capability_set.insert(Capability::I2s);
        capability_set.insert(Capability::AudioGpio);
    }
    if example.starts_with("button_") {
        capability_set.insert(Capability::ButtonGpio);
    }
    let requires_high_gpio_pins = example == "conway"
        || example.starts_with("ir")
        || example.starts_with("lcd_text")
        || example.starts_with("led2d")
        || example == "led_strip_example2_trait"
        || example == "rfid"
        || example == "servos"
        || example == "servo_example1_trait"
        || example == "servo_player_example1_trait"
        || example == "servo_player_example2_trait";
    if requires_high_gpio_pins {
        capability_set.insert(Capability::HighGpioPins);
    }
    let requires_rmt = example == "blinky_smart_led"
        || example.starts_with("conway")
        || example.starts_with("ir")
        || example.starts_with("led16x16")
        || example.starts_with("led2d")
        || example.starts_with("led_strip")
        || example == "wifi_dns_hex";
    if requires_rmt {
        capability_set.insert(Capability::Rmt);
    }
    let requires_wifi = example.starts_with("wifi_") || example.starts_with("clock_");
    if requires_wifi {
        capability_set.insert(Capability::Wifi);
    }
    let requires_extended_gpio = example.starts_with("clock_")
        || example.starts_with("lcd_text")
        || example == "led2d_example1_trait";
    if requires_extended_gpio {
        capability_set.insert(Capability::ExtendedGpio);
    }

    capability_set
}

fn missing_capabilities(
    chip_capabilities: CapabilitySet,
    required_capabilities: CapabilitySet,
) -> Vec<&'static str> {
    let mut missing_capabilities = Vec::new();
    for capability in ALL_CAPABILITIES {
        if required_capabilities.contains(capability) && !chip_capabilities.contains(capability) {
            missing_capabilities.push(capability.name());
        }
    }
    missing_capabilities
}

fn explicit_example_skip_reason(chip_feature: &str, example_name: &str) -> Option<&'static str> {
    if example_name.starts_with("led_probe_c3_") && chip_feature != CHIP_FEATURE_ESP32C3 {
        return Some("C3-only GPIO probe example");
    }

    if chip_feature == CHIP_FEATURE_ESP32S2 {
        let s2_stack_limited_examples = [
            "clock_console_simple",
            "clock_lcd",
            "clock_led4",
            "clock_led8x12",
            "clock_servos",
            "clock_sync_example1_trait",
            "wifi_auto_custom_checkbox",
            "wifi_auto_example1_trait",
            "wifi_auto_force_button",
            "wifi_dns_hex",
        ];
        if s2_stack_limited_examples.contains(&example_name) {
            return Some("ESP32-S2 linker memory budget");
        }
    }

    None
}

fn demo_requirements(demo_name: &str) -> CapabilitySet {
    let mut capability_set = CapabilitySet::empty();

    let requires_wifi = demo_name.starts_with("demo_f");
    if requires_wifi {
        capability_set.insert(Capability::Wifi);
    }
    let requires_rmt = demo_name.starts_with("demo_a")
        || demo_name.starts_with("demo_b")
        || demo_name.starts_with("demo_f");
    if requires_rmt {
        capability_set.insert(Capability::Rmt);
    }
    let requires_extended_gpio = demo_name.starts_with("demo_f");
    if requires_extended_gpio {
        capability_set.insert(Capability::ExtendedGpio);
    }

    capability_set
}

fn explicit_demo_skip_reason(chip_feature: &str, demo_name: &str) -> Option<&'static str> {
    if chip_feature == CHIP_FEATURE_ESP32S2 && demo_name == "demo_f1_dns" {
        return Some("ESP32-S2 linker memory budget");
    }
    None
}

const BUILD_TARGET_ESP32: BuildTarget = BuildTarget {
    label: "esp32",
    target: TARGET_XTENSA_ESP32,
    chip_feature: CHIP_FEATURE_ESP32,
    toolchain: Some("+esp"),
    build_std: true,
};
const BUILD_TARGET_ESP32C2: BuildTarget = BuildTarget {
    label: "esp32c2",
    target: TARGET_RISCV32IMC,
    chip_feature: CHIP_FEATURE_ESP32C2,
    toolchain: None,
    build_std: false,
};
const BUILD_TARGET_ESP32C3: BuildTarget = BuildTarget {
    label: "esp32c3",
    target: TARGET_RISCV32IMC,
    chip_feature: CHIP_FEATURE_ESP32C3,
    toolchain: None,
    build_std: false,
};
const BUILD_TARGET_ESP32C6: BuildTarget = BuildTarget {
    label: "esp32c6",
    target: TARGET_RISCV32IMAC,
    chip_feature: CHIP_FEATURE_ESP32C6,
    toolchain: None,
    build_std: false,
};
const BUILD_TARGET_ESP32H2: BuildTarget = BuildTarget {
    label: "esp32h2",
    target: TARGET_RISCV32IMAC,
    chip_feature: CHIP_FEATURE_ESP32H2,
    toolchain: None,
    build_std: false,
};
const BUILD_TARGET_ESP32S2: BuildTarget = BuildTarget {
    label: "esp32s2",
    target: TARGET_XTENSA_ESP32S2,
    chip_feature: CHIP_FEATURE_ESP32S2,
    toolchain: Some("+esp"),
    build_std: true,
};
const BUILD_TARGET_ESP32S3: BuildTarget = BuildTarget {
    label: "esp32s3",
    target: TARGET_XTENSA_ESP32S3,
    chip_feature: CHIP_FEATURE_ESP32S3,
    toolchain: Some("+esp"),
    build_std: true,
};

const ALL_PROCESSOR_TARGETS: &[BuildTarget] = &[
    BUILD_TARGET_ESP32,
    BUILD_TARGET_ESP32C2,
    BUILD_TARGET_ESP32C3,
    BUILD_TARGET_ESP32C6,
    BUILD_TARGET_ESP32H2,
    BUILD_TARGET_ESP32S2,
    BUILD_TARGET_ESP32S3,
];
const CHECK_ALL_TARGETS: &[BuildTarget] = ALL_PROCESSOR_TARGETS;

/// Locates the requested Xtensa linker and returns its parent directory.
fn find_xtensa_linker_dir(linker: &str) -> Option<PathBuf> {
    // Check PATH first.
    if Command::new(linker)
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

    if let Some(dir) = find_in(&esp_toolchain, linker, 6) {
        return Some(dir);
    }

    None
}

/// Returns `Some(linker_dir)` when the full Xtensa toolchain is ready, `None` otherwise.
///
/// Requires the `esp` rustup toolchain with `rust-src` and Xtensa GCC linkers.
fn require_xtensa_toolchain() -> Option<PathBuf> {
    const XTENSA_LINKERS: &[&str] = &[
        "xtensa-esp32-elf-gcc",
        "xtensa-esp32s2-elf-gcc",
        "xtensa-esp32s3-elf-gcc",
    ];
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

    let mut linker_dirs = Vec::new();
    for linker in XTENSA_LINKERS {
        let Some(linker_dir) = find_xtensa_linker_dir(linker) else {
            eprintln!(
                "{}",
                format!(
                    "error: {linker} not found.\n\
                     Run `espup install` to install the Xtensa GCC linkers, then re-run check-all."
                )
                .red()
                .bold()
            );
            return None;
        };
        linker_dirs.push(linker_dir);
    }

    let first_linker_dir = linker_dirs[0].clone();
    if linker_dirs
        .iter()
        .any(|linker_dir| *linker_dir != first_linker_dir)
    {
        eprintln!(
            "{}",
            "error: Xtensa linker binaries were found in different directories.\n\
             Run `espup install` and ensure the Xtensa toolchain is installed correctly."
                .red()
                .bold()
        );
        return None;
    }

    Some(first_linker_dir)
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
    /// Pre-push validation: required host tests first, then full check-all
    CheckPrePush,
    /// Check documentation workflows and build docs
    CheckDocs,
    /// Build all examples (catches linker errors)
    CheckExamples,
    /// Check all examples for all supported ESP processor feature/target combinations
    CheckExamplesAllProcessors,
    /// Build all demos (catches linker errors)
    CheckDemos,
    /// Verify README Rust example extraction + compile
    CheckReadmeExample,
    /// Generate board-specific examples from templates
    #[command(alias = "generate-blinky-examples")]
    GenerateBoardExamples,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::CheckAll => check_all(),
        Commands::CheckPrePush => check_pre_push(),
        Commands::CheckDocs => check_docs(),
        Commands::CheckExamples => check_examples(),
        Commands::CheckExamplesAllProcessors => check_examples_all_processors(),
        Commands::CheckDemos => check_demos(),
        Commands::CheckReadmeExample => check_readme_example(),
        Commands::GenerateBoardExamples => generate_board_examples(),
    }
}

fn xtensa_linker_dir_if_needed(targets: &[BuildTarget]) -> Option<PathBuf> {
    if targets.iter().any(|build_target| build_target.build_std) {
        require_xtensa_toolchain()
    } else {
        Some(PathBuf::new())
    }
}

fn check_docs() -> ExitCode {
    let root = workspace_root();
    println!("{}", "==> cargo check-docs: device-envoy-esp".cyan().bold());

    if let Err(err) = check_shared_markdown_sync(&root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = board_examples_generated::generate_board_examples(&root) {
        eprintln!("Error generating board examples: {err}");
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
        TARGET_RISCV32IMAC,
        "--features",
        "doc-images,esp32c6",
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
    if let Err(err) = board_examples_generated::generate_board_examples(&root) {
        eprintln!("Error generating board examples: {err}");
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

    // Build the library itself for all supported chips.
    // Xtensa chips use -Zbuild-std because the `esp` toolchain ships no prebuilt Xtensa sysroot.
    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(CHECK_ALL_TARGETS) else {
        return ExitCode::FAILURE;
    };
    let lib_build_results: Vec<bool> = CHECK_ALL_TARGETS
        .par_iter()
        .copied()
        .map(|build_target| build_lib_for_target(&root, &xtensa_linker_dir, build_target))
        .collect();
    if lib_build_results.iter().any(|ok| !ok) {
        return ExitCode::FAILURE;
    }

    if check_examples_for_targets(ALL_PROCESSOR_TARGETS, true) != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    if check_demos_for_targets(ALL_PROCESSOR_TARGETS) != ExitCode::SUCCESS {
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
        TARGET_RISCV32IMAC,
        "--features",
        "doc-images,esp32c6",
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
        .args(["--target", TARGET_RISCV32IMAC, "--no-default-features"])
        .args(["--features", CHIP_FEATURE_ESP32C6])
        .arg("--config")
        .arg(&device_envoy_core_patch);
    if !run(&mut packaged_check_command) {
        return ExitCode::FAILURE;
    }

    println!("\n{}", "==> All checks passed! 🎉".green().bold());
    ExitCode::SUCCESS
}

fn check_s3_just_feature_flags(workspace_root: &Path) -> Result<(), String> {
    let justfile_path = workspace_root.join("justfile");
    let justfile = std::fs::read_to_string(&justfile_path)
        .map_err(|error| format!("error: failed to read {}: {error}", justfile_path.display()))?;

    let mut invalid_lines = Vec::new();
    for (line_index, line) in justfile.lines().enumerate() {
        let is_s3_recipe_line =
            line.contains("cargo +esp run") || line.contains("cargo +esp check");
        if !is_s3_recipe_line || !line.contains("--target xtensa-esp32s3-none-elf") {
            continue;
        }
        if line.contains("--no-default-features") && !line.contains("--features esp32s3") {
            invalid_lines.push(line_index + 1);
        }
    }

    if invalid_lines.is_empty() {
        return Ok(());
    }

    Err(format!(
        "error: justfile S3 commands missing `--features esp32s3` at lines: {}",
        invalid_lines
            .iter()
            .map(|line_number| line_number.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn check_pre_push() -> ExitCode {
    let root = workspace_root();
    println!(
        "{}",
        "==> cargo check-pre-push: device-envoy-esp".cyan().bold()
    );

    if let Err(err) = check_s3_just_feature_flags(&root) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }

    let required_host_tests = ["compile_fail", "wifi_auto_portal"];
    for host_test in required_host_tests {
        println!("--> required host test: {host_test}");
        if !run(Command::new("cargo").current_dir(&root).args([
            "test",
            "--test",
            host_test,
            "--target",
            "x86_64-unknown-linux-gnu",
            "--features",
            "host",
        ])) {
            return ExitCode::FAILURE;
        }
    }

    check_all()
}

fn check_examples() -> ExitCode {
    check_examples_for_targets(CHECK_ALL_TARGETS, true)
}

fn check_examples_all_processors() -> ExitCode {
    check_examples_for_targets(ALL_PROCESSOR_TARGETS, false)
}

fn check_examples_for_targets(targets: &[BuildTarget], link_examples: bool) -> ExitCode {
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
    for generated_board_example in board_examples_generated::generated_board_example_names() {
        if !examples
            .iter()
            .any(|example| example == generated_board_example)
        {
            examples.push(generated_board_example.to_string());
        }
    }
    examples.sort();

    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(targets) else {
        return ExitCode::FAILURE;
    };
    let example_results: Vec<bool> = targets
        .par_iter()
        .copied()
        .map(|build_target| {
            check_examples_for_target(
                &root,
                &xtensa_linker_dir,
                build_target,
                &examples,
                link_examples,
            )
        })
        .collect();
    if example_results.iter().any(|ok| !ok) {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[derive(Debug, Clone)]
struct DemoInfo {
    name: String,
}

fn check_demos() -> ExitCode {
    check_demos_for_targets(CHECK_ALL_TARGETS)
}

fn check_demos_for_targets(targets: &[BuildTarget]) -> ExitCode {
    let root = workspace_root();
    let demos = discover_demo_bins(&root);
    if demos.is_empty() {
        println!("{}", "No demos found.".yellow());
        return ExitCode::SUCCESS;
    }

    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(targets) else {
        return ExitCode::FAILURE;
    };
    let demo_results: Vec<bool> = targets
        .par_iter()
        .copied()
        .map(|build_target| check_demos_for_target(&root, &xtensa_linker_dir, build_target, &demos))
        .collect();
    if demo_results.iter().any(|ok| !ok) {
        return ExitCode::FAILURE;
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
    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(CHECK_ALL_TARGETS) else {
        return ExitCode::FAILURE;
    };
    let embedded_results: Vec<bool> = CHECK_ALL_TARGETS
        .par_iter()
        .copied()
        .map(|build_target| {
            check_embedded_tests_for_target(
                &root,
                &xtensa_linker_dir,
                build_target,
                &compile_pass_tests,
                &compile_fail_tests,
            )
        })
        .collect();
    if embedded_results.iter().any(|ok| !ok) {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn build_lib_for_target(root: &Path, xtensa_linker_dir: &Path, build_target: BuildTarget) -> bool {
    println!("{}", format!("--> build lib ({})", build_target.label).cyan());
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root);
    configure_target_artifact_dir(&mut cmd, root, build_target);
    if build_target.build_std {
        prepend_path(&mut cmd, xtensa_linker_dir);
    }
    if let Some(tc) = build_target.toolchain {
        cmd.arg(tc);
    }
    cmd.args([
        "build",
        "--lib",
        "--release",
        "--target",
        build_target.target,
        "--no-default-features",
        "--features",
        build_target.chip_feature,
    ]);
    if build_target.build_std {
        cmd.arg("-Zbuild-std=core,alloc");
    }
    run(&mut cmd)
}

fn check_examples_for_target(
    root: &Path,
    xtensa_linker_dir: &Path,
    build_target: BuildTarget,
    examples: &[String],
    link_examples: bool,
) -> bool {
    let chip_capabilities = chip_capabilities(build_target.chip_feature);
    let target_message = if link_examples {
        format!("--> build examples ({})", build_target.label)
    } else {
        format!("--> check examples ({})", build_target.label)
    };
    println!("{}", target_message.cyan());
    for example in examples {
        if let Some(required_chip_feature) = board_examples_generated::board_example_required_chip(example)
        {
            if required_chip_feature != build_target.chip_feature {
                println!(
                    "    skip example: {example} (board example targets {required_chip_feature}, not {})",
                    build_target.label
                );
                continue;
            }
        }
        if let Some(skip_reason) = explicit_example_skip_reason(build_target.chip_feature, example) {
            println!(
                "    skip example: {example} ({skip_reason} on {})",
                build_target.label
            );
            continue;
        }
        let missing_capabilities =
            missing_capabilities(chip_capabilities, example_requirements(example));
        if !missing_capabilities.is_empty() {
            println!(
                "    skip example: {example} ({} unavailable on {})",
                missing_capabilities.join(", "),
                build_target.label
            );
            continue;
        }
        if link_examples {
            println!("    build example: {example}");
        } else {
            println!("    check example: {example}");
        }
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        configure_target_artifact_dir(&mut cmd, root, build_target);
        if build_target.build_std {
            prepend_path(&mut cmd, xtensa_linker_dir);
        }
        if let Some(tc) = build_target.toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            if link_examples { "build" } else { "check" },
            "--example",
            example,
            "--release",
            "--target",
            build_target.target,
            "--no-default-features",
            "--features",
            build_target.chip_feature,
        ]);
        if build_target.build_std {
            cmd.arg("-Zbuild-std=core,alloc");
        }
        if !run(&mut cmd) {
            return false;
        }
    }
    true
}

fn check_demos_for_target(
    root: &Path,
    xtensa_linker_dir: &Path,
    build_target: BuildTarget,
    demos: &[DemoInfo],
) -> bool {
    let chip_capabilities = chip_capabilities(build_target.chip_feature);
    println!("{}", format!("--> build demos ({})", build_target.label).cyan());
    for demo in demos {
        if let Some(skip_reason) = explicit_demo_skip_reason(build_target.chip_feature, &demo.name) {
            println!(
                "    skip demo: {} ({skip_reason} on {})",
                demo.name, build_target.label
            );
            continue;
        }
        let missing_capabilities =
            missing_capabilities(chip_capabilities, demo_requirements(&demo.name));
        if !missing_capabilities.is_empty() {
            println!(
                "    skip demo: {} ({} unavailable on {})",
                demo.name,
                missing_capabilities.join(", "),
                build_target.label
            );
            continue;
        }
        println!("    build demo: {}", demo.name);
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        configure_target_artifact_dir(&mut cmd, root, build_target);
        if build_target.build_std {
            prepend_path(&mut cmd, xtensa_linker_dir);
        }
        if let Some(tc) = build_target.toolchain {
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
            build_target.target,
            "--no-default-features",
            "--features",
            build_target.chip_feature,
        ]);
        if build_target.build_std {
            cmd.arg("-Zbuild-std=core,alloc");
        }
        if !run(&mut cmd) {
            return false;
        }
    }
    true
}

fn check_embedded_tests_for_target(
    root: &Path,
    xtensa_linker_dir: &Path,
    build_target: BuildTarget,
    compile_pass_tests: &[&str],
    compile_fail_tests: &[&str],
) -> bool {
    // TODO00 Expand embedded compile-test pin mappings so these tests also run on ESP32, ESP32-C2, and ESP32-H2.
    if matches!(
        build_target.chip_feature,
        CHIP_FEATURE_ESP32 | CHIP_FEATURE_ESP32C2 | CHIP_FEATURE_ESP32H2
    ) {
        println!(
            "    skip embedded tests on {} (compile-only embedded test pin maps are currently maintained for ESP32-C3/ESP32-C6/ESP32-S2/ESP32-S3)",
            build_target.label
        );
        return true;
    }

    println!("{}", format!("    target: {}", build_target.label).cyan());
    for embedded_test in compile_pass_tests {
        if let Some(reason) =
            explicit_embedded_test_skip_reason(build_target.chip_feature, embedded_test)
        {
            println!(
                "      skip compile-pass test: {embedded_test} ({reason} on {})",
                build_target.label
            );
            continue;
        }
        println!("      compile-pass test: {embedded_test}");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        configure_target_artifact_dir(&mut cmd, root, build_target);
        if build_target.build_std {
            prepend_path(&mut cmd, xtensa_linker_dir);
        }
        if let Some(tc) = build_target.toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            "build",
            "--test",
            embedded_test,
            "--release",
            "--target",
            build_target.target,
            "--no-default-features",
            "--features",
            build_target.chip_feature,
        ]);
        if build_target.build_std {
            cmd.arg("-Zbuild-std=core,alloc");
        }
        if !run(&mut cmd) {
            return false;
        }
    }

    for embedded_test in compile_fail_tests {
        if let Some(reason) =
            explicit_embedded_test_skip_reason(build_target.chip_feature, embedded_test)
        {
            println!(
                "      skip compile-fail test: {embedded_test} ({reason} on {})",
                build_target.label
            );
            continue;
        }
        println!("      compile-fail test: {embedded_test}");
        let compile_fail_features = format!("{},compile-fail-tests", build_target.chip_feature);
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        // Compile-fail tests are expected to fail. Keep their output non-colored so
        // expected rustc errors do not show as alarming red blocks in the check log.
        cmd.env("CARGO_TERM_COLOR", "never");
        configure_target_artifact_dir(&mut cmd, root, build_target);
        if build_target.build_std {
            prepend_path(&mut cmd, xtensa_linker_dir);
        }
        if let Some(tc) = build_target.toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            "build",
            "--test",
            embedded_test,
            "--release",
            "--target",
            build_target.target,
            "--no-default-features",
            "--features",
            &compile_fail_features,
        ]);
        if build_target.build_std {
            cmd.arg("-Zbuild-std=core,alloc");
        }
        if run_expect_failure(&mut cmd) {
            eprintln!(
                "{}",
                format!(
                    "error: compile-fail test `{embedded_test}` unexpectedly compiled for target `{}`",
                    build_target.target
                )
                .red()
                .bold()
            );
            return false;
        }
    }

    true
}

fn explicit_embedded_test_skip_reason(
    chip_feature: &str,
    embedded_test: &str,
) -> Option<&'static str> {
    // TODO00 Add ESP32-C3 coverage for two-strip SPI compile tests once a stable second SPI/pin mapping is defined for this test target.
    if embedded_test == "led_strip_spi_two_strips_compile" && chip_feature == CHIP_FEATURE_ESP32C3 {
        return Some("two-strip SPI compile test is only mapped for ESP32-C6/ESP32-S3");
    }
    // TODO00 Add an ESP32-C3-specific two-display Led4 compile test variant with valid C3 GPIO ranges.
    if embedded_test == "led4_two_displays_compile" && chip_feature == CHIP_FEATURE_ESP32C3 {
        return Some("second Led4 display pin map requires GPIOs unavailable on ESP32-C3");
    }

    None
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

    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(CHECK_ALL_TARGETS) else {
        return ExitCode::FAILURE;
    };
    for build_target in CHECK_ALL_TARGETS {
        println!(
            "{}",
            format!("    README example target: {}", build_target.label).cyan()
        );
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&root);
        configure_target_artifact_dir(&mut cmd, &root, *build_target);
        if build_target.build_std {
            prepend_path(&mut cmd, &xtensa_linker_dir);
        }
        if let Some(tc) = build_target.toolchain {
            cmd.arg(tc);
        }
        cmd.args([
            "check",
            "--release",
            "--example",
            "__readme_example_generated",
            "--target",
            build_target.target,
            "--no-default-features",
            "--features",
            build_target.chip_feature,
        ]);
        if build_target.build_std {
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

fn generate_board_examples() -> ExitCode {
    let root = workspace_root();
    println!(
        "{}",
        "==> cargo xtask generate-board-examples: device-envoy-esp"
            .cyan()
            .bold()
    );

    if let Err(err) = board_examples_generated::generate_board_examples(&root) {
        eprintln!("Error generating board examples: {err}");
        return ExitCode::FAILURE;
    }

    println!("{}", "==> Board examples generated".green().bold());
    ExitCode::SUCCESS
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

fn target_artifact_dir(workspace_root: &Path, build_target: BuildTarget) -> PathBuf {
    workspace_root
        .join("../../target/check-all")
        .join(build_target.label)
}

fn configure_target_artifact_dir(cmd: &mut Command, workspace_root: &Path, build_target: BuildTarget) {
    cmd.env(
        "CARGO_TARGET_DIR",
        target_artifact_dir(workspace_root, build_target),
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
