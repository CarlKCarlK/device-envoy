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
//!     LcdTextSimple {
//!         i2c: I2C0,
//!         sda_pin: PIN_4,
//!         scl_pin: PIN_5,
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
//!     I2cs0 {
//!         i2c: I2C0,
//!         sda_pin: PIN_4,
//!         scl_pin: PIN_5,
//!
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

use device_envoy_core::lcd_text::LcdTextWrite;

pub mod lcd_text_generated;
pub use device_envoy_core::lcd_text::LcdText;

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::lcd_text::{
    __I2csSignal, __LcdText, __LcdTextFrame, __assert_unique_addresses, __max_lcd_cells,
    __run_i2cs, __write_text,
};

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

macro_schema::define! {
    /// Macro to generate multiple LCD text device types that share one I2C
    /// resource.
    ///
    /// This page provides the primary documentation and examples for grouped LCD
    /// text devices that share one I2C peripheral.
    ///
    /// For a single LCD type, see [`lcd_text!`](macro@crate::lcd_text).
    ///
    /// **See the [lcd_text module documentation](mod@crate::lcd_text) for usage
    /// examples.**
    #[cfg(not(feature = "host"))]
    pub i2cs {
        /// I2C peripheral, for example `I2C0`.
        i2c: ident,
        /// GPIO pin for I2C data (SDA).
        sda_pin: ident,
        /// GPIO pin for I2C clock (SCL).
        scl_pin: ident,
        /// Each member is one LCD text display on the shared bus.
        members 1.. {
            /// Display width in characters.
            width: expr,
            /// Display height in characters; at most 4.
            height: expr,
            /// I2C address, for example `0x27`; must be unique within the group.
            address: expr,
        },
    }

    generate {
        const _: () = {
            $crate::lcd_text::__assert_unique_addresses([$for lcd in $decl.members { $lcd.address, }]);
        };
        const $upper(__, $decl.name, _MAX_LCD_CELLS): usize =
            $crate::lcd_text::__max_lcd_cells([$for lcd in $decl.members { $lcd.width, }], [$for lcd in $decl.members { $lcd.height, }]);

        $for lcd in $decl.members {
            static $upper($lcd.name, _FRAME_SIGNAL):
                $crate::lcd_text::__I2csSignal<
                    $crate::lcd_text::__LcdTextFrame<{ $upper(__, $decl.name, _MAX_LCD_CELLS) }>
                > =
                $crate::lcd_text::__I2csSignal::new();
        }

        $decl.attrs
        #[doc = $decl.doc]
        #[doc = "\n\nA group of LCD text devices that share one I2C peripheral and pin pair."]
        $decl.vis struct $decl.name;

        struct $ident(__, $decl.name, Devices) {
            $for lcd in $decl.members {
                $snake($lcd.name): &'static $lcd.name,
            }
        }

        impl $ident(__, $decl.name, Devices) {
            fn into_tuple(self) -> ($for lcd in $decl.members {&'static $lcd.name,}) {
                (
                    $for lcd in $decl.members {self.$snake($lcd.name),}
                )
            }
        }

        impl $decl.name {
            fn __new_devices(
                i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.i2c>,
                sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.sda_pin>,
                scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.scl_pin>,
                spawner: embassy_executor::Spawner,
            ) -> $crate::Result<$ident(__, $decl.name, Devices)> {
                let i2c = embassy_rp::i2c::I2c::new_blocking(
                    i2c_peripheral,
                    scl,
                    sda,
                    embassy_rp::i2c::Config::default(),
                );

                let token = $snake(__i2cs_task_, $decl.name)(i2c);
                spawner.spawn(token.map_err($crate::Error::TaskSpawn)?);

                $for lcd in $decl.members {
                    static $upper($lcd.name, _INSTANCE): $lcd.name = $lcd.name;
                    let $snake($lcd.name) = &$upper($lcd.name, _INSTANCE);
                }

                Ok($ident(__, $decl.name, Devices) {
                    $for lcd in $decl.members {
                        $snake($lcd.name),
                    }
                })
            }

            #[doc = "Construct the shared I2C runtime task and all generated LCD text device handles in this group."]
            pub fn new(
                i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.i2c>,
                sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.sda_pin>,
                scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.scl_pin>,
                spawner: embassy_executor::Spawner,
            ) -> $crate::Result<($for lcd in $decl.members {&'static $lcd.name,})> {
                Ok(Self::__new_devices(i2c_peripheral, sda, scl, spawner)?.into_tuple())
            }
        }

        $for lcd in $decl.members {
            $lcd.attrs
            #[doc = $lcd.doc]
            $lcd.vis struct $lcd.name;

            impl $crate::lcd_text::__LcdText<$lcd.width, $lcd.height> for $lcd.name {
                const ADDRESS: u8 = $lcd.address;

                fn write_text(&self, text: impl AsRef<str>) {
                    $crate::lcd_text::__write_text::<
                        { $lcd.width },
                        { $lcd.height },
                        { $upper(__, $decl.name, _MAX_LCD_CELLS) },
                    >(&$upper($lcd.name, _FRAME_SIGNAL), text.as_ref());
                }
            }

            impl $lcd.name {
                /// Display width in characters.
                pub const WIDTH: usize = $lcd.width;
                /// Display height in characters.
                pub const HEIGHT: usize = $lcd.height;
                /// LCD I2C address.
                pub const ADDRESS: u8 = $lcd.address;

                /// Construct this LCD text device and spawn its shared I2C task.
                /// See the [lcd_text module documentation](mod@device_envoy_rp::lcd_text) for usage examples.
                pub fn new(
                    i2c_peripheral: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.i2c>,
                    sda: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.sda_pin>,
                    scl: embassy_rp::Peri<'static, embassy_rp::peripherals::$decl.scl_pin>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let $snake(__, $decl.name, _devices) =
                        $decl.name::__new_devices(i2c_peripheral, sda, scl, spawner)?;
                    Ok($snake(__, $decl.name, _devices).$snake($lcd.name))
                }

            }
        }

        #[embassy_executor::task]
        async fn $snake(__i2cs_task_, $decl.name)(
            i2c: embassy_rp::i2c::I2c<'static, embassy_rp::peripherals::$decl.i2c, embassy_rp::i2c::Blocking>,
        ) -> ! {
            $crate::lcd_text::__run_i2cs(
                $crate::lcd_text::RpLcdTextWrite::__new(i2c),
                [$for lcd in $decl.members { &$upper($lcd.name, _FRAME_SIGNAL), }],
                [$for lcd in $decl.members { $lcd.address, }],
                [$for lcd in $decl.members { $lcd.width, }],
                [$for lcd in $decl.members { $lcd.height, }],
            )
            .await
        }
    }
}

macro_schema::define! {
    /// Macro to generate a single LCD text device type with a direct constructor.
    ///
    /// For multiple LCD types sharing one I2C peripheral, see
    /// [`i2cs!`](macro@crate::i2cs).
    ///
    /// **See the [lcd_text module documentation](mod@crate::lcd_text) for usage
    /// examples.**
    #[cfg(not(feature = "host"))]
    pub lcd_text {
        /// I2C peripheral, for example `I2C0`.
        i2c: ident,
        /// GPIO pin for I2C data (SDA).
        sda_pin: ident,
        /// GPIO pin for I2C clock (SCL).
        scl_pin: ident,
        /// Display width in characters.
        width: expr,
        /// Display height in characters; at most 4.
        height: expr,
        /// I2C address, for example `0x27`; must be unique within the group.
        address: expr,
    }

    generate {
        // A one-member `i2cs!` group; the group stays out of the docs.
        $crate::lcd_text::i2cs! {
            #[doc(hidden)]
            $decl.vis $ident($decl.name, Group) {
                i2c: $decl.i2c,
                sda_pin: $decl.sda_pin,
                scl_pin: $decl.scl_pin,

                $decl.attrs
                #[doc = $decl.doc]
                $decl.name {
                    width: $decl.width,
                    height: $decl.height,
                    address: $decl.address,
                },
            }
        }
    }
}
