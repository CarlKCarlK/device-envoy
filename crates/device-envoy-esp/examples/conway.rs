#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_example_common::conway::conway_with_led2d_ir_kepler;
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    esp_hal::gpio::{Level, Output, OutputConfig},
    init_and_start,
    ir::{IrKeplerEsp, IrKeplerStatic},
    led2d,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::Current,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_row_major();

led2d! {
    Led16x16 {
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        engine: Engine::Spi,
        max_frames: 30,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let led16x16 = Led16x16::new(p.GPIO2, p.SPI2, spawner)?;

    #[cfg(target_arch = "xtensa")]
    let ir_rmt_channel = rmt80.channel4; // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    #[cfg(not(target_arch = "xtensa"))]
    let ir_rmt_channel = rmt80.channel2; // On ESP32-C6, channels 0–3 all support RX.

    static IR_KEPLER_STATIC: IrKeplerStatic = IrKeplerEsp::new_static();
    let ir_kepler = IrKeplerEsp::new(&IR_KEPLER_STATIC, p.GPIO7, ir_rmt_channel, spawner)?;

    conway_with_led2d_ir_kepler(led16x16, ir_kepler).await
}
