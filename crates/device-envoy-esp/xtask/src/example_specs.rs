use crate::boards::{BoardId, BoardProfile};

#[derive(Clone, Copy)]
pub(crate) enum BlinkyKind {
    Plain,
    SmartRmt,
    SmartSpi,
}

pub(crate) const LED16X16_VARIANTS: [bool; 2] = [false, true];
pub(crate) const AUDIO_EXAMPLE_BASE_NAMES: [&str; 5] = [
    "audio",
    "audio_example1",
    "audio_example1_trait",
    "audio_example2_trait",
    "audio_example3_trait",
];
pub(crate) const IR_EXAMPLE_BASE_NAMES: [&str; 6] = [
    "ir",
    "ir_example1_trait",
    "ir_kepler",
    "ir_kepler_example1_trait",
    "ir_keplers",
    "ir_mapping_example1_trait",
];
pub(crate) const CLOCK_EXAMPLE_BASE_NAMES: [&str; 6] = [
    "clock_console_simple",
    "clock_lcd",
    "clock_led4",
    "clock_led8x12",
    "clock_servos",
    "clock_sync_example1_trait",
];

pub(crate) struct AudioBoardConfig {
    pub(crate) data_pin_num: u8,
    pub(crate) bit_clock_pin_num: u8,
    pub(crate) word_select_pin_num: u8,
    pub(crate) button_pin_num: u8,
    pub(crate) dma_ident: &'static str,
}

pub(crate) fn blinky_kind(board_profile: BoardProfile) -> BlinkyKind {
    if board_profile.built_in_smart_led.is_some() {
        if board_profile.rmt_count > 0 {
            return BlinkyKind::SmartRmt;
        }
        if board_profile.spi_count > 0 {
            return BlinkyKind::SmartSpi;
        }
    }
    BlinkyKind::Plain
}

pub(crate) fn blinky_led_pin_num(board_profile: BoardProfile) -> u8 {
    match blinky_kind(board_profile) {
        BlinkyKind::Plain => board_profile
            .built_in_plain_led
            .unwrap_or(board_profile.default_external_plain_led),
        BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => board_profile
            .built_in_smart_led
            .expect("smart-led blinky kind requires built_in_smart_led pin"),
    }
}

pub(crate) fn blinky_led_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", blinky_led_pin_num(board_profile))
}

pub(crate) fn blinky_built_in_led(board_profile: BoardProfile) -> bool {
    match blinky_kind(board_profile) {
        BlinkyKind::Plain => board_profile.built_in_plain_led.is_some(),
        BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => board_profile.built_in_smart_led.is_some(),
    }
}

pub(crate) fn blinky_example_name(board_profile: BoardProfile) -> String {
    format!(
        "blinky_{}_{}",
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

pub(crate) fn supports_led16x16_examples(board_profile: BoardProfile) -> bool {
    board_profile.chip_feature() != "esp32c2"
}

pub(crate) fn panel16x16_pin_num(_board_profile: BoardProfile) -> u8 {
    2
}

pub(crate) fn panel16x16_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", panel16x16_pin_num(board_profile))
}

pub(crate) fn led_strip1_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .built_in_smart_led
        .unwrap_or(board_profile.default_external_smart_led)
}

pub(crate) fn led_strip1_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", led_strip1_pin_num(board_profile))
}

pub(crate) fn led_strip1_built_in(board_profile: BoardProfile) -> bool {
    board_profile.built_in_smart_led.is_some()
}

