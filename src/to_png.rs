#![cfg(feature = "host")]

use crate::led2d::Frame2d;
use png::{BitDepth, ColorType, Encoder, ScaledFloat};
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const PREVIEW_INVERSE_GAMMA: f32 = 2.2;

/// Render a `Frame2d` into a PNG file sized to the requested maximum dimension.
pub fn write_frame_png<const W: usize, const H: usize>(
    frame: &Frame2d<W, H>,
    output_path: impl AsRef<Path>,
    target_max_dimension: u32,
) -> Result<(), Box<dyn Error>> {
    let output_path = output_path.as_ref();
    let panel_width = W as u32;
    let panel_height = H as u32;
    let cell_size = select_cell_size(panel_width, panel_height, target_max_dimension);
    let led_margin = (cell_size / 8).max(1);
    write_panel_png(frame, output_path, cell_size, led_margin)?;
    println!("wrote PNG to {}", output_path.display());
    Ok(())
}

/// Render multiple `Frame2d` values into a looping APNG file.
pub fn write_frames_apng<const W: usize, const H: usize>(
    frames: &[Frame2d<W, H>],
    output_path: impl AsRef<Path>,
    target_max_dimension: u32,
    frame_delay_ms: u32,
) -> Result<(), Box<dyn Error>> {
    assert!(!frames.is_empty(), "frames must not be empty");
    assert!(frame_delay_ms > 0, "frame_delay_ms must be positive");
    let output_path = output_path.as_ref();
    let panel_width = W as u32;
    let panel_height = H as u32;
    let cell_size = select_cell_size(panel_width, panel_height, target_max_dimension);
    let led_margin = (cell_size / 8).max(1);
    let frame_count = u32::try_from(frames.len()).expect("frame count must fit in u32");
    let delay_num = u16::try_from(frame_delay_ms).expect("frame_delay_ms must fit in u16");
    let delay_den = 1000u16;

    let (width, height, first_pixels) = panel_pixels(&frames[0], cell_size, led_margin);
    let mut pixels = Vec::with_capacity(frames.len());
    pixels.push(first_pixels);
    for frame in frames.iter().skip(1) {
        let (frame_width, frame_height, frame_pixels) = panel_pixels(frame, cell_size, led_margin);
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
    println!("wrote APNG to {}", output_path.display());
    Ok(())
}

fn select_cell_size(panel_width: u32, panel_height: u32, target_max_dimension: u32) -> u32 {
    assert!(target_max_dimension > 0, "target_max_dimension must be positive");
    let mut cell_size = target_max_dimension;
    while cell_size > 1 {
        let led_margin = (cell_size / 8).max(1);
        let led_radius = (cell_size - (led_margin * 2)) / 2;
        let output_width = panel_width * cell_size + led_radius * 2;
        let output_height = panel_height * cell_size + led_radius * 2;
        let max_dimension = output_width.max(output_height);
        if max_dimension <= target_max_dimension {
            break;
        }
        cell_size -= 1;
    }
    cell_size
}

fn write_panel_png<const W: usize, const H: usize>(
    frame: &Frame2d<W, H>,
    output_path: &Path,
    cell_size: u32,
    led_margin: u32,
) -> Result<(), Box<dyn Error>> {
    let (width, height, pixels) = panel_pixels(frame, cell_size, led_margin);
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

fn panel_pixels<const W: usize, const H: usize>(
    frame: &Frame2d<W, H>,
    cell_size: u32,
    led_margin: u32,
) -> (u32, u32, Vec<u8>) {
    assert!(cell_size > 0, "cell_size must be positive");
    assert!(
        led_margin < cell_size / 2,
        "led_margin must fit inside cell"
    );
    let led_radius = (cell_size - (led_margin * 2)) / 2;
    assert!(led_radius > 0, "led_radius must be positive");
    let fade_width = led_radius / 3;
    assert!(fade_width > 0, "fade_width must be positive");

    let border = led_radius;
    assert!(border > 0, "border must be positive");
    let width = (W as u32) * cell_size + border * 2;
    let height = (H as u32) * cell_size + border * 2;
    let mut bytes = vec![0u8; (width * height * 3 * 2) as usize];
    let center = (cell_size - 1) as i32 / 2;
    let led_radius_f = led_radius as f32;
    let inner_radius_f = (led_radius - fade_width) as f32;
    let radius_sq = (led_radius as i32) * (led_radius as i32);

    for row_index in 0..H {
        for column_index in 0..W {
            let pixel = frame.0[row_index][column_index];
            let cell_origin_x = (column_index as u32) * cell_size;
            let cell_origin_y = (row_index as u32) * cell_size;

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
                        let y = border + cell_origin_y + local_y;
                        let pixel_index = ((y * width + x) * 3 * 2) as usize;
                        let red = linear_to_u16(inverse_gamma_to_linear(pixel.r) * intensity);
                        let green = linear_to_u16(inverse_gamma_to_linear(pixel.g) * intensity);
                        let blue = linear_to_u16(inverse_gamma_to_linear(pixel.b) * intensity);
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
    }

    (width, height, bytes)
}

fn inverse_gamma_to_linear(channel: u8) -> f32 {
    let normalized = (channel as f32) / 255.0;
    normalized.powf(PREVIEW_INVERSE_GAMMA)
}

fn linear_to_u16(value: f32) -> u16 {
    let clamped = value.clamp(0.0, 1.0);
    (clamped * 65535.0).round() as u16
}
