//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMappingRp`] for usage examples.

use heapless::LinearMap;

use crate::ir::{IrMapping, IrMappingAdapter, IrRp};

pub use device_envoy_core::ir::mapping::IrMappingStatic;

/// A generic device abstraction that maps IR remote button presses to user-defined button types.
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::ir::{IrMapping as _};
/// use device_envoy_rp::ir_mapping;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
/// #[derive(Debug, Clone, Copy)]
/// enum RemoteButton { Power, Play, Stop }
///
/// ir_mapping! {
///     IrMapping00: {
///         pio: PIO0,
///         pin: PIN_15,
///         button: RemoteButton,
///         capacity: 3,
///     }
/// }
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     let button_map = [
///         (0x0000, 0x45, RemoteButton::Power),
///         (0x0000, 0x0C, RemoteButton::Play),
///         (0x0000, 0x08, RemoteButton::Stop),
///     ];
///
///     let ir_mapping00 = IrMapping00::new(p.PIO0, p.PIN_15, &button_map, spawner)?;
///
///     loop {
///         let button = ir_mapping00.wait_for_press().await;
///         // Use button...
///     }
/// }
/// ```
pub struct IrMappingRp<'a, B, const N: usize> {
    mapping: IrMappingAdapter<IrRp<'a>, B, N>,
}

impl<B, const N: usize> IrMapping<B> for IrMappingRp<'_, B, N>
where
    B: Copy,
{
    async fn wait_for_press(&self) -> B {
        self.mapping.wait_for_press().await
    }
}

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

/// Generate one or more typed IR-mapping constructors sharing one PIO resource.
///
/// This macro is built on top of [`irs!`](macro@crate::irs).
#[doc(hidden)]
#[macro_export]
macro_rules! ir_mappings {
    (
        pio: $pio:ident,
        button: $button_ty:ty,
        capacity: $capacity:expr,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                pio: $pio,
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0 _IR>]::new(pio, pin, spawner)?;
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin0: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map0: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0,)> {
                    let name0 = $name0::new(pio, pin0, button_map0, spawner)?;
                    Ok((name0,))
                }
            }
        }
    };
    (
        pio: $pio:ident,
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
                pio: $pio,
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0 _IR>]::new(pio, pin, spawner)?;
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name1 _IR>]::new(pio, pin, spawner)?;
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin0: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    pin1: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    button_map0: &[(u16, u8, $button_ty)],
                    button_map1: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1)> {
                    let (ir0, ir1) = [<__ $group_name _IRS>]::new(pio, pin0, pin1, spawner)?;
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

/// Generate one typed IR-mapping constructor.
#[doc(hidden)]
#[macro_export]
macro_rules! ir_mapping {
    (
        $name:ident : {
            pio: $pio:ident,
            pin: $pin:ident,
            button: $button_ty:ty,
            capacity: $capacity:expr $(,)?
        }
    ) => {
        $crate::ir::paste::paste! {
            $crate::ir_mappings! {
                pio: $pio,
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
/// Macro to generate one IR-mapping type.
///
/// Use this when you need one mapped IR receiver.
#[doc(inline)]
pub use ir_mapping;
#[allow(unused_imports)]
/// Macro to generate multiple IR-mapping types on one PIO resource.
///
/// Use this when you need multiple mapped receivers sharing one PIO resource.
#[doc(inline)]
pub use ir_mappings;
