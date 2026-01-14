#![cfg(feature = "host")]

use device_kit::led2d::{Frame2d, Led2dFont, render_text_to_frame};
use device_kit::to_png::{write_frame_png, write_frames_apng};
use embassy_time::Duration;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use smart_leds::RGB8;
use smart_leds::colors;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

include!("../video_frames_data.rs");

type Frame = Frame2d<12, 8>;
type Led12x4Frame = Frame2d<12, 4>;
type Led8x12Frame = Frame2d<8, 12>;

#[test]
fn led2d_graphics_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_png_matches_expected("led2d_graphics.png", 200, build_frame)
}

#[test]
fn led2d1_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_png_matches_expected("led2d1.png", 200, build_led2d1_frame)
}

#[test]
fn led2d2_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_apng_matches_expected("led2d2.png", 200, 400, build_led2d2_frames)
}

#[test]
fn santa_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_santa_apng_matches_expected()
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

fn build_led2d1_frame() -> Led12x4Frame {
    let mut frame: Led12x4Frame = Frame2d::new();
    let font_variant = Led2dFont::Font3x4Trim;
    let font = font_variant.to_font();
    let spacing_reduction = font_variant.spacing_reduction();
    let colors = [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW];

    render_text_to_frame(&mut frame, &font, "Rust", &colors, spacing_reduction)
        .expect("text render must succeed");

    frame
}

fn build_led2d2_frames() -> [Led8x12Frame; 2] {
    [build_led2d2_frame_0(), build_led2d2_frame_1()]
}

fn build_led2d2_frame_0() -> Led8x12Frame {
    let mut frame: Led8x12Frame = Frame2d::new();
    let font_variant = Led2dFont::Font4x6Trim;
    let font = font_variant.to_font();
    let spacing_reduction = font_variant.spacing_reduction();

    render_text_to_frame(&mut frame, &font, "Go", &[], spacing_reduction)
        .expect("text render must succeed");

    frame
}

fn build_led2d2_frame_1() -> Led8x12Frame {
    let mut frame: Led8x12Frame = Frame2d::new();
    let font_variant = Led2dFont::Font4x6Trim;
    let font = font_variant.to_font();
    let spacing_reduction = font_variant.spacing_reduction();
    let colors = [colors::HOT_PINK, colors::LIME];

    render_text_to_frame(&mut frame, &font, "\nGo", &colors, spacing_reduction)
        .expect("text render must succeed");

    frame
}

fn assert_santa_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    let frame_delay_ms =
        u32::try_from(SANTA_FRAME_DURATION.as_millis()).expect("santa frame delay must fit in u32"); // cmk use ?
    for (_, duration) in SANTA_FRAMES.iter() {
        assert!(
            duration.as_millis() == SANTA_FRAME_DURATION.as_millis(),
            "santa frames must share a constant duration"
        );
    }
    let santa_frames: Vec<Frame2d<12, 8>> = SANTA_FRAMES
        .iter()
        .map(|(pixels, _)| Frame2d(*pixels))
        .collect();
    assert_apng_matches_expected_for_frames("santa.png", 200, frame_delay_ms, &santa_frames)
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

fn docs_assets_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from("docs");
    path.push("assets");
    path.push(filename);
    path
}

fn assert_png_matches_expected<F, const W: usize, const H: usize>(
    filename: &str,
    max_dimension: u32,
    build_frame: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Frame2d<W, H>,
{
    let frame = build_frame();
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_frame_png(&frame, &expected_path, max_dimension)?;
        println!("updated PNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected PNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_frame_png(&frame, &output_path, max_dimension)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "PNG bytes must match");

    let _ = fs::remove_file(&output_path);
    Ok(())
}

fn assert_apng_matches_expected<F, const W: usize, const H: usize, const N: usize>(
    filename: &str,
    max_dimension: u32,
    frame_delay_ms: u32,
    build_frames: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> [Frame2d<W, H>; N],
{
    let frames = build_frames();
    assert_apng_matches_expected_for_frames(filename, max_dimension, frame_delay_ms, &frames)
}

fn assert_apng_matches_expected_for_frames<const W: usize, const H: usize>(
    filename: &str,
    max_dimension: u32,
    frame_delay_ms: u32,
    frames: &[Frame2d<W, H>],
) -> Result<(), Box<dyn Error>> {
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_frames_apng(frames, &expected_path, max_dimension, frame_delay_ms)?;
        println!("updated APNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected APNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_frames_apng(frames, &output_path, max_dimension, frame_delay_ms)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "APNG bytes must match");

    let _ = fs::remove_file(&output_path);
    Ok(())
}
