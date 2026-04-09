#![allow(missing_docs)]
#![no_std]
#![no_main]
use core::{convert::Infallible, future::pending};

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::Result;
use device_envoy_rp::led_strip::led_strip;
use device_envoy_rp::led_strip::{Current, Frame1d, LedStrip as _, colors};
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;

led_strip! {
    LedStripLen8 {
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

    let led_strip_len8 = LedStripLen8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    info!("LED strip demo starting (GPIO0 data, VSYS power)");

    let mut position: isize = 0;
    let mut direction: isize = 1;

    loop {
        update_bounce(&led_strip_len8, position as usize).await?;

        position += direction;
        if position <= 0 {
            position = 0;
            direction = 1;
        } else if position as usize >= LedStripLen8::LEN - 1 {
            position = (LedStripLen8::LEN - 1) as isize;
            direction = -1;
        }

        Timer::after_millis(500).await;
    }
}

async fn update_bounce(led_strip_len8: &LedStripLen8, position: usize) -> Result<()> {
    assert!(position < LedStripLen8::LEN);
    let mut frame = Frame1d::new();
    frame[position] = colors::WHITE;
    led_strip_len8.write_frame(frame);
    Ok(())
}
