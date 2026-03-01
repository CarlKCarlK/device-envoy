#![allow(missing_docs)]
//! Compile-only verification for Led2d with custom LED layout.
//!
//! This verifies that the led2d! macro works with custom LED layouts.
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use defmt_rtt as _;
use device_envoy::Result;
use device_envoy::led_strip::Current;
use device_envoy::led_strip::Gamma;
use device_envoy::led2d;
use device_envoy::led2d::Frame2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_time::Duration;
use smart_leds::colors;

// Example with a custom mapping for a 3x2 display (6 LEDs total)
// LED indices 0..5 map row-major, rows left-to-right
const LED2X3_ROW_MAJOR: LedLayout<6, 3, 2> =
    LedLayout::new([(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);

led2d! {
    pub Led2x3 {
        pio: PIO0,
        pin: PIN_3,
        dma: DMA_CH0,
        led_layout: LED2X3_ROW_MAJOR,
        max_current: Current::Milliamps(100),
        gamma: Gamma::Linear,
        max_frames: 6,
        font: Led2dFont::Font3x4Trim,
    }
}

/// Verify Led2x3 construction with custom mapping
async fn test_led2x3_custom_mapping(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let led2x3 = Led2x3::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;

    // Verify write_frame works
    let mut frame = Frame2d::<3, 2>::new();
    frame[(0, 0)] = colors::RED;
    frame[(Led2x3::WIDTH - 1, 0)] = colors::GREEN;
    frame[(0, Led2x3::HEIGHT - 1)] = colors::BLUE;
    frame[(Led2x3::WIDTH - 1, Led2x3::HEIGHT - 1)] = colors::YELLOW;
    led2x3.write_frame(frame)?;

    // Verify animate works
    let mut frames = heapless::Vec::<_, { Led2x3::MAX_FRAMES }>::new();
    for y_index in 0..Led2x3::HEIGHT {
        for x_index in 0..Led2x3::WIDTH {
            let mut frame = Frame2d::<3, 2>::new();
            frame[(x_index, y_index)] = colors::CYAN;
            frames.push((frame, Duration::from_millis(200))).ok();
        }
    }
    led2x3.animate(frames)?;

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // This main function exists only to satisfy the compiler.
    // The actual verification happens at compile time via the functions above.
}

// panic_probe provides a panic handler for host, but we need one for embedded
#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
