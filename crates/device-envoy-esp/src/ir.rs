//! A device abstraction for infrared receivers using the NEC protocol.
//!
//! This page provides the primary documentation and examples for receiving NEC infrared input on ESP devices.
//! It covers raw address/command events, mapped application keys, and Kepler remote keys.
//! Traits define the shared API; macros generate concrete device types.
//! Choose [`ir!`](macro@crate::ir) for raw NEC events, [`ir_mapping!`](macro@crate::ir_mapping)
//! when mapping to your own enum, and [`ir_kepler!`](macro@crate::ir_kepler) for the
//! SunFounder Kepler remote.
//!
//! **After reading the examples below, see also:**
//!
//! - **IR: Raw events** — [`ir!`](macro@crate::ir), [`Ir`](trait@crate::ir::Ir), [`IrGenerated`](ir_generated::IrGenerated)
//! - **IrMapping: Mapped events** — [`ir_mapping!`](macro@crate::ir_mapping), [`IrMapping`](trait@crate::ir::IrMapping), [`IrMappingGenerated`](ir_generated::IrMappingGenerated)
//! - **IrKepler: Kepler mapped events** — [`ir_kepler!`](macro@crate::ir_kepler), [`IrKepler`](trait@crate::ir::IrKepler), [`IrKeplerGenerated`](ir_generated::IrKeplerGenerated)
//!
//! # Example: Read Raw NEC Events
//!
//! In this example, the generated `Ir7` type emits raw NEC press events with address and command bytes.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! use device_envoy_esp::{Result, init_and_start, init_and_start::rmt_mode, ir, ir::{Ir as _, IrEvent}};
//! # use esp_backtrace as _;
//! # use log::info;
//! #
//! ir! {
//!     Ir7 { pin: GPIO7 }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
//!     esp_println::logger::init_logger(log::LevelFilter::Info);
//!
//!     #[cfg(target_arch = "xtensa")]
//!     let channel_creator = rmt80.channel4;
//!     #[cfg(not(target_arch = "xtensa"))]
//!     let channel_creator = rmt80.channel2;
//!
//!     let ir7 = Ir7::new(p.GPIO7, channel_creator, spawner)?;
//!
//!     loop {
//!         let IrEvent::Press { addr, cmd } = ir7.wait_for_press().await;
//!         info!("IR press: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
//!     }
//! }
//! ```
//!
//! # Example: Map NEC Events to App Keys
//!
//! In this example, the generated `IrMapping7` type maps raw NEC address/command pairs into
//! an application-defined enum.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! use device_envoy_esp::{Result, init_and_start, init_and_start::rmt_mode, ir::IrMapping as _, ir_mapping};
//! # use esp_backtrace as _;
//! #
//! #[derive(Clone, Copy, Debug, Eq, PartialEq)]
//! enum RemoteKeys {
//!     Power,
//!     Plus,
//!     Minus,
//! }
//!
//! ir_mapping! {
//!     IrMapping7 {
//!         pin: GPIO7,
//!         button: RemoteKeys,
//!         capacity: 3,
//!     }
//! }
//!
//! const REMOTE_KEYS_MAP: [(u16, u8, RemoteKeys); 3] = [
//!     (0x0000, 0x45, RemoteKeys::Power),
//!     (0x0000, 0x09, RemoteKeys::Plus),
//!     (0x0000, 0x15, RemoteKeys::Minus),
//! ];
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
//!
//!     #[cfg(target_arch = "xtensa")]
//!     let channel_creator = rmt80.channel4;
//!     #[cfg(not(target_arch = "xtensa"))]
//!     let channel_creator = rmt80.channel2;
//!
//!     let ir_mapping7 = IrMapping7::new(p.GPIO7, channel_creator, &REMOTE_KEYS_MAP, spawner)?;
//!
//!     loop {
//!         let remote_key = ir_mapping7.wait_for_press().await;
//!         match remote_key {
//!             RemoteKeys::Power => {}
//!             RemoteKeys::Plus => {}
//!             RemoteKeys::Minus => {}
//!         }
//!     }
//! }
//! ```
//!
//! # Example: Read Kepler Remote Keys
//!
//! In this example, the generated `IrKepler7` type returns typed keys from the SunFounder
//! Kepler remote key mapping.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! use device_envoy_esp::{Result, init_and_start, init_and_start::rmt_mode, ir::IrKepler as _, ir::KeplerKeys, ir_kepler};
//! # use esp_backtrace as _;
//! # use log::info;
//! #
//! ir_kepler! {
//!     IrKepler7 { pin: GPIO7 }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
//!     esp_println::logger::init_logger(log::LevelFilter::Info);
//!
//!     #[cfg(target_arch = "xtensa")]
//!     let channel_creator = rmt80.channel4;
//!     #[cfg(not(target_arch = "xtensa"))]
//!     let channel_creator = rmt80.channel2;
//!
//!     let ir_kepler7 = IrKepler7::new(p.GPIO7, channel_creator, spawner)?;
//!
//!     loop {
//!         let kepler_key = ir_kepler7.wait_for_press().await;
//!         match kepler_key {
//!             KeplerKeys::Power => info!("Power"),
//!             KeplerKeys::PlayPause => info!("PlayPause"),
//!             _ => info!("Other: {:?}", kepler_key),
//!         }
//!     }
//! }
//! ```
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

