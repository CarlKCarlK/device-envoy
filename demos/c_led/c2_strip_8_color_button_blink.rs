#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    led_strip::{Frame1d, RGB8, colors, led_strip},
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

    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    const COLORS: [RGB8; 4] = [colors::YELLOW, colors::ORANGE, colors::GREEN, colors::BLUE];
    let mut colors_iter = COLORS.iter().cycle();
    let mut color = *colors_iter.next().unwrap();

    let mut steady_frame = Frame1d::filled(color);
    let mut blink_frame = steady_frame; // copy

    let mut led_index_iter = (0..LedStrip8::LEN).cycle();
    let mut led_index = led_index_iter.next().unwrap();

    loop {
        blink_frame[led_index] = colors::BLACK;
        steady_frame[led_index] = color;
        const BLINK_DELAY: Duration = Duration::from_millis(150);
        led_strip8.animate([(blink_frame, BLINK_DELAY), (steady_frame, BLINK_DELAY)])?;

        // Wait for a button press.
        // Tells if a long or short press.
        // Long press returns as soon as it becomes long (no need to release).
        match button.wait_for_press_duration().await {
            // If short, fill "hole" with current color and move to next LED.
            PressDuration::Short => {
                blink_frame[led_index] = color;
                led_index = led_index_iter.next().unwrap()
            }

            // On a long, changes color for subsequent LEDs.
            PressDuration::Long => {
                color = *colors_iter.next().unwrap();
            }
        }
    }
}
