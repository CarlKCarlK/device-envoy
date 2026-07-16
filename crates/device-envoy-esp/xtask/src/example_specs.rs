use crate::boards::BoardProfile;

#[derive(Clone, Copy)]
pub(crate) enum BlinkyKind {
    Plain,
    SmartRmt,
    SmartSpi,
}

pub(crate) fn blinky_kind(board_profile: BoardProfile) -> BlinkyKind {
    match (
        board_profile.built_in_smart_led,
        board_profile.rmt_count,
        board_profile.spi_count,
    ) {
        (Some(_), rmt_count, _) if rmt_count > 0 => BlinkyKind::SmartRmt,
        (Some(_), _, spi_count) if spi_count > 0 => BlinkyKind::SmartSpi,
        _ => BlinkyKind::Plain,
    }
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

pub(crate) fn led_strip1_pin_num(board_profile: BoardProfile) -> u8 {
    board_profile
        .built_in_smart_led
        .unwrap_or(board_profile.external_smart_led)
}

pub(crate) fn led_strip1_built_in(board_profile: BoardProfile) -> bool {
    board_profile.built_in_smart_led.is_some()
}
