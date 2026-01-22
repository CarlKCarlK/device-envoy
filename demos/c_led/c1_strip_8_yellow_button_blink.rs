#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use defmt::info;
use device_kit::{
    Result,
    button::{Button, PressedTo},
    led_strip::{Frame1d, colors, led_strip},
};
use embassy_executor::Spawner;
use embassy_time::Duration; // , Timer};
use {defmt_rtt as _, panic_probe as _};

led_strip! {
    LedStrip8 {
        pin: PIN_0,
        len: 8,
        max_frames: 2,
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
        let mut solid_frame = Frame1d::new();
        let mut solid_and_blink_frame = Frame1d::new();
        for led_index in 0..LedStrip8::LEN {
            // Add the next blink LED
            solid_and_blink_frame[led_index] = colors::YELLOW;

            led_strip8
                .animate([
                    (solid_frame, BLINK_DELAY),
                    (solid_and_blink_frame, BLINK_DELAY),
                ])
                .await?;
            button.wait_for_press().await;

            // Add the next solid LED
            solid_frame[led_index] = colors::YELLOW;
        }
    }
}
