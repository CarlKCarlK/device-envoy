#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_example_common::conway::run_conway;
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    esp_hal::gpio::{Level, Output, OutputConfig},
    init_and_start,
    ir::{IrKepler, IrKeplerStatic},
    led2d,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::Current,
    led2d::Led2d as _,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_row_major();
const PANEL_16X16_PIN_NUM: u8 = 2;
const IR_PIN_NUM: u8 = 7;

led2d! {
    Led16x16Conway {
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
    info!(
        "Conway: 16x16 SPI on GPIO{}, IR receiver on GPIO{}",
        PANEL_16X16_PIN_NUM, IR_PIN_NUM
    );

    // TODO0 Keep MAX98357A I2S inputs quiet in this non-audio example.
    let _audio_idle_pins = (
        Output::new(p.GPIO21, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO11, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO12, Level::Low, OutputConfig::default()),
    );

    let led16x16_conway = Led16x16Conway::new(p.GPIO2, p.SPI2, spawner)?;

    static IR_KEPLER_STATIC: IrKeplerStatic = IrKepler::new_static();
    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let ir_rmt_channel = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let ir_rmt_channel = rmt80.channel2;
    let ir_kepler = IrKepler::new(&IR_KEPLER_STATIC, p.GPIO7, ir_rmt_channel, spawner)?;
    run_conway(led16x16_conway, ir_kepler).await
}
