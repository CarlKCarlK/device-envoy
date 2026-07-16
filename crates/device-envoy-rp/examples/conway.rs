#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::conway::conway_with_led2d_ir_kepler;
use device_envoy_rp::led_strip::Current;
use device_envoy_rp::led2d;
use device_envoy_rp::led2d::{Led2dFont, layout::LedLayout};
use device_envoy_rp::{Result, ir_kepler};
use embassy_executor::Spawner;
use embassy_rp::init;
use panic_probe as _;

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_column_major();

led2d! {
    Led16x16 {
        pin: PIN_6,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(500),
        max_frames: 30,
        font: Led2dFont::Font4x6Trim,
    }
}

ir_kepler! {
    IrKepler15 { pio: PIO1, pin: PIN_15 }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Conway's Game of Life on 16x16 LED panel (IR remote on GPIO15)");
    let p = init(Default::default());

    let led16x16 = Led16x16::new(p.PIN_6, p.PIO0, p.DMA_CH0, spawner)?;
    let ir_kepler15 = IrKepler15::new(p.PIO1, p.PIN_15, spawner)?;
    conway_with_led2d_ir_kepler(led16x16, ir_kepler15).await
}
