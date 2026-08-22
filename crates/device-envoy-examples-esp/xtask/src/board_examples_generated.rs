use crate::boards::{AudioWiring, BOARD_PROFILES, validate_board_profiles};
use crate::example_specs::{
    BlinkyKind, blinky_built_in_led, blinky_kind, blinky_led_pin_num, led_strip1_built_in,
    led_strip1_pin_num,
};
use minijinja::{Environment, context};
use std::collections::BTreeMap;
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
enum BoardTemplateRequirement {
    Wifi,
    Audio,
    MinRmt(u8),
    MinSpi(u8),
    LargeStack,
}

struct BoardTemplateSpec {
    requirements: Vec<BoardTemplateRequirement>,
}

const BOARD_TEMPLATE_CONTEXT_KEYS: &[&str] = &[
    "example_name",
    "board_slug",
    "chip_name",
    "chip_feature",
    "spi_count",
    "blinky_kind",
    "led_pin",
    "built_in_led",
    "panel_pin",
    "led_strip1_pin",
    "led_strip1_built_in",
    "strip8_pin",
    "panel12x8_pin",
    "button_pin",
    "led2d_output_engine_note",
    "led2d_engine_clause",
    "led2d_runtime_resource",
    "init_and_start_statement",
    "ir_supported",
    "ir_pin",
    "ir_receiver0_pin",
    "ir_receiver1_pin",
    "ir_rx_channel",
    "ir_rx_channel2",
    "clock_supported",
    "force_portal_button_pin",
    "lcd_sda_pin",
    "lcd_scl_pin",
    "servo_bottom_pin",
    "servo_top_pin",
    "rfid_sck_pin",
    "rfid_mosi_pin",
    "rfid_miso_pin",
    "rfid_cs_pin",
    "rfid_rst_pin",
    "led4_cell_pins",
    "led4_segment_pins",
    "data_pin",
    "bit_clock_pin",
    "word_select_pin",
    "dma_identifier",
    "cyd_display_sck_pin",
    "cyd_display_mosi_pin",
    "cyd_display_miso_pin",
    "cyd_display_cs_pin",
    "cyd_display_dc_pin",
    "cyd_display_rst_pin",
    "cyd_display_backlight_pin",
    "cyd_touch_sck_pin",
    "cyd_touch_mosi_pin",
    "cyd_touch_miso_pin",
    "cyd_touch_cs_pin",
    "cyd_touch_irq_pin",
];

