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
enum ChipId {
    Esp32,
    C2,
    C3,
    C6,
    H2,
    S2,
    S3,
}

impl ChipId {
    fn feature(self) -> &'static str {
        match self {
            ChipId::Esp32 => "esp32",
            ChipId::C2 => "esp32c2",
            ChipId::C3 => "esp32c3",
            ChipId::C6 => "esp32c6",
            ChipId::H2 => "esp32h2",
            ChipId::S2 => "esp32s2",
            ChipId::S3 => "esp32s3",
        }
    }

    fn name(self) -> &'static str {
        match self {
            ChipId::Esp32 => "ESP32",
            ChipId::C2 => "ESP32-C2",
            ChipId::C3 => "ESP32-C3",
            ChipId::C6 => "ESP32-C6",
            ChipId::H2 => "ESP32-H2",
            ChipId::S2 => "ESP32-S2",
            ChipId::S3 => "ESP32-S3",
        }
    }

    fn directory(self) -> &'static str {
        match self {
            ChipId::Esp32 => "esp32",
            ChipId::C2 => "c2",
            ChipId::C3 => "c3",
            ChipId::C6 => "c6",
            ChipId::H2 => "h2",
            ChipId::S2 => "s2",
            ChipId::S3 => "s3",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoardId {
    Generic,
    Luatos,
    Devkitc1N8,
    Devkitm1V1_0,
    Devkitc1V1_1N16r8,
    Devkitc1V1_0N16r8,
}

impl BoardId {
    fn directory(self) -> &'static str {
        match self {
            BoardId::Generic => "generic",
            BoardId::Luatos => "luatos",
            BoardId::Devkitc1N8 => "devkitc1_n8",
            BoardId::Devkitm1V1_0 => "devkitm1_v1_0",
            BoardId::Devkitc1V1_1N16r8 => "devkitc1_v1_1_n16r8",
            BoardId::Devkitc1V1_0N16r8 => "devkitc1_v1_0_n16r8",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            BoardId::Generic => "generic",
            BoardId::Luatos => "luatos",
            BoardId::Devkitc1N8 => "esp32-c6-devkitc-1-n8",
            BoardId::Devkitm1V1_0 => "esp8684-devkitm-1-v1.0",
            BoardId::Devkitc1V1_1N16r8 => "esp32-s3-devkitc-1-v1.1-n16r8",
            BoardId::Devkitc1V1_0N16r8 => "esp32-s3-devkitc-1-v1.0-n16r8",
        }
    }
}

#[derive(Clone, Copy)]
struct BoardProfile {
    chip_id: ChipId,
    board_id: BoardId,
    rmt_count: u8,
    spi_count: u8,
    built_in_smart_led: Option<u8>,
    built_in_plain_led: Option<u8>,
    default_external_plain_led: u8,
    default_external_smart_led: u8,
}

impl BoardProfile {
    fn chip_dir(self) -> &'static str {
        self.chip_id.directory()
    }

    fn board_dir(self) -> &'static str {
        self.board_id.directory()
    }

    fn board_slug(self) -> &'static str {
        self.board_id.slug()
    }

    fn chip_feature(self) -> &'static str {
        self.chip_id.feature()
    }

    fn chip_name(self) -> &'static str {
        self.chip_id.name()
    }

    fn blinky_kind(self) -> BlinkyKind {
        if self.built_in_smart_led.is_some() {
            if self.rmt_count > 0 {
                return BlinkyKind::SmartRmt;
            }
            if self.spi_count > 0 {
                return BlinkyKind::SmartSpi;
            }
        }
        BlinkyKind::Plain
    }

    fn blinky_led_pin_num(self) -> u8 {
        match self.blinky_kind() {
            BlinkyKind::Plain => self
                .built_in_plain_led
                .unwrap_or(self.default_external_plain_led),
            BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => self
                .built_in_smart_led
                .expect("smart-led blinky kind requires built_in_smart_led pin"),
        }
    }

    fn blinky_led_pin_ident(self) -> String {
        format!("GPIO{}", self.blinky_led_pin_num())
    }

    fn blinky_built_in_led(self) -> bool {
        match self.blinky_kind() {
            BlinkyKind::Plain => self.built_in_plain_led.is_some(),
            BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => self.built_in_smart_led.is_some(),
        }
    }

    fn led_strip1_pin_num(self) -> u8 {
        self.built_in_smart_led
            .unwrap_or(self.default_external_smart_led)
    }

    fn led_strip1_pin_ident(self) -> String {
        format!("GPIO{}", self.led_strip1_pin_num())
    }

    fn led_strip1_built_in(self) -> bool {
        self.built_in_smart_led.is_some()
    }

    fn supports_led16x16_examples(self) -> bool {
        self.chip_id != ChipId::C2
    }

    fn panel16x16_pin_num(self) -> u8 {
        2
    }
}

const LED16X16_VARIANTS: [bool; 2] = [false, true];

const BOARD_PROFILES: &[BoardProfile] = &[
    BoardProfile {
        chip_id: ChipId::Esp32,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 0,
        default_external_smart_led: 0,
    },
    BoardProfile {
        chip_id: ChipId::C2,
        board_id: BoardId::Generic,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 8,
    },
    BoardProfile {
        chip_id: ChipId::C2,
        board_id: BoardId::Devkitm1V1_0,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: Some(8),
        built_in_plain_led: None,
        default_external_plain_led: 0,
        default_external_smart_led: 2,
    },
    BoardProfile {
        chip_id: ChipId::C3,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 7,
        default_external_smart_led: 7,
    },
    BoardProfile {
        chip_id: ChipId::C3,
        board_id: BoardId::Luatos,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 7,
        default_external_smart_led: 7,
    },
    BoardProfile {
        chip_id: ChipId::C6,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 8,
    },
    BoardProfile {
        chip_id: ChipId::C6,
        board_id: BoardId::Devkitc1N8,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(8),
        built_in_plain_led: None,
        default_external_plain_led: 0,
        default_external_smart_led: 2,
    },
    BoardProfile {
        chip_id: ChipId::H2,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 8,
    },
    BoardProfile {
        chip_id: ChipId::S2,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        default_external_plain_led: 0,
        default_external_smart_led: 0,
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Generic,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(38),
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 10,
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Devkitc1V1_1N16r8,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(38),
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 10,
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Devkitc1V1_0N16r8,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(48),
        built_in_plain_led: None,
        default_external_plain_led: 8,
        default_external_smart_led: 10,
    },
];

