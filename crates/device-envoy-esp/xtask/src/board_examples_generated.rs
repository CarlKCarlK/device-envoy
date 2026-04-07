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
    IR_EXAMPLE_BASE_NAMES, LED16X16_VARIANTS,
};
use minijinja::{context, Environment};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PASSTHROUGH_EXAMPLE_BASE_NAMES: &[&str] = &[
    "button_example1_trait",
    "button_read",
    "flash_block_example1_trait",
    "lcd_text",
    "lcd_text_example1_trait",
    "lcd_texts",
    "led16x16test",
    "led16x16test_async",
    "led2d",
    "led2d_example1_trait",
    "led2d_example2_trait",
    "led_example1_trait",
    "led_probe_c3_d4_d6",
    "led_probe_c3_gpio_sweep",
    "led_strip8_spi",
    "led_strip_example1_trait",
    "led_strip_example2_trait",
    "led_strip_len8",
    "rfid",
    "servo_basic",
    "servo_example1_trait",
    "servo_player_example1_trait",
    "servo_player_example2_trait",
    "servos",
    "wifi_auto_custom_checkbox",
    "wifi_auto_example1_trait",
    "wifi_auto_force_button",
    "wifi_dns_hex",
    "wifi_scan",
];

const TALK1_BASE_NAMES: &[&str] = &[
    "a1_strip_8_blue_gray",
    "a3_strip_8_blue_white_blink_animate",
    "a4_strip_96_blue_white_dot",
    "b1_panel_12x8_rust_cursor",
    "b2_panel_12x8_text_graphics",
    "f1_dns",
];

