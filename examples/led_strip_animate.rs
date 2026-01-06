#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future;

use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::{Current, Frame, Gamma, colors, led_strip};
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

led_strip! {
    LedStrip4 {
        pin: PIN_4,
        len: 96,
        pio: PIO1,
        dma: DMA_CH3,
        max_current: Current::Milliamps(1000),
        gamma: Gamma::Linear,
        max_frames: 3,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip4 = LedStrip4::new(p.PIN_4, p.PIO1, p.DMA_CH3, spawner)?;

    let frame_duration = Duration::from_secs(1);
    led_strip4
        .animate([
            (Frame::filled(colors::GREEN), frame_duration),
            (Frame::filled(colors::YELLOW), frame_duration),
            (Frame::filled(colors::RED), frame_duration),
        ])
        .await?;

    Ok(future::pending::<Infallible>().await) // Run forever
}