fn validate_board_profiles() -> Result<(), Box<dyn Error>> {
    for board_profile in BOARD_PROFILES {
        if let Some(built_in_smart_led) = board_profile.built_in_smart_led {
            if built_in_smart_led == board_profile.default_external_plain_led {
                return Err(format!(
                    "invalid board profile {} {}: default_external_plain_led must differ from built_in_smart_led (GPIO{})",
                    board_profile.chip_feature(),
                    board_profile.board_dir(),
                    built_in_smart_led
                )
                .into());
            }
            if built_in_smart_led == board_profile.default_external_smart_led {
                return Err(format!(
                    "invalid board profile {} {}: default_external_smart_led must differ from built_in_smart_led (GPIO{})",
                    board_profile.chip_feature(),
                    board_profile.board_dir(),
                    built_in_smart_led
                )
                .into());
            }
        }
    }
    Ok(())
}

fn blinky_example_name(board_profile: BoardProfile) -> String {
    format!(
        "blinky_{}_{}",
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

fn led16x16_example_name(board_profile: BoardProfile, use_spi: bool) -> String {
    if use_spi {
        format!(
            "led16x16_plus_1_spi_{}_{}",
            board_profile.chip_feature(),
            board_profile.board_dir()
        )
    } else {
        format!(
            "led16x16_plus_1_{}_{}",
            board_profile.chip_feature(),
            board_profile.board_dir()
        )
    }
}

pub fn generate_board_examples(workspace_root: &Path) -> Result<(), Box<dyn Error>> {
    validate_board_profiles()?;

    let examples_dir = workspace_root.join("examples");
    let templates_dir = examples_dir.join("templates");

    let blinky_plain_template = fs::read_to_string(templates_dir.join("blinky_plain.rs.j2"))?;
    let blinky_rmt_template = fs::read_to_string(templates_dir.join("blinky_rmt.rs.j2"))?;
    let blinky_spi_template = fs::read_to_string(templates_dir.join("blinky_spi.rs.j2"))?;
    let led16x16_plus_1_template = fs::read_to_string(templates_dir.join("led16x16_plus_1.rs.j2"))?;
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
    for board_profile in BOARD_PROFILES {
        let output_path = examples_dir
            .join(board_profile.chip_dir())
            .join(board_profile.board_dir())
            .join("blinky.rs");
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)?;
        }

        let template_name = match board_profile.blinky_kind() {
            BlinkyKind::Plain => "blinky_plain",
            BlinkyKind::SmartRmt => "blinky_rmt",
            BlinkyKind::SmartSpi => "blinky_spi",
        };
        let example_name = blinky_example_name(*board_profile);
        let led_pin_ident = board_profile.blinky_led_pin_ident();

        let generated_source =
            minijinja_environment
                .get_template(template_name)?
                .render(context! {
                    example_name => example_name.as_str(),
                    board_slug => board_profile.board_slug(),
                    chip_name => board_profile.chip_name(),
                    chip_feature => board_profile.chip_feature(),
                    led_pin_num => board_profile.blinky_led_pin_num(),
                    led_pin_ident => led_pin_ident.as_str(),
                    built_in_led => board_profile.blinky_built_in_led(),
                })?;
        write_if_changed(&output_path, &generated_source)?;
        expected_generated_paths.push(output_path);
    }

    for board_profile in BOARD_PROFILES {
        if !board_profile.supports_led16x16_examples() {
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
            let panel_pin_ident = format!("GPIO{}", board_profile.panel16x16_pin_num());
            let led_strip1_pin_ident = board_profile.led_strip1_pin_ident();
            let generated_source =
                minijinja_environment
                    .get_template(template_name)?
                    .render(context! {
                        example_name => example_name.as_str(),
                        board_slug => board_profile.board_slug(),
                        chip_name => board_profile.chip_name(),
                        chip_feature => board_profile.chip_feature(),
                        panel_pin_num => board_profile.panel16x16_pin_num(),
                        panel_pin_ident => panel_pin_ident.as_str(),
                        led_strip1_pin_num => board_profile.led_strip1_pin_num(),
                        led_strip1_pin_ident => led_strip1_pin_ident.as_str(),
                        led_strip1_built_in => board_profile.led_strip1_built_in(),
                    })?;
            write_if_changed(&output_path, &generated_source)?;
            expected_generated_paths.push(output_path);
        }
    }

    cleanup_stale_nested_generated_examples(&examples_dir, &expected_generated_paths)?;

    Ok(())
}

pub fn generated_board_example_names() -> Vec<String> {
    let mut names = Vec::new();
    names.extend(
        BOARD_PROFILES
            .iter()
            .map(|board_profile| blinky_example_name(*board_profile)),
    );
    for board_profile in BOARD_PROFILES {
        if !board_profile.supports_led16x16_examples() {
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
        if blinky_example_name(*board_profile) == example_name {
            return Some(board_profile.chip_feature());
        }
    }

    for board_profile in BOARD_PROFILES {
        if !board_profile.supports_led16x16_examples() {
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
