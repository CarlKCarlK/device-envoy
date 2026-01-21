#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led_strip::{Current, Frame1d, Gamma, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    pub LedStrip96 { // can add 'pub' to make struct public
        pin: PIN_4,
        len: 96,
        // Optionals
        pio: PIO1,     // which of 2 or 3 PIO resources to use
        dma: DMA_CH5,  // which of 12 DMA resources to use
        max_current: Current::Milliamps(500), // limit max current to 500mA
        gamma: Gamma::Gamma2_2, // apply gamma correction
        max_frames: 1, // Allocate space for 1 frame of animation
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip96 = LedStrip96::new(p.PIN_4, p.PIO1, p.DMA_CH5, spawner)?;

    let mut frame1d = Frame1d::filled(colors::BLUE);
    for dot_index in (0..LedStrip96::LEN).cycle() {
        frame1d[dot_index] = colors::WHITE;
        led_strip96.write_frame(frame1d).await?;
        Timer::after(Duration::from_millis(50)).await;
        frame1d[dot_index] = colors::BLUE;
    }

    // Issues:
    // - Because of the weird wiring, it would be very hard to write text or draw a line.
    // - If we turned on all LEDs to white, the power draw would be
    //   96 x 60mA = 5.76A which is way too much for my power supply.

    future::pending().await
}