pub(crate) fn led16x16_example_name(board_profile: BoardProfile, use_spi: bool) -> String {
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

pub(crate) fn audio_example_name(board_profile: BoardProfile, base_name: &str) -> String {
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

pub(crate) fn supports_ir_examples(board_profile: BoardProfile) -> bool {
    !matches!(board_profile.chip_feature(), "esp32c2" | "esp32h2")
}

pub(crate) fn supports_conway_example(board_profile: BoardProfile) -> bool {
    supports_ir_examples(board_profile)
}

pub(crate) fn ir_example_name(board_profile: BoardProfile, base_name: &str) -> String {
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

pub(crate) fn conway_example_name(board_profile: BoardProfile) -> String {
    format!(
        "conway_{}_{}",
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

pub(crate) fn supports_clock_examples(board_profile: BoardProfile) -> bool {
    matches!(
        board_profile.chip_feature(),
        "esp32" | "esp32c6" | "esp32s2" | "esp32s3"
    )
}

pub(crate) fn clock_example_name(board_profile: BoardProfile, base_name: &str) -> String {
    format!(
        "{}_{}_{}",
        base_name,
        board_profile.chip_feature(),
        board_profile.board_dir()
    )
}

pub(crate) fn clock_force_portal_button_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => 0,
        _ => 6,
    }
}

pub(crate) fn clock_force_portal_button_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_force_portal_button_pin_num(board_profile))
}

pub(crate) fn clock_lcd_sda_pin_num(_board_profile: BoardProfile) -> u8 {
    16
}

pub(crate) fn clock_lcd_sda_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_lcd_sda_pin_num(board_profile))
}

pub(crate) fn clock_lcd_scl_pin_num(_board_profile: BoardProfile) -> u8 {
    17
}

pub(crate) fn clock_lcd_scl_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_lcd_scl_pin_num(board_profile))
}

pub(crate) fn clock_led8x12_panel_pin_num(_board_profile: BoardProfile) -> u8 {
    18
}

pub(crate) fn clock_led8x12_panel_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_led8x12_panel_pin_num(board_profile))
}

pub(crate) fn clock_servos_bottom_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => 4,
        _ => 10,
    }
}

pub(crate) fn clock_servos_bottom_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_servos_bottom_pin_num(board_profile))
}

pub(crate) fn clock_servos_top_pin_num(_board_profile: BoardProfile) -> u8 {
    18
}

pub(crate) fn clock_servos_top_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_servos_top_pin_num(board_profile))
}

pub(crate) fn clock_led4_cell_pin_nums(board_profile: BoardProfile) -> [u8; 4] {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => [14, 13, 12, 15],
        _ => [14, 13, 12, 11],
    }
}

pub(crate) fn clock_led4_cell_pin_idents(board_profile: BoardProfile) -> [String; 4] {
    clock_led4_cell_pin_nums(board_profile).map(|pin_num| format!("GPIO{pin_num}"))
}

pub(crate) fn clock_led4_segment_pin_nums(board_profile: BoardProfile) -> [u8; 8] {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => [4, 16, 17, 18, 19, 21, 1, 2],
        "esp32s3" => [10, 9, 46, 3, 8, 18, 17, 16],
        _ => [10, 9, 4, 3, 8, 18, 17, 16],
    }
}

pub(crate) fn clock_led4_segment_pin_idents(board_profile: BoardProfile) -> [String; 8] {
    clock_led4_segment_pin_nums(board_profile).map(|pin_num| format!("GPIO{pin_num}"))
}

pub(crate) fn ir_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => 4,
        _ => 7,
    }
}

pub(crate) fn ir_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", ir_pin_num(board_profile))
}

pub(crate) fn ir_kepler_receiver0_pin_num(board_profile: BoardProfile) -> u8 {
    ir_pin_num(board_profile)
}

pub(crate) fn ir_kepler_receiver0_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", ir_kepler_receiver0_pin_num(board_profile))
}

pub(crate) fn ir_kepler_receiver1_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => 5,
        _ => 4,
    }
}

pub(crate) fn ir_kepler_receiver1_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", ir_kepler_receiver1_pin_num(board_profile))
}

pub(crate) fn ir_rx_channel_num(board_profile: BoardProfile) -> u8 {
    if board_profile.chip_feature() == "esp32s3" {
        4
    } else {
        2
    }
}

pub(crate) fn ir_rx_channel_ident(board_profile: BoardProfile) -> String {
    format!("channel{}", ir_rx_channel_num(board_profile))
}

