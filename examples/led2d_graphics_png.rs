#![cfg(feature = "host")]

// check-all: skip (host-only PNG generation)

use device_kit::led2d::Frame2d;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use png::{BitDepth, ColorType, Encoder};
use smart_leds::colors;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output_path = output_path_from_args();
    let frame = build_frame();
    write_png(&frame, &output_path)?;
    println!("wrote PNG to {}", output_path.display());
    Ok(())
}

fn output_path_from_args() -> PathBuf {
    let mut args = std::env::args().skip(1);
    if let Some(path) = args.next() {
        return PathBuf::from(path);
    }
    PathBuf::from("led2d_graphics.png")
}

fn build_frame() -> Frame2d<12, 8> {
    const WIDTH: usize = 12;
    const HEIGHT: usize = 8;
    const DIAMETER: u32 = 6;

    let mut frame = Frame2d::<WIDTH, HEIGHT>::new();

    Rectangle::new(Frame2d::<WIDTH, HEIGHT>::TOP_LEFT, Frame2d::<WIDTH, HEIGHT>::SIZE)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
        .draw(&mut frame)
        .expect("rectangle draw must succeed");

    frame[0][0] = colors::CYAN;

    let circle_top_left = centered_top_left(WIDTH, HEIGHT, DIAMETER as usize);
    Circle::new(circle_top_left, DIAMETER)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::GREEN, 1))
        .draw(&mut frame)
        .expect("circle draw must succeed");

    frame
}

fn centered_top_left(width: usize, height: usize, size: usize) -> Point {
    assert!(size <= width, "size must fit within width");
    assert!(size <= height, "size must fit within height");
    Point::new(((width - size) / 2) as i32, ((height - size) / 2) as i32)
}

fn write_png<const W: usize, const H: usize>(
    frame: &Frame2d<W, H>,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let file = File::create(output_path)?;
    let mut encoder = Encoder::new(BufWriter::new(file), W as u32, H as u32);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&frame_pixels(frame))?;
    Ok(())
}

fn frame_pixels<const W: usize, const H: usize>(frame: &Frame2d<W, H>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(H * W * 3);
    for row_index in 0..H {
        for column_index in 0..W {
            let pixel = frame.0[row_index][column_index];
            bytes.push(pixel.r);
            bytes.push(pixel.g);
            bytes.push(pixel.b);
        }
    }
    bytes
}
