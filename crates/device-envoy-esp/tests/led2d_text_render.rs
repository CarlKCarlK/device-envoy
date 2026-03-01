#![allow(missing_docs)]

use device_envoy_esp::led2d::{render_text_to_frame, Frame2d, Led2dFont};
use png::{BitDepth, ColorType, Decoder, Encoder};
use smart_leds::{colors, RGB8};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

const REFERENCE_DIR: &str = "tests/data/text_render";

#[test]
fn font3x4_on_12x4_matches_reference() {
    run_render_test::<12, 4>(
        "font3x4_12x4",
        Led2dFont::Font3x4Trim,
        "RUST",
        &four_colors(),
    );
}

#[test]
fn font4x6_on_12x4_clips_bottom_matches_reference() {
    run_render_test::<12, 4>(
        "font4x6_12x4",
        Led2dFont::Font4x6,
        "RUST\ntwo",
        &four_colors(),
    );
}

#[test]
fn font6x10_on_24x16_clips_and_colors_cycle() {
    run_render_test::<24, 16>(
        "font6x10_24x16",
        Led2dFont::Font6x10,
        "Hello Rust\nWrap me",
        &[colors::CYAN, colors::MAGENTA],
    );
}

#[test]
fn font3x4_on_12x4_no_colors_defaults_to_white() {
    run_render_test::<12, 4>("font3x4_12x4_white", Led2dFont::Font3x4Trim, "RUST", &[]);
}

fn run_render_test<const W: usize, const H: usize>(
    name: &str,
    font: Led2dFont,
    text: &str,
    colors: &[RGB8],
) {
    let mut frame: Frame2d<W, H> = Frame2d::new();
    render_text_to_frame(&mut frame, &font.to_font(), text, colors, (0, 0))
        .expect("render must succeed");

    if let Some(output_dir) = generation_dir() {
        let output_path = output_dir.join(format!("{name}.png"));
        write_png(&frame, &output_path);
        return;
    }

    let reference_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(REFERENCE_DIR)
        .join(format!("{name}.png"));
    let reference = read_png::<W, H>(&reference_path);
    assert_eq!(
        frame_pixels(&frame),
        reference,
        "rendered output for {name} did not match reference at {}",
        reference_path.display()
    );
}

fn generation_dir() -> Option<PathBuf> {
    let env_value = std::env::var("DEVICE_ENVOY_ESP_GENERATE_TEXT_PNGS").ok()?;
    let output_dir = if env_value.is_empty() {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("device-envoy-esp-text-pngs");
        temp_dir
    } else {
        PathBuf::from(env_value)
    };
    std::fs::create_dir_all(&output_dir).expect("failed to create PNG output directory");
    Some(output_dir)
}

fn write_png<const W: usize, const H: usize>(frame: &Frame2d<W, H>, path: &Path) {
    let file = File::create(path).expect("failed to create PNG file");
    let mut encoder = Encoder::new(BufWriter::new(file), W as u32, H as u32);
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header().expect("failed to write PNG header");
    writer
        .write_image_data(&frame_pixels(frame))
        .expect("failed to write PNG data");
}

fn read_png<const W: usize, const H: usize>(path: &Path) -> Vec<u8> {
    let file =
        File::open(path).unwrap_or_else(|_| panic!("missing reference PNG at {}", path.display()));
    let decoder = Decoder::new(file);
    let mut reader = decoder.read_info().expect("failed to read PNG");
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .expect("failed to decode PNG");
    assert_eq!(info.width, W as u32, "reference PNG width mismatch");
    assert_eq!(info.height, H as u32, "reference PNG height mismatch");
    buffer[..info.buffer_size()].to_vec()
}

fn frame_pixels<const W: usize, const H: usize>(frame: &Frame2d<W, H>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(H * W * 3);
    for row_index in 0..H {
        for col_index in 0..W {
            let pixel = frame.0[row_index][col_index];
            bytes.push(pixel.r);
            bytes.push(pixel.g);
            bytes.push(pixel.b);
        }
    }
    bytes
}

fn four_colors() -> [RGB8; 4] {
    [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW]
}
