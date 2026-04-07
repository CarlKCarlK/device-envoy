use crate::boards::{validate_board_profiles, BOARD_PROFILES};
use crate::example_specs::{
    audio_bit_clock_pin_ident, audio_bit_clock_pin_num, audio_button_pin_ident,
    audio_button_pin_num, audio_data_pin_ident, audio_data_pin_num, audio_dma_ident,
    audio_example_name, audio_word_select_pin_ident, audio_word_select_pin_num,
    blinky_built_in_led, blinky_example_name, blinky_kind, blinky_led_pin_ident,
    blinky_led_pin_num, clock_example_name, clock_force_portal_button_pin_ident,
    clock_force_portal_button_pin_num, clock_lcd_scl_pin_ident, clock_lcd_scl_pin_num,
    clock_lcd_sda_pin_ident, clock_lcd_sda_pin_num, clock_led4_cell_pin_idents,
    clock_led4_cell_pin_nums, clock_led4_segment_pin_idents, clock_led4_segment_pin_nums,
    clock_led8x12_panel_pin_ident, clock_led8x12_panel_pin_num, clock_servos_bottom_pin_ident,
    clock_servos_bottom_pin_num, clock_servos_top_pin_ident, clock_servos_top_pin_num,
    conway_example_name, ir_example_name, ir_kepler_receiver0_pin_ident,
    ir_kepler_receiver0_pin_num, ir_pin2_ident, ir_pin2_num, ir_pin_ident, ir_pin_num,
    ir_rx_channel2_ident, ir_rx_channel2_num, ir_rx_channel_ident, ir_rx_channel_num,
    led16x16_example_name, led_strip1_built_in, led_strip1_pin_ident, led_strip1_pin_num,
    panel16x16_pin_ident, panel16x16_pin_num, supports_audio_examples, supports_clock_examples,
    supports_conway_example, supports_ir_examples, supports_led16x16_examples, talk1_example_name,
    talk1_panel12x8_pin_ident, talk1_panel12x8_pin_num, talk1_strip8_pin_ident,
    talk1_strip8_pin_num, BlinkyKind, AUDIO_EXAMPLE_BASE_NAMES, CLOCK_EXAMPLE_BASE_NAMES,
    IR_EXAMPLE_BASE_NAMES,
};
use minijinja::{context, Environment};
use minijinja::value::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const GENERATED_EXAMPLES_BEGIN_MARKER: &str = "# BEGIN GENERATED BOARD EXAMPLES";
const GENERATED_EXAMPLES_END_MARKER: &str = "# END GENERATED BOARD EXAMPLES";
const LEGACY_TALK1_BEGIN_MARKER: &str = "# BEGIN GENERATED TALK1 EXAMPLES";
const LEGACY_TALK1_END_MARKER: &str = "# END GENERATED TALK1 EXAMPLES";

const TALK1_BASE_NAMES: &[&str] = &[
    "a1_strip_8_blue_gray",
    "a3_strip_8_blue_white_blink_animate",
    "a4_strip_96_blue_white_dot",
    "b1_panel_12x8_rust_cursor",
    "b2_panel_12x8_text_graphics",
    "f1_dns",
];
const CONWAY_BASE_NAMES: &[&str] = &["conway"];
const BLINKY_BASE_NAMES: &[&str] = &["blinky"];
const LED16X16_BASE_NAMES: &[&str] = &["led16x16_plus_1", "led16x16_plus_1_spi"];

static PASSTHROUGH_EXAMPLE_BASE_NAMES: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq, Eq)]
enum PassthroughTemplateMode {
    Copy,
    Render,
}

fn passthrough_example_name(board_profile: crate::boards::BoardProfile, base_name: &str) -> String {
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

fn audio_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        data_pin_num => audio_data_pin_num(board_profile),
        data_pin_ident => audio_data_pin_ident(board_profile),
        bit_clock_pin_num => audio_bit_clock_pin_num(board_profile),
        bit_clock_pin_ident => audio_bit_clock_pin_ident(board_profile),
        word_select_pin_num => audio_word_select_pin_num(board_profile),
        word_select_pin_ident => audio_word_select_pin_ident(board_profile),
        button_pin_num => audio_button_pin_num(board_profile),
        button_pin_ident => audio_button_pin_ident(board_profile),
        dma_ident => audio_dma_ident(board_profile),
    }
}