pub mod ir_generated;
mod kepler;
mod mapping;

pub use device_envoy_core::ir::{Ir, IrEvent, IrKepler, IrMapping};
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::ir::IrStatic as __IrStatic;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::ir::kepler::KEPLER_MAPPING as __KEPLER_MAPPING;
// Must be `pub` for macro expansion at downstream call sites.
pub use kepler::KeplerKeys;
pub use mapping::__build_button_map;

#[cfg(target_os = "none")]
use device_envoy_core::ir::decode_nec_frame;

/// Shared IR receive loop used by macro-generated per-instance tasks.
#[doc(hidden)]
#[cfg(target_os = "none")]
pub async fn __ir_receiver_task_loop(
    mut channel: esp_hal::rmt::Channel<'static, esp_hal::Async, esp_hal::rmt::Rx>,
    ir_static: &'static __IrStatic,
) -> ! {
    let mut pulse_codes = [esp_hal::rmt::PulseCode::default(); 96];

    loop {
        for pulse_code in &mut pulse_codes {
            pulse_code.reset();
        }

        if let Ok(symbol_count) = channel.receive(&mut pulse_codes).await {
            if let Some((addr, cmd)) = decode_nec_from_pulses(&pulse_codes[..symbol_count]) {
                ir_static.send(IrEvent::Press { addr, cmd }).await;
            }
        }
    }
}

#[cfg(target_os = "none")]
fn decode_nec_from_pulses(pulse_codes: &[esp_hal::rmt::PulseCode]) -> Option<(u16, u8)> {
    use esp_hal::gpio::Level;

    let mut runs = [(Level::Low, 0u16); 256];
    let mut run_count = 0usize;

    for pulse_code in pulse_codes {
        let length1 = pulse_code.length1();
        if length1 > 0 && run_count < runs.len() {
            runs[run_count] = (pulse_code.level1(), length1);
            run_count += 1;
        }

        let length2 = pulse_code.length2();
        if length2 == 0 {
            break;
        }
        if run_count < runs.len() {
            runs[run_count] = (pulse_code.level2(), length2);
            run_count += 1;
        }
    }

    if run_count < 2 {
        return None;
    }

    if is_nec_repeat_runs(&runs[..run_count]) {
        return None;
    }

    let mut leader_index = None;
    for run_index in 0..(run_count - 1) {
        let (level0, duration0) = runs[run_index];
        let (level1, duration1) = runs[run_index + 1];
        if level0 == Level::Low
            && level1 == Level::High
            && within(duration0, 9000, 2200)
            && within(duration1, 4500, 1600)
        {
            leader_index = Some(run_index + 2);
            break;
        }
    }
    let mut run_index = leader_index?;

    let mut frame = 0u32;
    for bit_index in 0..32u32 {
        if run_index + 1 >= run_count {
            return None;
        }
        let (mark_level, mark_duration) = runs[run_index];
        let (space_level, space_duration) = runs[run_index + 1];
        run_index += 2;

        if mark_level != Level::Low || space_level != Level::High {
            return None;
        }
        if !(250..=900).contains(&mark_duration) {
            return None;
        }

        let bit_value = if (250..=900).contains(&space_duration) {
            0u32
        } else if (1200..=2200).contains(&space_duration) {
            1u32
        } else {
            return None;
        };

        frame |= bit_value << bit_index;
    }

    // TODO Handle NEC repeat frames explicitly (leader + 2.25ms + 560us pattern).
    decode_nec_frame(frame)
}

#[inline]
#[cfg(target_os = "none")]
fn within(value: u16, target: u16, tolerance: u16) -> bool {
    let min = target.saturating_sub(tolerance);
    let max = target.saturating_add(tolerance);
    (min..=max).contains(&value)
}

#[cfg(target_os = "none")]
fn is_nec_repeat_runs(runs: &[(esp_hal::gpio::Level, u16)]) -> bool {
    use esp_hal::gpio::Level;

    if runs.len() < 2 {
        return false;
    }

    let (level0, duration0) = runs[0];
    let (level1, duration1) = runs[1];

    level0 == Level::Low
        && level1 == Level::High
        && within(duration0, 9000, 2200)
        && within(duration1, 2250, 1000)
}

