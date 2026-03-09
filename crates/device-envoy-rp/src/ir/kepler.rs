//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKepler`](trait@crate::ir::IrKepler) and this module's macros for generated types.

pub use device_envoy_core::ir::kepler::KeplerKeys;

#[doc(hidden)]
#[macro_export]
macro_rules! ir_keplers {
    ($($tt:tt)*) => { $crate::__ir_keplers_impl! { $($tt)* } };
}

/// Internal implementation helper for [`ir_keplers!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __ir_keplers_impl {
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
                [<__ $group_name:camel Mappings>] {
                    [<__ $name0:camel Mapping>]: { pin: $pin0 }
                }
            }

            impl $crate::ir::IrKepler for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                    <[<__ $name0:camel Mapping>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
                }
            }

            static [<$name0:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                mapping: &'static [<__ $name0:camel Mapping>],
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0:camel Mapping>]::new(
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
                [<__ $group_name:camel Mappings>] {
                    [<__ $name0:camel Mapping>]: { pin: $pin0 },
                    [<__ $name1:camel Mapping>]: { pin: $pin1 }
                }
            }

            static [<$name0:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _KEPLER_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                mapping: &'static [<__ $name0:camel Mapping>],
            }

            pub struct $name1 {
                mapping: &'static [<__ $name1:camel Mapping>],
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0:camel Mapping>]::new(
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
                    let mapping = [<__ $name1:camel Mapping>]::new(
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
                    <[<__ $name0:camel Mapping>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
                }
            }

            impl $crate::ir::IrKepler for $name1 {
                async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                    <[<__ $name1:camel Mapping>] as $crate::ir::IrMapping<$crate::ir::KeplerKeys>>::wait_for_press(self.mapping).await
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
                    let (mapping0, mapping1) = [<__ $group_name:camel Mappings>]::new(
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
                [<__ $name:camel Keplers>] {
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
/// Alternative macro to share one PIO resource with other Kepler IR receivers (includes syntax details).
#[doc(inline)]
pub use ir_keplers;