fn ir_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        ir_pin_num => ir_pin_num(board_profile),
        ir_pin_ident => ir_pin_ident(board_profile),
        ir_receiver0_pin_num => ir_kepler_receiver0_pin_num(board_profile),
        ir_receiver0_pin_ident => ir_kepler_receiver0_pin_ident(board_profile),
        ir_receiver1_pin_num => ir_pin2_num(board_profile),
        ir_receiver1_pin_ident => ir_pin2_ident(board_profile),
        ir_rx_channel_num => ir_rx_channel_num(board_profile),
        ir_rx_channel_ident => ir_rx_channel_ident(board_profile),
        ir_rx_channel2_num => ir_rx_channel2_num(board_profile),
        ir_rx_channel2_ident => ir_rx_channel2_ident(board_profile),
    }
}

fn clock_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        force_portal_button_pin_num => clock_force_portal_button_pin_num(board_profile),
        force_portal_button_pin_ident => clock_force_portal_button_pin_ident(board_profile),
        lcd_sda_pin_num => clock_lcd_sda_pin_num(board_profile),
        lcd_sda_pin_ident => clock_lcd_sda_pin_ident(board_profile),
        lcd_scl_pin_num => clock_lcd_scl_pin_num(board_profile),
        lcd_scl_pin_ident => clock_lcd_scl_pin_ident(board_profile),
        led8x12_panel_pin_num => clock_led8x12_panel_pin_num(board_profile),
        led8x12_panel_pin_ident => clock_led8x12_panel_pin_ident(board_profile),
        servo_bottom_pin_num => clock_servos_bottom_pin_num(board_profile),
        servo_bottom_pin_ident => clock_servos_bottom_pin_ident(board_profile),
        servo_top_pin_num => clock_servos_top_pin_num(board_profile),
        servo_top_pin_ident => clock_servos_top_pin_ident(board_profile),
        led4_cell_pin_nums => clock_led4_cell_pin_nums(board_profile),
        led4_cell_pin_idents => clock_led4_cell_pin_idents(board_profile),
        led4_segment_pin_nums => clock_led4_segment_pin_nums(board_profile),
        led4_segment_pin_idents => clock_led4_segment_pin_idents(board_profile),
    }
}

fn conway_example_name_family(board_profile: crate::boards::BoardProfile, _base_name: &str) -> String {
    conway_example_name(board_profile)
}

fn conway_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        ir_pin_num => ir_pin_num(board_profile),
        ir_pin_ident => ir_pin_ident(board_profile),
        ir_rx_channel_num => ir_rx_channel_num(board_profile),
        ir_rx_channel_ident => ir_rx_channel_ident(board_profile),
    }
}

fn talk1_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        talk1_strip8_pin_num => talk1_strip8_pin_num(board_profile),
        talk1_strip8_pin_ident => talk1_strip8_pin_ident(board_profile),
        talk1_panel12x8_pin_num => talk1_panel12x8_pin_num(board_profile),
        talk1_panel12x8_pin_ident => talk1_panel12x8_pin_ident(board_profile),
        force_portal_button_pin_num => clock_force_portal_button_pin_num(board_profile),
        force_portal_button_pin_ident => clock_force_portal_button_pin_ident(board_profile),
    }
}

fn blinky_example_name_family(board_profile: crate::boards::BoardProfile, _base_name: &str) -> String {
    blinky_example_name(board_profile)
}

fn blinky_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    let led_pin_ident = blinky_led_pin_ident(board_profile);
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        led_pin_num => blinky_led_pin_num(board_profile),
        led_pin_ident => led_pin_ident.as_str(),
        built_in_led => blinky_built_in_led(board_profile),
    }
}

fn led16x16_example_name_family(
    board_profile: crate::boards::BoardProfile,
    base_name: &str,
) -> String {
    let use_spi = base_name == "led16x16_plus_1_spi";
    led16x16_example_name(board_profile, use_spi)
}

