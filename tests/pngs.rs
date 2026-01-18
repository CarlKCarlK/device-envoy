#![cfg(feature = "host")]

use device_kit::led_strip::{Frame1d, Rgb888};
use device_kit::led2d::{Frame2d, Led2dFont, render_text_to_frame};
use device_kit::to_png::{write_frame_png_with_gamma, write_frames_apng_with_gamma};
use embassy_time::Duration;
use embedded_graphics::{
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
};
use png::{BitDepth, ColorType, Encoder, ScaledFloat};
use smart_leds::RGB8;
use smart_leds::colors;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

include!("../video_frames_data.rs");

type Frame = Frame2d<12, 8>;
type Led12x4Frame = Frame2d<12, 4>;
type Led8x12Frame = Frame2d<8, 12>;
type LedStripGpio0Frame = Frame1d<8>;
type LedStripSimpleFrame = Frame1d<48>;
type LedStripAnimatedFrame = Frame1d<96>;

#[test]
fn led2d_graphics_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_png_matches_expected("led2d_graphics.png", 200, build_frame)
}

#[test]
fn led2d1_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_png_matches_expected("led2d1.png", 200, build_led2d1_frame)
}

#[test]
fn led2d1_linear_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_png_matches_expected_with_gamma("led2d1_linear.png", 200, 1.0, build_led2d1_frame)
}

#[test]
fn led_strip_simple_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_strip_png_matches_expected("led_strip_simple.png", 600, build_led_strip_simple_frame)
}

#[test]
fn led_strip_gpio0_png_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_strip_png_matches_expected("led_strip_gpio0.png", 150, build_led_strip_gpio0_frame)
}

#[test]
fn led_strip_animated_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_strip_apng_matches_expected(
        "led_strip_animated.png",
        800,
        300,
        build_led_strip_animated_frames,
    )
}

#[test]
fn led2d2_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_apng_matches_expected("led2d2.png", 200, 1000, build_led2d2_frames)
}

#[test]
fn santa_linear_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_santa_apng_matches_expected("santa_linear.png", None, 1.0)
}

#[test]
fn santa_gamma_apng_matches_expected() -> Result<(), Box<dyn Error>> {
    assert_santa_apng_matches_expected("santa.png", Some(2.2), 2.2)
}

