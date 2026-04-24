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
#[doc(hidden)]
pub use paste;

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

#[doc(hidden)]
#[macro_export]
macro_rules! irs {
    (
        $group_name:ident {
            $first_name:ident : { pin: $first_pin:ident $(,)? }
            $(, $rest_name:ident : { pin: $rest_pin:ident $(,)? })* $(,)?
        }
    ) => {
        $crate::__irs_impl! {
            $group_name,
            [($first_name, $first_pin) $(, ($rest_name, $rest_pin))*]
        }
    };
}

/// Internal implementation helper for [`irs!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __irs_impl {
    (
        $group_name:ident,
        [($name0:ident, $pin0:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name0:upper _IR>]: $name0 = $name0 { ir_static: &[<$name0:upper _IR_STATIC>] };

            #[embassy_executor::task]
            async fn [<__ $name0:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            pub struct $name0 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name0:lower _ir_receiver_task>](channel, &[<$name0:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name0:upper _IR>])
                }
            }

            impl $crate::ir::Ir for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
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
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name1:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();

            static [<$name0:upper _IR>]: $name0 = $name0 { ir_static: &[<$name0:upper _IR_STATIC>] };
            static [<$name1:upper _IR>]: $name1 = $name1 { ir_static: &[<$name1:upper _IR_STATIC>] };

            #[embassy_executor::task]
            async fn [<__ $name0:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name1:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            pub struct $name0 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name1 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name0:lower _ir_receiver_task>](channel, &[<$name0:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name0:upper _IR>])
                }
            }

            impl $name1 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name1:lower _ir_receiver_task>](channel, &[<$name1:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name1:upper _IR>])
                }
            }

            impl $crate::ir::Ir for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name1 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
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
                    let name0 = $name0::new(pin0, channel_creator0, spawner)?;
                    let name1 = $name1::new(pin1, channel_creator1, spawner)?;
                    Ok((name0, name1))
                }
            }
        }
    };
    (
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident), ($name2:ident, $pin2:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name1:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name2:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();

            static [<$name0:upper _IR>]: $name0 = $name0 { ir_static: &[<$name0:upper _IR_STATIC>] };
            static [<$name1:upper _IR>]: $name1 = $name1 { ir_static: &[<$name1:upper _IR_STATIC>] };
            static [<$name2:upper _IR>]: $name2 = $name2 { ir_static: &[<$name2:upper _IR_STATIC>] };

            #[embassy_executor::task]
            async fn [<__ $name0:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name1:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name2:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            pub struct $name0 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name1 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name2 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name0:lower _ir_receiver_task>](channel, &[<$name0:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name0:upper _IR>])
                }
            }

            impl $name1 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name1:lower _ir_receiver_task>](channel, &[<$name1:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name1:upper _IR>])
                }
            }

            impl $name2 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin2<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name2:lower _ir_receiver_task>](channel, &[<$name2:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name2:upper _IR>])
                }
            }

            impl $crate::ir::Ir for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name1 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name2 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            pub struct $group_name;
            impl $group_name {
                pub fn new(
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin1: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator1: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin2: $crate::esp_hal::peripherals::$pin2<'static>,
                    channel_creator2: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1, &'static $name2)> {
                    let name0 = $name0::new(pin0, channel_creator0, spawner)?;
                    let name1 = $name1::new(pin1, channel_creator1, spawner)?;
                    let name2 = $name2::new(pin2, channel_creator2, spawner)?;
                    Ok((name0, name1, name2))
                }
            }
        }
    };
    (
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident), ($name2:ident, $pin2:ident), ($name3:ident, $pin3:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name1:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name2:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static [<$name3:upper _IR_STATIC>]: $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();

            static [<$name0:upper _IR>]: $name0 = $name0 { ir_static: &[<$name0:upper _IR_STATIC>] };
            static [<$name1:upper _IR>]: $name1 = $name1 { ir_static: &[<$name1:upper _IR_STATIC>] };
            static [<$name2:upper _IR>]: $name2 = $name2 { ir_static: &[<$name2:upper _IR_STATIC>] };
            static [<$name3:upper _IR>]: $name3 = $name3 { ir_static: &[<$name3:upper _IR_STATIC>] };

            #[embassy_executor::task]
            async fn [<__ $name0:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name1:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name2:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            #[embassy_executor::task]
            async fn [<__ $name3:lower _ir_receiver_task>](
                channel: $crate::esp_hal::rmt::Channel<'static, $crate::esp_hal::Async, $crate::esp_hal::rmt::Rx>,
                ir_static: &'static $crate::ir::__IrStatic,
            ) -> ! {
                $crate::ir::__ir_receiver_task_loop(channel, ir_static).await
            }

            pub struct $name0 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name1 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name2 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            pub struct $name3 {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name0:lower _ir_receiver_task>](channel, &[<$name0:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name0:upper _IR>])
                }
            }

            impl $name1 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name1:lower _ir_receiver_task>](channel, &[<$name1:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name1:upper _IR>])
                }
            }

            impl $name2 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin2<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name2:lower _ir_receiver_task>](channel, &[<$name2:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name2:upper _IR>])
                }
            }

            impl $name3 {
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin3<'static>,
                    channel_creator: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let channel = channel_creator
                        .configure_rx(&$crate::init_and_start::rmt::nec_rx_config())
                        .map_err($crate::Error::RmtConfig)?
                        .with_pin(pin);
                    spawner
                        .spawn([<__ $name3:lower _ir_receiver_task>](channel, &[<$name3:upper _IR_STATIC>]).map_err($crate::Error::TaskSpawn)?);
                    Ok(&[<$name3:upper _IR>])
                }
            }

            impl $crate::ir::Ir for $name0 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name1 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name2 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            impl $crate::ir::Ir for $name3 {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }

            pub struct $group_name;
            impl $group_name {
                pub fn new(
                    pin0: $crate::esp_hal::peripherals::$pin0<'static>,
                    channel_creator0: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin1: $crate::esp_hal::peripherals::$pin1<'static>,
                    channel_creator1: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin2: $crate::esp_hal::peripherals::$pin2<'static>,
                    channel_creator2: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    pin3: $crate::esp_hal::peripherals::$pin3<'static>,
                    channel_creator3: impl $crate::esp_hal::rmt::RxChannelCreator<'static, $crate::esp_hal::Async>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1, &'static $name2, &'static $name3)> {
                    let name0 = $name0::new(pin0, channel_creator0, spawner)?;
                    let name1 = $name1::new(pin1, channel_creator1, spawner)?;
                    let name2 = $name2::new(pin2, channel_creator2, spawner)?;
                    let name3 = $name3::new(pin3, channel_creator3, spawner)?;
                    Ok((name0, name1, name2, name3))
                }
            }
        }
    };
    (
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident), ($name2:ident, $pin2:ident), ($name3:ident, $pin3:ident), ($($tail:tt)+)]
    ) => {
        compile_error!("irs! currently supports up to 4 receivers in one group.");
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ir {
    (
        $(#[$attrs:meta])*
        $vis:vis $name:ident : { $($deprecated_fields:tt)* }
    ) => {
        compile_error!(
            "ir! no longer supports `Name: { ... }`. Use `Name { ... }` instead."
        );
    };
    (
        $(#[$attrs:meta])*
        $vis:vis $name:ident { pin: $pin:ident $(,)? }
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                [<__ $name:camel Group>] {
                    [<__ $name:camel Ir>]: { pin: $pin }
                }
            }

            $(#[$attrs])*
            $vis type $name = [<__ $name:camel Ir>];
        }
    };
}

/// Macro to generate a Kepler IR struct type (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// ir_kepler! {
///     [attrs...]
///     [vis?] <Name> {
///         pin: <pin_ident>,
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO input pin connected to the IR receiver
///
/// # Related Macros
///
/// - [`ir_keplers!`](crate::ir_keplers) — Build multiple Kepler IR receivers
/// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_kepler;
/// Macro to generate multiple Kepler IR struct types (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// ir_keplers! {
///     <GroupName> {
///         <Name0>: { pin: <pin0_ident> },
///         <Name1>: { pin: <pin1_ident> }, // optional
///         <Name2>: { pin: <pin2_ident> }, // optional
///         <Name3>: { pin: <pin3_ident> }, // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — One pin entry per generated Kepler receiver
///
/// Supports one to four generated receivers per invocation.
///
/// # Related Macros
///
/// - [`ir_kepler!`](crate::ir_kepler) — Generate a single Kepler IR receiver type
/// - [`irs!`](crate::irs) — Generate raw IR receivers
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_keplers;
/// Macro to generate an IR mapping struct type (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// ir_mapping! {
///     [attrs...]
///     [vis?] <Name> {
///         pin: <pin_ident>,
///         button: <button_type>,
///         capacity: <usize_expr>,
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO input pin connected to the IR receiver
/// - `button` — Output button/key type for mapping
/// - `capacity` — Maximum mapping entries (`heapless::LinearMap` capacity)
///
/// # Related Macros
///
/// - [`ir_mappings!`](crate::ir_mappings) — Generate multiple mapping receivers
/// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_mapping;
/// Macro to generate multiple IR mapping struct types (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// ir_mappings! {
///     button: <button_type>,
///     capacity: <usize_expr>,
///     <GroupName> {
///         <Name0>: { pin: <pin0_ident> },
///         <Name1>: { pin: <pin1_ident> }, // optional
///         <Name2>: { pin: <pin2_ident> }, // optional
///         <Name3>: { pin: <pin3_ident> }, // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `button` — Output button/key type for all generated mappings
/// - `capacity` — Maximum mapping entries (`heapless::LinearMap` capacity)
/// - `pin` — One pin entry per generated mapping receiver
///
/// Supports one to four generated mapping receivers per invocation.
///
/// # Related Macros
///
/// - [`ir_mapping!`](crate::ir_mapping) — Generate a single IR mapping receiver type
/// - [`irs!`](crate::irs) — Generate raw IR receivers
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_mappings;
/// Macro to generate an IR receiver struct type (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// ir! {
///     [attrs...]
///     [vis?] <Name> {
///         pin: <pin_ident>,
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO input pin connected to the IR receiver
///
/// # Related Macros
///
/// - [`irs!`](crate::irs) — Generate multiple IR receivers
/// - [`ir_mapping!`](crate::ir_mapping) — Generate a mapped-button IR receiver type
#[allow(unused_imports)]
#[doc(inline)]
pub use ir;
/// Macro to generate multiple IR receiver struct types (includes syntax details).
///
/// **See the [ir module documentation](mod@crate::ir) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// irs! {
///     <GroupName> {
///         <Name0>: { pin: <pin0_ident> },
///         <Name1>: { pin: <pin1_ident> }, // optional
///         <Name2>: { pin: <pin2_ident> }, // optional
///         <Name3>: { pin: <pin3_ident> }, // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — One pin entry per generated receiver
///
/// Supports one to four generated receivers per invocation.
///
/// # Related Macros
///
/// - [`ir!`](crate::ir!) — Generate a single IR receiver type
/// - [`ir_mappings!`](crate::ir_mappings) — Generate mapped-button receivers
#[allow(unused_imports)]
#[doc(inline)]
pub use irs;
