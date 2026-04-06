use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChipId {
    Esp32,
    C2,
    C3,
    C6,
    H2,
    S2,
    S3,
}

impl ChipId {
    pub(crate) fn feature(self) -> &'static str {
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

    pub(crate) fn name(self) -> &'static str {
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

    pub(crate) fn directory(self) -> &'static str {
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
pub(crate) enum BoardId {
    Generic,
    Luatos,
    Devkitc1N8,
    Devkitm1V1_0,
    Devkitc1V1_1N16r8,
    Devkitc1V1_0N16r8,
}

impl BoardId {
    pub(crate) fn directory(self) -> &'static str {
        match self {
            BoardId::Generic => "generic",
            BoardId::Luatos => "luatos",
            BoardId::Devkitc1N8 => "devkitc1_n8",
            BoardId::Devkitm1V1_0 => "devkitm1_v1_0",
            BoardId::Devkitc1V1_1N16r8 => "devkitc1_v1_1_n16r8",
            BoardId::Devkitc1V1_0N16r8 => "devkitc1_v1_0_n16r8",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
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
pub(crate) struct BoardProfile {
    pub(crate) chip_id: ChipId,
    pub(crate) board_id: BoardId,
    pub(crate) rmt_count: u8,
    pub(crate) spi_count: u8,
    pub(crate) built_in_smart_led: Option<u8>,
    pub(crate) built_in_plain_led: Option<u8>,
    pub(crate) default_external_plain_led: u8,
    pub(crate) default_external_smart_led: u8,
}

impl BoardProfile {
    pub(crate) fn chip_dir(self) -> &'static str {
        self.chip_id.directory()
    }

    pub(crate) fn board_dir(self) -> &'static str {
        self.board_id.directory()
    }

    pub(crate) fn board_slug(self) -> &'static str {
        self.board_id.slug()
    }

    pub(crate) fn chip_feature(self) -> &'static str {
        self.chip_id.feature()
    }

    pub(crate) fn chip_name(self) -> &'static str {
        self.chip_id.name()
    }
}

pub(crate) const BOARD_PROFILES: &[BoardProfile] = &[
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

pub(crate) fn validate_board_profiles() -> Result<(), Box<dyn Error>> {
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
