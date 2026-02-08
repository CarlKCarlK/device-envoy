#![allow(missing_docs)]
//! LED panel video player - plays looping 12x8 videos with button-controlled mode switching.
//!
//! This example cycles through multiple display modes using a button:
//! 1. **Test Pattern**: RGBY corners (Red top-left, Green top-right, Blue bottom-left, Yellow bottom-right)
//! 2. **Santa**: 65-frame video at 10 FPS
//! 3. **Cat**: Video converted from user's camera roll (when generated)
//!
//! Press the button at any time to advance to the next mode.
//!
//! # Hardware Setup
//!
//! - Two 12x4 LED panels creating a 12x8 display (rotated 90° from clock_led8x12)
//! - LED data on GPIO4
//! - Button on GPIO13 (wired to ground)
//! - Same physical wiring as clock_led8x12.rs but logically rotated
//!
//! # Converting Your Video to LED Frames
//!
//! ## Santa Video (Pre-configured)
//!
//! The santa frames are embedded from `examples/data/frame-data/video_frames_data.rs`, which is **auto-generated**
//! during the build process from PNG files in `SANTA_FRAMES_DIR`,
//! or directly from a source MP4 via `SANTA_VIDEO_PATH`.
//!
//! The build system automatically:
//!
//! 1. Detects when building the `video` example
//! 2. Runs `cargo xtask video-frames-gen` to convert 65 PNG files to Rust code
//! 3. Writes the result to `examples/data/frame-data/video_frames_data.rs`
//! 4. Copies it into `examples/data/frame-data/video_frames_data.rs` for compile-time include
//!
//! To use different frames:
//!
//! 1. Set `SANTA_FRAMES_DIR` to a directory containing `frame_000001.png` style files
//! 2. Delete `examples/data/frame-data/video_frames_data.rs` to force regeneration (or run `cargo clean`)
//! 3. Rebuild the example - frames will be regenerated automatically
//!
//! Manual generation:
//!
//! ```bash
//! cargo xtask video-frames-gen > examples/data/frame-data/video_frames_data.rs
//! ```
//!
//! To regenerate from the original MP4, set `SANTA_VIDEO_PATH` and rerun the generator:
//!
//! ```bash
//! SANTA_VIDEO_PATH="/path/to/santa.mp4" \
//!   cargo xtask video-frames-gen > examples/data/frame-data/video_frames_data.rs
//! ```
//!
//! If you still have the pre-extracted PNGs, set `SANTA_FRAMES_DIR` instead:
//!
//! ```bash
//! SANTA_FRAMES_DIR="/path/to/frames12x8_landscape" \
//!   cargo xtask video-frames-gen > examples/data/frame-data/video_frames_data.rs
//! ```
//!
//! ## Cat Video (From Video File)
//!
//! To add the cat video mode:
//! 1. Set `CAT_VIDEO_PATH` to your source MP4 path.
//! 2. Generate frames:
//!    ```bash
//!    CAT_VIDEO_PATH="/path/to/cat.mp4" \
//!      cargo xtask cat-frames-gen > examples/data/frame-data/cat_frames_data.rs
//!    ```
//! 3. Uncomment the cat-related lines in this file:
//!    - `include!("data/frame-data/cat_frames_data.rs");`
//!    - `Mode::Cat` enum variant
//!    - Cat playback logic in the match statement
//!
//! The xtask command uses ffmpeg to:
//!
//! - Extract frames at 10 FPS
//! - Rotate 90° clockwise (transpose=1)
//! - Scale to 12x8 pixels
//! - Convert to embedded Rust arrays

#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use defmt::info;
use defmt_rtt as _;
use device_envoy::Result;
use device_envoy::button::{Button, PressedTo};
use device_envoy::led_strip::Current;
use device_envoy::led_strip::Gamma;
use device_envoy::led2d;
use device_envoy::led2d::Frame2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;
use smart_leds::{RGB8, colors};

// Display: 12 wide × 8 tall built from two 12×4 serpentine panels stacked vertically.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

led2d! {
    Led12x8 {
        pio: PIO1,
        pin: PIN_4,
        dma: DMA_CH1,
        led_layout: LED_LAYOUT_12X8,
        max_current: Current::Milliamps(250),
        gamma: Gamma::Srgb,
        max_frames: 70,
        font: Led2dFont::Font3x4Trim,
    }
}

// Total frames in the video
// Now defined in generated files: SANTA_FRAME_COUNT, CAT_FRAME_COUNT (with per-frame durations)

