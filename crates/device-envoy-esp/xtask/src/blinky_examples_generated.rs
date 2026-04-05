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

#[derive(Clone, Copy)]
struct BlinkyBoardExample {
    example_name: &'static str,
    chip_dir: &'static str,
    board_dir: &'static str,
    board_slug: &'static str,
    chip_feature: &'static str,
    chip_name: &'static str,
    blinky_kind: BlinkyKind,
    led_pin_num: u8,
    led_pin_ident: &'static str,
    built_in_led: bool,
}

const BLINKY_BOARD_EXAMPLES: &[BlinkyBoardExample] = &[
    BlinkyBoardExample {
        example_name: "blinky_esp32_generic",
        chip_dir: "esp32",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32",
        chip_name: "ESP32",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 0,
        led_pin_ident: "GPIO0",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c2_generic",
        chip_dir: "c2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c2",
        chip_name: "ESP32-C2",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c3_generic",
        chip_dir: "c3",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c3",
        chip_name: "ESP32-C3",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 7,
        led_pin_ident: "GPIO7",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c3_luatos",
        chip_dir: "c3",
        board_dir: "luatos",
        board_slug: "luatos",
        chip_feature: "esp32c3",
        chip_name: "ESP32-C3",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 7,
        led_pin_ident: "GPIO7",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c6_generic",
        chip_dir: "c6",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32c6",
        chip_name: "ESP32-C6",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32c6_devkitc1_n8",
        chip_dir: "c6",
        board_dir: "devkitc1_n8",
        board_slug: "esp32-c6-devkitc-1-n8",
        chip_feature: "esp32c6",
        chip_name: "ESP32-C6",
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32h2_generic",
        chip_dir: "h2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32h2",
        chip_name: "ESP32-H2",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 8,
        led_pin_ident: "GPIO8",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s2_generic",
        chip_dir: "s2",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32s2",
        chip_name: "ESP32-S2",
        blinky_kind: BlinkyKind::Plain,
        led_pin_num: 0,
        led_pin_ident: "GPIO0",
        built_in_led: false,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_generic",
        chip_dir: "s3",
        board_dir: "generic",
        board_slug: "generic",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 38,
        led_pin_ident: "GPIO38",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_devkitc1_v1_1_n16r8",
        chip_dir: "s3",
        board_dir: "devkitc1_v1_1_n16r8",
        board_slug: "esp32-s3-devkitc-1-v1.1-n16r8",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 38,
        led_pin_ident: "GPIO38",
        built_in_led: true,
    },
    BlinkyBoardExample {
        example_name: "blinky_esp32s3_devkitc1_v1_0_n16r8",
        chip_dir: "s3",
        board_dir: "devkitc1_v1_0_n16r8",
        board_slug: "esp32-s3-devkitc-1-v1.0-n16r8",
        chip_feature: "esp32s3",
        chip_name: "ESP32-S3",
        blinky_kind: BlinkyKind::SmartRmt,
        led_pin_num: 48,
        led_pin_ident: "GPIO48",
        built_in_led: true,
    },
];

pub fn generate_blinky_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    let examples_dir = workspace_root.join("examples");
    let templates_dir = examples_dir.join("templates");

    let blinky_plain_template = fs::read_to_string(templates_dir.join("blinky_plain.rs.j2"))?;
    let blinky_rmt_template = fs::read_to_string(templates_dir.join("blinky_rmt.rs.j2"))?;
    let blinky_spi_template = fs::read_to_string(templates_dir.join("blinky_spi.rs.j2"))?;

    let mut minijinja_environment = Environment::new();
    minijinja_environment.add_template("blinky_plain", &blinky_plain_template)?;
    minijinja_environment.add_template("blinky_rmt", &blinky_rmt_template)?;
    minijinja_environment.add_template("blinky_spi", &blinky_spi_template)?;

    cleanup_legacy_flat_generated_examples(&examples_dir)?;

    let mut expected_generated_paths = Vec::new();
    for board_example in BLINKY_BOARD_EXAMPLES {
        let output_path = examples_dir
            .join(board_example.chip_dir)
            .join(board_example.board_dir)
            .join("blinky.rs");
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }

        let template_name = match board_example.blinky_kind {
            BlinkyKind::Plain => "blinky_plain",
            BlinkyKind::SmartRmt => "blinky_rmt",
            BlinkyKind::SmartSpi => "blinky_spi",
        };

        let generated_source =
            minijinja_environment
                .get_template(template_name)?
                .render(context! {
                    example_name => board_example.example_name,
                    board_slug => board_example.board_slug,
                    chip_name => board_example.chip_name,
                    chip_feature => board_example.chip_feature,
                    led_pin_num => board_example.led_pin_num,
                    led_pin_ident => board_example.led_pin_ident,
                    built_in_led => board_example.built_in_led,
                })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;

    Ok(())
}

pub fn generated_blinky_example_names() -> Vec<&'static str> {
    BLINKY_BOARD_EXAMPLES
        .iter()
        .map(|board_example| board_example.example_name)
        .collect()
}

pub fn board_example_required_chip(example_name: &str) -> Option<&'static str> {
    BLINKY_BOARD_EXAMPLES
        .iter()
        .find(|board_example| board_example.example_name == example_name)
        .map(|board_example| board_example.chip_feature)
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
            let candidate = board_dir_path.join("blinky.rs");
            if !candidate.exists() || expected_paths.contains(&candidate) {
                continue;
            }
            let existing = fs::read_to_string(&candidate)?;
            if existing.starts_with("// @generated by `cargo xtask generate-blinky-examples`") {
                fs::remove_file(&candidate)?;
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
