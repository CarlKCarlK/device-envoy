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
//! - [`LcdTextGenerated`](lcd_text_generated::LcdTextGenerated) — Sample
//!   generated LCD text type showing the constructor path.
//! - [`I2csGenerated`](lcd_text_generated::I2csGenerated) — Sample generated
//!   I2C group type for multiple LCD text devices.
//! - [`LcdText`] — Core LCD text trait implemented by generated types.
//!
//! # Text Behavior
//!
//! `write_text(...)` behavior:
//!
//! - `\n` starts a new LCD row.
//! - characters past `WIDTH` are ignored
//! - rows past `HEIGHT` are ignored
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
//! # use panic_probe as _;
//! # use defmt_rtt as _;
//! # use core::{convert::Infallible, future::pending};
//! use device_envoy_rp::{Result, lcd_text, lcd_text::LcdText as _};
//!
//! lcd_text! {
//!     i2c: I2C0,
//!     sda_pin: PIN_4,
//!     scl_pin: PIN_5,
//!     LcdTextSimple {
//!         width: 16,
//!         height: 2,
//!         address: 0x27
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let _ = example(spawner).await;
//! #     core::panic!("done");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
//!
//!     lcd_text_simple.write_text("Hello from\ndevice-envoy!");
//!
//!     pending().await
//! }
//! ```
//!
//! # Example: Two LCDs Sharing One I2C Peripheral
//!
//! In this example, the generated group type is `I2cs0`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use defmt_rtt as _;
//! # use core::{convert::Infallible, future::pending};
//! use device_envoy_rp::{Result, i2cs, lcd_text::LcdText as _};
//!
//! i2cs! {
//!     i2c: I2C0,
//!     sda_pin: PIN_4,
//!     scl_pin: PIN_5,
//!     I2cs0 {
//!         LcdText16x2 { width: 16, height: 2, address: 0x27 },
//!         LcdText20x4 { width: 20, height: 4, address: 0x3F },
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let _ = example(spawner).await;
//! #     core::panic!("done");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let (lcd_text16x2, lcd_text20x4) = I2cs0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
//!
//!     lcd_text16x2.write_text("16x2\nready");
//!     lcd_text20x4.write_text("20x4\nshared i2c\naddress 0x3F");
//!
//!     pending().await
//! }
//! ```

use embassy_rp::i2c;

use device_envoy_core::lcd_text::{LcdTextDriver, LcdTextFrame, LcdTextWrite};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use heapless::Vec;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use paste;

pub mod lcd_text_generated;
pub use device_envoy_core::lcd_text::LcdText;

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub type __I2csSignal<T> = Signal<CriticalSectionRawMutex, T>;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::lcd_text::LcdTextDriver as __LcdTextDriver;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::lcd_text::LcdText as __LcdText;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::lcd_text::render_lcd_text_frame as __render_lcd_text_frame;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub type __LcdTextFrame<const MAX_CHARS: usize> =
    device_envoy_core::lcd_text::LcdTextFrame<MAX_CHARS>;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub const fn __max_lcd_cells<const N: usize>(widths: [usize; N], heights: [usize; N]) -> usize {
    let mut max_cells = 0;
    let mut index = 0;
    while index < N {
        let cells = widths[index] * heights[index];
        if cells > max_cells {
            max_cells = cells;
        }
        index += 1;
    }
    max_cells
}
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub async fn __select_array<Fut, const N: usize>(futures: [Fut; N]) -> (Fut::Output, usize)
where
    Fut: core::future::Future,
{
    embassy_futures::select::select_array(futures).await
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub const fn __assert_unique_addresses<const N: usize>(addresses: [u8; N]) {
    let mut first_index = 0;
    while first_index < N {
        let mut second_index = first_index + 1;
        while second_index < N {
            if addresses[first_index] == addresses[second_index] {
                panic!("duplicate lcd_text I2C address in i2cs! group");
            }
            second_index += 1;
        }
        first_index += 1;
    }
}

#[doc(hidden)]
pub async fn __write_lcd_text_cells<const ADDRESS_COUNT: usize, const MAX_CHARS: usize>(
    lcd_text_driver: &mut LcdTextDriver,
    lcd_text_write: &mut impl LcdTextWrite,
    initialized_addresses: &mut Vec<u8, ADDRESS_COUNT>,
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

    let mut lcd_text_frame = LcdTextFrame::<MAX_CHARS>::new_blank(width, height);
    let cell_count = core::cmp::min(width * height, cells.len());
    for cell_index in 0..cell_count {
        lcd_text_frame.cells[cell_index] = cells[cell_index];
    }

    let _ = lcd_text_driver
        .write_frame(lcd_text_write, &lcd_text_frame)
        .await;
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub struct RpLcdTextWrite<T: i2c::Instance + 'static> {
    i2c: i2c::I2c<'static, T, i2c::Blocking>,
}

impl<T: i2c::Instance + 'static> RpLcdTextWrite<T> {
    // Must be `pub` for macro expansion at downstream call sites.
    #[doc(hidden)]
    pub fn __new(i2c: i2c::I2c<'static, T, i2c::Blocking>) -> Self {
        Self { i2c }
    }
}

