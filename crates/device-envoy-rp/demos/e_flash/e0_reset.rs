#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};
use device_envoy::{
    Result,
    flash_array::FlashArray,
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

/// Reset marker (different type than BootCounter to clear flash)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
struct ResetMarker(u32);

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

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a flash array with one element.
    let mut flash_array = FlashArray::<1>::new(p.FLASH)?;

    // Write a ResetMarker (different type than BootCounter) to flash.
    // This clears any existing BootCounter since FlashArray type-checks on load.
    flash_array[0].save(&ResetMarker(0))?;

    // Display black (turn off all LEDs) on the panel
    let led12x8 = Led12x8::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;
    led12x8.write_text("", &[]).await?;

    future::pending().await // Keep running
}
