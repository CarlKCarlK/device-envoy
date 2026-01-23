// #![no_std]
// #![no_main]
// #![cfg(not(feature = "host"))]

// use core::{convert::Infallible, panic};

// use device_kit::{
//     Result,
//     button::{Button, PressDuration, PressedTo},
//     led_strip::{Frame1d, RGB8, colors, led_strip},
// };
// use embassy_executor::Spawner;
// use embassy_time::Duration;
// use {defmt_rtt as _, panic_probe as _};

// led_strip! {
//     LedStrip8 {
//         pin: PIN_0,
//         len: 8,
//         max_frames: 2,
//     }
// }

// #[embassy_executor::main]
// async fn main(spawner: Spawner) -> ! {
//     let err = inner_main(spawner).await.unwrap_err();
//     panic!("{err}");
// }

// async fn inner_main(spawner: Spawner) -> Result<Infallible> {
//     let p = embassy_rp::init(Default::default());

//     // A button just needs to know its pin and whether it connects to Vcc or Ground.
//     // (Pico 2 erratum E9 is avoided by wiring buttons to GND.)
//     let mut button = Button::new(p.PIN_13, PressedTo::Ground);

//     // We also create a LED strip on GPIO0 with length 8.
//     let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

//     const BLINK_DELAY: Duration = Duration::from_millis(150);
//     const COLORS: [RGB8; 4] = [colors::YELLOW, colors::ORANGE, colors::GREEN, colors::BLUE];

//     let mut led_index: usize = 0;
//     let mut color_index: usize = 0;
//     let mut base_frame = Frame1d::new();

//     loop {
//         let color = COLORS[color_index];
//         let short_frame = base_frame;
//         let mut long_frame = base_frame;
//         if led_index < LedStrip8::LEN {
//             long_frame[led_index] = color;
//         }
//         led_strip8.animate([(short_frame, BLINK_DELAY), (long_frame, BLINK_DELAY)])?;

//         match button.wait_for_press_duration().await {
//             PressDuration::Short => {
//                 if led_index == LedStrip8::LEN {
//                     led_index = 0;
//                     base_frame = Frame1d::new();
//                 }

//                 base_frame[led_index] = color;
//                 led_index += 1;
//             }
//             PressDuration::Long => {
//                 color_index = (color_index + 1) % COLORS.len();
//             }
//         }
//     }
// }

#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};

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

    // A button just needs to know its pin and whether it connects to Vcc or Ground.
    // (Pico 2 erratum E9 is avoided by wiring buttons to GND.)
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // We also create a LED strip on GPIO0 with length 8.
    let led_strip8 = LedStrip8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;

    const BLINK_DELAY: Duration = Duration::from_millis(150);

    loop {
        // These frames *ours*. `animate()` copies them. After 4 presses:
        // short frame: Ⓨ Ⓨ Ⓨ Ⓨ
        // long  frame: Ⓨ Ⓨ Ⓨ Ⓨ Ⓨ
        let mut short_frame = Frame1d::new();
        let mut long_frame = Frame1d::new();
        for led_index in 0..LedStrip8::LEN {
            long_frame[led_index] = colors::YELLOW;
            led_strip8.animate([(short_frame, BLINK_DELAY), (long_frame, BLINK_DELAY)])?;

            // Wait for the next *new* press: first ensure the button is stably released,
            // then return on a debounced down (does not wait for release).
            button.wait_for_press().await;

            // No manual concurrency here: the LED strip task keeps animating on its own,
            // and the button abstraction handles debouncing internally.

            short_frame[led_index] = colors::YELLOW;
        }
    }
}