impl<T: i2c::Instance + 'static> LcdTextWrite for RpLcdTextWrite<T> {
    fn write(&mut self, address: u8, data: u8) -> device_envoy_core::Result<()> {
        self.i2c
            .blocking_write(address, &[data])
            .map_err(|_| device_envoy_core::Error::LcdI2cWrite { address })
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

/// Implementation macro. Not part of the public API; use [`i2cs!`] instead.
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
            const _: () = {
                $crate::lcd_text::__assert_unique_addresses([$($address,)+]);
            };
            const [<__ $group_name:upper _MAX_LCD_CELLS>]: usize =
                $crate::lcd_text::__max_lcd_cells([$($width,)+], [$($height,)+]);

            $(
                static [<$lcd_name:upper _FRAME_SIGNAL>]:
                    $crate::lcd_text::__I2csSignal<
                        $crate::lcd_text::__LcdTextFrame<{ [<__ $group_name:upper _MAX_LCD_CELLS>] }>
                    > =
                    $crate::lcd_text::__I2csSignal::new();
            )+

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
                    spawner.spawn(token.map_err($crate::Error::TaskSpawn)?);

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

                impl $crate::lcd_text::__LcdText<$width, $height> for $lcd_name {
                    const ADDRESS: u8 = $address;

                    fn write_text(&self, text: impl AsRef<str>) {
                        ::core::assert!($width > 0, "lcd_text width must be > 0");
                        ::core::assert!($height > 0, "lcd_text height must be > 0");
                        ::core::assert!(
                            $height <= 4,
                            "lcd_text height must be <= 4 for HD44780 row map"
                        );
                        let lcd_text_frame =
                            $crate::lcd_text::__render_lcd_text_frame::<
                                $width,
                                $height,
                                { [<__ $group_name:upper _MAX_LCD_CELLS>] }
                            >(text.as_ref());
                        [<$lcd_name:upper _FRAME_SIGNAL>].signal(lcd_text_frame);
                    }
                }

                impl $lcd_name {
                    /// Display width in characters.
                    pub const WIDTH: usize = $width;
                    /// Display height in characters.
                    pub const HEIGHT: usize = $height;
                    /// LCD I2C address.
                    pub const ADDRESS: u8 = $address;

                    /// Construct this LCD text device and spawn its shared I2C task.
                    /// See the [lcd_text module documentation](mod@crate::lcd_text) for usage examples.
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

                }
            )+

            #[embassy_executor::task]
            async fn [<__i2cs_task_ $group_name:snake>](
                i2c: embassy_rp::i2c::I2c<'static, embassy_rp::peripherals::$i2c, embassy_rp::i2c::Blocking>,
            ) -> ! {
                let mut rp_lcd_text_write = $crate::lcd_text::RpLcdTextWrite::__new(i2c);
                let mut lcd_text_driver = $crate::lcd_text::__LcdTextDriver::new(0x27);
                const ADDRESS_COUNT: usize = [$($address,)+].len();
                let mut initialized_addresses: heapless::Vec<u8, ADDRESS_COUNT> = heapless::Vec::new();
                let addresses = [$($address,)+];
                let widths = [$($width,)+];
                let heights = [$($height,)+];

                loop {
                    let (lcd_text_frame, ready_index) = $crate::lcd_text::__select_array([
                        $([<$lcd_name:upper _FRAME_SIGNAL>].wait(),)+
                    ]).await;
                    $crate::lcd_text::__write_lcd_text_cells::<
                        ADDRESS_COUNT,
                        { [<__ $group_name:upper _MAX_LCD_CELLS>] }
                    >(
                        &mut lcd_text_driver,
                        &mut rp_lcd_text_write,
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
/// **See the [lcd_text module documentation](mod@crate::lcd_text) for usage
/// examples.**
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! lcd_text {
    ($($tt:tt)*) => { $crate::__lcd_text_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`lcd_text!`] instead.
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
