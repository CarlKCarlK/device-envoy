#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, future};

use defmt::info;
use defmt_rtt as _;
use device_kit::{Result, led_strip::ToRgb888, led2d, led2d::Frame2d, led2d::layout::LedLayout};
use embassy_executor::Spawner;
use embassy_rp::init;
use embedded_graphics::{
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use panic_probe as _;
use smart_leds::colors;

// Two 12×4 panels stacked vertically to create a 12×8 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

led2d! {
    Led12x8 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8,
        font: Led2dFont::Font4x6Trim,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("LED 2D Example: Rectangle border with centered circle on a 12x8 panel");
    let p = init(Default::default());

    let led12x8 = Led12x8::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    // Create a frame to draw on. This is just an in-memory 2D pixel buffer.
    let mut frame = Frame2d::new();

    // Use the embedded-graphics crate to draw a red rectangle border around the edge of the frame.
    Rectangle::new(Led12x8::TOP_LEFT, Led12x8::SIZE)
        .into_styled(PrimitiveStyle::with_stroke(colors::RED.to_rgb888(), 1))
        .draw(&mut frame)?;

    // Direct pixel access: set the upper-left LED pixel (x = 0, y = 0).
    // Frame2d stores LED colors directly, so we write an LED color here.
    frame[(0, 0)] = colors::CYAN;

    // Use the embedded-graphics crate to draw a green circle centered in the frame.
    const DIAMETER: u32 = 6;
    const CIRCLE_TOP_LEFT: Point =
        centered_top_left(Led12x8::WIDTH, Led12x8::HEIGHT, DIAMETER as usize);
    Circle::new(CIRCLE_TOP_LEFT, DIAMETER)
        .into_styled(PrimitiveStyle::with_stroke(colors::LIME.to_rgb888(), 1))
        .draw(&mut frame)?;

    // Write the frame to the LED panel.
    led12x8.write_frame(frame).await?;

    future::pending().await // Run forever
}

/// Calculate the top-left corner position to center a shape within a bounding box.
const fn centered_top_left(width: usize, height: usize, size: usize) -> Point {
    assert!(size <= width); // compile-time check
    assert!(size <= height); // compile-time check
    Point::new(((width - size) / 2) as i32, ((height - size) / 2) as i32)
}
