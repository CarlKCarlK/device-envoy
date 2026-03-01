//! Generate Rust code for embedded video frames from PNG files or video files.

use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

pub fn generate_frames() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(video_path) = std::env::var("SANTA_VIDEO_PATH") {
        return generate_video_frames(&video_path, "santa");
    }
    if let Ok(frames_dir) = std::env::var("SANTA_FRAMES_DIR") {
        return generate_frames_from_directory(Path::new(&frames_dir), "santa", 65);
    }
    Err(
        "Set either SANTA_VIDEO_PATH (MP4 input) or SANTA_FRAMES_DIR (pre-extracted frame PNGs)."
            .into(),
    )
}

pub fn generate_cat_frames() -> Result<(), Box<dyn std::error::Error>> {
    let video_path = std::env::var("CAT_VIDEO_PATH")
        .map_err(|_| "Set CAT_VIDEO_PATH to an input MP4 for cat-frames-gen.")?;
    generate_video_frames(&video_path, "cat")
}

pub fn generate_hand_frames() -> Result<(), Box<dyn std::error::Error>> {
    let video_path = std::env::var("HAND_VIDEO_PATH")
        .map_err(|_| "Set HAND_VIDEO_PATH to an input MP4 for hand-frames-gen.")?;
    generate_video_frames(&video_path, "hand")
}

pub fn generate_clock_frames() -> Result<(), Box<dyn std::error::Error>> {
    let video_path = std::env::var("CLOCK_VIDEO_PATH")
        .map_err(|_| "Set CLOCK_VIDEO_PATH to an input MP4 for clock-frames-gen.")?;
    generate_video_frames(&video_path, "clock")
}

fn generate_video_frames(video_path: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(video_path).exists() {
        return Err(format!("Input video file does not exist: {video_path}").into());
    }

    // Create temporary directory for extracted frames
    let temp_dir = std::env::temp_dir().join(format!("{}_frames_12x8", name));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    eprintln!("Extracting frames from video: {}", video_path);
    eprintln!("Output directory: {}", temp_dir.display());

    // Use ffmpeg to extract frames at 10 FPS, rotate CW, and scale to 12x8
    let status = Command::new("ffmpeg")
        .args([
            "-i",
            &video_path,
            "-vf",
            "fps=10,transpose=1,scale=12:8:flags=lanczos",
            "-q:v",
            "2",
            &format!("{}/frame_%06d.png", temp_dir.display()),
        ])
        .status()?;

    if !status.success() {
        return Err("ffmpeg failed to extract frames".into());
    }

    // Count extracted frames
    let frame_count = fs::read_dir(&temp_dir)?.count();
    eprintln!("Extracted {} frames", frame_count);

    generate_frames_from_directory(&temp_dir, name, frame_count)
}

fn generate_frames_from_directory(
    frames_dir: &Path,
    name: &str,
    frame_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if !frames_dir.exists() {
        return Err(format!(
            "Input frames directory does not exist: {}",
            frames_dir.display()
        )
        .into());
    }
    const FRAME_DURATION_MILLIS: u64 = 100;
    let upper_name = name.to_uppercase();
    let frame_duration_name = format!("{}_FRAME_DURATION", upper_name);
    let frame_count_name = format!("{}_FRAME_COUNT", upper_name);
    let frames_name = format!("{}_FRAMES", upper_name);

    println!("// Video frames generated from PNG files ({} video)", name);
    println!("// Auto-generated - do not edit manually");
    println!();

    println!("#[allow(dead_code)]");
    println!(
        "// Frame duration for 10 FPS (100ms per frame)\nconst {}: Duration = Duration::from_millis({});",
        frame_duration_name, FRAME_DURATION_MILLIS
    );
    println!();

    println!("#[allow(dead_code)]");
    println!("const {}: usize = {};", frame_count_name, frame_count);
    println!();
    println!(
        "#[allow(dead_code)]\nconst {}: [([[RGB8; 12]; 8], Duration); {}] = [",
        frames_name, frame_count_name
    );

    for frame_num in 1..=frame_count {
        let filename = frames_dir.join(format!("frame_{:06}.png", frame_num));

        if !filename.exists() {
            eprintln!("Warning: {} not found, skipping", filename.display());
            continue;
        }

        let decoder = png::Decoder::new(File::open(&filename)?);
        let mut reader = decoder.read_info()?;
        let info = reader.info();

        if info.width != 12 || info.height != 8 {
            eprintln!(
                "Warning: {} has wrong dimensions ({}x{}), expected 12x8",
                filename.display(),
                info.width,
                info.height
            );
            continue;
        }

        let mut buf = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut buf)?;

        println!("    // Frame {}", frame_num);
        println!("    (");
        println!("        [");

        // Flip rows vertically (row 0 in PNG becomes row 7 in output)
        for row in (0..8).rev() {
            print!("        [");
            for col in 0..12 {
                let pixel_index = (row * 12 + col)
                    * match reader.info().color_type {
                        png::ColorType::Rgb => 3,
                        png::ColorType::Rgba => 4,
                        _ => panic!("Unsupported color type: {:?}", reader.info().color_type),
                    };

                let r = buf[pixel_index];
                let g = buf[pixel_index + 1];
                let b = buf[pixel_index + 2];

                print!("RGB8::new({}, {}, {})", r, g, b);
                if col < 11 {
                    print!(", ");
                }
            }
            println!("],");
        }

        println!("        ],");
        println!("        {},", frame_duration_name);
        print!("    )");
        if frame_num < frame_count {
            println!(",");
        } else {
            println!();
        }
    }

    println!("];");

    Ok(())
}
