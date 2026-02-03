#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::future;
use core::{convert::Infallible, panic};
use device_envoy::{
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

    let steady_frame = Frame1d::filled(colors::YELLOW);
    let mut blink_frame = steady_frame; // copy

    // A button just needs to know its pin and whether it connects to Vcc or Ground.
    // (Pico 2 erratum E9 is avoided by wiring buttons to GND.)
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    for led_index in (0..LedStrip8::LEN).cycle() {
        blink_frame[led_index] = colors::BLACK; // add hole
        const BLINK_DELAY: Duration = Duration::from_millis(150);
        led_strip8.animate([(steady_frame, BLINK_DELAY), (blink_frame, BLINK_DELAY)])?;

        // Wait for a press. (If down already, waits for release, first.)
        // Do debouncing internally. When pressed, don't wait for release.
        button.wait_for_press().await;

        blink_frame[led_index] = colors::YELLOW;

        // Using device abstractions for LEDs and buttons lets us
        // avoid tricky async programming issues like loop/select & select/loop.
        // Each device abstraction handles its own business internally.
    }
    future::pending().await // Needed because compiler doesn't know "cycle" is infinite.
}