// Video frames and frame duration embedded at compile time
// Auto-generated during build from SANTA_FRAMES_DIR or SANTA_VIDEO_PATH
// See build.rs for generation logic
#[cfg(not(rust_analyzer))]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/data/frame-data/video_frames_data.rs"
));

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const SANTA_FRAME_DURATION: Duration = Duration::from_millis(100);

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const SANTA_FRAME_COUNT: usize = 1;

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const SANTA_FRAMES: [([[RGB8; 12]; 8], Duration); SANTA_FRAME_COUNT] =
    [([[colors::BLACK; 12]; 8], SANTA_FRAME_DURATION)];

// Cat video frames
// include!(concat!(
//     env!("CARGO_MANIFEST_DIR"),
//     "/examples/data/frame-data/cat_frames_data.rs"
// ));

// Hand video frames
// include!(concat!(
//     env!("CARGO_MANIFEST_DIR"),
//     "/examples/data/frame-data/hand_frames_data.rs"
// ));

// Clock video frames
#[cfg(not(rust_analyzer))]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/data/frame-data/clock_frames_data.rs"
));

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const CLOCK_FRAME_DURATION: Duration = Duration::from_millis(100);

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const CLOCK_FRAME_COUNT: usize = 1;

#[cfg(rust_analyzer)]
#[allow(dead_code)]
const CLOCK_FRAMES: [([[RGB8; 12]; 8], Duration); CLOCK_FRAME_COUNT] =
    [([[colors::BLACK; 12]; 8], CLOCK_FRAME_DURATION)];

/// Video display modes.
#[derive(defmt::Format, Clone, Copy)]
enum Mode {
    TestPattern,
    TestText,
    Santa,
    Clock,
}

impl Mode {
    /// Advance to the next mode in the cycle.
    fn next(self) -> Self {
        match self {
            Self::TestPattern => Self::TestText,
            Self::TestText => Self::Santa,
            Self::Santa => Self::Clock,
            Self::Clock => Self::TestPattern,
        }
    }
}

/// Create a test pattern frame with RGBY corners.
/// Tests all 4 corners and center cross to verify coordinate mapping.
fn create_test_pattern() -> Frame2d<12, 8> {
    let mut frame = Frame2d::new();

    frame[(0, 0)] = colors::RED; // Top-left
    frame[(Led12x8::WIDTH - 1, 0)] = colors::GREEN; // Top-right
    frame[(0, Led12x8::HEIGHT - 1)] = colors::BLUE; // Bottom-left
    frame[(Led12x8::WIDTH - 1, Led12x8::HEIGHT - 1)] = colors::YELLOW; // Bottom-right

    // Center cross for additional verification
    frame[(Led12x8::WIDTH / 2, Led12x8::HEIGHT / 2)] = colors::WHITE;

    frame.into()
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<()> {
    info!("Starting LED matrix video player (12x8 @ 10 FPS with gamma 2.2)");
    let p = embassy_rp::init(Default::default());

    // Set up the 12x8 LED display on GPIO4 with gamma 2.2 correction
    let led_12x8 = Led12x8::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    // Set up button on GPIO13 (wired to ground)
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    info!("Video player initialized - gamma correction applied automatically");

    let mut mode = Mode::TestPattern;

    loop {
        info!("Entering mode: {:?}", mode);

        match mode {
            Mode::TestPattern => {
                let test_pattern = create_test_pattern();
                led_12x8.write_frame(test_pattern)?;

                button.wait_for_press().await;
                mode = mode.next();
            }
            Mode::TestText => {
                let mut frame = Frame2d::new();
                led_12x8.write_text_to_frame(
                    "HELLO\nWORLD",
                    &[colors::CYAN, colors::MAGENTA],
                    &mut frame,
                )?;
                led_12x8.write_frame(frame)?;

                button.wait_for_press().await;
                mode = mode.next();
            }
            Mode::Santa => {
                let frames_with_duration = SANTA_FRAMES
                    .iter()
                    .map(|&(frame, duration)| (frame.into(), duration));
                led_12x8.animate(frames_with_duration)?;
                button.wait_for_press().await;
                mode = mode.next();
            }
            Mode::Clock => {
                let frames_with_duration = CLOCK_FRAMES
                    .iter()
                    .map(|&(frame, duration)| (frame.into(), duration));
                led_12x8.animate(frames_with_duration)?;
                button.wait_for_press().await;
                mode = mode.next();
            }
        }
    }
}
