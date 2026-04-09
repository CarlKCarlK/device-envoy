use crate::boards::{validate_board_profiles, AudioWiring, BOARD_PROFILES};
use crate::example_specs::{
    blinky_built_in_led, blinky_kind, blinky_led_pin_num, led_strip1_built_in, led_strip1_pin_num,
    supports_conway_example, supports_ir_examples, supports_led16x16_plus_1_example,
    supports_led16x16_plus_1_spi_example, BlinkyKind,
};
use minijinja::{context, Environment};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const GENERATED_EXAMPLES_BEGIN_MARKER: &str = "# BEGIN GENERATED BOARD EXAMPLES";
const GENERATED_EXAMPLES_END_MARKER: &str = "# END GENERATED BOARD EXAMPLES";
const LEGACY_TALK1_BEGIN_MARKER: &str = "# BEGIN GENERATED TALK1 EXAMPLES";
const LEGACY_TALK1_END_MARKER: &str = "# END GENERATED TALK1 EXAMPLES";

static BOARD_TEMPLATE_EXAMPLE_BASE_NAMES: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoardTemplateMode {
    Copy,
    Render,
}

fn board_template_example_name(board_profile: crate::boards::BoardProfile, base_name: &str) -> String {
    let base_name = base_name.replace('/', "_");
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

fn board_template_example_base_names() -> &'static [String] {
    BOARD_TEMPLATE_EXAMPLE_BASE_NAMES
        .get_or_init(|| {
            discover_board_template_example_base_names(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/templates"),
            )
            .unwrap_or_else(|error| {
                panic!("failed to discover board templates: {error}");
            })
        })
        .as_slice()
}

fn discover_board_template_example_base_names(
    templates_dir: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut base_names = Vec::new();
    discover_board_template_example_base_names_in_dir(templates_dir, templates_dir, &mut base_names)?;
    base_names.sort();
    Ok(base_names)
}

fn discover_board_template_example_base_names_in_dir(
    templates_root_dir: &Path,
    templates_scan_dir: &Path,
    base_names: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    for template_entry in fs::read_dir(templates_scan_dir)? {
        let template_entry = template_entry?;
        let template_path = template_entry.path();
        if template_path.is_dir() {
            discover_board_template_example_base_names_in_dir(
                templates_root_dir,
                &template_path,
                base_names,
            )?;
            continue;
        }
        if !template_path.is_file() {
            continue;
        }
        let Some(relative_path) = template_path
            .strip_prefix(templates_root_dir)
            .ok()
            .and_then(|value| value.to_str())
        else {
            continue;
        };
        let Some(base_name) = relative_path.strip_suffix(".rs.j2") else {
            continue;
        };
        if !is_board_template(base_name) {
            continue;
        }
        base_names.push(base_name.to_string());
    }
    Ok(())
}

fn is_board_template(base_name: &str) -> bool {
    let _ = base_name;
    true
}

fn board_template_supports_board(
    base_name: &str,
    board_profile: crate::boards::BoardProfile,
) -> bool {
    if base_name.starts_with("audio") && board_profile.audio_wiring.is_none() {
        return false;
    }
    if base_name.starts_with("clock_") && !board_profile.wifi_supported {
        return false;
    }
    match base_name {
        "conway" => supports_conway_example(board_profile),
        "led16x16_plus_1" => supports_led16x16_plus_1_example(board_profile),
        "led16x16_plus_1_spi" => supports_led16x16_plus_1_spi_example(board_profile),
        "ir"
        | "ir_example1_trait"
        | "ir_kepler"
        | "ir_kepler_example1_trait"
        | "ir_keplers"
        | "ir_mapping_example1_trait" => supports_ir_examples(board_profile),
        "clock_console_simple"
        | "clock_sync_example1_trait"
        | "clock_servos"
        | "clock_lcd"
        | "clock_led4"
        | "clock_led8x12" => board_profile.wifi_supported,
        _ => true,
    }
}

