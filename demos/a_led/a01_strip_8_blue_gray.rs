#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::convert::Infallible;
use core::future;

use device_kit::Result;
use device_kit::led_strip::{Current, Frame1d, colors, led_strip};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip8 {
        pin: PIN_0,
        len: 8,
        max_current: Current::Milliamps(50),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    let palette = [colors::BLUE, colors::GRAY];
    let mut frame1d = Frame1d::new();
    for (pixel_index, pixel) in frame1d.iter_mut().enumerate() {
        *pixel = palette[pixel_index % palette.len()];
    }

    led_strip8.write_frame(frame1d).await?;

    future::pending().await
}
