use crate::boards::BoardProfile;

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
            .unwrap_or(board_profile.external_plain_led),
        BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => board_profile
            .built_in_smart_led
            .expect("smart-led blinky kind requires built_in_smart_led pin"),
    }
}

pub(crate) fn blinky_built_in_led(board_profile: BoardProfile) -> bool {
    match blinky_kind(board_profile) {
        BlinkyKind::Plain => board_profile.built_in_plain_led.is_some(),
        BlinkyKind::SmartRmt | BlinkyKind::SmartSpi => board_profile.built_in_smart_led.is_some(),
    }
}

pub(crate) fn supports_led16x16_plus_1_example(board_profile: BoardProfile) -> bool {
    board_profile.rmt_count >= 2
}

pub(crate) fn supports_led16x16_plus_1_spi_example(board_profile: BoardProfile) -> bool {
    board_profile.rmt_count >= 2 && board_profile.spi_count >= 2
}

pub(crate) fn led_strip1_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .built_in_smart_led
        .unwrap_or(board_profile.external_smart_led)
}

pub(crate) fn led_strip1_built_in(board_profile: BoardProfile) -> bool {
    board_profile.built_in_smart_led.is_some()
}

pub(crate) fn supports_ir_examples(board_profile: BoardProfile) -> bool {
    board_profile.rmt_count > 0
}

pub(crate) fn supports_conway_example(board_profile: BoardProfile) -> bool {
    supports_ir_examples(board_profile) && board_profile.spi_count > 0
}

pub(crate) fn supports_audio_examples(board_profile: BoardProfile) -> bool {
    board_profile.audio_wiring.is_some()
}

pub(crate) fn audio_data_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .audio_wiring
        .expect("audio_data_pin_num called for unsupported board")
        .data_pin_num
}

pub(crate) fn audio_bit_clock_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .audio_wiring
        .expect("audio_bit_clock_pin_num called for unsupported board")
        .bit_clock_pin_num
}

pub(crate) fn audio_word_select_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .audio_wiring
        .expect("audio_word_select_pin_num called for unsupported board")
        .word_select_pin_num
}

pub(crate) fn audio_dma_ident(board_profile: BoardProfile) -> &'static str {
    board_profile
        .audio_wiring
        .expect("audio_dma_ident called for unsupported board")
        .dma_ident
}
