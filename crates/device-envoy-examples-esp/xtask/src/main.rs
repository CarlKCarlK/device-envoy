//! Build automation tasks for the device-envoy-esp project.
//!
//! Run with: `cargo xtask <command>`

mod audio_player_generated;
mod board_examples_generated;
mod boards;
mod example_specs;
mod ir_generated;
mod led2d_generated;
mod led_generated;
mod led_strip_generated;
mod servo_player_generated;

use bitflags::bitflags;
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
const CHIP_FEATURE_ESP32C5: &str = "esp32c5";
const CHIP_FEATURE_ESP32C6: &str = "esp32c6";
const CHIP_FEATURE_ESP32C61: &str = "esp32c61";
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

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Capability: u16 {
        const RMT = 1 << 0;
        const I2S = 1 << 1;
        const AUDIO_GPIO = 1 << 2;
        const BUTTON_GPIO = 1 << 3;
        const WIFI = 1 << 4;
        /// Chip has sufficient linker memory budget for large WiFi/stack examples.
        const LARGE_STACK = 1 << 5;
        /// Chip exposes at least two independent SPI peripherals.
        const DUAL_SPI = 1 << 6;
        /// Chip exposes LEDC timers/channels.
        const LEDC = 1 << 7;
    }
}

const ALL_CAPABILITIES: [Capability; 8] = [
    Capability::RMT,
    Capability::I2S,
    Capability::AUDIO_GPIO,
    Capability::BUTTON_GPIO,
    Capability::WIFI,
    Capability::LARGE_STACK,
    Capability::DUAL_SPI,
    Capability::LEDC,
];

impl Capability {
    const fn name(self) -> &'static str {
        match self {
            Capability::RMT => "RMT",
            Capability::I2S => "I2S",
            Capability::AUDIO_GPIO => "audio GPIO mapping",
            Capability::BUTTON_GPIO => "button GPIO mapping",
            Capability::WIFI => "Wi-Fi",
            Capability::LARGE_STACK => "large stack/linker budget",
            Capability::DUAL_SPI => "dual SPI",
            Capability::LEDC => "LEDC",
            _ => "unknown capability",
        }
    }
}

/// Returns the first `BoardProfile` for `chip_feature`. Per-chip hardware facts
/// (rmt_count, wifi_supported, spi_count, etc.) are identical across all boards for a
/// given chip, so any profile for the chip is authoritative.
fn chip_profile(chip_feature: &str) -> &'static boards::BoardProfile {
    boards::BOARD_PROFILES
        .iter()
        .find(|p| p.chip_feature() == chip_feature)
        .unwrap_or_else(|| panic!("unknown chip feature: {chip_feature}"))
}

fn chip_capabilities(chip_feature: &str) -> Capability {
    let profile = chip_profile(chip_feature);
    let mut caps = Capability::empty();
    // RMT and I2S are always enabled together in build.rs.
    if profile.rmt_count > 0 {
        caps.insert(Capability::RMT);
        caps.insert(Capability::I2S);
    }
    // Audio GPIO mapping is available whenever the board has a wiring defined.
    if profile.audio_wiring.is_some() {
        caps.insert(Capability::AUDIO_GPIO);
    }
    // Any free GPIO can serve as a button pin.
    caps.insert(Capability::BUTTON_GPIO);
    if profile.wifi_supported {
        caps.insert(Capability::WIFI);
    }
    if !profile.stack_constrained {
        caps.insert(Capability::LARGE_STACK);
    }
    if profile.spi_count >= 2 {
        caps.insert(Capability::DUAL_SPI);
    }
    if !matches!(chip_feature, CHIP_FEATURE_ESP32C5 | CHIP_FEATURE_ESP32C61) {
        caps.insert(Capability::LEDC);
    }
    caps
}

/// Strips the `_{chip_feature}_{board_dir}` suffix from a board-generated example name
/// (e.g. `audio_example3_esp32c6_generic` → `audio_example3`), returning the base
/// template name. Returns the full name unchanged for top-level examples that carry no
/// board suffix.
fn example_base_name(full_name: &str) -> &str {
    const CHIP_NEEDLES: &[&str] = &[
        "_esp32c2_",
        "_esp32c3_",
        "_esp32c5_",
        "_esp32c6_",
        "_esp32c61_",
        "_esp32h2_",
        "_esp32s2_",
        "_esp32s3_",
        "_esp32_",
    ];
    for needle in CHIP_NEEDLES {
        if let Some(pos) = full_name.find(needle) {
            return &full_name[..pos];
        }
    }
    full_name
}

