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
                [<__ $group_name:camel Irs>] {
                    [<__ $name0:camel Ir>]: { pin: $pin0 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0:camel Ir>]::new(pio, pin, spawner)?;
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
                            <[<__ $name0:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                [<__ $group_name:camel Irs>] {
                    [<__ $name0:camel Ir>]: { pin: $pin0 },
                    [<__ $name1:camel Ir>]: { pin: $pin1 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name1 {
                ir: &'static [<__ $name1:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0:camel Ir>]::new(pio, pin, spawner)?;
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
                    let ir = [<__ $name1:camel Ir>]::new(pio, pin, spawner)?;
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
                            <[<__ $name0:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                            <[<__ $name1:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                    let (ir0, ir1) = [<__ $group_name:camel Irs>]::new(pio, pin0, pin1, spawner)?;
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
    (
        pio: $pio:ident,
        button: $button_ty:ty,
        capacity: $capacity:expr,
        $group_name:ident {
            $name0:ident : { pin: $pin0:ident $(,)? },
            $name1:ident : { pin: $pin1:ident $(,)? },
            $name2:ident : { pin: $pin2:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                pio: $pio,
                [<__ $group_name:camel Irs>] {
                    [<__ $name0:camel Ir>]: { pin: $pin0 },
                    [<__ $name1:camel Ir>]: { pin: $pin1 },
                    [<__ $name2:camel Ir>]: { pin: $pin2 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();
            static [<$name2:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name2> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name1 {
                ir: &'static [<__ $name1:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name2 {
                ir: &'static [<__ $name2:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0:camel Ir>]::new(pio, pin, spawner)?;
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
                    let ir = [<__ $name1:camel Ir>]::new(pio, pin, spawner)?;
                    Ok([<$name1:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $name2 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin2>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name2:camel Ir>]::new(pio, pin, spawner)?;
                    Ok([<$name2:upper _MAPPING_CELL>].init(Self {
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
                            <[<__ $name0:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                            <[<__ $name1:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name2 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name2:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                    pin2: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin2>,
                    button_map0: &[(u16, u8, $button_ty)],
                    button_map1: &[(u16, u8, $button_ty)],
                    button_map2: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1, &'static $name2)> {
                    let (ir0, ir1, ir2) = [<__ $group_name:camel Irs>]::new(
                        pio, pin0, pin1, pin2, spawner,
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
                    let name2 = [<$name2:upper _MAPPING_CELL>].init($name2 {
                        ir: ir2,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map2,
                        ),
                    });
                    Ok((name0, name1, name2))
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
            $name1:ident : { pin: $pin1:ident $(,)? },
            $name2:ident : { pin: $pin2:ident $(,)? },
            $name3:ident : { pin: $pin3:ident $(,)? }
        }
        $(,)?
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                pio: $pio,
                [<__ $group_name:camel Irs>] {
                    [<__ $name0:camel Ir>]: { pin: $pin0 },
                    [<__ $name1:camel Ir>]: { pin: $pin1 },
                    [<__ $name2:camel Ir>]: { pin: $pin2 },
                    [<__ $name3:camel Ir>]: { pin: $pin3 }
                }
            }

            static [<$name0:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name0> =
                ::static_cell::StaticCell::new();
            static [<$name1:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name1> =
                ::static_cell::StaticCell::new();
            static [<$name2:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name2> =
                ::static_cell::StaticCell::new();
            static [<$name3:upper _MAPPING_CELL>]: ::static_cell::StaticCell<$name3> =
                ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir: &'static [<__ $name0:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name1 {
                ir: &'static [<__ $name1:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name2 {
                ir: &'static [<__ $name2:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            pub struct $name3 {
                ir: &'static [<__ $name3:camel Ir>],
                button_map: ::heapless::LinearMap<(u16, u8), $button_ty, $capacity>,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name0:camel Ir>]::new(pio, pin, spawner)?;
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
                    let ir = [<__ $name1:camel Ir>]::new(pio, pin, spawner)?;
                    Ok([<$name1:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $name2 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin2>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name2:camel Ir>]::new(pio, pin, spawner)?;
                    Ok([<$name2:upper _MAPPING_CELL>].init(Self {
                        ir,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map,
                        ),
                    }))
                }
            }

            impl $name3 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin3>,
                    button_map: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let ir = [<__ $name3:camel Ir>]::new(pio, pin, spawner)?;
                    Ok([<$name3:upper _MAPPING_CELL>].init(Self {
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
                            <[<__ $name0:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                            <[<__ $name1:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name2 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name2:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
                        if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                            return button;
                        }
                    }
                }
            }

            impl $crate::ir::IrMapping<$button_ty> for $name3 {
                async fn wait_for_press(&self) -> $button_ty {
                    loop {
                        let $crate::ir::IrEvent::Press { addr, cmd } =
                            <[<__ $name3:camel Ir>] as $crate::ir::Ir>::wait_for_press(self.ir).await;
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
                    pin2: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin2>,
                    pin3: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin3>,
                    button_map0: &[(u16, u8, $button_ty)],
                    button_map1: &[(u16, u8, $button_ty)],
                    button_map2: &[(u16, u8, $button_ty)],
                    button_map3: &[(u16, u8, $button_ty)],
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1, &'static $name2, &'static $name3)> {
                    let (ir0, ir1, ir2, ir3) = [<__ $group_name:camel Irs>]::new(
                        pio, pin0, pin1, pin2, pin3, spawner,
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
                    let name2 = [<$name2:upper _MAPPING_CELL>].init($name2 {
                        ir: ir2,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map2,
                        ),
                    });
                    let name3 = [<$name3:upper _MAPPING_CELL>].init($name3 {
                        ir: ir3,
                        button_map: $crate::ir::__build_button_map::<$button_ty, $capacity>(
                            button_map3,
                        ),
                    });
                    Ok((name0, name1, name2, name3))
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
            $name1:ident : { pin: $pin1:ident $(,)? },
            $name2:ident : { pin: $pin2:ident $(,)? },
            $name3:ident : { pin: $pin3:ident $(,)? },
            $($tail_name:ident : { pin: $tail_pin:ident $(,)? }),+
        }
        $(,)?
    ) => {
        compile_error!("ir_mappings! currently supports up to 4 receivers in one group.");
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir_mapping {
    ($($tt:tt)*) => { $crate::__ir_mapping_impl! { $($tt)* } };
}

/// Internal implementation helper for [`ir_mapping!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __ir_mapping_impl {
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
                [<__ $name:camel Mappings>] {
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
/// Alternative macro to share one PIO resource with other IR mappings (includes syntax details).
#[doc(inline)]
pub use ir_mappings;
