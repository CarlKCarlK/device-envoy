#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led_strip::{Frame1d, colors, led_strip},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// Define a struct `LedStrip8` to control 8 LEDs on PIN_0
led_strip! {
    LedStrip8 {
        pin: PIN_0,
        len: 8,
    }
}

// Nice trick: Two "mains" lets us use Results.
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a struct to control the LED strip from Pico resources.
    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    // Fill an array of pixels with alternating blue and gray colors
    let mut frame1d = Frame1d::new(); // just an owned array of RGB pixels
    let palette = [colors::BLUE, colors::LIGHT_GRAY];
    for pixel_index in 0..frame1d.len() {
        frame1d[pixel_index] = palette[pixel_index % 2];
    }

    // Write the frame to the LED strip. Will stay until replaced.
    led_strip8.write_frame(frame1d)?;

    future::pending().await // run forever
}
