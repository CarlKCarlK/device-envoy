#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, panic};
use device_envoy_rp::lcd_text::{LcdText, LcdTextStatic};
use device_envoy_rp::{Error, Result};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Common backpack addresses: 0x27 or 0x3F.
    const LCD_ADDRESS: u8 = 0x27;
    static LCD_TEXT_STATIC: LcdTextStatic = LcdText::<16, 2>::new_static();
    let lcd_text = LcdText::<16, 2>::new(
        &LCD_TEXT_STATIC,
        p.I2C0,
        p.PIN_4,
        p.PIN_5,
        LCD_ADDRESS,
        spawner,
    )?;

    let mut text = heapless::String::<64>::new();
    text.push_str("Hello from\n").map_err(|_| Error::FormatError)?;
    text.push_str("device-envoy!")
        .map_err(|_| Error::FormatError)?;

    lcd_text.write_text(text, 0).await?;

    core::future::pending().await
}