fn led16x16_example_context(board_profile: crate::boards::BoardProfile, _base_name: &str) -> Value {
    let panel_pin_ident = panel16x16_pin_ident(board_profile);
    let led_strip1_pin_ident_value = led_strip1_pin_ident(board_profile);
    context! {
        board_slug => board_profile.board_slug(),
        chip_name => board_profile.chip_name(),
        chip_feature => board_profile.chip_feature(),
        panel_pin_num => panel16x16_pin_num(board_profile),
        panel_pin_ident => panel_pin_ident.as_str(),
        led_strip1_pin_num => led_strip1_pin_num(board_profile),
        led_strip1_pin_ident => led_strip1_pin_ident_value.as_str(),
        led_strip1_built_in => led_strip1_built_in(board_profile),
    }
}

fn supports_all_boards(_board_profile: crate::boards::BoardProfile) -> bool {
    true
}

fn default_template_name(_board_profile: crate::boards::BoardProfile, base_name: &str) -> String {
    base_name.to_string()
}

fn default_output_relative_path(
    _board_profile: crate::boards::BoardProfile,
    base_name: &str,
) -> PathBuf {
    PathBuf::from(format!("{base_name}.rs"))
}

fn talk1_template_name(_board_profile: crate::boards::BoardProfile, base_name: &str) -> String {
    format!("talk1_{base_name}")
}

fn talk1_output_relative_path(
    _board_profile: crate::boards::BoardProfile,
    base_name: &str,
) -> PathBuf {
    PathBuf::from("talk1").join(format!("{base_name}.rs"))
}

fn blinky_template_name(board_profile: crate::boards::BoardProfile, _base_name: &str) -> String {
    match blinky_kind(board_profile) {
        BlinkyKind::Plain => "blinky_plain",
        BlinkyKind::SmartRmt => "blinky_rmt",
        BlinkyKind::SmartSpi => "blinky_spi",
    }
    .to_string()
}

fn blinky_output_relative_path(
    _board_profile: crate::boards::BoardProfile,
    _base_name: &str,
) -> PathBuf {
    PathBuf::from("blinky.rs")
}

