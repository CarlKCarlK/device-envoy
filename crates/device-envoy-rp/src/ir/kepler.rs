//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKeplerRp`] for usage examples.

use device_envoy_core::ir::{IrKepler, IrMapping as _};
use crate::ir::mapping::IrMappingRp;

pub use device_envoy_core::ir::kepler::KeplerKeys;

/// Type alias for the Kepler button mapping.
///
/// See [`IrKeplerRp`] for usage examples.
type IrKeplerMapping<'a> = IrMappingRp<'a, KeplerKeys, 21>;

/// A device abstraction for the SunFounder Kepler Kit IR remote.
///
/// This provides a simple interface for the Kepler remote with built-in button mappings.
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_envoy_rp::{ir::IrKepler as _, ir_kepler};
///
/// ir_kepler! {
///     IrKepler15: { pio: PIO0, pin: PIN_15 }
/// }
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     let ir_kepler15 = IrKepler15::new(p.PIO0, p.PIN_15, spawner)?;
///
///     loop {
///         let button = ir_kepler15.wait_for_press().await;
///         defmt::info!("Button: {:?}", button);
///     }
/// }
/// ```
pub struct IrKeplerRp<'a> {
    mapping: IrKeplerMapping<'a>,
}

impl IrKepler for IrKeplerRp<'_> {
    async fn wait_for_press(&self) -> KeplerKeys {
        self.mapping.wait_for_press().await
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir_keplers {
    (
        pio: $pio:ident,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
                pio: $pio,
                button: $crate::ir::KeplerKeys,
                capacity: 21,
                [<__ $group_name _MAPPINGS>] {
                    [<__ $name0 _MAPPING>]: { pin: $pin0 }
                }
            }

            impl $crate::ir::IrKepler for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                    <[<__ $name0 _MAPPING>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
                }
            }

            static [<$name0:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                mapping: &'static [<__ $name0 _MAPPING>],
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0 _MAPPING>]::new(
                        pio,
                        pin,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    Ok([<$name0:upper _KEPLER_CELL>].init(Self { mapping }))
                }
            }

            pub struct $group_name;
            impl $group_name {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin0: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0,)> {
                    let name0 = $name0::new(pio, pin0, spawner)?;
                    Ok((name0,))
                }
            }
        }
    };
    (
        pio: $pio:ident,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? },
            $name1:ident : { pin: $pin1:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
                pio: $pio,
                button: $crate::ir::KeplerKeys,
                capacity: 21,
                [<__ $group_name _MAPPINGS>] {
                    [<__ $name0 _MAPPING>]: { pin: $pin0 },
                    [<__ $name1 _MAPPING>]: { pin: $pin1 }
                }
            }

            static [<$name0:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                mapping: &'static [<__ $name0 _MAPPING>],
            }

            pub struct $name1 {
                mapping: &'static [<__ $name1 _MAPPING>],
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0 _MAPPING>]::new(
                        pio,
                        pin,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    Ok([<$name0:upper _KEPLER_CELL>].init(Self { mapping }))
                }
            }

            impl $name1 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name1 _MAPPING>]::new(
                        pio,
                        pin,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    Ok([<$name1:upper _KEPLER_CELL>].init(Self { mapping }))
                }
            }

            impl $crate::ir::IrKepler for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                    <[<__ $name0 _MAPPING>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
                }
            }

            impl $crate::ir::IrKepler for $name1 {
                async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                    <[<__ $name1 _MAPPING>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
                }
            }

            pub struct $group_name;
            impl $group_name {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin0: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    pin1: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1)> {
                    let (mapping0, mapping1) = [<__ $group_name _MAPPINGS>]::new(
                        pio,
                        pin0,
                        pin1,
                        &$crate::ir::__KEPLER_MAPPING,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    let name0 = [<$name0:upper _KEPLER_CELL>].init($name0 { mapping: mapping0 });
                    let name1 = [<$name1:upper _KEPLER_CELL>].init($name1 { mapping: mapping1 });
                    Ok((name0, name1))
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir_kepler {
    (
        $name:ident : { pio: $pio:ident, pin: $pin:ident $(,)? }
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_keplers! {
                pio: $pio,
                [<__ $name _KEPLERS>] {
                    $name: { pin: $pin }
                }
            }
        }
    };
}

#[allow(unused_imports)]
/// Macro to generate a Kepler IR struct type (includes syntax details).
#[doc(inline)]
pub use ir_kepler;
#[allow(unused_imports)]
/// Alternative macro to share one PIO resource with other Kepler IR receivers (includes examples).
#[doc(inline)]
pub use ir_keplers;