fn board_template_placeholder_reason(
    _example_name: &str,
    base_name: &str,
    board_profile: crate::boards::BoardProfile,
) -> Option<String> {
    if base_name.starts_with("clock_") && !board_profile.wifi_supported {
        return Some(format!(
            "this example requires Wi-Fi, and {} does not offer that resource",
            board_profile.chip_name()
        ));
    }

    if base_name == "led16x16_plus_1_spi" && !supports_led16x16_plus_1_spi_example(board_profile) {
        if board_profile.spi_count < 2 {
            let resource_word = if board_profile.spi_count == 1 {
                "resource"
            } else {
                "resources"
            };
            return Some(format!(
                "this example requires two SPI resources, and {} offers {} SPI {}",
                board_profile.chip_name(),
                board_profile.spi_count,
                resource_word
            ));
        }
        if board_profile.rmt_count < 2 {
            let resource_word = if board_profile.rmt_count == 1 {
                "resource"
            } else {
                "resources"
            };
            return Some(format!(
                "this example requires two RMT resources, and {} offers {} RMT {}",
                board_profile.chip_name(),
                board_profile.rmt_count,
                resource_word
            ));
        }
    }

    if base_name == "led16x16_plus_1" && !supports_led16x16_plus_1_example(board_profile) {
        let resource_word = if board_profile.rmt_count == 1 {
            "resource"
        } else {
            "resources"
        };
        return Some(format!(
            "this example requires two RMT resources, and {} offers {} RMT {}",
            board_profile.chip_name(),
            board_profile.rmt_count,
            resource_word
        ));
    }

    if base_name.starts_with("audio") && board_profile.audio_wiring.is_none() {
        return Some(format!(
            "this example requires I2S resources, and {} does not offer that resource",
            board_profile.chip_name()
        ));
    }

    if !supports_ir_examples(board_profile) {
        return match base_name {
            "conway"
            | "ir"
            | "ir_example1_trait"
            | "ir_kepler"
            | "ir_kepler_example1_trait"
            | "ir_keplers"
            | "ir_mapping_example1_trait" => Some(format!(
                "our IR decoder needs an RMT resource, and {} does not offer that resource",
                board_profile.chip_name()
            )),
            _ => None,
        };
    }

    None
}

fn board_template_placeholder_source(
    example_name: &str,
    base_name: &str,
    board_profile: crate::boards::BoardProfile,
) -> String {
    let unsupported_reason = board_template_placeholder_reason(example_name, base_name, board_profile);
    let wiring_note = if let Some(reason) = unsupported_reason.as_deref() {
        format!("//! - {reason}\n")
    } else {
        "//! - This is a placeholder for an unsupported board profile.\n".to_string()
    };
    let info_message = if let Some(reason) = unsupported_reason.as_deref() {
        format!("{example_name}: {reason}")
    } else {
        format!("{example_name}: not supported on this board profile")
    };
    format!(
        "// @generated by cargo xtask generate-board-examples.\n\
#![allow(missing_docs)]\n\
//! {base_name}: not supported on this board profile.\n\
//!\n\
//! Wiring:\n\
{wiring_note}\
\n\
#![no_std]\n\
#![no_main]\n\
\n\
use core::convert::Infallible;\n\
\n\
use embassy_executor::Spawner;\n\
\n\
use esp_backtrace as _;\n\
use log::info;\n\
\n\
use device_envoy_esp::{{Result, init_and_start}};\n\
\n\
esp_bootloader_esp_idf::esp_app_desc!();\n\
\n\
#[esp_rtos::main]\n\
async fn main(spawner: Spawner) -> ! {{\n\
    let err = inner_main(spawner).await.unwrap_err();\n\
    panic!(\"{{err:?}}\");\n\
}}\n\
\n\
async fn inner_main(spawner: Spawner) -> Result<Infallible> {{\n\
    init_and_start!(p);\n\
    esp_println::logger::init_logger(log::LevelFilter::Info);\n\
\n\
    let _ = spawner;\n\
    info!(\"{info_message}\");\n\
    core::future::pending().await\n\
}}\n"
    )
}

