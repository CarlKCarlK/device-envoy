#![allow(missing_docs)]
//! Compile-only verification that four LED strips on one PIO compile.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::led_strip::Current;
use device_envoy_rp::led_strip::led_strips;
use embassy_executor::Spawner;

led_strips! {
    pio: PIO0,
    pub LedStripsPio0Four {
        Gpio0Strip: { pin: PIN_0, dma: DMA_CH0, len: 8, max_current: Current::Milliamps(120) },
        Gpio3Strip: { pin: PIN_3, dma: DMA_CH1, len: 8, max_current: Current::Milliamps(120) },
        Gpio4Strip: { pin: PIN_4, dma: DMA_CH2, len: 8, max_current: Current::Milliamps(120) },
        Gpio5Strip: { pin: PIN_5, dma: DMA_CH3, len: 8, max_current: Current::Milliamps(120) }
    }
}

async fn test_four_led_strips_on_one_pio(
    peripherals: embassy_rp::Peripherals,
    spawner: Spawner,
) -> Result<()> {
    let (_gpio0_strip, _gpio3_strip, _gpio4_strip, _gpio5_strip) = LedStripsPio0Four::new(
        peripherals.PIO0,
        peripherals.PIN_0,
        peripherals.DMA_CH0,
        peripherals.PIN_3,
        peripherals.DMA_CH1,
        peripherals.PIN_4,
        peripherals.DMA_CH2,
        peripherals.PIN_5,
        peripherals.DMA_CH3,
        spawner,
    )?;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