fn generate_family_examples(
    minijinja_environment: &Environment,
    examples_dir: &Path,
    expected_generated_paths: &mut Vec<PathBuf>,
    base_names: &[&str],
    supports_board: fn(crate::boards::BoardProfile) -> bool,
    example_name_fn: fn(crate::boards::BoardProfile, &str) -> String,
    context_fn: fn(crate::boards::BoardProfile, &str) -> Value,
    template_name_fn: fn(crate::boards::BoardProfile, &str) -> String,
    output_relative_path_fn: fn(crate::boards::BoardProfile, &str) -> PathBuf,
) -> Result<(), Box<dyn Error>> {
    for board_profile in BOARD_PROFILES {
        if !supports_board(*board_profile) {
            continue;
        }
        for base_name in base_names {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(output_relative_path_fn(*board_profile, base_name));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = example_name_fn(*board_profile, base_name);
            let extra_context = context_fn(*board_profile, base_name);
            let template_name = template_name_fn(*board_profile, base_name);
            let generated_source = minijinja_environment
                .get_template(&template_name)?
                .render(context! {
                    example_name => example_name.as_str(),
                    ..extra_context
                })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }
    Ok(())
}

fn add_family_generated_names(
    names: &mut Vec<String>,
    base_names: &[&str],
    supports_board: fn(crate::boards::BoardProfile) -> bool,
    example_name_fn: fn(crate::boards::BoardProfile, &str) -> String,
) {
    for board_profile in BOARD_PROFILES {
        if !supports_board(*board_profile) {
            continue;
        }
        for base_name in base_names {
            names.push(example_name_fn(*board_profile, base_name));
        }
    }
}

fn add_family_manifest_entries(
    entries: &mut Vec<ExampleManifestEntry>,
    base_names: &[&str],
    supports_board: fn(crate::boards::BoardProfile) -> bool,
    example_name_fn: fn(crate::boards::BoardProfile, &str) -> String,
    output_relative_path_fn: fn(crate::boards::BoardProfile, &str) -> PathBuf,
) {
    for board_profile in BOARD_PROFILES {
        if !supports_board(*board_profile) {
            continue;
        }
        for base_name in base_names {
            entries.push(ExampleManifestEntry {
                name: example_name_fn(*board_profile, base_name),
                path: format!(
                    "examples/{}/{}/{}",
                    board_profile.chip_dir(),
                    board_profile.board_dir(),
                    output_relative_path_fn(*board_profile, base_name).display()
                ),
            });
        }
    }
}

fn find_family_required_chip(
    example_name: &str,
    base_names: &[&str],
    supports_board: fn(crate::boards::BoardProfile) -> bool,
    example_name_fn: fn(crate::boards::BoardProfile, &str) -> String,
) -> Option<&'static str> {
    for board_profile in BOARD_PROFILES {
        if !supports_board(*board_profile) {
            continue;
        }
        for base_name in base_names {
            if example_name_fn(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }
    None
}

fn passthrough_example_base_names() -> &'static [String] {
    PASSTHROUGH_EXAMPLE_BASE_NAMES
        .get_or_init(|| {
            discover_passthrough_example_base_names(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/templates"),
            )
            .unwrap_or_else(|error| {
                panic!("failed to discover passthrough templates: {error}");
            })
        })
        .as_slice()
}

fn discover_passthrough_example_base_names(
    templates_dir: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut base_names = Vec::new();
    for template_entry in fs::read_dir(templates_dir)? {
        let template_entry = template_entry?;
        let template_path = template_entry.path();
        if !template_path.is_file() {
            continue;
        }
        let Some(file_name) = template_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(base_name) = file_name.strip_suffix(".rs.j2") else {
            continue;
        };
        if !is_passthrough_template(base_name) {
            continue;
        }
        base_names.push(base_name.to_string());
    }
    base_names.sort();
    Ok(base_names)
}

fn is_passthrough_template(base_name: &str) -> bool {
    if base_name == "conway"
        || base_name == "blinky_plain"
        || base_name == "blinky_rmt"
        || base_name == "blinky_spi"
        || base_name == "led16x16_plus_1"
        || base_name == "led16x16_plus_1_spi"
    {
        return false;
    }
    if AUDIO_EXAMPLE_BASE_NAMES.iter().any(|name| name == &base_name) {
        return false;
    }
    if IR_EXAMPLE_BASE_NAMES.iter().any(|name| name == &base_name) {
        return false;
    }
    if CLOCK_EXAMPLE_BASE_NAMES.iter().any(|name| name == &base_name) {
        return false;
    }
    true
}

fn passthrough_template_mode(
    template_source: &str,
) -> Result<PassthroughTemplateMode, Box<dyn Error>> {
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
            "copy" => Ok(PassthroughTemplateMode::Copy),
            "render" => Ok(PassthroughTemplateMode::Render),
            _ => Err(format!(
                "invalid @board-example mode `{mode}` (expected `copy` or `render`)"
            )
            .into()),
        };
    }
    Ok(PassthroughTemplateMode::Copy)
}

