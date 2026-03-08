//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! This module provides the [`lcd_text!`](macro@crate::lcd_text) macro, which generates
//! a concrete LCD type per instance (including static state and Embassy task).
//! See [`lcd_text_generated::LcdTextGenerated`] for the sample generated type.

use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;

pub use device_envoy_core::lcd_text::{LcdTextError, LcdTextFrame, LcdTextStatic, MAX_LCD_CHARS};
use device_envoy_core::lcd_text::{LcdTextDriver, LcdTextWrite};
pub use paste;

pub mod lcd_text_generated;

#[doc(hidden)]
pub fn __render_lcd_text_frame<const W: usize, const H: usize>(text: &str) -> LcdTextFrame {
    let mut lcd_text_buffer = [[b' '; W]; H];

    for (row_index, line) in text.split('\n').enumerate() {
        if row_index >= H {
            break;
        }

        for (column_index, ch) in line.chars().enumerate() {
            if column_index >= W {
                break;
            }
            lcd_text_buffer[row_index][column_index] = if ch.is_ascii() { ch as u8 } else { b'?' };
        }
    }

    LcdTextFrame::from_rows(lcd_text_buffer)
}

#[doc(hidden)]
pub async fn __lcd_text_device_loop(
    i2c: i2c::I2c<'static, I2C0, i2c::Blocking>,
    lcd_text_static: &'static LcdTextStatic,
    address: u8,
) -> ! {
    let mut rp_lcd_text_write = RpLcdTextWrite { i2c };
    let mut lcd_text_driver = LcdTextDriver::new(address);
    if let Err(_error) = lcd_text_driver.init(&mut rp_lcd_text_write).await {
        core::future::pending().await
    }

    loop {
        let lcd_text_frame = lcd_text_static.wait_frame().await;
        let _ = lcd_text_driver
            .write_frame(&mut rp_lcd_text_write, &lcd_text_frame)
            .await;
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

/// Macro to generate a concrete LCD text device type with its own Embassy task and static state.
///
/// Syntax:
///
/// ```text
/// lcd_text! {
///     [vis] Name {
///         width: 16,
///         height: 2,
///         i2c: I2C0,
///         sda_pin: PIN_4,
///         scl_pin: PIN_5,
///     }
/// }
/// ```
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! lcd_text {
    (
        $vis:vis $name:ident {
            width: $width:expr,
            height: $height:expr,
            i2c: $i2c:ident,
            sda_pin: $sda_pin:ident,
            scl_pin: $scl_pin:ident $(,)?
        }
    ) => {
        $crate::lcd_text::paste::paste! {
            /// A generated LCD text device type.
            $vis struct $name;

            static [<$name:upper _LCD_TEXT_STATIC>]: $crate::lcd_text::LcdTextStatic =
                $crate::lcd_text::LcdTextStatic::new();

            impl $name {
                /// Display width in characters.
                pub const WIDTH: usize = $width;
                /// Display height in characters.
                pub const HEIGHT: usize = $height;

                /// Create a new LCD text device instance.
                pub fn new(
                    i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$i2c>,
                    sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$sda_pin>,
                    scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$scl_pin>,
                    address: u8,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    ::core::assert!($width > 0, "lcd_text width must be > 0");
                    ::core::assert!($height > 0, "lcd_text height must be > 0");
                    ::core::assert!($height <= 4, "lcd_text height must be <= 4 for HD44780 row map");
                    ::core::assert!(
                        $width * $height <= $crate::lcd_text::MAX_LCD_CHARS,
                        "lcd_text width*height must fit MAX_LCD_CHARS"
                    );

                    let i2c = embassy_rp::i2c::I2c::new_blocking(
                        i2c_peripheral,
                        scl,
                        sda,
                        embassy_rp::i2c::Config::default(),
                    );
                    let token = [<__lcd_text_task_ $name:snake>](i2c, address);
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                    static LCD_TEXT_INSTANCE_CELL: ::static_cell::StaticCell<$name> = ::static_cell::StaticCell::new();
                    Ok(LCD_TEXT_INSTANCE_CELL.init($name))
                }

                /// Write text to this LCD using clamp-to-frame behavior.
                pub fn write_text(&self, text: impl AsRef<str>) {
                    let lcd_text_frame = $crate::lcd_text::__render_lcd_text_frame::<$width, $height>(
                        text.as_ref(),
                    );
                    [<$name:upper _LCD_TEXT_STATIC>].signal_frame(lcd_text_frame);
                }
            }

            #[embassy_executor::task]
            async fn [<__lcd_text_task_ $name:snake>](
                i2c: embassy_rp::i2c::I2c<'static, embassy_rp::peripherals::I2C0, embassy_rp::i2c::Blocking>,
                address: u8,
            ) -> ! {
                $crate::lcd_text::__lcd_text_device_loop(
                    i2c,
                    &[<$name:upper _LCD_TEXT_STATIC>],
                    address,
                )
                .await
            }
        }
    };
}

#[cfg(not(feature = "host"))]
pub use lcd_text;
