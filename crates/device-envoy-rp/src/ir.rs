//! A device abstraction for infrared receivers using the NEC protocol.
//!
//! This page provides the primary documentation and examples for receiving NEC infrared input on RP devices.
//! It covers raw address/command events, mapped application keys, and Kepler remote keys.
//! Traits define the shared API; macros generate concrete device types.
//! Choose [`ir!`](macro@crate::ir) for raw NEC events, [`ir_mapping!`](macro@crate::ir_mapping)
//! when mapping to your own enum, and [`ir_kepler!`](macro@crate::ir_kepler) for the
//! SunFounder Kepler remote.
//!
//! **After reading the examples below, see also:**
//!
//! - **Ir: Raw events** — [`ir!`](macro@crate::ir), [`Ir`](trait@crate::ir::Ir), [`IrGenerated`](ir_generated::IrGenerated)
//! - **IrMapping: Mapped events** — [`ir_mapping!`](macro@crate::ir_mapping), [`IrMapping`](trait@crate::ir::IrMapping), [`IrMappingGenerated`](ir_generated::IrMappingGenerated)
//! - **IrKepler: Kepler mapped events** — [`ir_kepler!`](macro@crate::ir_kepler), [`IrKepler`](trait@crate::ir::IrKepler), [`IrKeplerGenerated`](ir_generated::IrKeplerGenerated)

//!
//! # Example: Read Raw NEC Events
//!
//! In this example, the generated `Ir15` type emits raw NEC press events with address and command bytes.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::{convert::Infallible, future::pending};
//! use device_envoy_rp::{Result, ir, ir::Ir as _, ir::IrEvent};
//! # use panic_probe as _;
//! # use defmt::info;
//! #
//! ir! {
//!     Ir15 { pio: PIO0, pin: PIN_15 }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let ir15 = Ir15::new(p.PIO0, p.PIN_15, spawner)?;
//!
//!     loop {
//!         let IrEvent::Press { addr, cmd } = ir15.wait_for_press().await;
//!         info!("IR press: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
//!     }
//! }
//! ```
//!
//! # Example: Map NEC Events to App Keys
//!
//! In this example, the generated `IrMapping15` type maps raw NEC address/command pairs into
//! an application-defined enum.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::{convert::Infallible, future::pending};
//! use device_envoy_rp::{Result, ir::IrMapping as _, ir_mapping};
//! # use panic_probe as _;
//! #
//! #[derive(Clone, Copy, Debug, Eq, PartialEq)]
//! enum RemoteKeys {
//!     Power,
//!     Plus,
//!     Minus,
//! }
//!
//! ir_mapping! {
//!     IrMapping15 {
//!         pio: PIO0,
//!         pin: PIN_15,
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
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let ir_mapping15 = IrMapping15::new(p.PIO0, p.PIN_15, &REMOTE_KEYS_MAP, spawner)?;
//!
//!     loop {
//!         let remote_key = ir_mapping15.wait_for_press().await;
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
//! In this example, the generated `IrKepler15` type returns typed keys from the SunFounder
//! Kepler remote key mapping.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::{convert::Infallible, future::pending};
//! use device_envoy_rp::{Result, ir::IrKepler as _, ir::KeplerKeys, ir_kepler};
//! # use panic_probe as _;
//! # use defmt::info;
//! #
//! ir_kepler! {
//!     IrKepler15 { pio: PIO0, pin: PIN_15 }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let ir_kepler15 = IrKepler15::new(p.PIO0, p.PIN_15, spawner)?;
//!
//!     loop {
//!         let kepler_key = ir_kepler15.wait_for_press().await;
//!         match kepler_key {
//!             KeplerKeys::Power => info!("Power"),
//!             KeplerKeys::PlayPause => info!("PlayPause"),
//!             _ => info!("Other: {:?}", kepler_key),
//!         }
//!     }
//! }
//! ```
//!
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Pin, Pull};
use embassy_rp::pio::{
    Common, Config, FifoJoin, Instance, PioPin, ShiftConfig, ShiftDirection, StateMachine,
};
use fixed::traits::ToFixed;

use crate::{Error, Result};

use device_envoy_core::ir::IrStatic;
use device_envoy_core::ir::decode_nec_frame;
pub use device_envoy_core::ir::{Ir, IrEvent, IrKepler, IrMapping};
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use paste;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::ir::IrStatic as __IrStatic;
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::ir::kepler::KEPLER_MAPPING as __KEPLER_MAPPING;

// ============================================================================
// Submodules
// ============================================================================

pub mod ir_generated;
mod kepler;
mod mapping;

