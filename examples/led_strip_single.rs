#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future;

use defmt::info;
use device_kit::Result;
use device_kit::led_strip::{Current, Frame1d, colors, led_strip};
use embassy_executor::Spawner;
use embassy_time::Duration;
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip {
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

    // Create strip - no tuple unpacking needed!
    let led_strip = LedStrip::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    info!("LED strip initialized with {} LEDs", LedStrip::LEN);

    // Create frames for the animation

    let rainbow_frame = Frame1d::from([
        colors::RED,
        colors::ORANGE,
        colors::YELLOW,
        colors::GREEN,
        colors::CYAN,
        colors::BLUE,
        colors::PURPLE,
        colors::MAGENTA,
    ]);

    let black_frame = Frame1d::new();

    info!("Starting rainbow animation...");
    const FRAME_DURATION: Duration = Duration::from_secs(1);
    led_strip.animate([
        (rainbow_frame, FRAME_DURATION),
        (black_frame, FRAME_DURATION),
    ])?;

    future::pending().await // run forever
}
