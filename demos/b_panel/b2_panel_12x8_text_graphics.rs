#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};

use device_envoy::{
    Result,
    led_strip::{ToRgb888, colors},
    led2d,
    led2d::Frame2d,
    led2d::Led2dFont,
    led2d::layout::LedLayout,
};
use embassy_executor::Spawner;
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use {defmt_rtt as _, panic_probe as _};

// As before: Two 12x4 panels stacked vertically to create a 12x8 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

led2d! {
    Led12x8 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8_ROTATED,
        font: Led2dFont::Font4x6Trim,
        // Same options as led_strip! (PIO, DMA, gamma, max current, max frames)
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

    // A 2D array of pixels.
    let mut frame2d = Frame2d::new();

    // Can write text to a frame instead of directly to the LED panel.
    let text_colors = [colors::ORANGE, colors::HOT_PINK];
    led12x8.write_text_to_frame("Go", &text_colors, &mut frame2d)?;

    // Can read and write the frame's pixels directly.
    // Index with tuple (x, y). Origin (0,0) is top-left.
    // Fill in the letter "o" with 4 pixels.
    frame2d[(5, 3)] = colors::HOT_PINK;
    frame2d[(6, 3)] = colors::HOT_PINK;
    frame2d[(5, 4)] = colors::HOT_PINK;
    frame2d[(6, 4)] = colors::HOT_PINK;

    // With embedded-graphics, you can draw any shapes you want (and text too).

    // - We use smart-leds' `RGB8` color type throughout device-envoy.
    //   embedded-graphics uses its own `Rgb888`, so we convert.
    // - Named colors follow different conventions.
    //   smart-leds uses web/X11 colors (GREEN = 0,128,0).
    //   embedded-graphics uses full RGB channels (GREEN = 0,255,0).
    let line_style = PrimitiveStyle::with_stroke(colors::RED.to_rgb888(), 1);
    // These geometry points are compile-time constants.
    const MID_Y: i32 = Led12x8::HEIGHT as i32 / 2;
    const MID_LEFT: Point = Point::new(Led12x8::BOTTOM_LEFT.x, MID_Y);
    const MID_RIGHT: Point = Point::new(Led12x8::BOTTOM_RIGHT.x, MID_Y);
    // With embedded-graphics, you can draw any shapes you want (and text too).
    Line::new(MID_LEFT, Led12x8::BOTTOM_RIGHT)
        .into_styled(line_style)
        .draw(&mut frame2d)?;
    Line::new(MID_RIGHT, Led12x8::BOTTOM_LEFT)
        .into_styled(line_style)
        .draw(&mut frame2d)?;

    // Write the frame to the LED panel. It stays until you replace it.
    led12x8.write_frame(frame2d)?;

    future::pending().await // run forever
}

// Not shown today:
//   -- 2D animations. They work as you'd expect from the 1D example.
//      Animations take an iterator of (frame, duration) and copy it into their own frame array.
//   -- Having up to four LED strips and panels share one PIO resource, via the `led_strips!` macro.
//      On a Pico 2, this lets you control up to 12 strips and panels total.
