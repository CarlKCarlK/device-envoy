#![allow(missing_docs)]
//! Compile-only verification for Led12x4 construction using led_strips! led2d configuration.
//!
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use defmt_rtt as _;
use device_envoy::Result;
use device_envoy::led_strip::led_strips;
use device_envoy::led_strip::{Current, colors};
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use panic_probe as _;

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();

led_strips! {
    pio: PIO0,
    pub LedStripsPio0 {
        Gpio3Pio0Led2d: {
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(500),
            led2d: {
                led_layout: LED_LAYOUT_12X4,
                font: Led2dFont::Font3x4Trim,
            }
        }
    }
}

led_strips! {
    pio: PIO1,
    pub LedStripsPio1 {
        Gpio3Pio1Led2d: {
            dma: DMA_CH1,
            pin: PIN_3,
            len: 48,
            max_current: Current::Milliamps(500),
            led2d: {
                led_layout: LED_LAYOUT_12X4,
                font: Led2dFont::Font3x4Trim,
            }
        }
    }
}

/// Verify Gpio3Pio0LedStrip with write_text
async fn test_led12x4_pio0_write_text(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (gpio3_pio0_led2d,) = LedStripsPio0::new(p.PIO0, p.PIN_3, p.DMA_CH0, spawner)?;

    gpio3_pio0_led2d
        .write_text(
            "1234",
            &[colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW],
        )
        .await?;

    Ok(())
}

/// Verify Gpio3Pio1LedStrip constructor
async fn test_led12x4_pio1(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (_gpio3_pio1_led2d,) = LedStripsPio1::new(p.PIO1, p.PIN_3, p.DMA_CH1, spawner)?;

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
