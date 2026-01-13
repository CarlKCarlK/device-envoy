#![cfg(feature = "host")]

// cmk000000 need to inverse the gamma?

// check-all: skip (host-only PNG generation)

use device_kit::led2d::Frame2d;
use device_kit::to_png::write_frame_png;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use smart_leds::colors;
use std::error::Error;

type Frame = Frame2d<12, 8>;
fn main() -> Result<(), Box<dyn Error>> {
    let frame = build_frame();
    write_frame_png(&frame, "docs/assets/led2d_graphics.png", 200)?;
    Ok(())
}

fn build_frame() -> Frame {
    let mut frame: Frame = Frame::new();

    Rectangle::new(Frame::TOP_LEFT, Frame::SIZE)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
        .draw(&mut frame)
        .expect("rectangle draw must succeed");

    frame[0][0] = colors::CYAN;

    const DIAMETER: u32 = 6;
    const CIRCLE_TOP_LEFT: Point = centered_top_left(12, 8, DIAMETER as usize);
    Circle::new(CIRCLE_TOP_LEFT, DIAMETER)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::GREEN, 1))
        .draw(&mut frame)
        .expect("circle draw must succeed");

    frame
}

const fn centered_top_left(width: usize, height: usize, size: usize) -> Point {
    assert!(size <= width, "size must fit within width");
    assert!(size <= height, "size must fit within height");
    Point::new(((width - size) / 2) as i32, ((height - size) / 2) as i32)
}
