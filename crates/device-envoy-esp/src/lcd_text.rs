//! A device abstraction for HD44780-compatible character LCDs (e.g., 16x2, 20x2, 20x4).
//!
//! This page provides the primary documentation and examples for LCD text
//! devices.
//!
//! **After reading the examples below, see also:**
//!
//! - [`lcd_text!`](macro@crate::lcd_text) — Macro to generate a single LCD
//!   text type (includes syntax details).
//! - [`i2cs!`](macro@crate::i2cs) — Macro to generate multiple LCD text types
//!   sharing one I2C resource (includes syntax details).
//! - [`LcdText`] — Core LCD text trait implemented by generated types.
//!
//! # Text Behavior
//!
//! `write_text(...)` behavior:
//!
//! - `\n` starts a new LCD row.
//! - Characters past `WIDTH` on a row are "ignored".
//! - Rows past `HEIGHT` are "ignored".
//! - Non-ASCII Unicode characters are replaced with `?`.
//! - Missing characters are padded with spaces.
//!
//! # Example: Write Text on One LCD
//!
//! In this example, the generated type is `LcdTextSimple`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::lcd_text::LcdText as _;
//! use device_envoy_esp::{Result, init_and_start, lcd_text};
//!
//! lcd_text! {
//!     i2c: I2C0,
//!     sda_pin: GPIO16,
//!     scl_pin: GPIO17,
//!     LcdTextSimple {
//!         width: 16,
//!         height: 2,
//!         address: 0x27
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     match example(spawner).await {
//! #         Ok(infallible) => match infallible {},
//! #         Err(error) => panic!("{error:?}"),
//! #     }
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p);
//!     let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;
//!
//!     lcd_text_simple.write_text("Hello from\ndevice-envoy!");
//!
//!     core::future::pending().await
//! }
//! ```
//!
//! # Example: Two LCDs Sharing One I2C Peripheral
//!
//! In this example, the generated group type is `LcdTexts0`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::lcd_text::LcdText as _;
//! use device_envoy_esp::{Result, i2cs, init_and_start};
//!
//! i2cs! {
//!     i2c: I2C0,
//!     sda_pin: GPIO16,
//!     scl_pin: GPIO17,
//!     LcdTexts0 {
//!         LcdText16x2 { width: 16, height: 2, address: 0x27 },
//!         LcdText20x4 { width: 20, height: 4, address: 0x3F },
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     match example(spawner).await {
//! #         Ok(infallible) => match infallible {},
//! #         Err(error) => panic!("{error:?}"),
//! #     }
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p);
//!     let (lcd_text16x2, lcd_text20x4) = LcdTexts0::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;
//!
//!     lcd_text16x2.write_text("16x2\nready");
//!     lcd_text20x4.write_text("20x4\nshared i2c\naddress 0x3F");
//!
//!     core::future::pending().await
//! }
//! ```

use device_envoy_core::lcd_text::{LcdTextDriver, LcdTextError, LcdTextFrame, LcdTextWrite};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use heapless::Vec;

#[doc(hidden)]
pub use paste;

pub use device_envoy_core::lcd_text::LcdText;

#[doc(hidden)]
pub const __MAX_LCD_CHARS: usize = device_envoy_core::lcd_text::MAX_LCD_CHARS;
#[doc(hidden)]
pub type __I2csSignal<T> = Signal<CriticalSectionRawMutex, T>;
#[doc(hidden)]
pub use device_envoy_core::lcd_text::render_lcd_text_frame as __render_lcd_text_frame;
#[doc(hidden)]
pub use device_envoy_core::lcd_text::LcdText as __LcdText;
#[doc(hidden)]
pub use device_envoy_core::lcd_text::LcdTextDriver as __LcdTextDriver;
#[doc(hidden)]
pub use device_envoy_core::lcd_text::LcdTextFrame as __LcdTextFrame;
#[doc(hidden)]
pub async fn __select_array<Fut, const N: usize>(futures: [Fut; N]) -> (Fut::Output, usize)
where
    Fut: core::future::Future,
{
    embassy_futures::select::select_array(futures).await
}

#[doc(hidden)]
pub async fn __write_lcd_text_cells(
    lcd_text_driver: &mut LcdTextDriver,
    lcd_text_write: &mut impl LcdTextWrite,
    initialized_addresses: &mut Vec<u8, 8>,
    address: u8,
    width: usize,
    height: usize,
    cells: &[u8],
) {
    let first_use_of_address = !initialized_addresses
        .iter()
        .any(|initialized_address| *initialized_address == address);

    lcd_text_driver.set_address(address);
    if first_use_of_address {
        if lcd_text_driver.init(lcd_text_write).await.is_err() {
            return;
        }
        let _ = initialized_addresses.push(address);
    }

    let mut lcd_text_frame = LcdTextFrame::new_blank(width, height);
    let cell_count = core::cmp::min(width * height, cells.len());
    for cell_index in 0..cell_count {
        lcd_text_frame.cells[cell_index] = cells[cell_index];
    }

    let _ = lcd_text_driver
        .write_frame(lcd_text_write, &lcd_text_frame)
        .await;
}

#[doc(hidden)]
pub struct EspLcdTextWrite {
    i2c: crate::esp_hal::i2c::master::I2c<'static, crate::esp_hal::Blocking>,
}

impl EspLcdTextWrite {
    #[doc(hidden)]
    pub fn __new(i2c: crate::esp_hal::i2c::master::I2c<'static, crate::esp_hal::Blocking>) -> Self {
        Self { i2c }
    }
}

impl LcdTextWrite for EspLcdTextWrite {
    fn write(&mut self, address: u8, data: u8) -> core::result::Result<(), LcdTextError> {
        self.i2c
            .write(address, &[data])
            .map_err(|_| LcdTextError::I2cWrite { address })
    }
}

