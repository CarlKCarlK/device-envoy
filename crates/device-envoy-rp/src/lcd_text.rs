//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! This page provides the primary documentation for generated LCD text device
//! types.
//!
//! **After reading the generated type pages above, see also:**
//!
//! - [`LcdTextDriver`] — low-level
//!   HD44780-over-PCF8574 write driver used by generated types.
//! - [`LcdTextFrame`] — fixed-size
//!   character frame payload sent to the runtime task.

use embassy_rp::i2c;

pub use device_envoy_core::lcd_text::{LcdTextError, LcdTextFrame, LcdTextStatic, MAX_LCD_CHARS};
use device_envoy_core::lcd_text::{LcdTextDriver, LcdTextWrite};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use heapless::Vec;
pub use paste;

pub mod lcd_text_generated;

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct I2cLcdTextCommand {
    pub address: u8,
    pub frame: LcdTextFrame,
}

#[doc(hidden)]
pub type I2cLcdTextCommandSignal = Signal<CriticalSectionRawMutex, I2cLcdTextCommand>;

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
pub async fn __i2cs_device_loop<T: i2c::Instance + 'static>(
    i2c: i2c::I2c<'static, T, i2c::Blocking>,
    command_signal: &'static I2cLcdTextCommandSignal,
) -> ! {
    let mut rp_lcd_text_write = RpLcdTextWrite { i2c };
    let mut lcd_text_driver = LcdTextDriver::new(0x27);
    let mut initialized_addresses: Vec<u8, 8> = Vec::new();

    loop {
        let i2c_lcd_text_command = command_signal.wait().await;
        let first_use_of_address = !initialized_addresses
            .iter()
            .any(|initialized_address| *initialized_address == i2c_lcd_text_command.address);

        lcd_text_driver.set_address(i2c_lcd_text_command.address);
        if first_use_of_address {
            if lcd_text_driver.init(&mut rp_lcd_text_write).await.is_err() {
                continue;
            }
            let _ = initialized_addresses.push(i2c_lcd_text_command.address);
        }

        let _ = lcd_text_driver
            .write_frame(&mut rp_lcd_text_write, &i2c_lcd_text_command.frame)
            .await;
    }
}

struct RpLcdTextWrite<T: i2c::Instance + 'static> {
    i2c: i2c::I2c<'static, T, i2c::Blocking>,
}

impl<T: i2c::Instance + 'static> LcdTextWrite for RpLcdTextWrite<T> {
    fn write(&mut self, address: u8, data: u8) -> core::result::Result<(), LcdTextError> {
        self.i2c
            .blocking_write(address, &[data])
            .map_err(|_| LcdTextError::I2cWrite { address })
    }
}

