#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    button::{Button, PressedTo},
    led_strip::{Frame1d, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::Duration;
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip8 {
        pin: PIN_0,
        len: 8,
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
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    demo_c1(&led_strip8, &mut button).await?;

    future::pending().await
}

async fn demo_c1(led_strip8: &LedStrip8, button: &mut Button<'_>) -> Result<()> {
    const BLINK_DELAY: Duration = Duration::from_millis(150);

    loop {
        for led_index in 0..LedStrip8::LEN {
            let mut off_frame = Frame1d::filled(colors::BLACK);
            let mut on_frame = Frame1d::filled(colors::BLACK);
            for solid_index in 0..led_index {
                off_frame[solid_index] = colors::YELLOW;
                on_frame[solid_index] = colors::YELLOW;
            }
            on_frame[led_index] = colors::YELLOW;

            led_strip8
                .animate([(off_frame, BLINK_DELAY), (on_frame, BLINK_DELAY)])
                .await?;
            button.wait_for_press().await;

            let mut frame1d = Frame1d::filled(colors::BLACK);
            for solid_index in 0..=led_index {
                frame1d[solid_index] = colors::YELLOW;
            }
            led_strip8.write_frame(frame1d).await?;
        }

        let frame1d = Frame1d::filled(colors::BLACK);
        led_strip8.write_frame(frame1d).await?;
    }
}