/// Macro to generate multiple LCD text device types that share one I2C
/// resource (includes syntax details).
///
/// For a single LCD type, see [`lcd_text!`](macro@crate::lcd_text).
///
/// **See the [lcd_text module documentation](mod@crate::lcd_text) for usage
/// examples.**
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! i2cs {
    ($($tt:tt)*) => { $crate::__i2cs_impl! { $($tt)* } };
}

#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __i2cs_impl {
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
            $(
                static [<$lcd_name:upper _FRAME_SIGNAL>]:
                    $crate::lcd_text::__I2csSignal<$crate::lcd_text::__LcdTextFrame> =
                    $crate::lcd_text::__I2csSignal::new();
            )+

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
                    i2c_peripheral: $crate::esp_hal::peripherals::$i2c<'static>,
                    sda: $crate::esp_hal::peripherals::$sda_pin<'static>,
                    scl: $crate::esp_hal::peripherals::$scl_pin<'static>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<[<__ $group_name Devices>]> {
                    let i2c = $crate::esp_hal::i2c::master::I2c::new(
                        i2c_peripheral,
                        $crate::esp_hal::i2c::master::Config::default(),
                    )
                    .map_err($crate::Error::I2cConfig)?
                    .with_sda(sda)
                    .with_scl(scl);

                    let token = [<__i2cs_task_ $group_name:snake>](i2c);
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;

                    $(
                        static [<$lcd_name:upper _INSTANCE>]: $lcd_name = $lcd_name;
                        let [<$lcd_name:snake>] = &[<$lcd_name:upper _INSTANCE>];
                    )+

                    Ok([<__ $group_name Devices>] {
                        $(
                            [<$lcd_name:snake>],
                        )+
                    })
                }

                pub fn new(
                    i2c_peripheral: $crate::esp_hal::peripherals::$i2c<'static>,
                    sda: $crate::esp_hal::peripherals::$sda_pin<'static>,
                    scl: $crate::esp_hal::peripherals::$scl_pin<'static>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<($(&'static $lcd_name,)+)> {
                    Ok(Self::__new_devices(i2c_peripheral, sda, scl, spawner)?.into_tuple())
                }
            }

            $(
                $lcd_vis struct $lcd_name;

                impl $crate::lcd_text::__LcdText<$width, $height> for $lcd_name {
                    const ADDRESS: u8 = $address;

                    fn write_text(&self, text: impl AsRef<str>) {
                        ::core::assert!($width > 0, "lcd_text width must be > 0");
                        ::core::assert!($height > 0, "lcd_text height must be > 0");
                        ::core::assert!(
                            $height <= 4,
                            "lcd_text height must be <= 4 for HD44780 row map"
                        );
                        ::core::assert!(
                            $width * $height <= $crate::lcd_text::__MAX_LCD_CHARS,
                            "lcd_text width*height must fit MAX_LCD_CHARS"
                        );

                        let lcd_text_frame =
                            $crate::lcd_text::__render_lcd_text_frame::<$width, $height>(text.as_ref());
                        [<$lcd_name:upper _FRAME_SIGNAL>].signal(lcd_text_frame);
                    }
                }

                impl $lcd_name {
                    pub const WIDTH: usize = $width;
                    pub const HEIGHT: usize = $height;
                    pub const ADDRESS: u8 = $address;

                    pub fn new(
                        i2c_peripheral: $crate::esp_hal::peripherals::$i2c<'static>,
                        sda: $crate::esp_hal::peripherals::$sda_pin<'static>,
                        scl: $crate::esp_hal::peripherals::$scl_pin<'static>,
                        spawner: embassy_executor::Spawner,
                    ) -> $crate::Result<&'static Self> {
                        let [<__ $group_name:snake _devices>] =
                            $group_name::__new_devices(i2c_peripheral, sda, scl, spawner)?;
                        Ok([<__ $group_name:snake _devices>].[<$lcd_name:snake>])
                    }

                }
            )+

            #[embassy_executor::task]
            async fn [<__i2cs_task_ $group_name:snake>](
                i2c: $crate::esp_hal::i2c::master::I2c<'static, $crate::esp_hal::Blocking>,
            ) -> ! {
                let mut esp_lcd_text_write = $crate::lcd_text::EspLcdTextWrite::__new(i2c);
                let mut lcd_text_driver = $crate::lcd_text::__LcdTextDriver::new(0x27);
                let mut initialized_addresses: heapless::Vec<u8, 8> = heapless::Vec::new();
                let addresses = [$($address,)+];
                let widths = [$($width,)+];
                let heights = [$($height,)+];

                loop {
                    let (lcd_text_frame, ready_index) = $crate::lcd_text::__select_array([
                        $([<$lcd_name:upper _FRAME_SIGNAL>].wait(),)+
                    ]).await;
                    $crate::lcd_text::__write_lcd_text_cells(
                        &mut lcd_text_driver,
                        &mut esp_lcd_text_write,
                        &mut initialized_addresses,
                        addresses[ready_index],
                        widths[ready_index],
                        heights[ready_index],
                        &lcd_text_frame.cells,
                    ).await;
                }
            }
        }
    };
}

#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use i2cs;

/// Macro to generate a single LCD text device type with a direct constructor.
///
/// For multiple LCD types sharing one I2C peripheral, see
/// [`i2cs!`](macro@crate::i2cs).
///
/// **See the [lcd_text module documentation](mod@crate::lcd_text) for usage
/// examples.**
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! lcd_text {
    ($($tt:tt)*) => { $crate::__lcd_text_impl! { $($tt)* } };
}

#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __lcd_text_impl {
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