pub fn generate_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    validate_board_profiles()?;

    let examples_dir = workspace_root.join("examples");
    let templates_dir = examples_dir.join("templates");
    let talk1_templates_dir = templates_dir.join("talk1");

    let blinky_plain_template = fs::read_to_string(templates_dir.join("blinky_plain.rs.j2"))?;
    let blinky_rmt_template = fs::read_to_string(templates_dir.join("blinky_rmt.rs.j2"))?;
    let blinky_spi_template = fs::read_to_string(templates_dir.join("blinky_spi.rs.j2"))?;
    let led16x16_plus_1_template = fs::read_to_string(templates_dir.join("led16x16_plus_1.rs.j2"))?;
    let led16x16_plus_1_spi_template =
        fs::read_to_string(templates_dir.join("led16x16_plus_1_spi.rs.j2"))?;
    let audio_template = fs::read_to_string(templates_dir.join("audio.rs.j2"))?;
    let audio_example1_template = fs::read_to_string(templates_dir.join("audio_example1.rs.j2"))?;
    let audio_example1_trait_template =
        fs::read_to_string(templates_dir.join("audio_example1_trait.rs.j2"))?;
    let audio_example2_trait_template =
        fs::read_to_string(templates_dir.join("audio_example2_trait.rs.j2"))?;
    let audio_example3_trait_template =
        fs::read_to_string(templates_dir.join("audio_example3_trait.rs.j2"))?;
    let ir_template = fs::read_to_string(templates_dir.join("ir.rs.j2"))?;
    let ir_example1_trait_template =
        fs::read_to_string(templates_dir.join("ir_example1_trait.rs.j2"))?;
    let ir_kepler_template = fs::read_to_string(templates_dir.join("ir_kepler.rs.j2"))?;
    let ir_kepler_example1_trait_template =
        fs::read_to_string(templates_dir.join("ir_kepler_example1_trait.rs.j2"))?;
    let ir_keplers_template = fs::read_to_string(templates_dir.join("ir_keplers.rs.j2"))?;
    let ir_mapping_example1_trait_template =
        fs::read_to_string(templates_dir.join("ir_mapping_example1_trait.rs.j2"))?;
    let conway_template = fs::read_to_string(templates_dir.join("conway.rs.j2"))?;
    let clock_console_simple_template =
        fs::read_to_string(templates_dir.join("clock_console_simple.rs.j2"))?;
    let clock_lcd_template = fs::read_to_string(templates_dir.join("clock_lcd.rs.j2"))?;
    let clock_led4_template = fs::read_to_string(templates_dir.join("clock_led4.rs.j2"))?;
    let clock_led8x12_template = fs::read_to_string(templates_dir.join("clock_led8x12.rs.j2"))?;
    let clock_servos_template = fs::read_to_string(templates_dir.join("clock_servos.rs.j2"))?;
    let clock_sync_example1_trait_template =
        fs::read_to_string(templates_dir.join("clock_sync_example1_trait.rs.j2"))?;
    let talk1_a1_strip_8_blue_gray_template =
        fs::read_to_string(talk1_templates_dir.join("a1_strip_8_blue_gray.rs.j2"))?;
    let talk1_a3_strip_8_blue_white_blink_animate_template =
        fs::read_to_string(talk1_templates_dir.join("a3_strip_8_blue_white_blink_animate.rs.j2"))?;
    let talk1_a4_strip_96_blue_white_dot_template =
        fs::read_to_string(talk1_templates_dir.join("a4_strip_96_blue_white_dot.rs.j2"))?;
    let talk1_b1_panel_12x8_rust_cursor_template =
        fs::read_to_string(talk1_templates_dir.join("b1_panel_12x8_rust_cursor.rs.j2"))?;
    let talk1_b2_panel_12x8_text_graphics_template =
        fs::read_to_string(talk1_templates_dir.join("b2_panel_12x8_text_graphics.rs.j2"))?;
    let talk1_f1_dns_template = fs::read_to_string(talk1_templates_dir.join("f1_dns.rs.j2"))?;

    let mut minijinja_environment = Environment::new();
    minijinja_environment.add_template("blinky_plain", &blinky_plain_template)?;
    minijinja_environment.add_template("blinky_rmt", &blinky_rmt_template)?;
    minijinja_environment.add_template("blinky_spi", &blinky_spi_template)?;
    minijinja_environment.add_template("led16x16_plus_1", &led16x16_plus_1_template)?;
    minijinja_environment.add_template("led16x16_plus_1_spi", &led16x16_plus_1_spi_template)?;
    minijinja_environment.add_template("audio", &audio_template)?;
    minijinja_environment.add_template("audio_example1", &audio_example1_template)?;
    minijinja_environment.add_template("audio_example1_trait", &audio_example1_trait_template)?;
    minijinja_environment.add_template("audio_example2_trait", &audio_example2_trait_template)?;
    minijinja_environment.add_template("audio_example3_trait", &audio_example3_trait_template)?;
    minijinja_environment.add_template("ir", &ir_template)?;
    minijinja_environment.add_template("ir_example1_trait", &ir_example1_trait_template)?;
    minijinja_environment.add_template("ir_kepler", &ir_kepler_template)?;
    minijinja_environment.add_template(
        "ir_kepler_example1_trait",
        &ir_kepler_example1_trait_template,
    )?;
    minijinja_environment.add_template("ir_keplers", &ir_keplers_template)?;
    minijinja_environment.add_template(
        "ir_mapping_example1_trait",
        &ir_mapping_example1_trait_template,
    )?;
    minijinja_environment.add_template("conway", &conway_template)?;
    minijinja_environment.add_template("clock_console_simple", &clock_console_simple_template)?;
    minijinja_environment.add_template("clock_lcd", &clock_lcd_template)?;
    minijinja_environment.add_template("clock_led4", &clock_led4_template)?;
    minijinja_environment.add_template("clock_led8x12", &clock_led8x12_template)?;
    minijinja_environment.add_template("clock_servos", &clock_servos_template)?;
    minijinja_environment.add_template(
        "clock_sync_example1_trait",
        &clock_sync_example1_trait_template,
    )?;
    minijinja_environment.add_template(
        "talk1_a1_strip_8_blue_gray",
        &talk1_a1_strip_8_blue_gray_template,
    )?;
    minijinja_environment.add_template(
        "talk1_a3_strip_8_blue_white_blink_animate",
        &talk1_a3_strip_8_blue_white_blink_animate_template,
    )?;
    minijinja_environment.add_template(
        "talk1_a4_strip_96_blue_white_dot",
        &talk1_a4_strip_96_blue_white_dot_template,
    )?;
    minijinja_environment.add_template(
        "talk1_b1_panel_12x8_rust_cursor",
        &talk1_b1_panel_12x8_rust_cursor_template,
    )?;
    minijinja_environment.add_template(
        "talk1_b2_panel_12x8_text_graphics",
        &talk1_b2_panel_12x8_text_graphics_template,
    )?;
    minijinja_environment.add_template("talk1_f1_dns", &talk1_f1_dns_template)?;

    cleanup_legacy_flat_generated_examples(&examples_dir)?;

    let mut expected_generated_paths = Vec::new();
    generate_passthrough_files(&templates_dir, &examples_dir, &mut expected_generated_paths)?;

    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &AUDIO_EXAMPLE_BASE_NAMES,
        supports_audio_examples,
        audio_example_name,
        audio_example_context,
        default_template_name,
        default_output_relative_path,
    )?;

    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &LED16X16_BASE_NAMES,
        supports_led16x16_examples,
        led16x16_example_name_family,
        led16x16_example_context,
        default_template_name,
        default_output_relative_path,
    )?;

    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &IR_EXAMPLE_BASE_NAMES,
        supports_ir_examples,
        ir_example_name,
        ir_example_context,
        default_template_name,
        default_output_relative_path,
    )?;
    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &CONWAY_BASE_NAMES,
        supports_conway_example,
        conway_example_name_family,
        conway_example_context,
        default_template_name,
        default_output_relative_path,
    )?;
    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &CLOCK_EXAMPLE_BASE_NAMES,
        supports_clock_examples,
        clock_example_name,
        clock_example_context,
        default_template_name,
        default_output_relative_path,
    )?;
    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &TALK1_BASE_NAMES,
        supports_all_boards,
        talk1_example_name,
        talk1_example_context,
        talk1_template_name,
        talk1_output_relative_path,
    )?;
    generate_family_examples(
        &minijinja_environment,
        &examples_dir,
        &mut expected_generated_paths,
        &BLINKY_BASE_NAMES,
        supports_all_boards,
        blinky_example_name_family,
        blinky_example_context,
        blinky_template_name,
        blinky_output_relative_path,
    )?;

    rustfmt_generated_files(&expected_generated_paths)?;
    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;
    sync_generated_examples_in_cargo_toml(workspace_root)?;

    Ok(())
}