fn board_template_example_name(
    board_profile: crate::boards::BoardProfile,
    base_name: &str,
) -> String {
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
    discover_board_template_example_base_names_in_dir(
        templates_dir,
        templates_dir,
        &mut base_names,
    )?;
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

fn parse_board_template_spec(template_source: &str) -> Result<BoardTemplateSpec, Box<dyn Error>> {
    let Some(template_line) = template_source.lines().next() else {
        return Err("missing @board-example template header".into());
    };
    let Some(spec_fragment) = template_line.split("@board-example").nth(1) else {
        return Err("missing @board-example template header".into());
    };
    let spec_text = spec_fragment
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("#}")
        .trim();
    let mut requirements = Vec::new();
    for token in spec_text.split_whitespace() {
        let requirement = if token == "wifi" {
            BoardTemplateRequirement::Wifi
        } else if token == "audio" {
            BoardTemplateRequirement::Audio
        } else if let Some(count) = token.strip_prefix("min_rmt=") {
            BoardTemplateRequirement::MinRmt(count.parse()?)
        } else if let Some(count) = token.strip_prefix("min_spi=") {
            BoardTemplateRequirement::MinSpi(count.parse()?)
        } else if token == "large_stack" {
            BoardTemplateRequirement::LargeStack
        } else {
            return Err(format!(
                "invalid @board-example requirement `{token}` (expected `wifi`, `audio`, `min_rmt=<n>`, `min_spi=<n>`, or `large_stack`)"
            )
            .into());
        };
        requirements.push(requirement);
    }
    Ok(BoardTemplateSpec { requirements })
}

fn strip_board_template_header(template_source: &str) -> String {
    let mut lines = template_source.lines();
    let _header = lines.next();
    let remainder = lines.collect::<Vec<_>>().join("\n");
    if template_source.ends_with('\n') {
        format!("{remainder}\n")
    } else {
        remainder
    }
}

fn board_template_uses_jinja_context(template_source: &str) -> bool {
    let template_body = strip_board_template_header(template_source);
    template_body.contains("{%")
        || BOARD_TEMPLATE_CONTEXT_KEYS.iter().any(|key| {
            let marker_with_space = ["{{ ", key].concat();
            let marker_without_space = ["{{", key].concat();
            template_body.contains(&marker_with_space)
                || template_body.contains(&marker_without_space)
        })
}

fn board_requirement_supported(
    requirement: BoardTemplateRequirement,
    board_profile: crate::boards::BoardProfile,
) -> bool {
    match requirement {
        BoardTemplateRequirement::Wifi => board_profile.wifi_supported,
        BoardTemplateRequirement::Audio => board_profile.audio_wiring.is_some(),
        BoardTemplateRequirement::MinRmt(count) => board_profile.rmt_count >= count,
        BoardTemplateRequirement::MinSpi(count) => board_profile.spi_count >= count,
        BoardTemplateRequirement::LargeStack => !board_profile.stack_constrained,
    }
}

fn board_template_supports_board(
    template_spec: &BoardTemplateSpec,
    board_profile: crate::boards::BoardProfile,
) -> bool {
    template_spec
        .requirements
        .iter()
        .copied()
        .all(|requirement| board_requirement_supported(requirement, board_profile))
}

fn board_template_placeholder_reason(
    template_spec: &BoardTemplateSpec,
    board_profile: crate::boards::BoardProfile,
) -> Option<String> {
    for requirement in &template_spec.requirements {
        if board_requirement_supported(*requirement, board_profile) {
            continue;
        }
        return Some(match requirement {
            BoardTemplateRequirement::Wifi => format!(
                "this example requires Wi-Fi, and {} does not offer that resource",
                board_profile.chip_name()
            ),
            BoardTemplateRequirement::Audio => format!(
                "this example requires I2S resources, and {} does not offer that resource",
                board_profile.chip_name()
            ),
            BoardTemplateRequirement::MinRmt(count) => {
                let resource_word = if board_profile.rmt_count == 1 {
                    "resource"
                } else {
                    "resources"
                };
                format!(
                    "this example requires {count} RMT resources, and {} offers {} RMT {}",
                    board_profile.chip_name(),
                    board_profile.rmt_count,
                    resource_word
                )
            }
            BoardTemplateRequirement::MinSpi(count) => {
                let resource_word = if board_profile.spi_count == 1 {
                    "resource"
                } else {
                    "resources"
                };
                format!(
                    "this example requires {count} SPI resources, and {} offers {} SPI {}",
                    board_profile.chip_name(),
                    board_profile.spi_count,
                    resource_word
                )
            }
            BoardTemplateRequirement::LargeStack => format!(
                "this example needs a large RAM/linker budget, and {} is stack-constrained",
                board_profile.chip_name()
            ),
        });
    }
    None
}

fn board_template_placeholder_source(
    example_name: &str,
    base_name: &str,
    template_spec: &BoardTemplateSpec,
    board_profile: crate::boards::BoardProfile,
) -> String {
    let unsupported_reason = board_template_placeholder_reason(template_spec, board_profile);
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
//! {base_name}: not supported on this board profile.\n\
//!\n\
//! Wiring:\n\
{wiring_note}\
\n\
#![no_std]\n\
#![no_main]\n\
\n\
use core::{{convert::Infallible, future::pending}};\n\
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
    pending().await\n\
}}\n"
    )
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

pub fn check_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    let before = generated_tree_snapshot(workspace_root)?;
    generate_board_examples(workspace_root)?;
    let after = generated_tree_snapshot(workspace_root)?;
    if before != after {
        return Err(
            "generated ESP examples are stale; run `cargo xtask generate-board-examples` and review the changes"
                .into(),
        );
    }
    Ok(())
}

fn generated_tree_snapshot(
    workspace_root: &Path,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    let mut snapshot = BTreeMap::new();
    snapshot_directory(&workspace_root.join("examples"), &mut snapshot)?;
    let manifest_path = workspace_root.join("Cargo.toml");
    snapshot.insert(manifest_path.clone(), fs::read(manifest_path)?);
    Ok(snapshot)
}