fn build_frame() -> Frame {
    let mut frame: Frame = Frame::new();

    Rectangle::new(Frame::TOP_LEFT, Frame::SIZE)
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
        .draw(&mut frame)
        .expect("rectangle draw must succeed");

    frame[(0, 0)] = colors::CYAN;

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
    let colors = [colors::CYAN, colors::RED, colors::YELLOW];

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

fn build_led_strip_simple_frame() -> LedStripSimpleFrame {
    let mut led_strip_simple_frame = LedStripSimpleFrame::new();
    for pixel_index in 0..LedStripSimpleFrame::LEN {
        led_strip_simple_frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
    }
    led_strip_simple_frame
}

fn build_led_strip_gpio0_frame() -> LedStripGpio0Frame {
    Frame1d::filled(colors::WHITE)
}

fn build_led_strip_animated_frames() -> [LedStripAnimatedFrame; 3] {
    [
        Frame1d::filled(colors::RED),
        Frame1d::filled(colors::GREEN),
        Frame1d::filled(colors::BLUE),
    ]
}

fn assert_santa_apng_matches_expected(
    filename: &str,
    gamma: Option<f32>,
    preview_inverse_gamma: f32,
) -> Result<(), Box<dyn Error>> {
    let frame_delay_ms =
        u32::try_from(SANTA_FRAME_DURATION.as_millis()).expect("santa frame delay must fit in u32"); // cmk use ?
    for (_, duration) in SANTA_FRAMES.iter() {
        assert!(
            duration.as_millis() == SANTA_FRAME_DURATION.as_millis(),
            "santa frames must share a constant duration"
        );
    }
    let santa_frames: Vec<Frame> = SANTA_FRAMES
        .iter()
        .map(|(pixels, _)| Frame2d(*pixels))
        .collect();
    let santa_frames = if let Some(gamma) = gamma {
        apply_gamma_to_frames(&santa_frames, gamma)
    } else {
        santa_frames
    };
    assert_apng_matches_expected_for_frames_with_gamma(
        filename,
        200,
        frame_delay_ms,
        preview_inverse_gamma,
        &santa_frames,
    )
}

fn apply_gamma_to_frames(frames: &[Frame], gamma: f32) -> Vec<Frame> {
    assert!(gamma > 0.0, "gamma must be positive");
    frames
        .iter()
        .map(|frame| apply_gamma_to_frame(frame, gamma))
        .collect()
}

fn apply_gamma_to_frame(frame: &Frame, gamma: f32) -> Frame {
    let mut corrected_frame = Frame::new();
    for y_index in 0..Frame::HEIGHT {
        for x_index in 0..Frame::WIDTH {
            let pixel = frame[(x_index, y_index)];
            corrected_frame[(x_index, y_index)] = RGB8::new(
                apply_gamma_to_channel(pixel.r, gamma),
                apply_gamma_to_channel(pixel.g, gamma),
                apply_gamma_to_channel(pixel.b, gamma),
            );
        }
    }
    corrected_frame
}

fn apply_gamma_to_channel(channel: u8, gamma: f32) -> u8 {
    let normalized = (channel as f32) / 255.0;
    let corrected = normalized.powf(gamma);
    assert!(
        (0.0..=1.0).contains(&corrected),
        "gamma corrected value must be in range"
    );
    let scaled = (corrected * 255.0).round();
    assert!(scaled <= 255.0, "gamma corrected value must fit in u8");
    scaled as u8
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
    assert_png_matches_expected_with_gamma(filename, max_dimension, 2.2, build_frame)
}

fn assert_strip_png_matches_expected<F, const N: usize>(
    filename: &str,
    target_width: u32,
    build_frame: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Frame1d<N>,
{
    assert_strip_png_matches_expected_with_gamma(filename, target_width, 2.2, build_frame)
}

fn assert_png_matches_expected_with_gamma<F, const W: usize, const H: usize>(
    filename: &str,
    max_dimension: u32,
    preview_inverse_gamma: f32,
    build_frame: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Frame2d<W, H>,
{
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let frame = build_frame();
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_frame_png_with_gamma(&frame, &expected_path, max_dimension, preview_inverse_gamma)?;
        println!("updated PNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected PNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_frame_png_with_gamma(&frame, &output_path, max_dimension, preview_inverse_gamma)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "PNG bytes must match");

    let _ = fs::remove_file(&output_path);
    Ok(())
}

fn assert_strip_png_matches_expected_with_gamma<F, const N: usize>(
    filename: &str,
    target_width: u32,
    preview_inverse_gamma: f32,
    build_frame: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Frame1d<N>,
{
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let frame = build_frame();
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_strip_png_with_gamma(&frame, &expected_path, target_width, preview_inverse_gamma)?;
        println!("updated PNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected PNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_strip_png_with_gamma(&frame, &output_path, target_width, preview_inverse_gamma)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "PNG bytes must match");

    let _ = fs::remove_file(&output_path);
    Ok(())
}

fn assert_strip_apng_matches_expected<F, const N: usize, const M: usize>(
    filename: &str,
    target_width: u32,
    frame_delay_ms: u32,
    build_frames: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> [Frame1d<N>; M],
{
    assert_strip_apng_matches_expected_with_gamma(
        filename,
        target_width,
        frame_delay_ms,
        2.2,
        build_frames,
    )
}

fn assert_strip_apng_matches_expected_with_gamma<F, const N: usize, const M: usize>(
    filename: &str,
    target_width: u32,
    frame_delay_ms: u32,
    preview_inverse_gamma: f32,
    build_frames: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> [Frame1d<N>; M],
{
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    assert!(frame_delay_ms > 0, "frame_delay_ms must be positive");
    let frames = build_frames();
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_strip_apng_with_gamma(
            &frames,
            &expected_path,
            target_width,
            frame_delay_ms,
            preview_inverse_gamma,
        )?;
        println!("updated APNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected APNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_strip_apng_with_gamma(
        &frames,
        &output_path,
        target_width,
        frame_delay_ms,
        preview_inverse_gamma,
    )?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "APNG bytes must match");

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
    assert_apng_matches_expected_for_frames_with_gamma(
        filename,
        max_dimension,
        frame_delay_ms,
        2.2,
        frames,
    )
}

fn assert_apng_matches_expected_for_frames_with_gamma<const W: usize, const H: usize>(
    filename: &str,
    max_dimension: u32,
    frame_delay_ms: u32,
    preview_inverse_gamma: f32,
    frames: &[Frame2d<W, H>],
) -> Result<(), Box<dyn Error>> {
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let expected_path = docs_assets_path(filename);
    if std::env::var_os("DEVICE_KIT_UPDATE_PNGS").is_some() {
        write_frames_apng_with_gamma(
            frames,
            &expected_path,
            max_dimension,
            frame_delay_ms,
            preview_inverse_gamma,
        )?;
        println!("updated APNG at {}", expected_path.display());
        return Ok(());
    }
    if !expected_path.exists() {
        return Err(format!("expected APNG is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    write_frames_apng_with_gamma(
        frames,
        &output_path,
        max_dimension,
        frame_delay_ms,
        preview_inverse_gamma,
    )?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_bytes = fs::read(&output_path)?;
    assert_eq!(expected_bytes, actual_bytes, "APNG bytes must match");

    let _ = fs::remove_file(&output_path);
    Ok(())
}

fn write_strip_png_with_gamma<const N: usize>(
    frame: &Frame1d<N>,
    output_path: &PathBuf,
    target_width: u32,
    preview_inverse_gamma: f32,
) -> Result<(), Box<dyn Error>> {
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let cell_size = select_strip_cell_size(N as u32, target_width);
    let led_margin = (cell_size / 8).max(1);
    let (width, height, pixels) = strip_pixels(frame, cell_size, led_margin, preview_inverse_gamma);

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let file = File::create(output_path)?;
    let mut encoder = Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_source_gamma(ScaledFloat::new(1.0));
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&pixels)?;
    Ok(())
}

fn write_strip_apng_with_gamma<const N: usize>(
    frames: &[Frame1d<N>],
    output_path: &PathBuf,
    target_width: u32,
    frame_delay_ms: u32,
    preview_inverse_gamma: f32,
) -> Result<(), Box<dyn Error>> {
    assert!(!frames.is_empty(), "frames must not be empty");
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let cell_size = select_strip_cell_size(N as u32, target_width);
    let led_margin = (cell_size / 8).max(1);
    let frame_count = u32::try_from(frames.len()).expect("frame count must fit in u32");
    let delay_num = u16::try_from(frame_delay_ms).expect("frame_delay_ms must fit in u16");
    let delay_den = 1000u16;

    let (width, height, first_pixels) =
        strip_pixels(&frames[0], cell_size, led_margin, preview_inverse_gamma);
    let mut pixels = Vec::with_capacity(frames.len());
    pixels.push(first_pixels);
    for frame in frames.iter().skip(1) {
        let (frame_width, frame_height, frame_pixels) =
            strip_pixels(frame, cell_size, led_margin, preview_inverse_gamma);
        assert!(frame_width == width, "frame width must match");
        assert!(frame_height == height, "frame height must match");
        pixels.push(frame_pixels);
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let file = File::create(output_path)?;
    let mut encoder = Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Sixteen);
    encoder.set_source_gamma(ScaledFloat::new(1.0));
    encoder.set_animated(frame_count, 0)?;
    let mut writer = encoder.write_header()?;
    for frame_pixels in pixels {
        writer.set_frame_delay(delay_num, delay_den)?;
        writer.write_image_data(&frame_pixels)?;
    }
    writer.finish()?;
    Ok(())
}

fn select_strip_cell_size(strip_len: u32, target_width: u32) -> u32 {
    assert!(strip_len > 0, "strip_len must be positive");
    assert!(target_width > 0, "target_width must be positive");
    let mut cell_size = target_width;
    while cell_size > 1 {
        let led_margin = (cell_size / 8).max(1);
        let led_radius = (cell_size - (led_margin * 2)) / 2;
        let output_width = strip_len * cell_size + led_radius * 2;
        if output_width <= target_width {
            break;
        }
        cell_size -= 1;
    }
    cell_size
}

fn strip_pixels<const N: usize>(
    frame: &Frame1d<N>,
    cell_size: u32,
    led_margin: u32,
    preview_inverse_gamma: f32,
) -> (u32, u32, Vec<u8>) {
    assert!(cell_size > 0, "cell_size must be positive");
    assert!(
        led_margin < cell_size / 2,
        "led_margin must fit inside cell"
    );
    assert!(
        preview_inverse_gamma > 0.0,
        "preview_inverse_gamma must be positive"
    );
    let led_radius = (cell_size - (led_margin * 2)) / 2;
    assert!(led_radius > 0, "led_radius must be positive");
    let fade_width = led_radius / 3;
    assert!(fade_width > 0, "fade_width must be positive");

    let border = led_radius;
    assert!(border > 0, "border must be positive");
    let width = (N as u32) * cell_size + border * 2;
    let height = cell_size + border * 2;
    let mut bytes = vec![0u8; (width * height * 3 * 2) as usize];
    let center = (cell_size - 1) as i32 / 2;
    let led_radius_f = led_radius as f32;
    let inner_radius_f = (led_radius - fade_width) as f32;
    let radius_sq = (led_radius as i32) * (led_radius as i32);

    for led_index in 0..N {
        let pixel = frame[led_index];
        let cell_origin_x = (led_index as u32) * cell_size;

        for local_y in 0..cell_size {
            let delta_y = local_y as i32 - center;
            for local_x in 0..cell_size {
                let delta_x = local_x as i32 - center;
                let distance_sq = delta_x * delta_x + delta_y * delta_y;
                if distance_sq <= radius_sq {
                    let distance = (distance_sq as f32).sqrt();
                    let intensity = if distance <= inner_radius_f {
                        1.0
                    } else {
                        let fade_span = led_radius_f - inner_radius_f;
                        (1.0 - (distance - inner_radius_f) / fade_span).max(0.0)
                    };
                    let x = border + cell_origin_x + local_x;
                    let y = border + local_y;
                    let pixel_index = ((y * width + x) * 3 * 2) as usize;
                    let red = linear_to_u16(
                        inverse_gamma_to_linear(pixel.r, preview_inverse_gamma) * intensity,
                    );
                    let green = linear_to_u16(
                        inverse_gamma_to_linear(pixel.g, preview_inverse_gamma) * intensity,
                    );
                    let blue = linear_to_u16(
                        inverse_gamma_to_linear(pixel.b, preview_inverse_gamma) * intensity,
                    );
                    bytes[pixel_index] = (red >> 8) as u8;
                    bytes[pixel_index + 1] = red as u8;
                    bytes[pixel_index + 2] = (green >> 8) as u8;
                    bytes[pixel_index + 3] = green as u8;
                    bytes[pixel_index + 4] = (blue >> 8) as u8;
                    bytes[pixel_index + 5] = blue as u8;
                }
            }
        }
    }

    (width, height, bytes)
}

fn inverse_gamma_to_linear(channel: u8, preview_inverse_gamma: f32) -> f32 {
    let normalized = (channel as f32) / 255.0;
    normalized.powf(preview_inverse_gamma)
}

fn linear_to_u16(value: f32) -> u16 {
    let clamped = value.clamp(0.0, 1.0);
    (clamped * 65535.0).round() as u16
}
