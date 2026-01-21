#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led_strip::{Frame1d, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip96 {
        pin: PIN_4,
        len: 96,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip96 = LedStrip96::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

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
