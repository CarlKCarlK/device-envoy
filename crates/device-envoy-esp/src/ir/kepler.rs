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
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
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
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0 _MAPPING>]::new(
                        pin,
                        channel_creator,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    Ok([<$name0:upper _KEPLER_CELL>].init(Self { mapping }))
                }
            }

            pub struct $group_name;
            impl $group_name {
                pub fn new(
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0,)> {
                    let name0 = $name0::new(pin0, channel_creator0, spawner)?;
                    Ok((name0,))
                }
            }
        }
    };
    (
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? },
            $name1:ident : { pin: $pin1:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
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
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name0 _MAPPING>]::new(
                        pin,
                        channel_creator,
                        &$crate::ir::__KEPLER_MAPPING,
                        spawner,
                    )?;
                    Ok([<$name0:upper _KEPLER_CELL>].init(Self { mapping }))
                }
            }

            impl $name1 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let mapping = [<__ $name1 _MAPPING>]::new(
                        pin,
                        channel_creator,
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
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin1: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator1: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1)> {
                    let (mapping0, mapping1) = [<__ $group_name _MAPPINGS>]::new(
                        pin0,
                        channel_creator0,
                        pin1,
                        channel_creator1,
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
        $name:ident : { pin: $pin:ident $(,)? }
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_keplers! {
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
/// Macro to generate multiple Kepler IR struct types (includes syntax details).
#[doc(inline)]
pub use ir_keplers;
