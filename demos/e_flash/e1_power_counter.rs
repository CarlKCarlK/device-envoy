#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};
use device_envoy::{
    Result,
    flash_array::FlashArray,
    led_strip::colors,
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// Set up LED layout for 12x8 panel.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

led2d! {
    Led12x8 {
        pio: PIO0,
        pin: PIN_4,
        led_layout: LED_LAYOUT_12X8_ROTATED,
        font: Led2dFont::Font7x12Trim,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

/// Boot counter (newtype). Serializes with `postcard` (Serde).
#[derive(serde::Serialize, serde::Deserialize)]
struct BootCounter(usize);

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let led12x8 = Led12x8::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    // Create a one-block flash array. Each block holds up to 3900 bytes
    // of serialized data (one 4 KB flash block minus metadata).
    let flash_array = FlashArray::<1>::new(p.FLASH)?;
    // Can destructure the array.
    let [mut boot_counter_block] = flash_array;

    // Read the boot counter. Wrong type -> None -> BootCounter(0).
    let mut boot_counter = boot_counter_block.load()?.unwrap_or(BootCounter(0));

    boot_counter.0 = (boot_counter.0 + 1) % 10; // Increment and wrap at 10.

    // Write counter back to flash. (Avoid tight loop.
    // Flash is typically good for ~100K erase cycles per block.)
    boot_counter_block.save(&boot_counter)?;

    // Display the counter on the LED panel (single digit 0-9)
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    led12x8
        .write_text(DIGITS[boot_counter.0], &[colors::RED])
        .await?;

    future::pending().await // Keep running
}
