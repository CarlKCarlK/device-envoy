use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChipId {
    Esp32,
    C2,
    C3,
    C5,
    C6,
    C61,
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
            ChipId::C5 => "esp32c5",
            ChipId::C6 => "esp32c6",
            ChipId::C61 => "esp32c61",
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
            ChipId::C5 => "ESP32-C5",
            ChipId::C6 => "ESP32-C6",
            ChipId::C61 => "ESP32-C61",
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
            ChipId::C5 => "c5",
            ChipId::C6 => "c6",
            ChipId::C61 => "c61",
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
    pub(crate) dma_identifier: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct RfidWiring {
    pub(crate) sck_pin_num: u8,
    pub(crate) mosi_pin_num: u8,
    pub(crate) miso_pin_num: u8,
    pub(crate) cs_pin_num: u8,
    pub(crate) rst_pin_num: u8,
}

#[derive(Clone, Copy)]
pub(crate) struct CydDisplayWiring {
    pub(crate) sck_pin_num: u8,
    pub(crate) mosi_pin_num: u8,
    pub(crate) miso_pin_num: u8,
    pub(crate) cs_pin_num: u8,
    pub(crate) dc_pin_num: u8,
    pub(crate) rst_pin_num: u8,
    pub(crate) backlight_pin_num: u8,
}

#[derive(Clone, Copy)]
pub(crate) struct CydTouchWiring {
    pub(crate) sck_pin_num: u8,
    pub(crate) mosi_pin_num: u8,
    pub(crate) miso_pin_num: u8,
    pub(crate) cs_pin_num: u8,
    pub(crate) irq_pin_num: u8,
}

#[derive(Clone, Copy)]
pub(crate) struct BoardProfile {
    pub(crate) chip_id: ChipId,
    pub(crate) board_id: BoardId,
    pub(crate) wifi_supported: bool,
    pub(crate) rmt_count: u8,
    pub(crate) spi_count: u8,
    pub(crate) built_in_smart_led: Option<u8>,
    pub(crate) built_in_plain_led: Option<u8>,
    pub(crate) external_plain_led: u8,
    pub(crate) external_smart_led: u8,
    pub(crate) button_pin: u8,
    pub(crate) led_strip_len_8_pin: u8,
    pub(crate) led_2d12x8_pin: u8,
    pub(crate) led_2d16x16_pin: u8,
    pub(crate) lcd_sda_pin: u8,
    pub(crate) lcd_scl_pin: u8,
    pub(crate) servo_pin: u8,
    pub(crate) servo2_pin: u8,
    pub(crate) led4_cell_pins: [u8; 4],
    pub(crate) led4_segment_pins: [u8; 8],
    pub(crate) ir_pin_rx_channel: (u8, u8),
    pub(crate) ir_pin_rx_channel2: (u8, u8),
    pub(crate) rfid_wiring: RfidWiring,
    pub(crate) cyd_display_wiring: CydDisplayWiring,
    pub(crate) cyd_touch_wiring: CydTouchWiring,
    pub(crate) audio_wiring: Option<AudioWiring>,
    /// Whether this chip's linker budget is too constrained for large WiFi/stack examples.
    pub(crate) stack_constrained: bool,
    /// GPIO pin numbers that are absent from this chip's peripheral map and will cause a
    /// compile error if referenced (e.g., GPIO6/7 on ESP32-H2, GPIO15–21 on ESP32-C3).
    pub(crate) unavailable_gpios: &'static [u8],
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
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 0,
        external_smart_led: 0,
        button_pin: 0,
        led_strip_len_8_pin: 2,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 4,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 15],
        led4_segment_pins: [4, 16, 17, 18, 19, 21, 1, 2],
        ir_pin_rx_channel: (4, 2),
        ir_pin_rx_channel2: (5, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 18,
            mosi_pin_num: 21,
            miso_pin_num: 19,
            cs_pin_num: 5,
            rst_pin_num: 4,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 14,
            mosi_pin_num: 13,
            miso_pin_num: 12,
            cs_pin_num: 15,
            dc_pin_num: 2,
            rst_pin_num: 4,
            backlight_pin_num: 21,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 25,
            mosi_pin_num: 32,
            miso_pin_num: 39,
            cs_pin_num: 33,
            irq_pin_num: 36,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 4,
            word_select_pin_num: 5,
            dma_identifier: "DMA_I2S0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[6, 7, 8, 9, 10, 11],
    },
    BoardProfile {
        chip_id: ChipId::C2,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 8,
        // Moved off GPIO6 (was shared with cyd_display_wiring's old cs_pin_num, which made
        // clock/skeleton_clock fail to compile on this board — see cyd_display_wiring below).
        button_pin: 18,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        // GPIO3, not 18: servo2_pin must differ from button_pin (both dual-servo and
        // servo+button examples need servo_pin/servo2_pin/button_pin to be three distinct
        // GPIOs; they previously collided on 18, breaking servos_calibrate compilation).
        servo2_pin: 3,
        led4_cell_pins: [10, 9, 8, 7],
        led4_segment_pins: [4, 3, 2, 1, 0, 5, 18, 19],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 4,
            mosi_pin_num: 5,
            miso_pin_num: 2,
            cs_pin_num: 3,
            rst_pin_num: 1,
        },
        // Rewired from the ESP8684-DevKitM-1 v1.1 header table (J1/J3), avoiding GPIO8/GPIO9
        // (strapping pins) and GPIO19/GPIO20 (UART0, kept free for logging). Previously this and
        // cyd_touch_wiring/button_pin collided on three GPIOs (see removed todo0 below), which
        // both broke clock/skeleton_clock compilation on this board and forced armatron_one_spi
        // to placeholder-stub via NoCydOneSpiPinConflict. All ten display+touch+button roles
        // used by a one-SPI CYD example are now on distinct GPIOs; verified by
        // `cargo xtask generate-board-examples` + building every affected example.
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            dc_pin_num: 3,
            rst_pin_num: 4,
            backlight_pin_num: 5,
        },
        cyd_touch_wiring: CydTouchWiring {
            // sck/mosi/miso are only used by the two-SPI `armatron` template, which is always
            // placeholder-stubbed on this single-SPI chip regardless of these values (a second
            // physical SPI peripheral doesn't exist to wire up) — left as-is; only cs/irq matter
            // for the real one-SPI example.
            sck_pin_num: 0,
            mosi_pin_num: 1,
            miso_pin_num: 2,
            cs_pin_num: 0,
            irq_pin_num: 1,
        },
        // ESP32-C2 audio examples are currently unsupported in
        // device-envoy-esp because the current esp-hal configuration for C2
        // does not expose the needed I2S support.
        audio_wiring: None,
        stack_constrained: false,
        unavailable_gpios: &[11, 12, 13, 14, 15, 16, 17, 21, 22, 23],
    },
    BoardProfile {
        chip_id: ChipId::C2,
        board_id: BoardId::Devkitm1V1_0,
        wifi_supported: true,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: Some(8),
        built_in_plain_led: None,
        external_plain_led: 0,
        external_smart_led: 2,
        // Moved off GPIO6 (was shared with cyd_display_wiring's old cs_pin_num, which made
        // clock/skeleton_clock fail to compile on this board — see cyd_display_wiring below).
        button_pin: 18,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        // GPIO3, not 18: servo2_pin must differ from button_pin (both dual-servo and
        // servo+button examples need servo_pin/servo2_pin/button_pin to be three distinct
        // GPIOs; they previously collided on 18, breaking servos_calibrate compilation).
        servo2_pin: 3,
        led4_cell_pins: [10, 9, 8, 7],
        led4_segment_pins: [4, 3, 2, 1, 0, 5, 18, 19],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        // Rewired from the ESP8684-DevKitM-1 v1.1 header table (J1/J3), avoiding GPIO8/GPIO9
        // (strapping pins) and GPIO19/GPIO20 (UART0, kept free for logging). Previously this and
        // cyd_touch_wiring/button_pin collided on three GPIOs (see removed todo0 below), which
        // both broke clock/skeleton_clock compilation on this board and forced armatron_one_spi
        // to placeholder-stub via NoCydOneSpiPinConflict. All ten display+touch+button roles
        // used by a one-SPI CYD example are now on distinct GPIOs; verified by
        // `cargo xtask generate-board-examples` + building every affected example.
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            dc_pin_num: 3,
            rst_pin_num: 4,
            backlight_pin_num: 5,
        },
        cyd_touch_wiring: CydTouchWiring {
            // sck/mosi/miso are only used by the two-SPI `armatron` template, which is always
            // placeholder-stubbed on this single-SPI chip regardless of these values (a second
            // physical SPI peripheral doesn't exist to wire up) — left as-is; only cs/irq matter
            // for the real one-SPI example.
            sck_pin_num: 0,
            mosi_pin_num: 1,
            miso_pin_num: 2,
            cs_pin_num: 0,
            irq_pin_num: 1,
        },
        // ESP32-C2 audio examples are currently unsupported in
        // device-envoy-esp because the current esp-hal configuration for C2
        // does not expose the needed I2S support.
        audio_wiring: None,
        stack_constrained: false,
        unavailable_gpios: &[11, 12, 13, 14, 15, 16, 17, 21, 22, 23],
    },
    BoardProfile {
        chip_id: ChipId::C3,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 7,
        external_smart_led: 7,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [10, 9, 8, 7],
        led4_segment_pins: [4, 3, 2, 1, 0, 5, 18, 19],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 19,
            mosi_pin_num: 18,
            miso_pin_num: 20,
            cs_pin_num: 21,
            dc_pin_num: 4,
            rst_pin_num: 5,
            backlight_pin_num: 7,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[15, 16, 17, 18, 19, 20, 21],
    },
    BoardProfile {
        chip_id: ChipId::C3,
        board_id: BoardId::Luatos,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 7,
        external_smart_led: 7,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [9, 8, 4, 5],
        led4_segment_pins: [7, 10, 3, 2, 19, 18, 1, 0],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 19,
            mosi_pin_num: 18,
            miso_pin_num: 20,
            cs_pin_num: 21,
            dc_pin_num: 4,
            rst_pin_num: 5,
            backlight_pin_num: 7,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 5,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[15, 16, 17, 18, 19, 20, 21],
    },
    BoardProfile {
        chip_id: ChipId::C5,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 8,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 12,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 11,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 4, 3, 8, 5, 7, 1],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: None,
        stack_constrained: false,
        unavailable_gpios: &[15, 16, 17, 18, 19, 20, 21, 22, 23],
    },
    BoardProfile {
        chip_id: ChipId::C6,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 8,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 4, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[],
    },
    BoardProfile {
        chip_id: ChipId::C61,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 0,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 8,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 12,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 11,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 4, 3, 8, 5, 7, 1],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: None,
        stack_constrained: false,
        unavailable_gpios: &[15, 16, 17, 18, 19, 20, 21, 22, 23],
    },
    BoardProfile {
        chip_id: ChipId::C6,
        board_id: BoardId::Devkitc1N8,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 1,
        built_in_smart_led: Some(8),
        built_in_plain_led: None,
        external_plain_led: 0,
        external_smart_led: 2,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 16,
        lcd_scl_pin: 17,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 4, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (7, 2),
        ir_pin_rx_channel2: (4, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 19,
            mosi_pin_num: 18,
            miso_pin_num: 20,
            cs_pin_num: 21,
            dc_pin_num: 4,
            rst_pin_num: 5,
            backlight_pin_num: 7,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[],
    },
    BoardProfile {
        chip_id: ChipId::H2,
        board_id: BoardId::Generic,
        wifi_supported: false,
        rmt_count: 2,
        spi_count: 1,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 8,
        button_pin: 0,
        led_strip_len_8_pin: 2,
        led_2d12x8_pin: 2,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 4,
        lcd_scl_pin: 5,
        servo_pin: 10,
        servo2_pin: 1,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 4, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (0, 2),
        ir_pin_rx_channel2: (1, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 4,
            mosi_pin_num: 5,
            miso_pin_num: 2,
            cs_pin_num: 3,
            rst_pin_num: 1,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 8,
            mosi_pin_num: 9,
            miso_pin_num: 10,
            cs_pin_num: 11,
            dc_pin_num: 12,
            rst_pin_num: 13,
            backlight_pin_num: 14,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            irq_pin_num: 5,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 1,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[6, 7, 15, 16, 17, 18, 19, 20, 21],
    },
    BoardProfile {
        chip_id: ChipId::S2,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: None,
        built_in_plain_led: None,
        external_plain_led: 0,
        external_smart_led: 0,
        button_pin: 0,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 16,
        lcd_scl_pin: 17,
        servo_pin: 4,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 15],
        led4_segment_pins: [4, 16, 17, 18, 19, 21, 1, 2],
        ir_pin_rx_channel: (4, 2),
        ir_pin_rx_channel2: (5, 3),
        rfid_wiring: RfidWiring {
            sck_pin_num: 18,
            mosi_pin_num: 21,
            miso_pin_num: 19,
            cs_pin_num: 5,
            rst_pin_num: 4,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 4,
            word_select_pin_num: 5,
            dma_identifier: "DMA_I2S0",
        }),
        stack_constrained: true,
        unavailable_gpios: &[],
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Generic,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(38),
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 10,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 16,
        lcd_scl_pin: 17,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 46, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (7, 4),
        ir_pin_rx_channel2: (4, 5),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[],
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Devkitc1V1_1N16r8,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(38),
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 10,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 16,
        lcd_scl_pin: 17,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 46, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (7, 4),
        ir_pin_rx_channel2: (4, 5),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[],
    },
    BoardProfile {
        chip_id: ChipId::S3,
        board_id: BoardId::Devkitc1V1_0N16r8,
        wifi_supported: true,
        rmt_count: 2,
        spi_count: 2,
        built_in_smart_led: Some(48),
        built_in_plain_led: None,
        external_plain_led: 8,
        external_smart_led: 10,
        button_pin: 6,
        led_strip_len_8_pin: 10,
        led_2d12x8_pin: 18,
        led_2d16x16_pin: 2,
        lcd_sda_pin: 16,
        lcd_scl_pin: 17,
        servo_pin: 10,
        servo2_pin: 18,
        led4_cell_pins: [14, 13, 12, 11],
        led4_segment_pins: [10, 9, 46, 3, 8, 18, 17, 16],
        ir_pin_rx_channel: (7, 4),
        ir_pin_rx_channel2: (4, 5),
        rfid_wiring: RfidWiring {
            sck_pin_num: 6,
            mosi_pin_num: 7,
            miso_pin_num: 2,
            cs_pin_num: 10,
            rst_pin_num: 5,
        },
        cyd_display_wiring: CydDisplayWiring {
            sck_pin_num: 1,
            mosi_pin_num: 2,
            miso_pin_num: 3,
            cs_pin_num: 4,
            dc_pin_num: 5,
            rst_pin_num: 7,
            backlight_pin_num: 8,
        },
        cyd_touch_wiring: CydTouchWiring {
            sck_pin_num: 9,
            mosi_pin_num: 10,
            miso_pin_num: 11,
            cs_pin_num: 12,
            irq_pin_num: 13,
        },
        audio_wiring: Some(AudioWiring {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            dma_identifier: "DMA_CH0",
        }),
        stack_constrained: false,
        unavailable_gpios: &[],
    },
];

