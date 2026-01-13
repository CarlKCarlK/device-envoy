#![cfg(feature = "host")]

use device_kit::led2d::Frame2d;
use device_kit::to_png::write_frame_png;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use smart_leds::colors;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type Frame = Frame2d<12, 8>;

#[test]
fn led2d_graphics_png_matches_expected() {
    let frame = build_frame();
    let expected_path = Path::new("docs/assets/led2d_graphics.png");
    assert!(
        expected_path.exists(),
        "expected PNG is missing at {}",
        expected_path.display()
    );

    let output_path = temp_output_path("led2d_graphics_actual.png");
    write_frame_png(&frame, &output_path, 200).expect("render PNG");

    let expected_bytes = fs::read(expected_path).expect("read expected PNG");
    let actual_bytes = fs::read(&output_path).expect("read actual PNG");
    assert_eq!(expected_bytes, actual_bytes, "PNG bytes must match");

    let _ = fs::remove_file(&output_path);
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

fn temp_output_path(filename: &str) -> PathBuf {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be valid")
        .as_nanos();
    let process_id = std::process::id();
    let mut path = std::env::temp_dir();
    path.push(format!("{filename}-{process_id}-{unix_time}"));
    path
}