/// Macro to generate multiple LCD text device types that share one I2C
/// resource (includes syntax details).
///
/// This page provides the primary documentation and examples for grouped LCD
/// text devices that share one I2C peripheral.
///
/// **Syntax:**
///
/// ```text
/// i2cs! {
///     i2c: <i2c_ident>,
///     sda_pin: <sda_pin_ident>,
///     scl_pin: <scl_pin_ident>,
///     [<visibility>] <GroupName> {
///         [<visibility>] <LcdName> {
///             width: <usize_expr>,
///             height: <usize_expr>,
///             address: <u8_expr>
///         },
///         // ...more LCD entries...
///     }
/// }
/// ```
///
/// **After reading the example below, see also:**
///
/// - [`lcd_text!`](macro@crate::lcd_text) — Simpler single-device wrapper
/// - [`LcdTextGenerated`](crate::lcd_text::lcd_text_generated::LcdTextGenerated)
/// - [`I2csGenerated`](crate::lcd_text::lcd_text_generated::I2csGenerated)
///
/// # Example: Two LCDs on One I2C Peripheral
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use defmt_rtt as _;
/// # use core::convert::Infallible;
/// use device_envoy_rp::{Result, i2cs};
///
/// i2cs! {
///     i2c: I2C0,
///     sda_pin: PIN_4,
///     scl_pin: PIN_5,
///     LcdTexts0 {
///         LcdText16x2 { width: 16, height: 2, address: 0x27 },
///         LcdText20x4 { width: 20, height: 4, address: 0x3F },
///     }
/// }
///
/// # #[embassy_executor::main]
/// # async fn main(spawner: embassy_executor::Spawner) -> ! {
/// #     let _ = example(spawner).await;
/// #     core::panic!("done");
/// # }
/// async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///     let (lcd_text16x2, lcd_text20x4) = LcdTexts0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
///
///     lcd_text16x2.write_text("16x2\nready");
///     lcd_text20x4.write_text("20x4\nshared i2c\naddress 0x3F");
///
///     core::future::pending().await
/// }
/// ```
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! i2cs {
    (
        i2c: $i2c:ident,
        sda_pin: $sda_pin:ident,
        scl_pin: $scl_pin:ident,
        $group_vis:vis $group_name:ident {
            $(
                $lcd_vis:vis $lcd_name:ident {
                    width: $width:expr,
                    height: $height:expr,
                    address: $address:expr
                }
            ),+ $(,)?
        }
    ) => {
        $crate::lcd_text::paste::paste! {
            static [<$group_name:upper _I2C_LCD_TEXT_COMMAND_SIGNAL>]:
                $crate::lcd_text::I2cLcdTextCommandSignal =
                $crate::lcd_text::I2cLcdTextCommandSignal::new();

            #[doc = "A generated group of LCD text devices that share one I2C peripheral and pin pair."]
            $group_vis struct $group_name;

            struct [<__ $group_name Devices>] {
                $(
                    [<$lcd_name:snake>]: &'static $lcd_name,
                )+
            }

            impl [<__ $group_name Devices>] {
                fn into_tuple(self) -> ($(&'static $lcd_name,)+) {
                    (
                        $(self.[<$lcd_name:snake>],)+
                    )
                }
            }

            impl $group_name {
                fn __new_devices(
                    i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$i2c>,
                    sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$sda_pin>,
                    scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$scl_pin>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<[<__ $group_name Devices>]> {
                    let i2c = embassy_rp::i2c::I2c::new_blocking(
                        i2c_peripheral,
                        scl,
                        sda,
                        embassy_rp::i2c::Config::default(),
                    );

                    let token = [<__i2cs_task_ $group_name:snake>](i2c);
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;

                    $(
                        static [<$lcd_name:upper _CELL>]: ::static_cell::StaticCell<$lcd_name> =
                            ::static_cell::StaticCell::new();
                        let [<$lcd_name:snake>] = [<$lcd_name:upper _CELL>].init($lcd_name);
                    )+

                    Ok([<__ $group_name Devices>] {
                        $(
                            [<$lcd_name:snake>],
                        )+
                    })
                }

                #[doc = "Construct the shared I2C runtime task and all generated LCD text device handles in this group."]
                pub fn new(
                    i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$i2c>,
                    sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$sda_pin>,
                    scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$scl_pin>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<($(&'static $lcd_name,)+)> {
                    Ok(Self::__new_devices(i2c_peripheral, sda, scl, spawner)?.into_tuple())
                }
            }

            $(
                /// A generated LCD text device type.
                $lcd_vis struct $lcd_name;

                impl $lcd_name {
                    /// Display width in characters.
                    pub const WIDTH: usize = $width;
                    /// Display height in characters.
                    pub const HEIGHT: usize = $height;
                    /// LCD I2C address.
                    pub const ADDRESS: u8 = $address;

                    /// Construct this LCD text device and spawn its shared I2C task.
                    pub fn new(
                        i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$i2c>,
                        sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$sda_pin>,
                        scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$scl_pin>,
                        spawner: embassy_executor::Spawner,
                    ) -> $crate::Result<&'static Self> {
                        let [<__ $group_name:snake _devices>] =
                            $group_name::__new_devices(i2c_peripheral, sda, scl, spawner)?;
                        Ok([<__ $group_name:snake _devices>].[<$lcd_name:snake>])
                    }

                    /// Write text to this LCD using clamp-to-frame behavior.
                    pub fn write_text(&self, text: impl AsRef<str>) {
                        ::core::assert!($width > 0, "lcd_text width must be > 0");
                        ::core::assert!($height > 0, "lcd_text height must be > 0");
                        ::core::assert!(
                            $height <= 4,
                            "lcd_text height must be <= 4 for HD44780 row map"
                        );
                        ::core::assert!(
                            $width * $height <= $crate::lcd_text::MAX_LCD_CHARS,
                            "lcd_text width*height must fit MAX_LCD_CHARS"
                        );

                        let lcd_text_frame =
                            $crate::lcd_text::__render_lcd_text_frame::<$width, $height>(text.as_ref());
                        [<$group_name:upper _I2C_LCD_TEXT_COMMAND_SIGNAL>].signal(
                            $crate::lcd_text::I2cLcdTextCommand {
                                address: $address,
                                frame: lcd_text_frame,
                            },
                        );
                    }
                }
            )+

            #[embassy_executor::task]
            async fn [<__i2cs_task_ $group_name:snake>](
                i2c: embassy_rp::i2c::I2c<'static, embassy_rp::peripherals::$i2c, embassy_rp::i2c::Blocking>,
            ) -> ! {
                $crate::lcd_text::__i2cs_device_loop(
                    i2c,
                    &[<$group_name:upper _I2C_LCD_TEXT_COMMAND_SIGNAL>],
                )
                .await
            }
        }
    };
}

#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use i2cs;

/// Macro to generate a single LCD text device type with a direct constructor.
///
/// This is a thin wrapper over [`i2cs!`](macro@crate::i2cs) that avoids tuple
/// destructuring for single-device setups.
///
/// **Syntax:**
///
/// ```text
/// lcd_text! {
///     i2c: <i2c_ident>,
///     sda_pin: <sda_pin_ident>,
///     scl_pin: <scl_pin_ident>,
///     [<visibility>] <LcdName> {
///         width: <usize_expr>,
///         height: <usize_expr>,
///         address: <u8_expr>
///     }
/// }
/// ```
///
/// # Example: One LCD with the Simplest Constructor Path
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use defmt_rtt as _;
/// # use core::convert::Infallible;
/// use device_envoy_rp::{Result, lcd_text};
///
/// lcd_text! {
///     i2c: I2C0,
///     sda_pin: PIN_4,
///     scl_pin: PIN_5,
///     LcdTextSimple {
///         width: 16,
///         height: 2,
///         address: 0x27
///     }
/// }
///
/// # #[embassy_executor::main]
/// # async fn main(spawner: embassy_executor::Spawner) -> ! {
/// #     let _ = example(spawner).await;
/// #     core::panic!("done");
/// # }
/// async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///     let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
///
///     lcd_text_simple.write_text("Hello from\ndevice-envoy!");
///
///     core::future::pending().await
/// }
/// ```
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! lcd_text {
    (
        i2c: $i2c:ident,
        sda_pin: $sda_pin:ident,
        scl_pin: $scl_pin:ident,
        $lcd_vis:vis $lcd_name:ident {
            width: $width:expr,
            height: $height:expr,
            address: $address:expr
        }
    ) => {
        $crate::lcd_text::paste::paste! {
            $crate::i2cs! {
                i2c: $i2c,
                sda_pin: $sda_pin,
                scl_pin: $scl_pin,
                [<LcdTextGroupFor $lcd_name>] {
                    $lcd_vis $lcd_name {
                        width: $width,
                        height: $height,
                        address: $address
                    }
                }
            }
        }
    };
}

#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use lcd_text;
