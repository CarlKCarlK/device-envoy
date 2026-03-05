//! 16x16 panel mapping test: animate one foreground dot through every position.
//!
//! This is useful for verifying layout/wiring direction on a large panel.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led2d,
    led2d::Led2d as _,
    led2d::{layout::LedLayout, Frame2d, Led2dFont},
    led_strip::{colors, Current},
};

esp_bootloader_esp_idf::esp_app_desc!();

const PANEL_16X16_PIN_NUM: u8 = 2;
const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_column_major();

led2d! {
    Led16x16Test {
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        max_frames: 4,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!(
        "led16x16test starting on GPIO{} (one moving foreground dot)",
        PANEL_16X16_PIN_NUM
    );

    let led16x16_test = Led16x16Test::new(p.GPIO2, rmt80.channel0, spawner)?;
    const DOT_DELAY: Duration = Duration::from_millis(500);
    const WIDTH: usize = <&'static Led16x16Test as device_envoy_esp::led2d::Led2d<16, 16>>::WIDTH;
    const HEIGHT: usize = <&'static Led16x16Test as device_envoy_esp::led2d::Led2d<16, 16>>::HEIGHT;

    loop {
        for y_index in 0..HEIGHT {
            for x_index in 0..WIDTH {
                let mut frame2d = Frame2d::<16, 16>::new();
                frame2d[(x_index, y_index)] = colors::WHITE;
                led16x16_test.write_frame(frame2d);
                Timer::after(DOT_DELAY).await;
            }
        }
    }
}
