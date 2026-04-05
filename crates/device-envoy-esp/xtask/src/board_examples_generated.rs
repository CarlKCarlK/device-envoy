use minijinja::{context, Environment};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum BlinkyKind {
    Plain,
    SmartRmt,
    #[allow(dead_code)]
    SmartSpi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoardId {
    Esp32Generic,
    Esp32c2Generic,
    Esp32c3Generic,
    Esp32c3Luatos,
    Esp32c6Generic,
    Esp32c6Devkitc1N8,
    Esp32h2Generic,
    Esp32s2Generic,
    Esp32s3Generic,
    Esp32s3Devkitc1V1_1N16r8,
    Esp32s3Devkitc1V1_0N16r8,
}

#[derive(Clone, Copy)]
struct BoardProfile {
    id: BoardId,
    chip_dir: &'static str,
    board_dir: &'static str,
    board_slug: &'static str,
    chip_feature: &'static str,
    chip_name: &'static str,
}

#[derive(Clone, Copy)]
struct BlinkyBoardExample {
    example_name: &'static str,
    board_id: BoardId,
    blinky_kind: BlinkyKind,
    led_pin_num: u8,
    led_pin_ident: &'static str,
    built_in_led: bool,
}

#[derive(Clone, Copy)]
struct Led16x16BoardExample {
    example_name: &'static str,
    board_id: BoardId,
    panel_pin_num: u8,
    panel_pin_ident: &'static str,
    led_strip1_pin_num: u8,
    led_strip1_pin_ident: &'static str,
    led_strip1_built_in: bool,
    use_spi: bool,
}

const BOARD_PROFILES: &[BoardProfile] = &[
    BoardProfile {
        id: BoardId::Esp32Generic,
        chip_dir: "esp32",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32",
        chip_name: "ESP32",
    },
    BoardProfile {
        id: BoardId::Esp32c2Generic,
        chip_dir: "c2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c2",
        chip_name: "ESP32-C2",
    },
    BoardProfile {
        id: BoardId::Esp32c3Generic,
        chip_dir: "c3",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c3",
        chip_name: "ESP32-C3",
    },
    BoardProfile {
        id: BoardId::Esp32c3Luatos,
        chip_dir: "c3",
        board_dir: "luatos",
        board_slug: "luatos",
        chip_feature: "esp32c3",
        chip_name: "ESP32-C3",
    },
    BoardProfile {
        id: BoardId::Esp32c6Generic,
        chip_dir: "c6",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c6",
        chip_name: "ESP32-C6",
    },
    BoardProfile {
        id: BoardId::Esp32c6Devkitc1N8,
        chip_dir: "c6",
        board_dir: "devkitc1_n8",
        board_slug: "esp32-c6-devkitc-1-n8",
        chip_feature: "esp32c6",
        chip_name: "ESP32-C6",
    },
    BoardProfile {
        id: BoardId::Esp32h2Generic,
        chip_dir: "h2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32h2",
        chip_name: "ESP32-H2",
    },
    BoardProfile {
        id: BoardId::Esp32s2Generic,
        chip_dir: "s2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32s2",
        chip_name: "ESP32-S2",
    },
    BoardProfile {
        id: BoardId::Esp32s3Generic,
        chip_dir: "s3",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
    },
    BoardProfile {
        id: BoardId::Esp32s3Devkitc1V1_1N16r8,
        chip_dir: "s3",
        board_dir: "devkitc1_v1_1_n16r8",
        board_slug: "esp32-s3-devkitc-1-v1.1-n16r8",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
    },
    BoardProfile {
        id: BoardId::Esp32s3Devkitc1V1_0N16r8,
        chip_dir: "s3",
        board_dir: "devkitc1_v1_0_n16r8",
        board_slug: "esp32-s3-devkitc-1-v1.0-n16r8",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
    },
];

const BLINKY_BOARD_EXAMPLES: &[BlinkyBoardExample] = &[
    BlinkyBoardExample {
        example_name: "blinky_esp32_generic",
        board_id: BoardId::Esp32Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 0,
        led_pin_ident: "GPIO0",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c2_generic",
        board_id: BoardId::Esp32c2Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c3_generic",
        board_id: BoardId::Esp32c3Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 7,
        led_pin_ident: "GPIO7",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c3_luatos",
        board_id: BoardId::Esp32c3Luatos,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 7,
        led_pin_ident: "GPIO7",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c6_generic",
        board_id: BoardId::Esp32c6Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c6_devkitc1_n8",
        board_id: BoardId::Esp32c6Devkitc1N8,
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32h2_generic",
        board_id: BoardId::Esp32h2Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s2_generic",
        board_id: BoardId::Esp32s2Generic,
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 0,
        led_pin_ident: "GPIO0",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_generic",
        board_id: BoardId::Esp32s3Generic,
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 38,
        led_pin_ident: "GPIO38",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_devkitc1_v1_1_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_1N16r8,
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 38,
        led_pin_ident: "GPIO38",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_devkitc1_v1_0_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_0N16r8,
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 48,
        led_pin_ident: "GPIO48",
        built_in_led: true,
    },
];

const LED16X16_BOARD_EXAMPLES: &[Led16x16BoardExample] = &[
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32_generic",
        board_id: BoardId::Esp32Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 0,
        led_strip1_pin_ident: "GPIO0",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32_generic",
        board_id: BoardId::Esp32Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 0,
        led_strip1_pin_ident: "GPIO0",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32c3_generic",
        board_id: BoardId::Esp32c3Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 7,
        led_strip1_pin_ident: "GPIO7",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32c3_generic",
        board_id: BoardId::Esp32c3Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 7,
        led_strip1_pin_ident: "GPIO7",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32c3_luatos",
        board_id: BoardId::Esp32c3Luatos,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 7,
        led_strip1_pin_ident: "GPIO7",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32c3_luatos",
        board_id: BoardId::Esp32c3Luatos,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 7,
        led_strip1_pin_ident: "GPIO7",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32c6_generic",
        board_id: BoardId::Esp32c6Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32c6_generic",
        board_id: BoardId::Esp32c6Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32c6_devkitc1_n8",
        board_id: BoardId::Esp32c6Devkitc1N8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: true,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32c6_devkitc1_n8",
        board_id: BoardId::Esp32c6Devkitc1N8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: true,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32h2_generic",
        board_id: BoardId::Esp32h2Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32h2_generic",
        board_id: BoardId::Esp32h2Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 8,
        led_strip1_pin_ident: "GPIO8",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32s2_generic",
        board_id: BoardId::Esp32s2Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 0,
        led_strip1_pin_ident: "GPIO0",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32s2_generic",
        board_id: BoardId::Esp32s2Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 0,
        led_strip1_pin_ident: "GPIO0",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32s3_generic",
        board_id: BoardId::Esp32s3Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 10,
        led_strip1_pin_ident: "GPIO10",
        led_strip1_built_in: false,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32s3_generic",
        board_id: BoardId::Esp32s3Generic,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 10,
        led_strip1_pin_ident: "GPIO10",
        led_strip1_built_in: false,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32s3_devkitc1_v1_1_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_1N16r8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 38,
        led_strip1_pin_ident: "GPIO38",
        led_strip1_built_in: true,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32s3_devkitc1_v1_1_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_1N16r8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 38,
        led_strip1_pin_ident: "GPIO38",
        led_strip1_built_in: true,
        use_spi: true,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_esp32s3_devkitc1_v1_0_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_0N16r8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 48,
        led_strip1_pin_ident: "GPIO48",
        led_strip1_built_in: true,
        use_spi: false,
    },
    Led16x16BoardExample {
        example_name: "led16x16_plus_1_spi_esp32s3_devkitc1_v1_0_n16r8",
        board_id: BoardId::Esp32s3Devkitc1V1_0N16r8,
        panel_pin_num: 2,
        panel_pin_ident: "GPIO2",
        led_strip1_pin_num: 48,
        led_strip1_pin_ident: "GPIO48",
        led_strip1_built_in: true,
        use_spi: true,
    },
];