fn board_template_mode(
    template_source: &str,
) -> Result<BoardTemplateMode, Box<dyn Error>> {
    for template_line in template_source.lines() {
        let Some(mode_fragment) = template_line.split("@board-example mode=").nth(1) else {
            continue;
        };
        let mode = mode_fragment
            .trim()
            .trim_end_matches("*/")
            .trim_end_matches("#}")
            .trim();
        return match mode {
            "copy" => Ok(BoardTemplateMode::Copy),
            "render" => Ok(BoardTemplateMode::Render),
            _ => Err(
                format!("invalid @board-example mode `{mode}` (expected `copy` or `render`)")
                    .into(),
            ),
        };
    }
    Ok(BoardTemplateMode::Copy)
}

pub fn generate_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    validate_board_profiles()?;

    let examples_dir = workspace_root.join("examples");
    let templates_dir = examples_dir.join("templates");

    cleanup_legacy_flat_generated_examples(&examples_dir)?;

    let mut expected_generated_paths = Vec::new();
    generate_board_template_files(&templates_dir, &examples_dir, &mut expected_generated_paths)?;

    rustfmt_generated_files(&expected_generated_paths)?;
    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;
    sync_generated_examples_in_cargo_toml(workspace_root)?;

    Ok(())
}

fn generate_board_template_files(
    example_templates_dir: &Path,
    examples_dir: &Path,
    expected_generated_paths: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    for base_name in board_template_example_base_names() {
        let template_path = example_templates_dir.join(format!("{base_name}.rs.j2"));
        let template_source = fs::read_to_string(&template_path)?;
        let template_mode = board_template_mode(&template_source)?;
        for board_profile in BOARD_PROFILES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = board_template_example_name(*board_profile, base_name);
            let generated_source = if !board_template_supports_board(
                base_name,
                *board_profile,
            ) {
                board_template_placeholder_source(&example_name, base_name, *board_profile)
            } else if template_mode == BoardTemplateMode::Render {
                let mut minijinja_environment = Environment::new();
                minijinja_environment.add_template(base_name.as_str(), &template_source)?;
                let board_supports_audio = board_profile.audio_wiring.is_some();
                let audio_wiring = board_profile.audio_wiring.unwrap_or(AudioWiring {
                    data_pin_num: u8::MAX,
                    bit_clock_pin_num: u8::MAX,
                    word_select_pin_num: u8::MAX,
                    dma_ident: "DMA_UNSUPPORTED_AUDIO",
                });
                minijinja_environment
                    .get_template(base_name.as_str())?
                    .render(context! {
                    example_name => example_name.as_str(),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    blinky_kind => match blinky_kind(*board_profile) {
                        BlinkyKind::Plain => "plain",
                        BlinkyKind::SmartRmt => "smart_rmt",
                        BlinkyKind::SmartSpi => "smart_spi",
                    },
                    led_pin_num => blinky_led_pin_num(*board_profile),
                    led_pin_ident => format!("GPIO{}", blinky_led_pin_num(*board_profile)),
                    built_in_led => blinky_built_in_led(*board_profile),
                    panel_pin_num => board_profile.led_2d16x16_pin,
                    panel_pin_ident => format!("GPIO{}", board_profile.led_2d16x16_pin),
                    led_strip1_pin_num => led_strip1_pin_num(*board_profile),
                    led_strip1_pin_ident => format!("GPIO{}", led_strip1_pin_num(*board_profile)),
                    led_strip1_built_in => led_strip1_built_in(*board_profile),
                    strip8_pin_num => board_profile.led_strip_len_8_pin,
                    strip8_pin_ident => format!("GPIO{}", board_profile.led_strip_len_8_pin),
                    panel12x8_pin_num => board_profile.led_2d12x8_pin,
                    ir_supported => supports_ir_examples(*board_profile),
                    ir_pin_num => board_profile.ir_pin_rx_channel.0,
                    ir_pin_ident => format!("GPIO{}", board_profile.ir_pin_rx_channel.0),
                    ir_receiver0_pin_num => board_profile.ir_pin_rx_channel.0,
                    ir_receiver0_pin_ident => format!("GPIO{}", board_profile.ir_pin_rx_channel.0),
                    ir_receiver1_pin_num => board_profile.ir_pin_rx_channel2.0,
                    ir_receiver1_pin_ident => format!("GPIO{}", board_profile.ir_pin_rx_channel2.0),
                    ir_rx_channel_num => board_profile.ir_pin_rx_channel.1,
                    ir_rx_channel_ident => format!("channel{}", board_profile.ir_pin_rx_channel.1),
                    ir_rx_channel2_num => board_profile.ir_pin_rx_channel2.1,
                    ir_rx_channel2_ident => format!("channel{}", board_profile.ir_pin_rx_channel2.1),
                    clock_supported => board_profile.wifi_supported,
                    force_portal_button_pin_num => board_profile.button_pin,
                    force_portal_button_pin_ident => format!("GPIO{}", board_profile.button_pin),
                    lcd_sda_pin_num => board_profile.lcd_sda_pin,
                    lcd_sda_pin_ident => format!("GPIO{}", board_profile.lcd_sda_pin),
                    lcd_scl_pin_num => board_profile.lcd_scl_pin,
                    lcd_scl_pin_ident => format!("GPIO{}", board_profile.lcd_scl_pin),
                    servo_bottom_pin_num => board_profile.servo_pin,
                    servo_bottom_pin_ident => format!("GPIO{}", board_profile.servo_pin),
                    servo_top_pin_num => board_profile.servo2_pin,
                    servo_top_pin_ident => format!("GPIO{}", board_profile.servo2_pin),
                    led4_cell_pin_nums => board_profile.led4_cell_pins,
                    led4_cell_pin_idents => board_profile.led4_cell_pins
                        .map(|pin_num| format!("GPIO{pin_num}")),
                    led4_segment_pin_nums => board_profile.led4_segment_pins,
                    led4_segment_pin_idents => board_profile.led4_segment_pins
                        .map(|pin_num| format!("GPIO{pin_num}")),
                    audio_supported => board_supports_audio,
                    data_pin_num => audio_wiring.data_pin_num,
                    data_pin_ident => if board_supports_audio { format!("GPIO{}", audio_wiring.data_pin_num) } else { "GPIO_UNSUPPORTED_AUDIO".to_string() },
                    bit_clock_pin_num => audio_wiring.bit_clock_pin_num,
                    bit_clock_pin_ident => if board_supports_audio { format!("GPIO{}", audio_wiring.bit_clock_pin_num) } else { "GPIO_UNSUPPORTED_AUDIO".to_string() },
                    word_select_pin_num => audio_wiring.word_select_pin_num,
                    word_select_pin_ident => if board_supports_audio { format!("GPIO{}", audio_wiring.word_select_pin_num) } else { "GPIO_UNSUPPORTED_AUDIO".to_string() },
                    button_pin_num => if board_supports_audio { board_profile.button_pin } else { u8::MAX },
                    button_pin_ident => if board_supports_audio { format!("GPIO{}", board_profile.button_pin) } else { "GPIO_UNSUPPORTED_AUDIO".to_string() },
                    dma_ident => audio_wiring.dma_ident,
                })?
            } else {
                template_source.clone()
            };
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
        let legacy_top_level_output_path = examples_dir.join(format!("{base_name}.rs"));
        if legacy_top_level_output_path.exists() {
            fs::remove_file(&legacy_top_level_output_path)?;
        }
    }

    Ok(())
}