fn generate_passthrough_files(
    example_templates_dir: &Path,
    examples_dir: &Path,
    expected_generated_paths: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    for base_name in passthrough_example_base_names() {
        let template_path = example_templates_dir.join(format!("{base_name}.rs.j2"));
        let template_source = fs::read_to_string(&template_path)?;
        let template_mode = passthrough_template_mode(&template_source)?;
        for board_profile in BOARD_PROFILES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let generated_source = if template_mode == PassthroughTemplateMode::Render {
                let mut minijinja_environment = Environment::new();
                minijinja_environment.add_template(base_name.as_str(), &template_source)?;
                minijinja_environment
                    .get_template(base_name.as_str())?
                    .render(context! {
                    example_name => passthrough_example_name(*board_profile, base_name),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    talk1_strip8_pin_num => talk1_strip8_pin_num(*board_profile),
                    talk1_strip8_pin_ident => talk1_strip8_pin_ident(*board_profile),
                    force_portal_button_pin_num => clock_force_portal_button_pin_num(*board_profile),
                    force_portal_button_pin_ident => clock_force_portal_button_pin_ident(*board_profile),
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
        for base_name in passthrough_example_base_names() {
            names.push(passthrough_example_name(*board_profile, base_name));
        }
    }
    add_family_generated_names(
        &mut names,
        &AUDIO_EXAMPLE_BASE_NAMES,
        supports_audio_examples,
        audio_example_name,
    );
    add_family_generated_names(
        &mut names,
        &IR_EXAMPLE_BASE_NAMES,
        supports_ir_examples,
        ir_example_name,
    );
    add_family_generated_names(
        &mut names,
        &CONWAY_BASE_NAMES,
        supports_conway_example,
        conway_example_name_family,
    );
    add_family_generated_names(
        &mut names,
        &CLOCK_EXAMPLE_BASE_NAMES,
        supports_clock_examples,
        clock_example_name,
    );
    add_family_generated_names(
        &mut names,
        &TALK1_BASE_NAMES,
        supports_all_boards,
        talk1_example_name,
    );
    add_family_generated_names(
        &mut names,
        &BLINKY_BASE_NAMES,
        supports_all_boards,
        blinky_example_name_family,
    );
    add_family_generated_names(
        &mut names,
        &LED16X16_BASE_NAMES,
        supports_led16x16_examples,
        led16x16_example_name_family,
    );
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
        for base_name in passthrough_example_base_names() {
            entries.push(ExampleManifestEntry {
                name: passthrough_example_name(*board_profile, base_name),
                path: format!(
                    "examples/{}/{}/{}.rs",
                    board_profile.chip_dir(),
                    board_profile.board_dir(),
                    base_name
                ),
            });
        }
    }

    add_family_manifest_entries(
        &mut entries,
        &AUDIO_EXAMPLE_BASE_NAMES,
        supports_audio_examples,
        audio_example_name,
        default_output_relative_path,
    );

    add_family_manifest_entries(
        &mut entries,
        &IR_EXAMPLE_BASE_NAMES,
        supports_ir_examples,
        ir_example_name,
        default_output_relative_path,
    );

    add_family_manifest_entries(
        &mut entries,
        &CONWAY_BASE_NAMES,
        supports_conway_example,
        conway_example_name_family,
        default_output_relative_path,
    );
    add_family_manifest_entries(
        &mut entries,
        &CLOCK_EXAMPLE_BASE_NAMES,
        supports_clock_examples,
        clock_example_name,
        default_output_relative_path,
    );
    add_family_manifest_entries(
        &mut entries,
        &TALK1_BASE_NAMES,
        supports_all_boards,
        talk1_example_name,
        talk1_output_relative_path,
    );
    add_family_manifest_entries(
        &mut entries,
        &BLINKY_BASE_NAMES,
        supports_all_boards,
        blinky_example_name_family,
        blinky_output_relative_path,
    );
    add_family_manifest_entries(
        &mut entries,
        &LED16X16_BASE_NAMES,
        supports_led16x16_examples,
        led16x16_example_name_family,
        default_output_relative_path,
    );

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
        (Some(_), None) | (None, Some(_)) => Err(format!(
            "Cargo.toml marker mismatch: {begin_marker} / {end_marker}"
        )
        .into()),
        (Some(begin_index), Some(end_index)) => {
            let end_of_marker = end_index + end_marker.len();
            let trailing_newline_len = source[end_of_marker..]
                .chars()
                .next()
                .is_some_and(|character| character == '\n') as usize;
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
        for base_name in passthrough_example_base_names() {
            if passthrough_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &AUDIO_EXAMPLE_BASE_NAMES,
        supports_audio_examples,
        audio_example_name,
    ) {
        return Some(required_chip_feature);
    }

    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &IR_EXAMPLE_BASE_NAMES,
        supports_ir_examples,
        ir_example_name,
    ) {
        return Some(required_chip_feature);
    }

    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &CONWAY_BASE_NAMES,
        supports_conway_example,
        conway_example_name_family,
    ) {
        return Some(required_chip_feature);
    }
    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &CLOCK_EXAMPLE_BASE_NAMES,
        supports_clock_examples,
        clock_example_name,
    ) {
        return Some(required_chip_feature);
    }
    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &TALK1_BASE_NAMES,
        supports_all_boards,
        talk1_example_name,
    ) {
        return Some(required_chip_feature);
    }
    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &BLINKY_BASE_NAMES,
        supports_all_boards,
        blinky_example_name_family,
    ) {
        return Some(required_chip_feature);
    }
    if let Some(required_chip_feature) = find_family_required_chip(
        example_name,
        &LED16X16_BASE_NAMES,
        supports_led16x16_examples,
        led16x16_example_name_family,
    ) {
        return Some(required_chip_feature);
    }

    None
}