fn passthrough_example_name(board_profile: crate::boards::BoardProfile, base_name: &str) -> String {
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
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

    for board_profile in BOARD_PROFILES {
        let output_path = examples_dir
            .join(board_profile.chip_dir())
            .join(board_profile.board_dir())
            .join("blinky.rs");
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }

        let template_name = match blinky_kind(*board_profile) {
            BlinkyKind::Plain => "blinky_plain",
            BlinkyKind::SmartRmt => "blinky_rmt",
            BlinkyKind::SmartSpi => "blinky_spi",
        };
        let example_name = blinky_example_name(*board_profile);
        let led_pin_ident = blinky_led_pin_ident(*board_profile);

        let generated_source =
            minijinja_environment
                .get_template(template_name)?
                .render(context! {
                    example_name => example_name.as_str(),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    led_pin_num => blinky_led_pin_num(*board_profile),
                    led_pin_ident => led_pin_ident.as_str(),
                    built_in_led => blinky_built_in_led(*board_profile),
                })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    for board_profile in BOARD_PROFILES {
        if !supports_audio_examples(*board_profile) {
            continue;
        }
        for base_name in AUDIO_EXAMPLE_BASE_NAMES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = audio_example_name(*board_profile, base_name);
            let generated_source =
                minijinja_environment
                    .get_template(base_name)?
                    .render(context! {
                        example_name => example_name.as_str(),
                        board_slug => board_profile.board_slug(),
                        chip_name => board_profile.chip_name(),
                        chip_feature => board_profile.chip_feature(),
                        data_pin_num => audio_data_pin_num(*board_profile),
                        data_pin_ident => audio_data_pin_ident(*board_profile),
                        bit_clock_pin_num => audio_bit_clock_pin_num(*board_profile),
                        bit_clock_pin_ident => audio_bit_clock_pin_ident(*board_profile),
                        word_select_pin_num => audio_word_select_pin_num(*board_profile),
                        word_select_pin_ident => audio_word_select_pin_ident(*board_profile),
                        button_pin_num => audio_button_pin_num(*board_profile),
                        button_pin_ident => audio_button_pin_ident(*board_profile),
                        dma_ident => audio_dma_ident(*board_profile),
                    })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_led16x16_examples(*board_profile) {
            continue;
        }
        for use_spi in LED16X16_VARIANTS {
            let output_filename = if use_spi {
                "led16x16_plus_1_spi.rs"
            } else {
                "led16x16_plus_1.rs"
            };
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(output_filename);
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let template_name = if use_spi {
                "led16x16_plus_1_spi"
            } else {
                "led16x16_plus_1"
            };
            let example_name = led16x16_example_name(*board_profile, use_spi);
            let panel_pin_ident = panel16x16_pin_ident(*board_profile);
            let led_strip1_pin_ident = led_strip1_pin_ident(*board_profile);
            let generated_source =
                minijinja_environment
                    .get_template(template_name)?
                    .render(context! {
                        example_name => example_name.as_str(),
                        board_slug => board_profile.board_slug(),
                        chip_name => board_profile.chip_name(),
                        chip_feature => board_profile.chip_feature(),
                        panel_pin_num => panel16x16_pin_num(*board_profile),
                        panel_pin_ident => panel_pin_ident.as_str(),
                        led_strip1_pin_num => led_strip1_pin_num(*board_profile),
                        led_strip1_pin_ident => led_strip1_pin_ident.as_str(),
                        led_strip1_built_in => led_strip1_built_in(*board_profile),
                    })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_ir_examples(*board_profile) {
            continue;
        }
        for base_name in IR_EXAMPLE_BASE_NAMES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = ir_example_name(*board_profile, base_name);
            let generated_source =
                minijinja_environment
                    .get_template(base_name)?
                    .render(context! {
                        example_name => example_name.as_str(),
                        board_slug => board_profile.board_slug(),
                        chip_name => board_profile.chip_name(),
                        chip_feature => board_profile.chip_feature(),
                        ir_pin_num => ir_pin_num(*board_profile),
                        ir_pin_ident => ir_pin_ident(*board_profile),
                        ir_receiver0_pin_num => ir_kepler_receiver0_pin_num(*board_profile),
                        ir_receiver0_pin_ident => ir_kepler_receiver0_pin_ident(*board_profile),
                        ir_receiver1_pin_num => ir_pin2_num(*board_profile),
                        ir_receiver1_pin_ident => ir_pin2_ident(*board_profile),
                        ir_rx_channel_num => ir_rx_channel_num(*board_profile),
                        ir_rx_channel_ident => ir_rx_channel_ident(*board_profile),
                        ir_rx_channel2_num => ir_rx_channel2_num(*board_profile),
                        ir_rx_channel2_ident => ir_rx_channel2_ident(*board_profile),
                    })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_conway_example(*board_profile) {
            continue;
        }
        let output_path = examples_dir
            .join(board_profile.chip_dir())
            .join(board_profile.board_dir())
            .join("conway.rs");
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }
        let example_name = conway_example_name(*board_profile);
        let generated_source = minijinja_environment
            .get_template("conway")?
            .render(context! {
                example_name => example_name.as_str(),
                board_slug => board_profile.board_slug(),
                chip_name => board_profile.chip_name(),
                chip_feature => board_profile.chip_feature(),
                ir_pin_num => ir_pin_num(*board_profile),
                ir_pin_ident => ir_pin_ident(*board_profile),
                ir_rx_channel_num => ir_rx_channel_num(*board_profile),
                ir_rx_channel_ident => ir_rx_channel_ident(*board_profile),
            })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    for board_profile in BOARD_PROFILES {
        if !supports_clock_examples(*board_profile) {
            continue;
        }
        for base_name in CLOCK_EXAMPLE_BASE_NAMES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = clock_example_name(*board_profile, base_name);
            let generated_source = minijinja_environment
                .get_template(base_name)?
                .render(context! {
                    example_name => example_name.as_str(),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    force_portal_button_pin_num => clock_force_portal_button_pin_num(*board_profile),
                    force_portal_button_pin_ident => clock_force_portal_button_pin_ident(*board_profile),
                    lcd_sda_pin_num => clock_lcd_sda_pin_num(*board_profile),
                    lcd_sda_pin_ident => clock_lcd_sda_pin_ident(*board_profile),
                    lcd_scl_pin_num => clock_lcd_scl_pin_num(*board_profile),
                    lcd_scl_pin_ident => clock_lcd_scl_pin_ident(*board_profile),
                    led8x12_panel_pin_num => clock_led8x12_panel_pin_num(*board_profile),
                    led8x12_panel_pin_ident => clock_led8x12_panel_pin_ident(*board_profile),
                    servo_bottom_pin_num => clock_servos_bottom_pin_num(*board_profile),
                    servo_bottom_pin_ident => clock_servos_bottom_pin_ident(*board_profile),
                    servo_top_pin_num => clock_servos_top_pin_num(*board_profile),
                    servo_top_pin_ident => clock_servos_top_pin_ident(*board_profile),
                    led4_cell_pin_nums => clock_led4_cell_pin_nums(*board_profile),
                    led4_cell_pin_idents => clock_led4_cell_pin_idents(*board_profile),
                    led4_segment_pin_nums => clock_led4_segment_pin_nums(*board_profile),
                    led4_segment_pin_idents => clock_led4_segment_pin_idents(*board_profile),
                })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    for board_profile in BOARD_PROFILES {
        for base_name in TALK1_BASE_NAMES {
            let template_name = format!("talk1_{base_name}");
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join("talk1")
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
            let example_name = talk1_example_name(*board_profile, base_name);
            let generated_source = minijinja_environment
                .get_template(&template_name)?
                .render(context! {
                    example_name => example_name.as_str(),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    talk1_strip8_pin_num => talk1_strip8_pin_num(*board_profile),
                    talk1_strip8_pin_ident => talk1_strip8_pin_ident(*board_profile),
                    talk1_panel12x8_pin_num => talk1_panel12x8_pin_num(*board_profile),
                    talk1_panel12x8_pin_ident => talk1_panel12x8_pin_ident(*board_profile),
                    force_portal_button_pin_num => clock_force_portal_button_pin_num(*board_profile),
                    force_portal_button_pin_ident => clock_force_portal_button_pin_ident(*board_profile),
                })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    rustfmt_generated_files(&expected_generated_paths)?;
    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;

    Ok(())
}

fn generate_passthrough_files(
    example_templates_dir: &Path,
    examples_dir: &Path,
    expected_generated_paths: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    for base_name in PASSTHROUGH_EXAMPLE_BASE_NAMES {
        let template_path = example_templates_dir.join(format!("{base_name}.rs.j2"));
        let generated_source = fs::read_to_string(&template_path)?;
        for board_profile in BOARD_PROFILES {
            let output_path = examples_dir
                .join(board_profile.chip_dir())
                .join(board_profile.board_dir())
                .join(format!("{base_name}.rs"));
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)?;
            }
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
        for base_name in PASSTHROUGH_EXAMPLE_BASE_NAMES {
            names.push(passthrough_example_name(*board_profile, base_name));
        }
    }
    for board_profile in BOARD_PROFILES {
        if !supports_audio_examples(*board_profile) {
            continue;
        }
        for base_name in AUDIO_EXAMPLE_BASE_NAMES {
            names.push(audio_example_name(*board_profile, base_name));
        }
    }
    for board_profile in BOARD_PROFILES {
        if !supports_ir_examples(*board_profile) {
            continue;
        }
        for base_name in IR_EXAMPLE_BASE_NAMES {
            names.push(ir_example_name(*board_profile, base_name));
        }
    }
    for board_profile in BOARD_PROFILES {
        if supports_conway_example(*board_profile) {
            names.push(conway_example_name(*board_profile));
        }
    }
    for board_profile in BOARD_PROFILES {
        if !supports_clock_examples(*board_profile) {
            continue;
        }
        for base_name in CLOCK_EXAMPLE_BASE_NAMES {
            names.push(clock_example_name(*board_profile, base_name));
        }
    }
    for board_profile in BOARD_PROFILES {
        for base_name in TALK1_BASE_NAMES {
            names.push(talk1_example_name(*board_profile, base_name));
        }
    }
    names.extend(
        BOARD_PROFILES
            .iter()
            .map(|board_profile| blinky_example_name(*board_profile)),
    );
    for board_profile in BOARD_PROFILES {
        if !supports_led16x16_examples(*board_profile) {
            continue;
        }
        for use_spi in LED16X16_VARIANTS {
            names.push(led16x16_example_name(*board_profile, use_spi));
        }
    }
    names
}

pub fn board_example_required_chip(example_name: &str) -> Option<&'static str> {
    for board_profile in BOARD_PROFILES {
        for base_name in PASSTHROUGH_EXAMPLE_BASE_NAMES {
            if passthrough_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_audio_examples(*board_profile) {
            continue;
        }
        for base_name in AUDIO_EXAMPLE_BASE_NAMES {
            if audio_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_ir_examples(*board_profile) {
            continue;
        }
        for base_name in IR_EXAMPLE_BASE_NAMES {
            if ir_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    for board_profile in BOARD_PROFILES {
        if supports_conway_example(*board_profile)
            && conway_example_name(*board_profile) == example_name
        {
            return Some(board_profile.chip_feature());
        }
    }
    for board_profile in BOARD_PROFILES {
        if !supports_clock_examples(*board_profile) {
            continue;
        }
        for base_name in CLOCK_EXAMPLE_BASE_NAMES {
            if clock_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }
    for board_profile in BOARD_PROFILES {
        for base_name in TALK1_BASE_NAMES {
            if talk1_example_name(*board_profile, base_name) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
    }

    for board_profile in BOARD_PROFILES {
        if blinky_example_name(*board_profile) == example_name {
            return Some(board_profile.chip_feature());
        }
    }

    for board_profile in BOARD_PROFILES {
        if !supports_led16x16_examples(*board_profile) {
            continue;
        }
        for use_spi in LED16X16_VARIANTS {
            if led16x16_example_name(*board_profile, use_spi) == example_name {
                return Some(board_profile.chip_feature());
            }
        }
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
    let generated_filenames = [
        "button_example1_trait.rs",
        "button_read.rs",
        "flash_block_example1_trait.rs",
        "lcd_text.rs",
        "lcd_text_example1_trait.rs",
        "lcd_texts.rs",
        "led16x16test.rs",
        "led16x16test_async.rs",
        "led2d.rs",
        "led2d_example1_trait.rs",
        "led2d_example2_trait.rs",
        "led_example1_trait.rs",
        "led_probe_c3_d4_d6.rs",
        "led_probe_c3_gpio_sweep.rs",
        "led_strip8_spi.rs",
        "led_strip_example1_trait.rs",
        "led_strip_example2_trait.rs",
        "led_strip_len8.rs",
        "rfid.rs",
        "servo_basic.rs",
        "servo_example1_trait.rs",
        "servo_player_example1_trait.rs",
        "servo_player_example2_trait.rs",
        "servos.rs",
        "wifi_auto_custom_checkbox.rs",
        "wifi_auto_example1_trait.rs",
        "wifi_auto_force_button.rs",
        "wifi_dns_hex.rs",
        "wifi_scan.rs",
        "audio.rs",
        "audio_example1.rs",
        "audio_example1_trait.rs",
        "audio_example2_trait.rs",
        "audio_example3_trait.rs",
        "ir.rs",
        "ir_example1_trait.rs",
        "ir_kepler.rs",
        "ir_kepler_example1_trait.rs",
        "ir_keplers.rs",
        "ir_mapping_example1_trait.rs",
        "conway.rs",
        "clock_console_simple.rs",
        "clock_lcd.rs",
        "clock_led4.rs",
        "clock_led8x12.rs",
        "clock_servos.rs",
        "clock_sync_example1_trait.rs",
        "talk1/a1_strip_8_blue_gray.rs",
        "talk1/a3_strip_8_blue_white_blink_animate.rs",
        "talk1/a4_strip_96_blue_white_dot.rs",
        "talk1/b1_panel_12x8_rust_cursor.rs",
        "talk1/b2_panel_12x8_text_graphics.rs",
        "talk1/f1_dns.rs",
        "blinky.rs",
        "led16x16_plus_1.rs",
        "led16x16_plus_1_spi.rs",
        "led16x16_and_builtin.rs",
        "led16x16_and_builtin_spi.rs",
    ];

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
            for generated_filename in generated_filenames {
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
