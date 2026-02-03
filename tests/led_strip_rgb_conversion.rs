#![allow(missing_docs)]
use device_envoy::led_strip::{RGB8, Rgb888, ToRgb8, ToRgb888};

#[test]
fn rgb888_to_rgb8_matches_rgb8() {
    let rgb8_color = RGB8::new(16, 32, 48);
    let rgb888_color = Rgb888::new(16, 32, 48);

    let converted = rgb888_color.to_rgb8();

    assert_eq!(rgb8_color, converted);
}

#[test]
fn rgb8_to_rgb888_matches_rgb888() {
    let rgb8_color = RGB8::new(16, 32, 48);
    let rgb888_color = Rgb888::new(16, 32, 48);

    let converted = rgb8_color.to_rgb888();

    assert_eq!(rgb888_color, converted);
}

#[test]
fn rgb888_to_rgb888_is_identity() {
    let rgb888_color = Rgb888::new(16, 32, 48);

    let converted = rgb888_color.to_rgb888();

    assert_eq!(rgb888_color, converted);
}