fn board_profile(board_id: BoardId) -> &'static BoardProfile {
    BOARD_PROFILES
        .iter()
        .find(|board_profile| board_profile.id == board_id)
        .unwrap_or_else(|| panic!("missing board profile for {board_id:?}"))
}

pub fn generate_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    let examples_dir = workspace_root.join("examples");
    let templates_dir = examples_dir.join("templates");

    let blinky_plain_template = fs::read_to_string(templates_dir.join("blinky_plain.rs.j2"))?;
    let blinky_rmt_template = fs::read_to_string(templates_dir.join("blinky_rmt.rs.j2"))?;
    let blinky_spi_template = fs::read_to_string(templates_dir.join("blinky_spi.rs.j2"))?;
    let led16x16_plus_1_template =
        fs::read_to_string(templates_dir.join("led16x16_plus_1.rs.j2"))?;
    let led16x16_plus_1_spi_template =
        fs::read_to_string(templates_dir.join("led16x16_plus_1_spi.rs.j2"))?;

    let mut minijinja_environment = Environment::new();
    minijinja_environment.add_template("blinky_plain", &blinky_plain_template)?;
    minijinja_environment.add_template("blinky_rmt", &blinky_rmt_template)?;
    minijinja_environment.add_template("blinky_spi", &blinky_spi_template)?;
    minijinja_environment.add_template("led16x16_plus_1", &led16x16_plus_1_template)?;
    minijinja_environment.add_template("led16x16_plus_1_spi", &led16x16_plus_1_spi_template)?;

    cleanup_legacy_flat_generated_examples(&examples_dir)?;

    let mut expected_generated_paths = Vec::new();
    for board_example in BLINKY_BOARD_EXAMPLES {
        let board_profile = board_profile(board_example.board_id);
        let output_path = examples_dir
            .join(board_profile.chip_dir)
            .join(board_profile.board_dir)
            .join("blinky.rs");
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }

        let template_name = match board_example.blinky_kind {
            BlinkyKind::Plain => "blinky_plain",
            BlinkyKind::SmartRmt => "blinky_rmt",
            BlinkyKind::SmartSpi => "blinky_spi",
        };

        let generated_source = minijinja_environment
            .get_template(template_name)?
            .render(context! {
                example_name => board_example.example_name,
                board_slug => board_profile.board_slug,
                chip_name => board_profile.chip_name,
                chip_feature => board_profile.chip_feature,
                led_pin_num => board_example.led_pin_num,
                led_pin_ident => board_example.led_pin_ident,
                built_in_led => board_example.built_in_led,
            })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    for board_example in LED16X16_BOARD_EXAMPLES {
        let board_profile = board_profile(board_example.board_id);
        let output_filename = if board_example.use_spi {
            "led16x16_plus_1_spi.rs"
        } else {
            "led16x16_plus_1.rs"
        };
        let output_path = examples_dir
            .join(board_profile.chip_dir)
            .join(board_profile.board_dir)
            .join(output_filename);
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }
        let template_name = if board_example.use_spi {
            "led16x16_plus_1_spi"
        } else {
            "led16x16_plus_1"
        };
        let generated_source = minijinja_environment
            .get_template(template_name)?
            .render(context! {
                example_name => board_example.example_name,
                board_slug => board_profile.board_slug,
                chip_name => board_profile.chip_name,
                chip_feature => board_profile.chip_feature,
                panel_pin_num => board_example.panel_pin_num,
                panel_pin_ident => board_example.panel_pin_ident,
                led_strip1_pin_num => board_example.led_strip1_pin_num,
                led_strip1_pin_ident => board_example.led_strip1_pin_ident,
                led_strip1_built_in => board_example.led_strip1_built_in,
            })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;

    Ok(())
}

pub fn generated_board_example_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    names.extend(
        BLINKY_BOARD_EXAMPLES
            .iter()
            .map(|board_example| board_example.example_name),
    );
    names.extend(
        LED16X16_BOARD_EXAMPLES
            .iter()
            .map(|board_example| board_example.example_name),
    );
    names
}

pub fn board_example_required_chip(example_name: &str) -> Option<&'static str> {
    if let Some(board_example) = BLINKY_BOARD_EXAMPLES
        .iter()
        .find(|board_example| board_example.example_name == example_name)
    {
        return Some(board_profile(board_example.board_id).chip_feature);
    }
    LED16X16_BOARD_EXAMPLES
        .iter()
        .find(|board_example| board_example.example_name == example_name)
        .map(|board_example| board_profile(board_example.board_id).chip_feature)
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
