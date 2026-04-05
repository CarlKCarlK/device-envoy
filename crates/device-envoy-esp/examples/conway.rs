#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use device_envoy_example_common::conway::conway_with_led2d_ir_kepler;
use embassy_executor::Spawner;
use esp_backtrace as _;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_esp::{
    init_and_start, ir_kepler, led2d,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::Current,
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_row_major();

led2d! {
    Led16x16 {
        pin: GPIO2,
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        engine: Engine::Spi,
        max_frames: 30,
    }
}

#[cfg(esp_gdma_family)] // C6, S3, etc
ir_kepler! {
    IrKepler7 { pin: GPIO7 }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
ir_kepler! {
    IrKepler7 { pin: GPIO4 }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let led16x16 = Led16x16::new(p.GPIO2, p.SPI2, spawner)?;

    #[cfg(feature = "esp32s3")]
    let channel_creator = rmt80.channel4; // ESP32-S3 requires RX channel 4+.
    #[cfg(not(feature = "esp32s3"))]
    let channel_creator = rmt80.channel2;

    #[cfg(esp_gdma_family)]
    let ir_kepler7 = IrKepler7::new(p.GPIO7, channel_creator, spawner)?;
    #[cfg(esp_pdma_family)]
    let ir_kepler7 = IrKepler7::new(p.GPIO4, channel_creator, spawner)?;

    conway_with_led2d_ir_kepler(led16x16, ir_kepler7).await
}
