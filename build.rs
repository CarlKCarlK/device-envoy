//! Build script for device-envoy.

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");

    // 1) Generate video frames data if building the video example
    // Check if we're building the video example by looking at CARGO_BIN_NAME or features
    let cargo_target_tmpdir = env::var("CARGO_TARGET_TMPDIR").ok();
    let should_generate = cargo_target_tmpdir
        .as_deref()
        .map(|s| s.contains("examples/video-"))
        .unwrap_or(false);

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source_frames_dir = manifest_dir.join("examples/data/frame-data");
    let frames_dir = manifest_dir.join("target/frame-data");
    fs::create_dir_all(&frames_dir).expect("Failed to create target/frame-data directory");
    let frames_path = frames_dir.join("video_frames_data.rs");

    let placeholder = r#"// Video frames generated from PNG files (santa video)
// Auto-generated - do not edit manually
// Placeholder - run `just video-frames` with SANTA_FRAMES_DIR set to generate real frames

#[allow(dead_code)]
// Frame duration for 10 FPS (100ms per frame)
const SANTA_FRAME_DURATION: Duration = Duration::from_millis(100);

#[allow(dead_code)]
const SANTA_FRAME_COUNT: usize = 1;

#[allow(dead_code)]
const SANTA_FRAMES: [([[RGB8; 12]; 8], Duration); SANTA_FRAME_COUNT] = [
    (
        [
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
            [RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0), RGB8::new(0, 0, 0)],
        ],
        SANTA_FRAME_DURATION,
    )
];
"#;

    if should_generate {
        eprintln!("Generating video frames data...");
        let output = Command::new("cargo")
            .args(["xtask", "video-frames-gen"])
            .output()
            .expect("Failed to run cargo xtask video-frames-gen");

        if output.status.success() {
            let frames_data = String::from_utf8_lossy(&output.stdout);
            fs::write(&frames_path, frames_data.as_bytes())
                .expect("Failed to write video_frames_data.rs");
            eprintln!("Video frames data generated successfully");
        } else {
            eprintln!(
                "Warning: Failed to generate video frames, creating placeholder: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            // Create a minimal placeholder file so the build doesn't fail
            fs::write(&frames_path, placeholder)
                .expect("Failed to write placeholder video_frames_data.rs");
        }
    } else {
        let source_video_frames = source_frames_dir.join("video_frames_data.rs");
        if source_video_frames.exists() {
            fs::copy(&source_video_frames, &frames_path)
                .expect("Failed to copy video_frames_data.rs to target/frame-data");
        } else if !frames_path.exists() {
            fs::write(&frames_path, placeholder)
                .expect("Failed to write placeholder video_frames_data.rs");
        }
    }

    for name in [
        "cat_frames_data.rs",
        "hand_frames_data.rs",
        "clock_frames_data.rs",
    ] {
        let source_path = source_frames_dir.join(name);
        let dest_path = frames_dir.join(name);
        if source_path.exists() && !dest_path.exists() {
            fs::copy(&source_path, &dest_path)
                .unwrap_or_else(|_| panic!("Failed to copy {name} to target/frame-data"));
        }
    }

    // Set up rerun trigger for video source directory
    if let Ok(home) = env::var("HOME") {
        let frames_dir = PathBuf::from(home).join("programs/ffmpeg-test/frames12x8_landscape");
        if frames_dir.exists() {
            println!("cargo:rerun-if-changed={}", frames_dir.display());
        }
    }

    // 2) Handle memory.x based on target
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    if target.starts_with("thumbv8m") {
        // Pico 2 ARM: copy our custom memory-pico2.x to OUT_DIR as memory.x
        let memory_x = fs::read_to_string("memory-pico2.x").expect("Failed to read memory-pico2.x");
        let dest = out_dir.join("memory.x");
        fs::write(&dest, memory_x).expect("Failed to write memory.x");
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rerun-if-changed=memory-pico2.x");
    } else if target.starts_with("riscv32imac") {
        // Pico 2 RISC-V: copy our custom memory-pico2-riscv.x to OUT_DIR as memory.x
        let memory_x = fs::read_to_string("memory-pico2-riscv.x")
            .expect("Failed to read memory-pico2-riscv.x");
        let dest = out_dir.join("memory.x");
        fs::write(&dest, memory_x).expect("Failed to write memory.x");
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rerun-if-changed=memory-pico2-riscv.x");
    } else if target.starts_with("thumbv6m") {
        // Pico 1W: copy our custom memory-pico1w.x to OUT_DIR as memory.x
        let memory_x =
            fs::read_to_string("memory-pico1w.x").expect("Failed to read memory-pico1w.x");
        let dest = out_dir.join("memory.x");
        fs::write(&dest, memory_x).expect("Failed to write memory.x");
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rerun-if-changed=memory-pico1w.x");
    }
}