pub fn generated_board_example_names() -> Vec<String> {
    let mut names = Vec::new();
    for board_profile in BOARD_PROFILES {
        for base_name in board_template_example_base_names() {
            names.push(board_template_example_name(*board_profile, base_name));
        }
    }
    names
}

#[derive(Clone)]
struct ExampleManifestEntry {
    name: String,
    path: String,
}

fn generated_board_example_manifest_entries() -> Vec<ExampleManifestEntry> {
    let mut entries = Vec::new();

    for board_profile in BOARD_PROFILES {
        for base_name in board_template_example_base_names() {
            entries.push(ExampleManifestEntry {
                name: board_template_example_name(*board_profile, base_name),
                path: format!(
                    "examples/{}/{}/{}.rs",
                    board_profile.chip_dir(),
                    board_profile.board_dir(),
                    base_name
                ),
            });
        }
    }

    entries
}

fn render_generated_examples_block() -> String {
    let mut block = String::new();
    block.push_str(GENERATED_EXAMPLES_BEGIN_MARKER);
    block.push('\n');
    for entry in generated_board_example_manifest_entries() {
        block.push_str("[[example]]\n");
        block.push_str(&format!("name = \"{}\"\n", entry.name));
        block.push_str(&format!("path = \"{}\"\n\n", entry.path));
    }
    block.push_str(GENERATED_EXAMPLES_END_MARKER);
    block.push('\n');
    block
}

