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
pub(crate) struct AudioWiring {
    pub(crate) data_pin_num: u8,
    pub(crate) bit_clock_pin_num: u8,
    pub(crate) word_select_pin_num: u8,
    pub(crate) button_pin_num: u8,
    pub(crate) dma_ident: &'static str,
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
    pub(crate) panel16x16_pin: u8,
    pub(crate) ir_pin: u8,
    pub(crate) ir_pin2: u8,
    pub(crate) ir_rx_channel: u8,
    pub(crate) ir_rx_channel2: u8,
    pub(crate) audio_wiring: Option<AudioWiring>,
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
        panel16x16_pin: 2,
        ir_pin: 4,
        ir_pin2: 5,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 4,
            word_select_pin_num: 5,
            button_pin_num: 0,
            dma_ident: "DMA_I2S0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        // TODO0Audio ESP32-C2 audio examples are currently unsupported in
        // device-envoy-esp because the current esp-hal configuration for C2
        // does not expose the needed I2S support.
        audio_wiring: None,
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        // TODO0Audio ESP32-C2 audio examples are currently unsupported in
        // device-envoy-esp because the current esp-hal configuration for C2
        // does not expose the needed I2S support.
        audio_wiring: None,
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 5,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 0,
        ir_pin2: 1,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 1,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 0,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 4,
        ir_pin2: 5,
        ir_rx_channel: 2,
        ir_rx_channel2: 3,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 4,
            word_select_pin_num: 5,
            button_pin_num: 0,
            dma_ident: "DMA_I2S0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 4,
        ir_rx_channel2: 5,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 4,
        ir_rx_channel2: 5,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
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
        panel16x16_pin: 2,
        ir_pin: 7,
        ir_pin2: 4,
        ir_rx_channel: 4,
        ir_rx_channel2: 5,
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
    },
];

pub(crate) fn validate_board_profiles() -> Result<(), Box<dyn Error>> {
    for board_profile in BOARD_PROFILES {
        let led_strip_pin = board_profile
            .built_in_smart_led
            .unwrap_or(board_profile.default_external_smart_led);
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
        if board_profile.panel16x16_pin == led_strip_pin {
            return Err(format!(
                "invalid board profile {} {}: panel16x16_pin must differ from led strip pin (GPIO{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                led_strip_pin
            )
            .into());
        }
        if board_profile.ir_pin == board_profile.ir_pin2 {
            return Err(format!(
                "invalid board profile {} {}: ir_pin must differ from ir_pin2 (GPIO{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                board_profile.ir_pin
            )
            .into());
        }
        if board_profile.ir_rx_channel == board_profile.ir_rx_channel2 {
            return Err(format!(
                "invalid board profile {} {}: ir_rx_channel must differ from ir_rx_channel2 (channel{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                board_profile.ir_rx_channel
            )
            .into());
        }
        if let Some(audio_wiring) = board_profile.audio_wiring {
            if audio_wiring.data_pin_num == audio_wiring.bit_clock_pin_num
                || audio_wiring.data_pin_num == audio_wiring.word_select_pin_num
                || audio_wiring.bit_clock_pin_num == audio_wiring.word_select_pin_num
            {
                return Err(format!(
                    "invalid board profile {} {}: audio data/bit_clock/word_select pins must be distinct",
                    board_profile.chip_feature(),
                    board_profile.board_dir()
                )
                .into());
            }
        }
    }
    Ok(())
}
