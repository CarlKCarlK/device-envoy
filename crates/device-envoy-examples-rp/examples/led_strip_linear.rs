//! LED strip linear demo: an eight-step brightness wedge, with no gamma correction and no power budget.
//!
//! Hardware:
//!
//! - 8-LED NeoPixel-style (WS2812) strip: data (Din) on GPIO0, +5V on VBUS, GND to GND
//! - Button on GPIO13, wired to ground; each press switches between the wedge and full white
//!
//! This version uses `gamma: Gamma::Linear` and `Current::Unlimited`, so the bright end of
//! the wedge crowds together and full white draws the strip's full current (Device Envoy
//! estimates 8 x 60 mA = 480 mA). Compare it with `led_strip_gamma`.
#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{
    Result,
    button::{Button as _, ButtonRp, PressedTo},
    led_strip::{Current, Frame1d, Gamma, LedStrip as _, RGB8, led_strip},
};
use embassy_executor::Spawner;
use panic_probe as _;

led_strip! {
    LedStripLen8 {
        pin: PIN_0,
        len: 8,
        max_current: Current::Unlimited,
        gamma: Gamma::Linear,
    }
}

/// Equal steps in app value: 0 to 255 in eight steps, leaving out 0 (off is off either way).
const WEDGE: [u8; 8] = [32, 64, 96, 128, 159, 191, 223, 255];

/// All eight LEDs at full white.
const FULL_WHITE: [u8; 8] = [255; 8];

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led_strip_len8 = LedStripLen8::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;
    let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);

    let mut pattern = Pattern::Wedge;
    loop {
        info!("Showing: {}", pattern.name());

        let mut frame = Frame1d::new();
        for (led_index, level) in pattern.levels().iter().enumerate() {
            frame[led_index] = RGB8::new(*level, *level, *level); // gray at this level
        }
        led_strip_len8.write_frame(frame);

        button.wait_for_press().await;
        pattern = pattern.next();
    }
}

#[derive(Clone, Copy)]
enum Pattern {
    Wedge,
    FullWhite,
}

impl Pattern {
    const fn levels(self) -> &'static [u8; 8] {
        match self {
            Self::Wedge => &WEDGE,
            Self::FullWhite => &FULL_WHITE,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Wedge => "eight-step wedge",
            Self::FullWhite => "all eight at full white",
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::Wedge => Self::FullWhite,
            Self::FullWhite => Self::Wedge,
        }
    }
}
