#![no_std]
#![no_main]

use core::future;
use defmt::info;
use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::led_strip;
use device_kit::led_strip::{Current, Frame, Rgb, colors};
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

led_strip! {
    Gpio5LedStrip {
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

async fn inner_main(spawner: Spawner) -> Result<()> {
    let p = embassy_rp::init(Default::default());

    let gpio5_led_strip = Gpio5LedStrip::new(p.PIN_5, p.PIO0, p.DMA_CH0, spawner)?;

    info!("Christmas marquee demo starting on GPIO5");

    const FRAME_DURATION: Duration = Duration::from_millis(80);
    const PULSE_SPACING: usize = 16;
    const TAIL_LENGTH: usize = 4;
    const FRAME_COUNT: usize = PULSE_SPACING;
    const GAP: Rgb = colors::BLACK;
    const HEAD: Rgb = colors::RED;
    const TAIL: Rgb = colors::GREEN;

    let mut frames = heapless::Vec::<(Frame<{ Gpio5LedStrip::LEN }>, Duration), FRAME_COUNT>::new();

    for frame_offset in 0..FRAME_COUNT {
        let mut frame = Frame::<{ Gpio5LedStrip::LEN }>::filled(GAP);

        for start in (0..Gpio5LedStrip::LEN).step_by(PULSE_SPACING) {
            let head_index = (start + frame_offset) % Gpio5LedStrip::LEN;
            frame[head_index] = HEAD;

            for distance in 1..=TAIL_LENGTH {
                let tail_index = (head_index + Gpio5LedStrip::LEN - distance) % Gpio5LedStrip::LEN;
                frame[tail_index] = TAIL;
            }
        }

        frames.push((frame, FRAME_DURATION)).ok();
    }

    info!(
        "Starting Christmas marquee animation with {} frames",
        frames.len()
    );

    gpio5_led_strip.animate(frames).await?;

    future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}