/// Maps each example base name to the capabilities it requires.
/// Base names must match exactly (no prefix matching).
const EXAMPLE_REQUIREMENTS_TABLE: &[(&str, &[Capability])] = &[
    ("audio", &[Capability::I2S, Capability::AUDIO_GPIO]),
    ("audio20k_one", &[Capability::I2S, Capability::AUDIO_GPIO]),
    ("audio_example1", &[Capability::I2S, Capability::AUDIO_GPIO]),
    ("audio_example2", &[Capability::I2S, Capability::AUDIO_GPIO]),
    ("audio_example3", &[Capability::I2S, Capability::AUDIO_GPIO]),
    ("blinky", &[]),
    ("button_example1", &[Capability::BUTTON_GPIO]),
    (
        "clock_console_simple",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    ("clock_lcd", &[Capability::WIFI, Capability::LARGE_STACK]),
    ("clock_led4", &[Capability::WIFI, Capability::LARGE_STACK]),
    (
        "clock_led8x12",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    (
        "clock_servos",
        &[Capability::WIFI, Capability::LARGE_STACK, Capability::LEDC],
    ),
    (
        "clock_sync_example1",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    ("conway", &[Capability::RMT]),
    ("cyd_tiles", &[Capability::DUAL_SPI]),
    (
        "cyd_touch_paint",
        &[Capability::DUAL_SPI, Capability::LARGE_STACK],
    ),
    (
        "dns_tester",
        &[
            Capability::WIFI,
            Capability::DUAL_SPI,
            Capability::LARGE_STACK,
        ],
    ),
    ("flash_block_example1", &[]),
    ("ir", &[Capability::RMT]),
    ("ir_example1", &[Capability::RMT]),
    ("ir_kepler", &[Capability::RMT]),
    ("ir_kepler_example1", &[Capability::RMT]),
    ("ir_keplers", &[Capability::RMT]),
    ("ir_mapping_example1", &[Capability::RMT]),
    ("lcd_text", &[]),
    ("lcd_text_example1", &[]),
    ("lcd_texts", &[]),
    ("led16x16_plus_1", &[Capability::RMT]),
    ("led16x16_plus_1_spi", &[Capability::RMT]),
    ("led16x16test", &[Capability::RMT]),
    ("led2d", &[Capability::RMT]),
    ("led2d_example1", &[Capability::RMT]),
    ("led2d_example2", &[Capability::RMT]),
    ("led_example1", &[]),
    ("led_strip8_spi", &[Capability::RMT]),
    ("led_strip_example1", &[Capability::RMT]),
    ("led_strip_example2", &[Capability::RMT]),
    ("led_strip_len8", &[Capability::RMT]),
    ("rfid", &[]),
    ("servo_basic", &[Capability::LEDC]),
    ("servo_example1", &[Capability::LEDC]),
    ("servo_player_example1", &[Capability::LEDC]),
    ("servo_player_example2", &[Capability::LEDC]),
    ("servos", &[Capability::LEDC]),
    (
        "servos_calibrate",
        &[Capability::BUTTON_GPIO, Capability::LEDC],
    ),
    ("talk1_a1_strip_8_blue_gray", &[Capability::RMT]),
    (
        "talk1_a3_strip_8_blue_white_blink_animate",
        &[Capability::RMT],
    ),
    ("talk1_a4_strip_96_blue_white_dot", &[Capability::RMT]),
    ("talk1_b1_panel_12x8_rust_cursor", &[Capability::RMT]),
    ("talk1_b2_panel_12x8_text_graphics", &[Capability::RMT]),
    (
        "talk1_f1_dns",
        &[
            Capability::RMT,
            Capability::WIFI,
            Capability::BUTTON_GPIO,
            Capability::LARGE_STACK,
        ],
    ),
    (
        "wifi_auto_custom_checkbox",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    (
        "wifi_auto_example1",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    (
        "wifi_auto_force_button",
        &[Capability::WIFI, Capability::LARGE_STACK],
    ),
    (
        "wifi_dns_hex",
        &[Capability::RMT, Capability::WIFI, Capability::LARGE_STACK],
    ),
    ("wifi_scan", &[Capability::WIFI]),
];

fn example_requirements(example: &str) -> Result<Capability, String> {
    let base = example_base_name(example);
    let mut matches = EXAMPLE_REQUIREMENTS_TABLE
        .iter()
        .filter(|(name, _)| *name == base);
    let Some((_, required_caps)) = matches.next() else {
        return Err(format!(
            "missing capability requirements mapping for example `{example}` (base `{base}`): \
add an exact `{base}` entry to EXAMPLE_REQUIREMENTS_TABLE"
        ));
    };
    if matches.next().is_some() {
        return Err(format!(
            "duplicate capability requirements mapping for example base `{base}` in EXAMPLE_REQUIREMENTS_TABLE"
        ));
    }
    let mut capability_set = Capability::empty();
    for &cap in *required_caps {
        capability_set.insert(cap);
    }
    Ok(capability_set)
}

fn validate_example_requirements_table(examples: &[String]) -> Result<(), String> {
    let discovered_bases: std::collections::BTreeSet<String> = examples
        .iter()
        .map(|example| example_base_name(example).to_string())
        .collect();

    let mut table_counts: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for (name, _) in EXAMPLE_REQUIREMENTS_TABLE {
        *table_counts.entry(*name).or_insert(0) += 1;
    }

    let missing: Vec<String> = discovered_bases
        .iter()
        .filter(|name| !table_counts.contains_key(name.as_str()))
        .cloned()
        .collect();
    let extras: Vec<String> = table_counts
        .keys()
        .filter(|name| !discovered_bases.contains(**name))
        .map(|name| (*name).to_string())
        .collect();
    let duplicates: Vec<String> = table_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, count)| format!("{name} ({count} entries)"))
        .collect();

    if missing.is_empty() && extras.is_empty() && duplicates.is_empty() {
        return Ok(());
    }

    let mut message = String::from(
        "example capability requirements table is out of sync with discovered examples.\n",
    );
    if !missing.is_empty() {
        message.push_str("missing entries (add these exact base names):\n");
        for name in &missing {
            message.push_str(&format!("  - {name}\n"));
        }
    }
    if !extras.is_empty() {
        message.push_str("extra/stale entries (remove or rename these):\n");
        for name in &extras {
            message.push_str(&format!("  - {name}\n"));
        }
    }
    if !duplicates.is_empty() {
        message.push_str("duplicate entries (must be unique):\n");
        for duplicate in &duplicates {
            message.push_str(&format!("  - {duplicate}\n"));
        }
    }
    Err(message)
}

fn missing_capabilities(
    chip_capabilities: Capability,
    required_capabilities: Capability,
) -> Vec<&'static str> {
    let mut missing_capabilities = Vec::new();
    for capability in ALL_CAPABILITIES {
        if required_capabilities.contains(capability) && !chip_capabilities.contains(capability) {
            missing_capabilities.push(capability.name());
        }
    }
    missing_capabilities
}

/// Returns the capabilities required by an embedded compile test, derived from the
/// test's name. GPIO requirements are handled separately via `scan_test_required_gpios`.
fn test_capabilities_required(test: &str) -> Capability {
    let mut caps = Capability::empty();
    if test.starts_with("ir_")
        || matches!(
            test,
            "led_strip_two_strips_compile" | "led2d_two_panels_compile"
        )
    {
        caps.insert(Capability::RMT);
    }
    if test == "clock_sync_two_compile" {
        caps.insert(Capability::WIFI);
    }
    if test == "led_strip_spi_two_strips_compile" {
        caps.insert(Capability::DUAL_SPI);
    }
    caps
}

/// Scans an embedded test source file and returns the sorted, deduplicated set of GPIO
/// pin numbers it references (e.g. `p.GPIO6` or `pin: GPIO7` → 6 and 7). Used to
/// automatically derive which chips can compile a test based on `unavailable_gpios`.
fn scan_test_required_gpios(root: &Path, test_stem: &str) -> Vec<u8> {
    let path = root.join("tests/embedded").join(format!("{test_stem}.rs"));
    let src = std::fs::read_to_string(&path).unwrap_or_default();
    let mut gpios = Vec::new();
    let mut pos = 0;
    while let Some(offset) = src[pos..].find("GPIO") {
        let abs = pos + offset + 4; // skip past "GPIO"
        let digits: String = src[abs..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            if let Ok(n) = digits.parse::<u8>() {
                gpios.push(n);
            }
        }
        pos = abs;
    }
    gpios.sort();
    gpios.dedup();
    gpios
}

fn explicit_example_skip_reason(_chip_feature: &str, _example_name: &str) -> Option<&'static str> {
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
const BUILD_TARGET_ESP32C5: BuildTarget = BuildTarget {
    label: "esp32c5",
    target: TARGET_RISCV32IMAC,
    chip_feature: CHIP_FEATURE_ESP32C5,
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
const BUILD_TARGET_ESP32C61: BuildTarget = BuildTarget {
    label: "esp32c61",
    target: TARGET_RISCV32IMAC,
    chip_feature: CHIP_FEATURE_ESP32C61,
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
    BUILD_TARGET_ESP32C5,
    BUILD_TARGET_ESP32C6,
    BUILD_TARGET_ESP32C61,
    BUILD_TARGET_ESP32H2,
    BUILD_TARGET_ESP32S2,
    BUILD_TARGET_ESP32S3,
];
const CHECK_ALL_TARGETS: &[BuildTarget] = ALL_PROCESSOR_TARGETS;

/// Discovers embedded tests from `tests/embedded/` by filename convention:
/// - `*_compile_fail.rs` → compile-fail tests
/// - `*_compile.rs`      → compile-pass tests
///
/// This replaces the hardcoded `COMPILE_PASS_TESTS`/`COMPILE_FAIL_TESTS` lists and
/// ensures new test files are picked up automatically.
fn discover_embedded_tests(root: &Path) -> (Vec<String>, Vec<String>) {
    let tests_dir = root.join("tests/embedded");
    let mut compile_pass = Vec::new();
    let mut compile_fail = Vec::new();
    let entries = std::fs::read_dir(&tests_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", tests_dir.display()));
    for entry in entries.flatten() {
        let name = entry.file_name().into_string().ok().unwrap_or_default();
        let Some(stem) = name.strip_suffix(".rs") else {
            continue;
        };
        if stem.ends_with("_compile_fail") {
            compile_fail.push(stem.to_owned());
        } else if stem.ends_with("_compile") {
            compile_pass.push(stem.to_owned());
        }
    }
    compile_pass.sort();
    compile_fail.sort();
    (compile_pass, compile_fail)
}

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
    /// Build all talk1 examples (backward-compatible alias for legacy demos)
    CheckDemos,
    /// Verify README Rust example extraction + compile
    CheckReadmeExample,
    /// Generate board-specific examples from templates
    #[command(alias = "generate-blinky-examples")]
    GenerateBoardExamples,
    /// Verify board-specific examples are current with their templates
    CheckBoardExamples,
    /// Run all checks for a single chip (fast developer iteration)
    CheckChip {
        /// Chip name: esp32, esp32c2, esp32c3, esp32c5, esp32c6, esp32c61, esp32h2, esp32s2, esp32s3
        chip: String,
    },
    /// Run embedded compile tests across all chips
    CheckEmbeddedTests,
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
        Commands::CheckBoardExamples => check_board_examples(),
        Commands::CheckChip { chip } => check_chip(&chip),
        Commands::CheckEmbeddedTests => check_embedded_tests(),
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
    if let Err(err) = generate_example_templates(&root) {
        eprintln!("Error generating examples from templates: {err}");
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
    if let Err(err) = generate_example_templates(&root) {
        eprintln!("Error generating examples from templates: {err}");
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

    // Pre-fetch all crate dependencies so the subsequent parallel builds do not contend
    // on the package cache lock.  If fetch fails, proceed anyway — packages will be
    // fetched on demand during the parallel builds (with some lock contention).
    if !run(Command::new("cargo").current_dir(&root).args(["fetch"])) {
        eprintln!(
            "warning: cargo fetch failed; parallel builds may see package cache lock contention"
        );
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
            // Exclude internal/temporary files (e.g. __readme_example_generated.rs)
            if name.starts_with("__") {
                return None;
            }
            name.ends_with(".rs")
                .then(|| name.trim_end_matches(".rs").to_owned())
        })
        .collect();
    for generated_board_example in board_examples_generated::generated_board_example_names() {
        if !examples
            .iter()
            .any(|example| example == &generated_board_example)
        {
            examples.push(generated_board_example);
        }
    }
    examples.sort();

    if let Err(error) = validate_example_requirements_table(&examples) {
        eprintln!("{}", format!("error: {error}").red().bold());
        return ExitCode::FAILURE;
    }

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

fn check_demos() -> ExitCode {
    check_examples_for_targets(CHECK_ALL_TARGETS, true)
}

fn check_embedded_tests() -> ExitCode {
    check_embedded_tests_for_targets(CHECK_ALL_TARGETS)
}

fn check_embedded_tests_for_targets(targets: &[BuildTarget]) -> ExitCode {
    let root = workspace_root();
    println!(
        "{}",
        "--> embedded tests (compile-pass + expected compile-fail)".cyan()
    );
    let (compile_pass_tests, compile_fail_tests) = discover_embedded_tests(&root);
    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(targets) else {
        return ExitCode::FAILURE;
    };
    let embedded_results: Vec<bool> = targets
        .par_iter()
        .copied()
        .map(|build_target| {
            check_embedded_tests_for_target(
                &root,
                &xtensa_linker_dir,
                build_target,
                &compile_pass_tests
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                &compile_fail_tests
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    if embedded_results.iter().any(|ok| !ok) {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn check_chip(chip_label: &str) -> ExitCode {
    let Some(build_target) = ALL_PROCESSOR_TARGETS
        .iter()
        .copied()
        .find(|t| t.label == chip_label)
    else {
        let valid = ALL_PROCESSOR_TARGETS
            .iter()
            .map(|t| t.label)
            .collect::<Vec<_>>();
        eprintln!(
            "{}",
            format!(
                "error: unknown chip '{chip_label}'; valid options are: {}",
                valid.join(", ")
            )
            .red()
            .bold()
        );
        return ExitCode::FAILURE;
    };

    let root = workspace_root();
    println!(
        "{}",
        format!(
            "==> cargo check-chip: {} (device-envoy-esp)",
            build_target.label
        )
        .cyan()
        .bold()
    );

    let Some(xtensa_linker_dir) = xtensa_linker_dir_if_needed(&[build_target]) else {
        return ExitCode::FAILURE;
    };

    if !build_lib_for_target(&root, &xtensa_linker_dir, build_target) {
        return ExitCode::FAILURE;
    }

    if check_examples_for_targets(&[build_target], true) != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    let (compile_pass_tests, compile_fail_tests) = discover_embedded_tests(&root);
    if !check_embedded_tests_for_target(
        &root,
        &xtensa_linker_dir,
        build_target,
        &compile_pass_tests
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &compile_fail_tests
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    ) {
        return ExitCode::FAILURE;
    }

    println!(
        "\n{}",
        format!("==> check-chip {} passed! 🎉", build_target.label)
            .green()
            .bold()
    );
    ExitCode::SUCCESS
}

fn build_lib_for_target(root: &Path, xtensa_linker_dir: &Path, build_target: BuildTarget) -> bool {
    println!(
        "{}",
        format!("--> build lib ({})", build_target.label).cyan()
    );
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

    // Build examples in parallel within a chip — there are no intra-chip dependencies
    // between examples and each gets its own `Command`, so this is safe.
    let failed = std::sync::atomic::AtomicBool::new(false);
    examples.par_iter().for_each(|example| {
        if failed.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        if let Some(required_chip_feature) =
            board_examples_generated::board_example_required_chip(example)
        {
            if required_chip_feature != build_target.chip_feature {
                println!(
                    "    skip example: {example} (board example targets {required_chip_feature}, not {})",
                    build_target.label
                );
                return;
            }
        }
        if let Some(skip_reason) = explicit_example_skip_reason(build_target.chip_feature, example)
        {
            println!(
                "    skip example: {example} ({skip_reason} on {})",
                build_target.label
            );
            return;
        }
        let required_capabilities = match example_requirements(example) {
            Ok(required_capabilities) => required_capabilities,
            Err(error) => {
                eprintln!("{}", format!("error: {error}").red().bold());
                failed.store(true, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        };
        let missing = missing_capabilities(chip_capabilities, required_capabilities);
        if !missing.is_empty() {
            println!(
                "    skip example: {example} ({} unavailable on {})",
                missing.join(", "),
                build_target.label
            );
            return;
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
            failed.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });
    !failed.load(std::sync::atomic::Ordering::Relaxed)
}

fn check_embedded_tests_for_target(
    root: &Path,
    xtensa_linker_dir: &Path,
    build_target: BuildTarget,
    compile_pass_tests: &[&str],
    compile_fail_tests: &[&str],
) -> bool {
    println!("{}", format!("    target: {}", build_target.label).cyan());
    let strict_embedded_skips = std::env::var("CHECK_EMBEDDED_STRICT").as_deref() == Ok("1");
    let mut skipped_count = 0usize;
    for embedded_test in compile_pass_tests {
        if let Some(reason) =
            explicit_embedded_test_skip_reason(root, build_target.chip_feature, embedded_test)
        {
            println!(
                "      skip compile-pass test: {embedded_test} ({reason} on {})",
                build_target.label
            );
            skipped_count += 1;
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
            explicit_embedded_test_skip_reason(root, build_target.chip_feature, embedded_test)
        {
            println!(
                "      skip compile-fail test: {embedded_test} ({reason} on {})",
                build_target.label
            );
            skipped_count += 1;
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

    if skipped_count > 0 {
        println!(
            "      skipped embedded tests on {}: {skipped_count}",
            build_target.label
        );
        if strict_embedded_skips {
            eprintln!(
                "{}",
                format!(
                    "error: CHECK_EMBEDDED_STRICT=1 and {skipped_count} embedded tests were skipped on {}",
                    build_target.label
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
    root: &Path,
    chip_feature: &str,
    embedded_test: &str,
) -> Option<&'static str> {
    // Capability-based skip: derive required capabilities from the test name and compare
    // against what the chip provides.
    let chip_caps = chip_capabilities(chip_feature);
    let required_caps = test_capabilities_required(embedded_test);
    if required_caps.contains(Capability::RMT) && !chip_caps.contains(Capability::RMT) {
        return Some("chip has no RMT peripheral");
    }
    if required_caps.contains(Capability::WIFI) && !chip_caps.contains(Capability::WIFI) {
        return Some("chip has no Wi-Fi");
    }
    if required_caps.contains(Capability::DUAL_SPI) && !chip_caps.contains(Capability::DUAL_SPI) {
        return Some("chip has fewer than two SPI peripherals");
    }

    // GPIO availability: scan the test source for all GPIO pin numbers it references,
    // then check whether any of those pins are absent on this chip.
    let profile = chip_profile(chip_feature);
    if !profile.unavailable_gpios.is_empty() {
        let required_gpios = scan_test_required_gpios(root, embedded_test);
        let blocked: Vec<u8> = required_gpios
            .iter()
            .copied()
            .filter(|g| profile.unavailable_gpios.contains(g))
            .collect();
        if !blocked.is_empty() {
            return Some("test references GPIO pins unavailable on this chip");
        }
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

    core::hint::black_box(&generated_example_cleanup);

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

    if let Err(err) = generate_example_templates(&root) {
        eprintln!("Error generating examples from templates: {err}");
        return ExitCode::FAILURE;
    }

    println!("{}", "==> Template-based examples generated".green().bold());
    ExitCode::SUCCESS
}

fn check_board_examples() -> ExitCode {
    let root = workspace_root();
    if let Err(error) = board_examples_generated::check_board_examples(&root) {
        eprintln!("Error checking generated examples: {error}");
        return ExitCode::FAILURE;
    }
    println!("{}", "==> Generated examples are current".green().bold());
    ExitCode::SUCCESS
}

fn generate_example_templates(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    board_examples_generated::generate_board_examples(root)?;
    Ok(())
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

fn configure_target_artifact_dir(
    cmd: &mut Command,
    workspace_root: &Path,
    build_target: BuildTarget,
) {
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
