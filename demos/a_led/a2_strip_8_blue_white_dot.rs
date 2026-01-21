#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led_strip::{Current, Frame1d, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
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
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    // Start with all blue LEDs
    let mut frame1d = Frame1d::filled(colors::BLUE);
    // Do 0,1,2,3 over and over (a computer-science foxtrot)
    for dot_index in (0..4).cycle() {
        frame1d[dot_index] = colors::WHITE;
        frame1d[dot_index + 4] = colors::WHITE;

        led_strip8.write_frame(frame1d).await?;
        Timer::after(Duration::from_millis(150)).await;

        frame1d[dot_index] = colors::BLUE;
        frame1d[dot_index + 4] = colors::BLUE;
    }

    // Needed because compiler can't tell this is an infinite loop
    future::pending().await // run forever
}
