use crate::boards::{AudioWiring, BoardProfile};

#[derive(Clone, Copy)]
pub(crate) enum BlinkyKind {
    Plain,
    SmartRmt,
    SmartSpi,
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

pub(crate) fn supports_led16x16_examples(board_profile: BoardProfile) -> bool {
    board_profile.chip_feature() != "esp32c2"
}

pub(crate) fn panel16x16_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile.panel16x16_pin
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

pub(crate) fn supports_ir_examples(board_profile: BoardProfile) -> bool {
    !matches!(board_profile.chip_feature(), "esp32c2")
}

pub(crate) fn supports_conway_example(board_profile: BoardProfile) -> bool {
    supports_ir_examples(board_profile)
}

pub(crate) fn supports_clock_examples(board_profile: BoardProfile) -> bool {
    matches!(
        board_profile.chip_feature(),
        "esp32" | "esp32c2" | "esp32c3" | "esp32c6" | "esp32s2" | "esp32s3"
    )
}

pub(crate) fn talk1_strip8_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32" | "esp32h2" => 2,
        _ => 10,
    }
}

pub(crate) fn talk1_strip8_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", talk1_strip8_pin_num(board_profile))
}

pub(crate) fn talk1_panel12x8_pin_num(board_profile: BoardProfile) -> u8 {
    if board_profile.chip_feature() == "esp32h2" {
        2
    } else {
        18
    }
}

pub(crate) fn talk1_panel12x8_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", talk1_panel12x8_pin_num(board_profile))
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

pub(crate) fn clock_lcd_sda_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32c2" => 4,
        _ => 16,
    }
}

pub(crate) fn clock_lcd_sda_pin_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", clock_lcd_sda_pin_num(board_profile))
}

pub(crate) fn clock_lcd_scl_pin_num(board_profile: BoardProfile) -> u8 {
    match board_profile.chip_feature() {
        "esp32c2" => 5,
        _ => 17,
    }
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
    board_profile.ir_pin
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

pub(crate) fn ir_pin2_num(board_profile: BoardProfile) -> u8 {
    board_profile.ir_pin2
}

pub(crate) fn ir_pin2_ident(board_profile: BoardProfile) -> String {
    format!("GPIO{}", ir_pin2_num(board_profile))
}

pub(crate) fn ir_rx_channel_num(board_profile: BoardProfile) -> u8 {
    board_profile.ir_rx_channel
}

pub(crate) fn ir_rx_channel_ident(board_profile: BoardProfile) -> String {
    format!("channel{}", ir_rx_channel_num(board_profile))
}

pub(crate) fn ir_rx_channel2_num(board_profile: BoardProfile) -> u8 {
    board_profile.ir_rx_channel2
}

pub(crate) fn ir_rx_channel2_ident(board_profile: BoardProfile) -> String {
    format!("channel{}", ir_rx_channel2_num(board_profile))
}

pub(crate) fn audio_board_config(board_profile: BoardProfile) -> Option<AudioWiring> {
    board_profile.audio_wiring
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
