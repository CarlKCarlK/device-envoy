#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_kit::{
    Result,
    led2d,
    led2d::Frame2d,
    led2d::layout::LedLayout,
    led_strip::colors,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// Two 12x4 panels stacked vertically to create a 12x8 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

led2d! {
    Led12x8 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8,
        font: Led2dFont::Font3x4Trim,
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

    let mut frame2d = Frame2d::new();
    let text_colors = [colors::BLUE, colors::LIGHT_GRAY];
    led12x8.write_text_to_frame("Rust", &text_colors, &mut frame2d)?;

    const CURSOR_X: usize = 1;
    const CURSOR_Y_START: usize = 4;
    const CURSOR_HEIGHT: usize = 3;
    for cursor_row_offset in 0..CURSOR_HEIGHT {
        frame2d[(CURSOR_X, CURSOR_Y_START + cursor_row_offset)] = colors::LIGHT_GRAY;
    }

    led12x8.write_frame(frame2d).await?;

    future::pending().await
}
