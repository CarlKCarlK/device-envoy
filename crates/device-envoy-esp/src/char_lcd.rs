//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! See [`CharLcd`] for usage.

pub use device_envoy_core::char_lcd::CharLcdStatic;

#[cfg(target_os = "none")]
use device_envoy_core::char_lcd::{CharLcdDriver, CharLcdError, CharLcdMessage, CharLcdWrite};
#[cfg(target_os = "none")]
use heapless::String;

#[cfg(target_os = "none")]
use crate::{Error, Result};

/// A device abstraction for an HD44780-compatible character LCD.
#[cfg(target_os = "none")]
pub struct CharLcd {
    char_lcd_static: &'static CharLcdStatic,
}

#[cfg(target_os = "none")]
impl CharLcd {
    /// Create static resources for the character LCD device.
    #[must_use]
    pub const fn new_static() -> CharLcdStatic {
        CharLcdStatic::new()
    }

    /// Create a new character LCD device from an initialized I2C bus.
    pub fn new(
        char_lcd_static: &'static CharLcdStatic,
        i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>,
        spawner: embassy_executor::Spawner,
    ) -> Result<Self> {
        let token = lcd_task(i2c, char_lcd_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self { char_lcd_static })
    }

    /// Send a message to the LCD (async, waits until queued).
    pub async fn write_text(&self, text: String<64>, duration_ms: u32) -> Result<()> {
        self.char_lcd_static
            .send_request(CharLcdMessage::Display { text, duration_ms })
            .await;
        self.char_lcd_static.receive_response().await?;
        Ok(())
    }
}

#[cfg(target_os = "none")]
struct EspCharLcdWrite {
    i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>,
}

#[cfg(target_os = "none")]
impl CharLcdWrite for EspCharLcdWrite {
    fn write(&mut self, address: u8, data: u8) -> core::result::Result<(), CharLcdError> {
        self.i2c.write(address, &[data]).map_err(|_| CharLcdError::I2cWrite { address })
    }
}

#[cfg(target_os = "none")]
#[embassy_executor::task]
async fn lcd_task(
    i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>,
    char_lcd_static: &'static CharLcdStatic,
) -> ! {
    let mut esp_char_lcd_write = EspCharLcdWrite { i2c };
    let mut char_lcd_driver = CharLcdDriver::new_default();
    if let Err(error) = char_lcd_driver.init(&mut esp_char_lcd_write).await {
        char_lcd_static.send_response(Err(error)).await;
        core::future::pending().await
    }

    loop {
        let char_lcd_message = char_lcd_static.receive_request().await;
        let write_result = char_lcd_driver
            .process_message(&mut esp_char_lcd_write, char_lcd_message)
            .await;
        char_lcd_static.send_response(write_result).await;
    }
}
