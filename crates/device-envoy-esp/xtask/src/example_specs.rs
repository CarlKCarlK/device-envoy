use crate::boards::BoardProfile;

#[derive(Clone, Copy)]
pub(crate) enum BlinkyKind {
    Plain,
    SmartRmt,
    SmartSpi,
}

pub(crate) const LED16X16_VARIANTS: [bool; 2] = [false, true];

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