pub use kepler::KeplerKeys;
pub use mapping::__build_button_map;

// ===== NEC Receiver (forward declaration) ==================================

/// NEC IR receiver using PIO
#[doc(hidden)] // Internal helper type; not part of public API
pub struct NecReceiver<'d, PIO: Instance, const SM: usize> {
    sm: StateMachine<'d, PIO, SM>,
}

// ===== PIO Trait and Implementations =======================================

/// Trait for PIO peripherals used with IR receivers.
///
/// This trait associates each PIO peripheral with its interrupt bindings.
#[doc(hidden)]
pub trait IrPioPeripheral: crate::pio_irqs::PioIrqMap {
    /// Spawn SM0 receive task for this PIO.
    fn spawn_task_sm0(
        receiver: NecReceiver<'static, Self, 0>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()>;

    /// Spawn SM1 receive task for this PIO.
    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()>;

    /// Spawn SM2 receive task for this PIO.
    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()>;

    /// Spawn SM3 receive task for this PIO.
    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()>;
}

impl IrPioPeripheral for embassy_rp::peripherals::PIO0 {
    fn spawn_task_sm0(
        receiver: NecReceiver<'static, Self, 0>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm0_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm1_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm2_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm3_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }
}

impl IrPioPeripheral for embassy_rp::peripherals::PIO1 {
    fn spawn_task_sm0(
        receiver: NecReceiver<'static, Self, 0>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm0_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm1_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm2_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm3_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }
}

#[cfg(feature = "pico2")]
impl IrPioPeripheral for embassy_rp::peripherals::PIO2 {
    fn spawn_task_sm0(
        receiver: NecReceiver<'static, Self, 0>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm0_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm1_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm2_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm3_task(receiver, ir_static);
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        Ok(())
    }
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __new_receiver<P, PIO, const SM: usize>(
    common: &mut Common<'static, PIO>,
    sm: StateMachine<'static, PIO, SM>,
    pin: Peri<'static, P>,
) -> NecReceiver<'static, PIO, SM>
where
    P: Pin + PioPin,
    PIO: Instance,
{
    let mut ir_pin = common.make_pio_pin(pin);
    // IR receivers idle HIGH and pull LOW when carrier is detected.
    ir_pin.set_pull(Pull::Up);
    NecReceiver::new(common, sm, ir_pin)
}

const_structures::define! {
    /// Macro to generate an IR receiver struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`irs!`](crate::irs) — Share one PIO resource with multiple IR receivers
    /// - [`ir_mapping!`](crate::ir_mapping) — Generate a mapped-button IR receiver type
    pub ir {
        /// PIO resource, for example `PIO0`.
        pio: ident,
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
    }

    generate {
        // A one-member `irs!` group; the group stays out of the docs.
        $crate::ir::irs! {
            #[doc(hidden)]
            $decl.vis $ident($decl.name, Group) {
                pio: $decl.pio,

                $decl.attrs
                #[doc = $decl.doc]
                $decl.name { pin: $decl.pin },
            }
        }

        impl $decl.name {
            /// Creates the receiver and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_rp::ir) for usage.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pin>>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let (ir,) = $ident($decl.name, Group)::new(pio, pin, spawner)?;
                Ok(ir)
            }
        }
    }
}
const_structures::define! {
    /// Macro to generate a Kepler IR struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_keplers!`](crate::ir_keplers) — Share one PIO resource with multiple Kepler IR receivers
    /// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
    pub ir_kepler {
        /// PIO resource, for example `PIO0`.
        pio: ident,
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
    }

    generate {
        // A one-member `ir_keplers!` group; the group stays out of the docs.
        $crate::ir::ir_keplers! {
            #[doc(hidden)]
            $decl.vis $ident($decl.name, Group) {
                pio: $decl.pio,

                $decl.attrs
                #[doc = $decl.doc]
                $decl.name { pin: $decl.pin },
            }
        }

        impl $decl.name {
            /// Creates the receiver and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_rp::ir) for usage.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pin>>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let (ir_kepler,) = $ident($decl.name, Group)::new(pio, pin, spawner)?;
                Ok(ir_kepler)
            }
        }
    }
}
const_structures::define! {
    /// Macro to generate multiple Kepler IR struct types that share one PIO resource.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_kepler!`](crate::ir_kepler) — Generate a single Kepler IR receiver type
    /// - [`irs!`](crate::irs) — Generate raw IR receivers sharing one PIO resource
    pub ir_keplers {
        /// PIO resource shared by every receiver in the group, for example `PIO0`.
        pio: ident,
        /// Each member is one Kepler remote receiver and uses one PIO state machine.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        $for ir in $decl.members {
            static $upper($ir.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static $upper($ir.name, _KEPLER_CELL): ::static_cell::StaticCell<$ir.name> =
                ::static_cell::StaticCell::new();

            $ir.attrs
            #[doc = $ir.doc]
            $decl.vis struct $ir.name {
                ir_static: &'static $crate::ir::__IrStatic,
                button_map: ::heapless::LinearMap<(u16, u8), $crate::ir::KeplerKeys, 21>,
            }

            impl $crate::ir::IrKepler for $ir.name {
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

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every Kepler receiver in the group and spawns their background tasks.
            ///
            /// Takes the PIO resource, then one pin per receiver in declaration order,
            /// then the spawner.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                $for ir in $decl.members {
                    $snake($ir.name, _pin): impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin>>,
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                let ::embassy_rp::pio::Pio { mut common, $for ir in $decl.members { $ident(sm, $ir.index), } .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio.into(),
                        <::embassy_rp::peripherals::$decl.pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                $for ir in $decl.members {
                    let pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin> =
                        $snake($ir.name, _pin).into();
                    let receiver = $crate::ir::__new_receiver(&mut common, $ident(sm, $ir.index), pin);
                    <::embassy_rp::peripherals::$decl.pio as $crate::ir::IrPioPeripheral>::$ident(spawn_task_sm, $ir.index)(
                        receiver,
                        &$upper($ir.name, _IR_STATIC),
                        spawner,
                    )?;
                }
                Ok(($for ir in $decl.members {
                    &*$upper($ir.name, _KEPLER_CELL).init($ir.name {
                        ir_static: &$upper($ir.name, _IR_STATIC),
                        button_map: $crate::ir::__build_button_map::<$crate::ir::KeplerKeys, 21>(&$crate::ir::__KEPLER_MAPPING),
                    }),
                }))
            }
        }
    }
}
const_structures::define! {
    /// Macro to generate an IR mapping struct type.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_mappings!`](crate::ir_mappings) — Share one PIO resource with multiple mapping receivers
    /// - [`ir!`](crate::ir!) — Generate a raw IR receiver type
    pub ir_mapping {
        /// PIO resource, for example `PIO0`.
        pio: ident,
        /// GPIO input pin connected to the IR receiver.
        pin: ident,
        /// Application button type that IR codes map to.
        button: ty,
        /// Maximum mapping entries; at least the number of entries you provide.
        capacity: expr,
    }

    generate {
        // A one-member `ir_mappings!` group; the group stays out of the docs.
        $crate::ir::ir_mappings! {
            #[doc(hidden)]
            $decl.vis $ident($decl.name, Group) {
                pio: $decl.pio,
                button: $decl.button,
                capacity: $decl.capacity,

                $decl.attrs
                #[doc = $decl.doc]
                $decl.name { pin: $decl.pin },
            }
        }

        impl $decl.name {
            /// Creates the receiver and spawns its background task.
            ///
            /// See the [ir module documentation](mod@device_envoy_rp::ir) for usage.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pin>>,
                button_map: &[(u16, u8, $decl.button)],
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let (ir_mapping,) = $ident($decl.name, Group)::new(pio, pin, button_map, spawner)?;
                Ok(ir_mapping)
            }
        }
    }
}
const_structures::define! {
    /// Macro to generate multiple IR mapping struct types that share one PIO resource.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir_mapping!`](crate::ir_mapping) — Generate a single IR mapping receiver type
    /// - [`irs!`](crate::irs) — Generate raw IR receivers sharing one PIO resource
    pub ir_mappings {
        /// PIO resource shared by every receiver in the group, for example `PIO0`.
        pio: ident,
        /// Application button type that IR codes map to.
        button: ty,
        /// Maximum mapping entries per receiver; at least the number of entries you provide.
        capacity: expr,
        /// Each member is one mapping receiver and uses one PIO state machine.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        $for ir in $decl.members {
            static $upper($ir.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static $upper($ir.name, _MAPPING_CELL): ::static_cell::StaticCell<$ir.name> =
                ::static_cell::StaticCell::new();

            $ir.attrs
            #[doc = $ir.doc]
            $decl.vis struct $ir.name {
                ir_static: &'static $crate::ir::__IrStatic,
                button_map: ::heapless::LinearMap<(u16, u8), $decl.button, $decl.capacity>,
            }

            impl $crate::ir::IrMapping<$decl.button> for $ir.name {
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

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every mapping receiver in the group and spawns their background tasks.
            ///
            /// Takes the PIO resource, then a pin and button map per receiver in declaration
            /// order,
            /// then the spawner.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                $for ir in $decl.members {
                    $snake($ir.name, _pin): impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin>>,
                    $snake($ir.name, _button_map): &[(u16, u8, $decl.button)],
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                let ::embassy_rp::pio::Pio { mut common, $for ir in $decl.members { $ident(sm, $ir.index), } .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio.into(),
                        <::embassy_rp::peripherals::$decl.pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                $for ir in $decl.members {
                    let pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin> =
                        $snake($ir.name, _pin).into();
                    let receiver = $crate::ir::__new_receiver(&mut common, $ident(sm, $ir.index), pin);
                    <::embassy_rp::peripherals::$decl.pio as $crate::ir::IrPioPeripheral>::$ident(spawn_task_sm, $ir.index)(
                        receiver,
                        &$upper($ir.name, _IR_STATIC),
                        spawner,
                    )?;
                }
                Ok(($for ir in $decl.members {
                    &*$upper($ir.name, _MAPPING_CELL).init($ir.name {
                        ir_static: &$upper($ir.name, _IR_STATIC),
                        button_map: $crate::ir::__build_button_map::<$decl.button, $decl.capacity>($snake($ir.name, _button_map)),
                    }),
                }))
            }
        }
    }
}
const_structures::define! {
    /// Macro to generate multiple IR receiver struct types that share one PIO resource.
    ///
    /// **See the [ir module documentation](mod@crate::ir) for usage examples.**
    ///
    /// # Related Macros
    ///
    /// - [`ir!`](crate::ir!) — Generate a single IR receiver type
    /// - [`ir_mappings!`](crate::ir_mappings) — Generate mapped-button receivers sharing one PIO
    pub irs {
        /// PIO resource shared by every receiver in the group, for example `PIO0`.
        pio: ident,
        /// Each member is one IR receiver and uses one PIO state machine.
        members 1..=4 {
            /// GPIO input pin connected to the IR receiver.
            pin: ident,
        },
    }

    generate {
        $for ir in $decl.members {
            static $upper($ir.name, _IR_STATIC): $crate::ir::__IrStatic = $crate::ir::__IrStatic::new();
            static $upper($ir.name, _IR): $ir.name = $ir.name {
                ir_static: &$upper($ir.name, _IR_STATIC),
            };

            $ir.attrs
            #[doc = $ir.doc]
            $decl.vis struct $ir.name {
                ir_static: &'static $crate::ir::__IrStatic,
            }

            impl $crate::ir::Ir for $ir.name {
                async fn wait_for_press(&self) -> $crate::ir::IrEvent {
                    self.ir_static.receive().await
                }
            }
        }

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// Creates every IR receiver in the group and spawns their background tasks.
            ///
            /// Takes the PIO resource, then one pin per receiver in declaration order,
            /// then the spawner.
            pub fn new(
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                $for ir in $decl.members {
                    $snake($ir.name, _pin): impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin>>,
                }
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<($for ir in $decl.members { &'static $ir.name, })> {
                let ::embassy_rp::pio::Pio { mut common, $for ir in $decl.members { $ident(sm, $ir.index), } .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio.into(),
                        <::embassy_rp::peripherals::$decl.pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                $for ir in $decl.members {
                    let pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$ir.pin> =
                        $snake($ir.name, _pin).into();
                    let receiver = $crate::ir::__new_receiver(&mut common, $ident(sm, $ir.index), pin);
                    <::embassy_rp::peripherals::$decl.pio as $crate::ir::IrPioPeripheral>::$ident(spawn_task_sm, $ir.index)(
                        receiver,
                        &$upper($ir.name, _IR_STATIC),
                        spawner,
                    )?;
                }
                Ok(($for ir in $decl.members { &$upper($ir.name, _IR), }))
            }
        }
    }
}

macro_rules! __define_ir_task {
    ($task_name:ident, $pio:ty, $sm:literal) => {
        #[embassy_executor::task]
        async fn $task_name(
            mut nec_receiver: NecReceiver<'static, $pio, $sm>,
            ir_static: &'static IrStatic,
        ) -> ! {
            loop {
                let raw_frame = nec_receiver.receive_frame().await;
                if let Some((addr, cmd)) = decode_nec_frame(raw_frame) {
                    ir_static.send(IrEvent::Press { addr, cmd }).await;
                }
            }
        }
    };
}

__define_ir_task!(ir_pio0_sm0_task, embassy_rp::peripherals::PIO0, 0);
__define_ir_task!(ir_pio0_sm1_task, embassy_rp::peripherals::PIO0, 1);
__define_ir_task!(ir_pio0_sm2_task, embassy_rp::peripherals::PIO0, 2);
__define_ir_task!(ir_pio0_sm3_task, embassy_rp::peripherals::PIO0, 3);

__define_ir_task!(ir_pio1_sm0_task, embassy_rp::peripherals::PIO1, 0);
__define_ir_task!(ir_pio1_sm1_task, embassy_rp::peripherals::PIO1, 1);
__define_ir_task!(ir_pio1_sm2_task, embassy_rp::peripherals::PIO1, 2);
__define_ir_task!(ir_pio1_sm3_task, embassy_rp::peripherals::PIO1, 3);

#[cfg(feature = "pico2")]
__define_ir_task!(ir_pio2_sm0_task, embassy_rp::peripherals::PIO2, 0);
#[cfg(feature = "pico2")]
__define_ir_task!(ir_pio2_sm1_task, embassy_rp::peripherals::PIO2, 1);
#[cfg(feature = "pico2")]
__define_ir_task!(ir_pio2_sm2_task, embassy_rp::peripherals::PIO2, 2);
#[cfg(feature = "pico2")]
__define_ir_task!(ir_pio2_sm3_task, embassy_rp::peripherals::PIO2, 3);

// ===== NEC Receiver Implementation =========================================

impl<'d, PIO: Instance, const SM: usize> NecReceiver<'d, PIO, SM> {
    fn new(
        common: &mut Common<'d, PIO>,
        mut sm: StateMachine<'d, PIO, SM>,
        ir_pin: embassy_rp::pio::Pin<'d, PIO>,
    ) -> Self {
        // PIO program (ported from nec_receive.pio)
        let prg = pio::pio_asm!(
            r#"
            ; Constants for burst detection and bit sampling
            ; These values are calibrated for 10 SM clock ticks per 562.5µs burst period
            .define BURST_LOOP_COUNTER 30    ; threshold for sync burst detection
            .define BIT_SAMPLE_DELAY 15      ; wait 1.5 burst periods before sampling

            .wrap_target
            next_burst:
                set x, BURST_LOOP_COUNTER
                wait 0 pin 0                 ; wait for burst to start (active low)

            burst_loop:
                jmp pin data_bit             ; burst ended before counter expired
                jmp x-- burst_loop           ; keep waiting for burst to end

                                             ; counter expired = sync burst detected
                mov isr, null                ; reset ISR for new frame
                wait 1 pin 0                 ; wait for sync burst to finish
                jmp next_burst               ; ready for first data bit

            data_bit:
                nop [BIT_SAMPLE_DELAY - 1]   ; wait 1.5 burst periods
                in pins, 1                   ; sample gap length: short=0, long=1
                                             ; autopush after 32 bits
            .wrap
            "#
        );

        let mut cfg = Config::default();

        // Input shift register: shift right, autopush after 32 bits
        let mut shift_config = ShiftConfig::default();
        shift_config.direction = ShiftDirection::Right;
        shift_config.auto_fill = true;
        shift_config.threshold = 32;
        cfg.shift_in = shift_config;

        // Join FIFOs to make a larger receive FIFO
        cfg.fifo_join = FifoJoin::RxOnly;

        // Set the IN pin for sampling
        cfg.set_in_pins(&[&ir_pin]);

        // Set the JMP pin for burst detection
        cfg.set_jmp_pin(&ir_pin);

        // Set clock divisor: 10 ticks per 562.5µs burst period
        // System clock is typically 125 MHz
        // Target: 10 / 562.5µs = 17,777.78 Hz
        let clock_freq = 125_000_000.0_f32; // 125 MHz system clock
        let target_freq = 10.0_f32 / 562.5e-6_f32; // 10 ticks per burst period
        let divisor: f32 = clock_freq / target_freq;
        cfg.clock_divider = divisor.to_fixed();

        // Load the PIO program first
        let loaded_program = common.load_program(&prg.program);

        // Configure using the loaded program (sets wrap, origin, etc.)
        cfg.use_program(&loaded_program, &[]);

        // Initialize and start the state machine
        sm.set_config(&cfg);
        sm.set_pin_dirs(embassy_rp::pio::Direction::In, &[&ir_pin]);
        sm.set_enable(true);

        // Keep the loaded program to prevent deallocation
        let _ = loaded_program;

        Self { sm }
    }

    /// Wait for and receive a 32-bit NEC frame from the PIO FIFO
    async fn receive_frame(&mut self) -> u32 {
        self.sm.rx().wait_pull().await
    }
}
