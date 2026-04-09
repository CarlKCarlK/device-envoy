#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::future::pending;
use core::{convert::Infallible, future::pending};
use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::Result;
use device_envoy_rp::led_strip::led_strip;
use device_envoy_rp::led_strip::{Current, Frame1d, LedStrip as _, RGB8, colors};
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

led_strip! {
    LedStripLen160 {
        pin: PIN_5,
        len: 160,
        max_current: Current::Milliamps(500),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(_) => unreachable!(),
        Err(e) => panic!("Fatal error: {:?}", e),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip_len160 = LedStripLen160::new(p.PIN_5, p.PIO0, p.DMA_CH0, spawner)?;

    info!("Christmas marquee demo starting on GPIO5");

    const FRAME_DURATION: Duration = Duration::from_millis(80);
    const PULSE_SPACING: usize = 16;
    const TAIL_LENGTH: usize = 4;
    const FRAME_COUNT: usize = PULSE_SPACING;
    const GAP: RGB8 = colors::BLACK;
    const HEAD: RGB8 = colors::RED;
    const TAIL: RGB8 = colors::GREEN;

    let mut frames = heapless::Vec::<_, FRAME_COUNT>::new();

    for frame_offset in 0..FRAME_COUNT {
        let mut frame = Frame1d::filled(GAP);

        for start in (0..LedStripLen160::LEN).step_by(PULSE_SPACING) {
            let head_index = (start + frame_offset) % LedStripLen160::LEN;
            frame[head_index] = HEAD;

            for distance in 1..=TAIL_LENGTH {
                let tail_index =
                    (head_index + LedStripLen160::LEN - distance) % LedStripLen160::LEN;
                frame[tail_index] = TAIL;
            }
        }

        frames.push((frame, FRAME_DURATION)).ok();
    }

    info!(
        "Starting Christmas marquee animation with {} frames",
        frames.len()
    );

    led_strip_len160.animate(frames);

    future::pending::<Result<Infallible>>().await // Run forever
}
