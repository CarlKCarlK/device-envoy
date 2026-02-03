#![allow(missing_docs)]
//! Compile-only verification that multiple led2d devices can coexist in the same file.
//!
//! This demonstrates that the associated constants approach (Gpio3Led2d::HEIGHT, Gpio4Led2d::HEIGHT)
//! prevents namespace collisions when multiple devices are defined.
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use defmt_rtt as _;
use device_envoy::Result;
use device_envoy::led_strip::Current;
use device_envoy::led_strip::led_strips;
use device_envoy::led2d::Frame2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_time::Duration;
use panic_probe as _;
use smart_leds::colors;

const LED_LAYOUT_4X12: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X8: LedLayout<64, 8, 8> = LedLayout::serpentine_column_major();

// Define strips for both devices using led_strips! led2d configuration.
led_strips! {
    pio: PIO0,
    pub LedStripsPio0 {
        Gpio3Led2d: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(500),
            led2d: {
                led_layout: LED_LAYOUT_4X12,
                font: Led2dFont::Font3x4Trim,
            }
        }
    }
}

led_strips! {
    pio: PIO1,
    pub LedStripsPio1 {
        Gpio4Led2d: {
            dma: DMA_CH1,
            pin: PIN_4,
            len: 64,
            max_current: Current::Milliamps(300),
            led2d: {
                led_layout: LED_LAYOUT_8X8,
                font: Led2dFont::Font3x4Trim,
            }
        }
    }
}

/// Verify both devices can be constructed and used together
async fn test_multiple_devices(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    // Construct first device
    let (gpio3_led2d,) = LedStripsPio0::new(p.PIO0, p.PIN_3, p.DMA_CH0, spawner)?;

    // Construct second device
    let (gpio4_led2d,) = LedStripsPio1::new(p.PIO1, p.PIN_4, p.DMA_CH1, spawner)?;

    // Verify associated constants don't collide
    // Create frame for 4x12 display
    let mut frame_4x12 = Frame2d::<12, 4>::new();
    frame_4x12[(0, 0)] = colors::RED;
    frame_4x12[(Gpio3Led2d::WIDTH - 1, Gpio3Led2d::HEIGHT - 1)] = colors::BLUE;
    gpio3_led2d.write_frame(frame_4x12)?;

    // Create frame for 8x8 display (different dimensions)
    let mut frame_8x8 = Frame2d::<8, 8>::new();
    frame_8x8[(0, 0)] = colors::GREEN;
    frame_8x8[(Gpio4Led2d::WIDTH - 1, Gpio4Led2d::HEIGHT - 1)] = colors::YELLOW;
    gpio4_led2d.write_frame(frame_8x8)?;

    // Verify animations work with both
    let frames_4x12 = [(frame_4x12, Duration::from_millis(100))];
    gpio3_led2d.animate(frames_4x12)?;

    let frames_8x8 = [(frame_8x8, Duration::from_millis(100))];
    gpio4_led2d.animate(frames_8x8)?;

    // Verify N constant is correct for each
    const _N_4X12: usize = Gpio3Led2d::N; // Should be 48
    const _N_8X8: usize = Gpio4Led2d::N; // Should be 64

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // This main function exists only to satisfy the compiler.
    // The actual verification happens at compile time via the functions above.
}

#[cfg(not(any(target_arch = "arm", target_arch = "riscv32", target_arch = "riscv64")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