macro_schema::define! {
    /// Macro to generate a Kepler IR struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_keplers!`](crate::ir_keplers) — Build multiple Kepler IR receivers
    /// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
    pub ir_kepler {
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
    }

    generate {
        // A self-contained receiver: it owns its RMT channel and background task.
        static $upper($decl.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
        static $upper($decl.name, _KEPLER_CELL): ::static_cell::StaticCell<$decl.name> =
            ::static_cell::StaticCell::new();

        #[::embassy_executor::task]
        async fn $snake(__, $decl.name, _ir_receiver_task)(
            channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
            ir_static: &'static $crate::ir::__IrStatic,
        ) -> ! {
            $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            ir_static: &'static $crate::ir::__IrStatic,
            button_map: ::heapless::LinearMap<(u16, u8), $crate::ir::KeplerKeys, 21>,
        }

        impl $decl.name {
            /// Creates the Kepler receiver on an RMT channel and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_esp::ir) for usage.
            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let channel = channel_creator
                    .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                    .map_err($crate::Error::RmtConfig)?
                    .with_pin(pin);
                spawner.spawn(
                    $snake(__, $decl.name, _ir_receiver_task)(channel, &$upper($decl.name, _IR_STATIC))
                        .map_err($crate::Error::TaskSpawn)?,
                );
                Ok($upper($decl.name, _KEPLER_CELL).init(Self {
                    ir_static: &$upper($decl.name, _IR_STATIC),
                    button_map: $crate::ir::__build_button_map::<$crate::ir::KeplerKeys, 21>(
                        &$crate::ir::__KEPLER_MAPPING,
                    ),
                }))
            }
        }

        impl $crate::ir::IrKepler for $decl.name {
            async fn wait_for_press(&self) -> $crate::ir::KeplerKeys {
                loop {
                    let $crate::ir::IrEvent::Press { addr, cmd } = self.ir_static.receive().await;
                    if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                        return button;
                    }
                }
            }
        }
    }
}
macro_schema::define! {
    /// Macro to generate multiple Kepler IR struct types.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_kepler!`](crate::ir_kepler) — Generate a single Kepler IR receiver type
    /// - [`irs!`](crate::irs) — Generate raw IR receivers
    pub ir_keplers {
        /// Each member is one Kepler remote receiver on its own RMT channel.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        // Each member is a self-contained `ir_kepler!` receiver.
        $for ir in $decl.members {
            $crate::ir::ir_kepler! {
                $ir.attrs
                #[doc = $ir.doc]
                $decl.vis $ir.name { pin: $ir.pin }
            }
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every Kepler receiver in the group.
            ///
            /// Takes a pin and RMT channel per receiver in declaration order, then the spawner.
            pub fn new(
                $for ir in $decl.members {
                    $snake($ir.name, _pin): $crate::esp_hal::peripherals::$ir.pin<'static>,
                    $snake($ir.name, _channel_creator): impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                Ok(($for ir in $decl.members {
                    $ir.name::new($snake($ir.name, _pin), $snake($ir.name, _channel_creator), spawner)?,
                }))
            }
        }
    }
}
macro_schema::define! {
    /// Macro to generate an IR mapping struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_mappings!`](crate::ir_mappings) — Generate multiple mapping receivers
    /// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
    pub ir_mapping {
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
        /// Application button type that IR codes map to.
        button: ty,
        /// Maximum mapping entries; at least the number of entries you provide.
        capacity: expr,
    }

    generate {
        // A self-contained receiver: it owns its RMT channel and background task.
        static $upper($decl.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
        static $upper($decl.name, _MAPPING_CELL): ::static_cell::StaticCell<$decl.name> =
            ::static_cell::StaticCell::new();

        #[::embassy_executor::task]
        async fn $snake(__, $decl.name, _ir_receiver_task)(
            channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
            ir_static: &'static $crate::ir::__IrStatic,
        ) -> ! {
            $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            ir_static: &'static $crate::ir::__IrStatic,
            button_map: ::heapless::LinearMap<(u16, u8), $decl.button, $decl.capacity>,
        }

        impl $decl.name {
            /// Creates the mapping receiver on an RMT channel and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_esp::ir) for usage.
            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                button_map: &[(u16, u8, $decl.button)],
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let channel = channel_creator
                    .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                    .map_err($crate::Error::RmtConfig)?
                    .with_pin(pin);
                spawner.spawn(
                    $snake(__, $decl.name, _ir_receiver_task)(channel, &$upper($decl.name, _IR_STATIC))
                        .map_err($crate::Error::TaskSpawn)?,
                );
                Ok($upper($decl.name, _MAPPING_CELL).init(Self {
                    ir_static: &$upper($decl.name, _IR_STATIC),
                    button_map: $crate::ir::__build_button_map::<$decl.button, $decl.capacity>(button_map),
                }))
            }
        }

        impl $crate::ir::IrMapping<$decl.button> for $decl.name {
            async fn wait_for_press(&self) -> $decl.button {
                loop {
                    let $crate::ir::IrEvent::Press { addr, cmd } = self.ir_static.receive().await;
                    if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                        return button;
                    }
                }
            }
        }
    }
}
macro_schema::define! {
    /// Macro to generate multiple IR mapping struct types.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_mapping!`](crate::ir_mapping) — Generate a single IR mapping receiver type
    /// - [`irs!`](crate::irs) — Generate raw IR receivers
    pub ir_mappings {
        /// Application button type that IR codes map to.
        button: ty,
        /// Maximum mapping entries per receiver; at least the number of entries you provide.
        capacity: expr,
        /// Each member is one mapping receiver on its own RMT channel.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        // Each member is a self-contained `ir_mapping!` receiver.
        $for ir in $decl.members {
            $crate::ir::ir_mapping! {
                $ir.attrs
                #[doc = $ir.doc]
                $decl.vis $ir.name { pin: $ir.pin, button: $decl.button, capacity: $decl.capacity }
            }
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every mapping receiver in the group.
            ///
            /// Takes a pin, RMT channel, and button map per receiver in declaration
            /// order, then the spawner.
            pub fn new(
                $for ir in $decl.members {
                    $snake($ir.name, _pin): $crate::esp_hal::peripherals::$ir.pin<'static>,
                    $snake($ir.name, _channel_creator): impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    $snake($ir.name, _button_map): &[(u16, u8, $decl.button)],
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                Ok(($for ir in $decl.members {
                    $ir.name::new($snake($ir.name, _pin), $snake($ir.name, _channel_creator), $snake($ir.name, _button_map), spawner)?,
                }))
            }
        }
    }
}
macro_schema::define! {
    /// Macro to generate an IR receiver struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`irs!`](crate::irs) — Generate multiple IR receivers
    /// - [`ir_mapping!`](crate::ir_mapping) — Generate a mapped-button IR receiver type
    pub ir {
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
    }

    generate {
        // A self-contained receiver: it owns its RMT channel and background task.
        static $upper($decl.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
        static $upper($decl.name, _IR): $decl.name = $decl.name { ir_static: &$upper($decl.name, _IR_STATIC) };

        #[::embassy_executor::task]
        async fn $snake(__, $decl.name, _ir_receiver_task)(
            channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
            ir_static: &'static $crate::ir::__IrStatic,
        ) -> ! {
            $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            ir_static: &'static $crate::ir::__IrStatic,
        }

        impl $decl.name {
            /// Creates the IR receiver on an RMT channel and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_esp::ir) for usage.
            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let channel = channel_creator
                    .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                    .map_err($crate::Error::RmtConfig)?
                    .with_pin(pin);
                spawner.spawn(
                    $snake(__, $decl.name, _ir_receiver_task)(channel, &$upper($decl.name, _IR_STATIC))
                        .map_err($crate::Error::TaskSpawn)?,
                );
                Ok(&$upper($decl.name, _IR))
            }
        }

        impl $crate::ir::Ir for $decl.name {
            async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                self.ir_static.receive().await
            }
        }
    }
}
macro_schema::define! {
    /// Macro to generate multiple IR receiver struct types.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir!`](crate::ir!) — Generate a single IR receiver type
    /// - [`ir_mappings!`](crate::ir_mappings) — Generate mapped-button receivers
    pub irs {
        /// Each member is one IR receiver on its own RMT channel.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        // Each member is a self-contained `ir!` receiver.
        $for ir in $decl.members {
            $crate::ir::ir! {
                $ir.attrs
                #[doc = $ir.doc]
                $decl.vis $ir.name { pin: $ir.pin }
            }
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every IR receiver in the group.
            ///
            /// Takes a pin and RMT channel per receiver in declaration order, then the spawner.
            pub fn new(
                $for ir in $decl.members {
                    $snake($ir.name, _pin): $crate::esp_hal::peripherals::$ir.pin<'static>,
                    $snake($ir.name, _channel_creator): impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                Ok(($for ir in $decl.members {
                    $ir.name::new($snake($ir.name, _pin), $snake($ir.name, _channel_creator), spawner)?,
                }))
            }
        }
    }
}
