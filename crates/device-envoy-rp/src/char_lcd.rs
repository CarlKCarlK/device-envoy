//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! See [`CharLcd`] for the primary usage example.

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::i2c::{self, Config as I2cConfig, SclPin, SdaPin};
use embassy_rp::peripherals::I2C0;
use heapless::String;

pub use device_envoy_core::char_lcd::CharLcdStatic;
use device_envoy_core::char_lcd::{CharLcdDriver, CharLcdMessage, CharLcdWrite};

use crate::{Error, Result};

/// A device abstraction for an HD44780-compatible character LCD.
///
/// ```rust,no_run
/// # #![no_std]
/// # use panic_probe as _;
/// # fn main() {}
/// use device_envoy_rp::char_lcd::{CharLcd, CharLcdStatic};
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     static CHAR_LCD_STATIC: CharLcdStatic = CharLcd::new_static();
///     let lcd = CharLcd::new(&CHAR_LCD_STATIC, p.I2C0, p.PIN_1, p.PIN_0, spawner)?;
///     let text: heapless::String<64> = "Hello!".try_into().unwrap();
///     lcd.write_text(text, 1_000).await;
///     Ok(())
/// }
/// ```
pub struct CharLcd {
    char_lcd_static: &'static CharLcdStatic,
}

impl CharLcd {
    /// Create CharLcd resources.
    #[must_use]
    pub const fn new_static() -> CharLcdStatic {
        CharLcdStatic::new()
    }

    /// Create a new CharLcd device.
    ///
    /// Note: Hardcoded to I2C0 peripheral (like WiFi's internal pins).
    /// However, SCL and SDA can be any pins compatible with I2C0.
    pub fn new<SCL, SDA>(
        char_lcd_static: &'static CharLcdStatic,
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
        let token = lcd_task(i2c, char_lcd_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self { char_lcd_static })
    }

    /// Send a message to the LCD (async, waits until queued).
    pub async fn write_text(&self, text: String<64>, duration_ms: u32) {
        self.char_lcd_static
            .send(CharLcdMessage::Display { text, duration_ms })
            .await;
    }
}

struct RpCharLcdWrite {
    i2c: i2c::I2c<'static, I2C0, i2c::Blocking>,
}

impl CharLcdWrite for RpCharLcdWrite {
    fn write(&mut self, address: u8, data: u8) {
        if self.i2c.blocking_write(address, &[data]).is_err() {
            // Keep the background task running if one bus write fails.
        }
    }
}

#[embassy_executor::task]
async fn lcd_task(
    i2c: i2c::I2c<'static, I2C0, i2c::Blocking>,
    char_lcd_static: &'static CharLcdStatic,
) -> ! {
    let mut rp_char_lcd_write = RpCharLcdWrite { i2c };
    let mut char_lcd_driver = CharLcdDriver::new_default();
    char_lcd_driver.init(&mut rp_char_lcd_write).await;

    loop {
        let char_lcd_message = char_lcd_static.receive().await;
        char_lcd_driver
            .process_message(&mut rp_char_lcd_write, char_lcd_message)
            .await;
    }
}
