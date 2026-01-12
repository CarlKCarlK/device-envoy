#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future;

use defmt::info;
use defmt_rtt as _;
use device_kit::Result;
use device_kit::led_strip::{Current, Frame1d, Gamma, colors, led_strips};
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;

led_strips! {
    pio: PIO1,                          // Optional; default is PIO0
    LedStrips1 {
        gpio3: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(250),
        },
        gpio4: {
            pin: PIN_4,
            len: 96,
            max_current: Current::Milliamps(1000),
            gamma: Gamma::Linear,
            max_frames: 3,
            dma: DMA_CH1,
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let (gpio3_led_strip, gpio4_led_strip) =
        LedStrips1::new(p.PIO1, p.PIN_3, p.DMA_CH0, p.PIN_4, p.DMA_CH1, spawner)?;

    info!("Setting every other LED to blue on GPIO3, animating GPIO4");

    let mut frame = Frame1d::new();
    for pixel_index in (0..frame.len()).step_by(2) {
        frame[pixel_index] = colors::BLUE;
    }
    gpio3_led_strip.write_frame(frame).await?;

    let frame_duration = Duration::from_secs(1);
    gpio4_led_strip
        .animate([
            (Frame1d::filled(colors::GREEN), frame_duration),
            (Frame1d::filled(colors::YELLOW), frame_duration),
            (Frame1d::filled(colors::RED), frame_duration),
        ])
        .await?;

    future::pending::<Result<Infallible>>().await // Run forever
}