fn remove_optional_marked_block(
    source: String,
    begin_marker: &str,
    end_marker: &str,
) -> Result<String, Box<dyn Error>> {
    let begin = source.find(begin_marker);
    let end = source.find(end_marker);
    match (begin, end) {
        (None, None) => Ok(source),
        (Some(_), None) | (None, Some(_)) => {
            Err(format!("Cargo.toml marker mismatch: {begin_marker} / {end_marker}").into())
        }
        (Some(begin_index), Some(end_index)) => {
            let end_of_marker = end_index + end_marker.len();
            let trailing_newline_len = source[end_of_marker..]
                .chars()
                .next()
                .is_some_and(|character| character == '\n')
                as usize;
            let remove_end = end_of_marker + trailing_newline_len;
            let mut trimmed = String::with_capacity(source.len());
            trimmed.push_str(&source[..begin_index]);
            trimmed.push_str(&source[remove_end..]);
            Ok(trimmed)
        }
    }
}

fn sync_generated_examples_in_cargo_toml(crate_root: &Path) -> Result<(), Box<dyn Error>> {
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let cargo_toml_source = fs::read_to_string(&cargo_toml_path)?;
    let cargo_toml_source = remove_optional_marked_block(
        cargo_toml_source,
        LEGACY_TALK1_BEGIN_MARKER,
        LEGACY_TALK1_END_MARKER,
    )?;
    let generated_block = render_generated_examples_block();

    let updated = if let (Some(begin_index), Some(end_index)) = (
        cargo_toml_source.find(GENERATED_EXAMPLES_BEGIN_MARKER),
        cargo_toml_source.find(GENERATED_EXAMPLES_END_MARKER),
    ) {
        let block_end = end_index + GENERATED_EXAMPLES_END_MARKER.len();
        let trailing_newline_len = cargo_toml_source[block_end..]
            .chars()
            .next()
            .is_some_and(|character| character == '\n') as usize;
        let remove_end = block_end + trailing_newline_len;
        let mut rewritten = String::with_capacity(cargo_toml_source.len() + generated_block.len());
        rewritten.push_str(&cargo_toml_source[..begin_index]);
        rewritten.push_str(&generated_block);
        rewritten.push_str(&cargo_toml_source[remove_end..]);
        rewritten
    } else {
        let first_example_index = cargo_toml_source
            .find("\n[[example]]")
            .ok_or("Cargo.toml is missing [[example]] entries to replace")?;
        let first_test_index = cargo_toml_source
            .find("\n[[test]]")
            .ok_or("Cargo.toml is missing [[test]] section used as insertion anchor")?;
        if first_test_index <= first_example_index {
            return Err("Cargo.toml has unexpected ordering: [[test]] before [[example]]".into());
        }
        let mut rewritten = String::with_capacity(cargo_toml_source.len() + generated_block.len());
        rewritten.push_str(&cargo_toml_source[..first_example_index + 1]);
        rewritten.push_str(&generated_block);
        rewritten.push('\n');
        rewritten.push_str(&cargo_toml_source[first_test_index + 1..]);
        rewritten
    };

    write_if_changed(&cargo_toml_path, &updated)?;
    Ok(())
}

