//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! See [`LcdText`] for the primary usage example.

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::i2c::{self, Config as I2cConfig, SclPin, SdaPin};
use embassy_rp::peripherals::I2C0;
use heapless::String;

pub use device_envoy_core::lcd_text::LcdTextStatic;
use device_envoy_core::lcd_text::{LcdTextDriver, LcdTextError, LcdTextMessage, LcdTextWrite};

use crate::{Error, Result};

/// A device abstraction for an HD44780-compatible character LCD.
///
/// ```rust,no_run
/// # #![no_std]
/// # use panic_probe as _;
/// # fn main() {}
/// use device_envoy_rp::lcd_text::{LcdText, LcdTextStatic};
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     static LCD_TEXT_STATIC: LcdTextStatic = LcdText::new_static();
///     let lcd = LcdText::new(&LCD_TEXT_STATIC, p.I2C0, p.PIN_1, p.PIN_0, spawner)?;
///     let text: heapless::String<64> = "Hello!".try_into().unwrap();
///     lcd.write_text(text, 1_000).await?;
///     Ok(())
/// }
/// ```
pub struct LcdText {
    lcd_text_static: &'static LcdTextStatic,
}

impl LcdText {
    /// Create LcdText resources.
    #[must_use]
    pub const fn new_static() -> LcdTextStatic {
        LcdTextStatic::new()
    }

    /// Create a new LcdText device.
    ///
    /// Note: Hardcoded to I2C0 peripheral (like WiFi's internal pins).
    /// However, SCL and SDA can be any pins compatible with I2C0.
    pub fn new<SCL, SDA>(
        lcd_text_static: &'static LcdTextStatic,
        i2c_peripheral: Peri<'static, I2C0>,
        scl: Peri<'static, SCL>,
        sda: Peri<'static, SDA>,
        spawner: Spawner,
    ) -> Result<Self>
    where
        SCL: SclPin<I2C0>,
        SDA: SdaPin<I2C0>,
    {
        let i2c = i2c::I2c::new_blocking(i2c_peripheral, scl, sda, I2cConfig::default());
        let token = lcd_task(i2c, lcd_text_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self { lcd_text_static })
    }

    /// Send a message to the LCD (async, waits until queued).
    pub async fn write_text(&self, text: String<64>, duration_ms: u32) -> Result<()> {
        self.lcd_text_static
            .send_request(LcdTextMessage::Display { text, duration_ms })
            .await;
        self.lcd_text_static.receive_response().await?;
        Ok(())
    }
}

struct RpLcdTextWrite {
    i2c: i2c::I2c<'static, I2C0, i2c::Blocking>,
}

impl LcdTextWrite for RpLcdTextWrite {
    fn write(&mut self, address: u8, data: u8) -> core::result::Result<(), LcdTextError> {
        self.i2c
            .blocking_write(address, &[data])
            .map_err(|_| LcdTextError::I2cWrite { address })
    }
}

#[embassy_executor::task]
async fn lcd_task(
    i2c: i2c::I2c<'static, I2C0, i2c::Blocking>,
    lcd_text_static: &'static LcdTextStatic,
) -> ! {
    let mut rp_lcd_text_write = RpLcdTextWrite { i2c };
    let mut lcd_text_driver = LcdTextDriver::new_default();
    if let Err(error) = lcd_text_driver.init(&mut rp_lcd_text_write).await {
        lcd_text_static.send_response(Err(error)).await;
        core::future::pending().await
    }

    loop {
        let lcd_text_message = lcd_text_static.receive_request().await;
        let write_result = lcd_text_driver
            .process_message(&mut rp_lcd_text_write, lcd_text_message)
            .await;
        lcd_text_static.send_response(write_result).await;
    }
}
