#![allow(missing_docs)]
#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use defmt_rtt as _;

use device_envoy::Result;
use device_envoy::led_strip::Current;
use device_envoy::led_strip::led_strips;
use embassy_executor::Spawner;

const MAX_CURRENT: Current = Current::Milliamps(250);

led_strips! {
    pio: PIO0,
    pub LedStripsPio0 {
        Pio0LedStrip: { pin: PIN_3, len: 48, max_current: MAX_CURRENT }
    }
}

led_strips! {
    pio: PIO1,
    pub LedStripsPio1 {
        Pio1LedStrip: { dma: DMA_CH1, pin: PIN_4, len: 48, max_current: MAX_CURRENT }
    }
}

#[cfg(feature = "pico2")]
led_strips! {
    pio: PIO2,
    pub LedStripsPio2 {
        Pio2LedStrip: { dma: DMA_CH2, pin: PIN_5, len: 48, max_current: MAX_CURRENT }
    }
}

/// Compile-only test to verify `led_strips!` works with all PIO blocks.
/// This prevents type mismatches between generated strip types and PIO splits.
#[allow(dead_code)]
async fn test_all_pios(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (_pio0_led_strip_48,) = LedStripsPio0::new(p.PIO0, p.PIN_3, p.DMA_CH0, spawner)?;
    let (_pio1_led_strip_48,) = LedStripsPio1::new(p.PIO1, p.PIN_4, p.DMA_CH1, spawner)?;

    // Test PIO2 (Pico 2 only)
    #[cfg(feature = "pico2")]
    {
        let (_pio2_led_strip_48,) = LedStripsPio2::new(p.PIO2, p.PIN_5, p.DMA_CH2, spawner)?;
    }

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // This main function exists only to satisfy the compiler.
}

// panic_probe provides a panic handler for host, but we need one for embedded
#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