pub fn board_example_required_chip(example_name: &str) -> Option<&'static str> {
    for board_profile in BOARD_PROFILES {
        for base_name in board_template_example_base_names() {
            if board_template_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    None
}

fn cleanup_legacy_flat_generated_examples(examples_dir: &Path) -> Result<(), Box<dyn Error>> {
    let chip_features: std::collections::BTreeSet<&str> = BOARD_PROFILES
        .iter()
        .map(|board_profile| board_profile.chip_feature())
        .collect();
    for chip_feature in chip_features {
        let legacy_path = examples_dir.join(format!(
            "blinky_board_generic_{chip_feature}__{chip_feature}.rs"
        ));
        if legacy_path.exists() {
            fs::remove_file(legacy_path)?;
        }
    }
    Ok(())
}

fn cleanup_stale_nested_generated_examples(
    examples_dir: &Path,
    expected_paths: &[PathBuf],
) -> Result<(), Box<dyn Error>> {
    let expected_paths: std::collections::HashSet<PathBuf> =
        expected_paths.iter().cloned().collect();
    let top_level_dirs: std::collections::BTreeSet<&str> = BOARD_PROFILES
        .iter()
        .map(|board_profile| board_profile.chip_dir())
        .collect();
    let mut generated_filenames: Vec<String> = board_template_example_base_names()
        .iter()
        .map(|base_name| format!("{base_name}.rs"))
        .collect();
    generated_filenames.extend([
        "conway.rs".to_string(),
        "blinky.rs".to_string(),
        "led16x16_plus_1.rs".to_string(),
        "led16x16_plus_1_spi.rs".to_string(),
        "led16x16_and_builtin.rs".to_string(),
        "led16x16_and_builtin_spi.rs".to_string(),
    ]);

    for top_level_dir in &top_level_dirs {
        let chip_dir = examples_dir.join(top_level_dir);
        if !chip_dir.exists() {
            continue;
        }
        for board_dir_entry in fs::read_dir(&chip_dir)? {
            let board_dir_entry = board_dir_entry?;
            let board_dir_path = board_dir_entry.path();
            if !board_dir_path.is_dir() {
                continue;
            }
            for generated_filename in &generated_filenames {
                let candidate = board_dir_path.join(generated_filename);
                if !candidate.exists() || expected_paths.contains(&candidate) {
                    continue;
                }
                let existing = fs::read_to_string(&candidate)?;
                if existing.starts_with("// @generated by `cargo xtask generate-blinky-examples`")
                    || existing
                        .starts_with("// @generated by `cargo xtask generate-board-examples`")
                    || existing.starts_with("// @generated by cargo xtask generate-board-examples")
                {
                    fs::remove_file(&candidate)?;
                }
            }
        }
    }

    Ok(())
}

fn write_if_changed(path: &Path, contents: &str) -> Result<(), Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == contents => Ok(()),
        _ => {
            fs::write(path, contents)?;
            Ok(())
        }
    }
}

fn rustfmt_generated_files(paths: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    if paths.is_empty() {
        return Ok(());
    }

    let mut rustfmt_command = Command::new("rustfmt");
    rustfmt_command.arg("--edition").arg("2024");
    for path in paths {
        rustfmt_command.arg(path);
    }

    let rustfmt_status = rustfmt_command.status()?;
    if rustfmt_status.success() {
        Ok(())
    } else {
        Err("rustfmt failed for generated board examples".into())
    }
}
