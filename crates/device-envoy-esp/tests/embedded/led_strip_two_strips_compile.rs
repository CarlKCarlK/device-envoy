//! Embedded compile-only test target for two LED strips on distinct RMT channels.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32")]
led_strip! {
    LedStripA {
        max_frames: 2,
        len: 8,
        pin: GPIO0,
        max_current: Current::Milliamps(120),
    }
}

#[cfg(not(feature = "esp32"))]
led_strip! {
    LedStripA {
        max_frames: 2,
        len: 8,
        pin: GPIO10,
        max_current: Current::Milliamps(120),
    }
}

#[cfg(feature = "esp32")]
led_strip! {
    LedStripB {
        len: 1,
        max_frames: 2,
        pin: GPIO1,
    }
}

#[cfg(feature = "esp32s3")]
led_strip! {
    LedStripB {
        len: 1,
        max_frames: 2,
        pin: GPIO48,
    }
}

#[cfg(all(
    target_arch = "xtensa",
    not(feature = "esp32"),
    not(feature = "esp32s3")
))]
led_strip! {
    LedStripB {
        len: 1,
        max_frames: 2,
        pin: GPIO0,
    }
}

#[cfg(not(target_arch = "xtensa"))]
led_strip! {
    LedStripB {
        max_frames: 2,
        len: 1,
        pin: GPIO8,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);

    #[cfg(feature = "esp32")]
    let led_strip_a = LedStripA::new(p.GPIO0, rmt80.channel0, spawner)?;
    #[cfg(not(feature = "esp32"))]
    let led_strip_a = LedStripA::new(p.GPIO10, rmt80.channel0, spawner)?;
    #[cfg(feature = "esp32")]
    let led_strip_b = LedStripB::new(p.GPIO1, rmt80.channel1, spawner)?;
    #[cfg(feature = "esp32s3")]
    let led_strip_b = LedStripB::new(p.GPIO48, rmt80.channel1, spawner)?;
    #[cfg(all(
        target_arch = "xtensa",
        not(feature = "esp32"),
        not(feature = "esp32s3")
    ))]
    let led_strip_b = LedStripB::new(p.GPIO0, rmt80.channel1, spawner)?;
    #[cfg(not(target_arch = "xtensa"))]
    let led_strip_b = LedStripB::new(p.GPIO8, rmt80.channel1, spawner)?;

    led_strip_a.write_frame(Frame1d([
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
    ]));
    led_strip_b.write_frame(Frame1d([colors::RED]));

    core::future::pending().await
}
