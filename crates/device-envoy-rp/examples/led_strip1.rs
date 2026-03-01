#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future;

use defmt::info;
use defmt_rtt as _;
use device_envoy::Result;
use device_envoy::led_strip::led_strip;
use device_envoy::led_strip::{Frame1d, colors};
use embassy_executor::Spawner;
use panic_probe as _;

led_strip! {
    LedStrip3 {
        pin: PIN_3,
        len: 48,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip3 = LedStrip3::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;

    info!("Setting LEDs to alternating blue/gray on GPIO3");

    let mut frame = Frame1d::new();
    for pixel_index in 0..LedStrip3::LEN {
        frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
    }
    led_strip3.write_frame(frame)?;

    future::pending::<Result<Infallible>>().await // Run forever
}