fn cleanup_legacy_flat_generated_examples(examples_dir: &Path) -> Result<(), Box<dyn Error>> {
    let legacy_paths: [PathBuf; 7] = [
        examples_dir.join("blinky_board_generic_esp32__esp32.rs"),
        examples_dir.join("blinky_board_generic_esp32c2__esp32c2.rs"),
        examples_dir.join("blinky_board_generic_esp32c3__esp32c3.rs"),
        examples_dir.join("blinky_board_generic_esp32c6__esp32c6.rs"),
        examples_dir.join("blinky_board_generic_esp32h2__esp32h2.rs"),
        examples_dir.join("blinky_board_generic_esp32s2__esp32s2.rs"),
        examples_dir.join("blinky_board_generic_esp32s3__esp32s3.rs"),
    ];
    for legacy_path in legacy_paths {
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
    let top_level_dirs = ["esp32", "c2", "c3", "c6", "h2", "s2", "s3"];
    let mut generated_filenames: Vec<String> = passthrough_example_base_names()
        .iter()
        .map(|base_name| format!("{base_name}.rs"))
        .collect();
    generated_filenames.extend([
        "audio.rs".to_string(),
        "audio_example1.rs".to_string(),
        "audio_example1_trait.rs".to_string(),
        "audio_example2_trait.rs".to_string(),
        "audio_example3_trait.rs".to_string(),
        "ir.rs".to_string(),
        "ir_example1_trait.rs".to_string(),
        "ir_kepler.rs".to_string(),
        "ir_kepler_example1_trait.rs".to_string(),
        "ir_keplers.rs".to_string(),
        "ir_mapping_example1_trait.rs".to_string(),
        "conway.rs".to_string(),
        "clock_console_simple.rs".to_string(),
        "clock_lcd.rs".to_string(),
        "clock_led4.rs".to_string(),
        "clock_led8x12.rs".to_string(),
        "clock_servos.rs".to_string(),
        "clock_sync_example1_trait.rs".to_string(),
        "talk1/a1_strip_8_blue_gray.rs".to_string(),
        "talk1/a3_strip_8_blue_white_blink_animate.rs".to_string(),
        "talk1/a4_strip_96_blue_white_dot.rs".to_string(),
        "talk1/b1_panel_12x8_rust_cursor.rs".to_string(),
        "talk1/b2_panel_12x8_text_graphics.rs".to_string(),
        "talk1/f1_dns.rs".to_string(),
        "blinky.rs".to_string(),
        "led16x16_plus_1.rs".to_string(),
        "led16x16_plus_1_spi.rs".to_string(),
        "led16x16_and_builtin.rs".to_string(),
        "led16x16_and_builtin_spi.rs".to_string(),
    ]);

    for top_level_dir in top_level_dirs {
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