pub(crate) fn ir_rx_channel1_num(board_profile: BoardProfile) -> u8 {
    if board_profile.chip_feature() == "esp32s3" {
        5
    } else {
        3
    }
}

pub(crate) fn ir_rx_channel1_ident(board_profile: BoardProfile) -> String {
    format!("channel{}", ir_rx_channel1_num(board_profile))
}

pub(crate) fn audio_board_config(board_profile: BoardProfile) -> Option<AudioBoardConfig> {
    if board_profile.board_id == BoardId::Luatos && board_profile.chip_feature() == "esp32c3" {
        // LuatOS ESP32-C3 exposes GPIO21 as UART0 TX; keep audio DIN off that pin.
        return Some(AudioBoardConfig {
            data_pin_num: 5,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        });
    }

    match board_profile.chip_feature() {
        "esp32" | "esp32s2" => Some(AudioBoardConfig {
            data_pin_num: 21,
            bit_clock_pin_num: 4,
            word_select_pin_num: 5,
            button_pin_num: 0,
            dma_ident: "DMA_I2S0",
        }),
        "esp32c3" | "esp32c6" | "esp32s3" => Some(AudioBoardConfig {
            data_pin_num: 21,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 6,
            dma_ident: "DMA_CH0",
        }),
        "esp32h2" => Some(AudioBoardConfig {
            data_pin_num: 1,
            bit_clock_pin_num: 3,
            word_select_pin_num: 4,
            button_pin_num: 0,
            dma_ident: "DMA_CH0",
        }),
        // TODO0Audio ESP32-C2 audio examples are currently unsupported in
        // device-envoy-esp because the current esp-hal configuration for C2
        // does not expose the needed I2S support.
        _ => None,
    }
}

pub(crate) fn supports_audio_examples(board_profile: BoardProfile) -> bool {
    audio_board_config(board_profile).is_some()
}

pub(crate) fn audio_data_pin_ident(board_profile: BoardProfile) -> String {
    format!(
        "GPIO{}",
        audio_board_config(board_profile)
            .expect("audio_data_pin_ident called for unsupported board")
            .data_pin_num
    )
}

pub(crate) fn audio_bit_clock_pin_ident(board_profile: BoardProfile) -> String {
    format!(
        "GPIO{}",
        audio_board_config(board_profile)
            .expect("audio_bit_clock_pin_ident called for unsupported board")
            .bit_clock_pin_num
    )
}

pub(crate) fn audio_word_select_pin_ident(board_profile: BoardProfile) -> String {
    format!(
        "GPIO{}",
        audio_board_config(board_profile)
            .expect("audio_word_select_pin_ident called for unsupported board")
            .word_select_pin_num
    )
}

pub(crate) fn audio_button_pin_ident(board_profile: BoardProfile) -> String {
    format!(
        "GPIO{}",
        audio_board_config(board_profile)
            .expect("audio_button_pin_ident called for unsupported board")
            .button_pin_num
    )
}

pub(crate) fn audio_data_pin_num(board_profile: BoardProfile) -> u8 {
    audio_board_config(board_profile)
        .expect("audio_data_pin_num called for unsupported board")
        .data_pin_num
}

pub(crate) fn audio_bit_clock_pin_num(board_profile: BoardProfile) -> u8 {
    audio_board_config(board_profile)
        .expect("audio_bit_clock_pin_num called for unsupported board")
        .bit_clock_pin_num
}

pub(crate) fn audio_word_select_pin_num(board_profile: BoardProfile) -> u8 {
    audio_board_config(board_profile)
        .expect("audio_word_select_pin_num called for unsupported board")
        .word_select_pin_num
}

pub(crate) fn audio_button_pin_num(board_profile: BoardProfile) -> u8 {
    audio_board_config(board_profile)
        .expect("audio_button_pin_num called for unsupported board")
        .button_pin_num
}

pub(crate) fn audio_dma_ident(board_profile: BoardProfile) -> &'static str {
    audio_board_config(board_profile)
        .expect("audio_dma_ident called for unsupported board")
        .dma_ident
}