fn snapshot_directory(
    directory: &Path,
    snapshot: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            snapshot_directory(&path, snapshot)?;
        } else {
            snapshot.insert(path.clone(), fs::read(path)?);
        }
    }
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
        let template_spec = parse_board_template_spec(&template_source)?;
        let template_uses_jinja_context = board_template_uses_jinja_context(&template_source);
        for board_profile in BOARD_PROFILES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = board_template_example_name(*board_profile, base_name);
            let uses_rmt80 = board_profile.rmt_count > 0;
            let generated_source = if !board_template_supports_board(&template_spec, *board_profile)
            {
                board_template_placeholder_source(
                    &example_name,
                    base_name,
                    &template_spec,
                    *board_profile,
                )
            } else if template_uses_jinja_context {
                let mut minijinja_environment = Environment::new();
                minijinja_environment.add_template(base_name.as_str(), &template_source)?;
                let audio_wiring = board_profile.audio_wiring.unwrap_or(AudioWiring {
                    data_pin_num: u8::MAX,
                    bit_clock_pin_num: u8::MAX,
                    word_select_pin_num: u8::MAX,
                    dma_identifier: "DMA_UNSUPPORTED_AUDIO",
                });
                minijinja_environment
                    .get_template(base_name.as_str())?
                    .render(context! {
                        example_name => example_name.as_str(),
                        board_slug => board_profile.board_slug(),
                        chip_name => board_profile.chip_name(),
                        chip_feature => board_profile.chip_feature(),
                        spi_count => board_profile.spi_count,
                        blinky_kind => match blinky_kind(*board_profile) {
                            BlinkyKind::Plain => "plain",
                            BlinkyKind::SmartRmt => "smart_rmt",
                            BlinkyKind::SmartSpi => "smart_spi",
                        },
                        led_pin => blinky_led_pin_num(*board_profile),
                        built_in_led => blinky_built_in_led(*board_profile),
                        panel_pin => board_profile.led_2d16x16_pin,
                        led_strip1_pin => led_strip1_pin_num(*board_profile),
                        led_strip1_built_in => led_strip1_built_in(*board_profile),
                        strip8_pin => board_profile.led_strip_len_8_pin,
                        panel12x8_pin => board_profile.led_2d12x8_pin,
                        button_pin => board_profile.button_pin,
                        led2d_output_engine_note => if uses_rmt80 {
                            "RMT output engine on this board profile (`RMT channel0`)"
                        } else {
                            "SPI output engine on this board profile (`SPI2`)"
                        },
                        led2d_engine_clause => if uses_rmt80 {
                            ""
                        } else {
                            "        engine: device_envoy_esp::led_strip::Engine::Spi,\n"
                        },
                        led2d_runtime_resource => if uses_rmt80 {
                            "rmt80.channel0"
                        } else {
                            "p.SPI2"
                        },
                        init_and_start_statement => if uses_rmt80 {
                            "init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);"
                        } else {
                            "init_and_start!(p);"
                        },
                        ir_supported => board_profile.rmt_count > 0,
                        ir_pin => board_profile.ir_pin_rx_channel.0,
                        ir_receiver0_pin => board_profile.ir_pin_rx_channel.0,
                        ir_receiver1_pin => board_profile.ir_pin_rx_channel2.0,
                        ir_rx_channel => board_profile.ir_pin_rx_channel.1,
                        ir_rx_channel2 => board_profile.ir_pin_rx_channel2.1,
                        clock_supported => board_profile.wifi_supported,
                        force_portal_button_pin => board_profile.button_pin,
                        lcd_sda_pin => board_profile.lcd_sda_pin,
                        lcd_scl_pin => board_profile.lcd_scl_pin,
                        servo_bottom_pin => board_profile.servo_pin,
                        servo_top_pin => board_profile.servo2_pin,
                        rfid_sck_pin => board_profile.rfid_wiring.sck_pin_num,
                        rfid_mosi_pin => board_profile.rfid_wiring.mosi_pin_num,
                        rfid_miso_pin => board_profile.rfid_wiring.miso_pin_num,
                        rfid_cs_pin => board_profile.rfid_wiring.cs_pin_num,
                        rfid_rst_pin => board_profile.rfid_wiring.rst_pin_num,
                        led4_cell_pins => board_profile.led4_cell_pins,
                        led4_segment_pins => board_profile.led4_segment_pins,
                        data_pin => audio_wiring.data_pin_num,
                        bit_clock_pin => audio_wiring.bit_clock_pin_num,
                        word_select_pin => audio_wiring.word_select_pin_num,
                        dma_identifier => audio_wiring.dma_identifier,
                        cyd_display_sck_pin => board_profile.cyd_display_wiring.sck_pin_num,
                        cyd_display_mosi_pin => board_profile.cyd_display_wiring.mosi_pin_num,
                        cyd_display_miso_pin => board_profile.cyd_display_wiring.miso_pin_num,
                        cyd_display_cs_pin => board_profile.cyd_display_wiring.cs_pin_num,
                        cyd_display_dc_pin => board_profile.cyd_display_wiring.dc_pin_num,
                        cyd_display_rst_pin => board_profile.cyd_display_wiring.rst_pin_num,
                        cyd_display_backlight_pin => board_profile.cyd_display_wiring.backlight_pin_num,
                        cyd_touch_sck_pin => board_profile.cyd_touch_wiring.sck_pin_num,
                        cyd_touch_mosi_pin => board_profile.cyd_touch_wiring.mosi_pin_num,
                        cyd_touch_miso_pin => board_profile.cyd_touch_wiring.miso_pin_num,
                        cyd_touch_cs_pin => board_profile.cyd_touch_wiring.cs_pin_num,
                        cyd_touch_irq_pin => board_profile.cyd_touch_wiring.irq_pin_num,
                    })?
            } else {
                strip_board_template_header(&template_source)
            };
            // Always write (not write_if_changed) so the file mtime is updated on every
            // check-all run.  write_if_changed suppresses the write when content is
            // unchanged, which leaves the mtime stale and allows Cargo to reuse a
            // cached artifact compiled against an older version of the crate.
            fs::write(&output_path, &generated_source)?;
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
