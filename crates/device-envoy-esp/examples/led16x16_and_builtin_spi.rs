//! Mixed backend demo: 16x16 panel via SPI and built-in LED via RMT.
//!
//! Wiring:
//! - GPIO2: 16x16 NeoPixel-style (WS2812) panel data-in (SPI MOSI)
//! - GPIO8 (C6) or GPIO48 (S3): built-in NeoPixel-style (WS2812) LED (RMT)

#![no_std]
#![no_main]

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led2d,
    led2d::{layout::LedLayout, Frame2d, Led2d, Led2dFont},
    led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
};

esp_bootloader_esp_idf::esp_app_desc!();

const PANEL_16X16_PIN_NUM: u8 = 2;
#[cfg(target_arch = "riscv32")]
const BUILTIN_LED_PIN_NUM: u8 = 8;
#[cfg(target_arch = "xtensa")]
const BUILTIN_LED_PIN_NUM: u8 = 48;
const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_column_major();

led2d! {
    Led16x16DualSpi {
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        engine: Engine::Spi,
        max_frames: 4,
    }
}

led_strip! {
    BuiltinLedDualRmt {
        len: 1,
        max_current: Current::Milliamps(10),
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
        "led16x16_and_builtin_spi: panel on GPIO{} via SPI2, built-in LED on GPIO{} via RMT",
        PANEL_16X16_PIN_NUM, BUILTIN_LED_PIN_NUM
    );

    let led16x16_dual_spi = Led16x16DualSpi::new(p.GPIO2, p.SPI2, spawner)?;
    #[cfg(target_arch = "riscv32")]
    let builtin_led_dual_rmt = BuiltinLedDualRmt::new(p.GPIO8, rmt80.channel1, spawner)?;
    #[cfg(target_arch = "xtensa")]
    let builtin_led_dual_rmt = BuiltinLedDualRmt::new(p.GPIO48, rmt80.channel1, spawner)?;

    let mut panel_x_index = 0usize;
    let mut panel_y_index = 0usize;
    let mut builtin_led_on = false;
    let mut tick_index = 0usize;
    const TICK: Duration = Duration::from_millis(120);

    loop {
        let builtin_frame = if builtin_led_on {
            Frame1d([colors::MAGENTA])
        } else {
            Frame1d([colors::BLACK])
        };
        builtin_led_dual_rmt.write_frame(builtin_frame);
        builtin_led_on = !builtin_led_on;

        if tick_index % 2 == 0 {
            let mut panel_frame2d = Frame2d::<16, 16>::new();
            panel_frame2d[(panel_x_index, panel_y_index)] = colors::WHITE;
            Led2d::write_frame(&led16x16_dual_spi, panel_frame2d);
            panel_x_index += 1;
            if panel_x_index >= 16 {
                panel_x_index = 0;
                panel_y_index = (panel_y_index + 1) % 16;
            }
        }

        tick_index = tick_index.wrapping_add(1);
        Timer::after(TICK).await;
    }
}
