//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMapping`](trait@crate::ir::IrMapping) and this module's macros for generated types.

use heapless::LinearMap;

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __build_button_map<B: Copy, const N: usize>(
    button_map: &[(u16, u8, B)],
) -> LinearMap<(u16, u8), B, N> {
    let mut linear_map = LinearMap::new();
    for &(addr, cmd, button) in button_map {
        let previous_button = match linear_map.insert((addr, cmd), button) {
            Ok(previous_button) => previous_button,
            Err(_) => panic!("button_map entries exceed IrMapping capacity"),
        };
        assert!(
            previous_button.is_none(),
            "button_map contains duplicate (addr, cmd) entries"
        );
    }
    linear_map
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir_mappings {
    ($($tt:tt)*) => { $crate::__ir_mappings_impl! { $($tt)* } };
}

/// Internal implementation helper for [`ir_mappings!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __ir_mappings_impl {
    (
        button: $button_ty:ty,
        capacity: $capacity:expr,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                [<__ $group_name _IRS>] {
                    [<__ $name0 _IR>]: { pin: $pin0 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0 _IR>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0 _IR>]::new(pin, channel_creator, spawner)?;
                    Ok([<$name0:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name0 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name0 _IR>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            pub struct $group_name;

            impl $group_name {
                pub fn new(
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    button_map0: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0,)> {
                    let name0 = $name0::new(pin0, channel_creator0, button_map0, spawner)?;
                    Ok((name0,))
                }
            }
        }
    };
    (
        button: $button_ty:ty,
        capacity: $capacity:expr,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? },
            $name1:ident : { pin: $pin1:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                [<__ $group_name _IRS>] {
                    [<__ $name0 _IR>]: { pin: $pin0 },
                    [<__ $name1 _IR>]: { pin: $pin1 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0 _IR>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name1 {
                ir: &'static [<__ $name1 _IR>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0 _IR>]::new(pin, channel_creator, spawner)?;
                    Ok([<$name0:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $name1 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name1 _IR>]::new(pin, channel_creator, spawner)?;
                    Ok([<$name1:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name0 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name0 _IR>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name1 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name1 _IR>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            pub struct $group_name;

            impl $group_name {
                pub fn new(
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin1: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator1: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    button_map0: &[(u16, u8, $button_ty)],
                    button_map1: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1)> {
                    let (ir0, ir1) = [<__ $group_name _IRS>]::new(
                        pin0,
                        channel_creator0,
                        pin1,
                        channel_creator1,
                        spawner,
                    )?;
                    let name0 = [<$name0:upper _MAPPING_CELL>].init($name0 {
                        ir: ir0,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map0,
                        ),
                    });
                    let name1 = [<$name1:upper _MAPPING_CELL>].init($name1 {
                        ir: ir1,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map1,
                        ),
                    });
                    Ok((name0, name1))
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir_mapping {
    (
        $name:ident : {
            pin: $pin:ident,
            button: $button_ty:ty,
            capacity: $capacity:expr $(,)?
        }
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
                button: $button_ty,
                capacity: $capacity,
                [<__ $name _MAPPINGS>] {
                    $name: { pin: $pin }
                }
            }
        }
    };
}

#[allow(unused_imports)]
/// Macro to generate an IR mapping struct type (includes syntax details).
#[doc(inline)]
pub use ir_mapping;
#[allow(unused_imports)]
/// Macro to generate multiple IR mapping struct types (includes syntax details).
#[doc(inline)]
pub use ir_mappings;
