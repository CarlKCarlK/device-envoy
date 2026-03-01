#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_envoy::{Result, led_strip::colors, led2d, led2d::Led2dFont, led2d::layout::LedLayout};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// Two 12x4 panels stacked vertically to create a 12x8 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

// Define a struct `Led12x8` to control a 12x8 LED panel on PIN_4
led2d! {
    Led12x8 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8_ROTATED,
        // Use a 4x6 pixel font with no gap between characters
        font: Led2dFont::Font4x6Trim,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led12x8 = Led12x8::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    // Text supports "\n" for multiple lines.
    // Colors are per-character, repeat as needed, and default to white.
    led12x8
        .write_text(
            "Go\nGo",
            &[
                colors::LIGHT_GRAY,
                colors::LIGHT_GRAY,
                colors::ORANGE,
                colors::HOT_PINK,
            ],
        )
        .await?;

    future::pending().await // run forever
}