pub(crate) fn validate_board_profiles() -> Result<(), Box<dyn Error>> {
    for board_profile in BOARD_PROFILES {
        let led_strip_pin = board_profile
            .built_in_smart_led
            .unwrap_or(board_profile.external_smart_led);
        if let Some(built_in_smart_led) = board_profile.built_in_smart_led {
            if built_in_smart_led == board_profile.external_plain_led {
                return Err(format!(
                    "invalid board profile {} {}: external_plain_led must differ from built_in_smart_led (GPIO{})",
                    board_profile.chip_feature(),
                    board_profile.board_dir(),
                    built_in_smart_led
                )
                .into());
            }
            if built_in_smart_led == board_profile.external_smart_led {
                return Err(format!(
                    "invalid board profile {} {}: external_smart_led must differ from built_in_smart_led (GPIO{})",
                    board_profile.chip_feature(),
                    board_profile.board_dir(),
                    built_in_smart_led
                )
                .into());
            }
        }
        if board_profile.led_2d16x16_pin == led_strip_pin {
            return Err(format!(
                "invalid board profile {} {}: led_2d16x16_pin must differ from led strip pin (GPIO{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                led_strip_pin
            )
            .into());
        }
        if board_profile.ir_pin_rx_channel.0 == board_profile.ir_pin_rx_channel2.0 {
            return Err(format!(
                "invalid board profile {} {}: ir_pin_rx_channel and ir_pin_rx_channel2 pins must differ (GPIO{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                board_profile.ir_pin_rx_channel.0
            )
            .into());
        }
        if board_profile.ir_pin_rx_channel.1 == board_profile.ir_pin_rx_channel2.1 {
            return Err(format!(
                "invalid board profile {} {}: ir_pin_rx_channel and ir_pin_rx_channel2 channels must differ (channel{})",
                board_profile.chip_feature(),
                board_profile.board_dir(),
                board_profile.ir_pin_rx_channel.1
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
