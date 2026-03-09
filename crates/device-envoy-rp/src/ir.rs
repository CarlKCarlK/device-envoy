//! A device abstraction for infrared receivers using the NEC protocol.
//!
//! See [`IrRp`], [`IrMappingRp`], and [`IrKeplerRp`] for usage examples.
//!
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Pin, Pull};
use embassy_rp::pio::{
    Common, Config, FifoJoin, Instance, PioPin, ShiftConfig, ShiftDirection, StateMachine,
};
use fixed::traits::ToFixed;

use crate::{Error, Result};

use device_envoy_core::ir::decode_nec_frame;
pub use device_envoy_core::ir::{Ir, IrEvent, IrKepler, IrMapping, IrMappingAdapter, IrStatic};
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use paste;

// ============================================================================
// Submodules
// ============================================================================

mod kepler;
mod mapping;

pub use kepler::{IrKeplerRp, IrKeplerStatic, KEPLER_MAPPING, KeplerButton};
pub use mapping::{IrMappingRp, IrMappingStatic, __build_button_map};

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
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm1_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm2_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_sm3_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }
}

impl IrPioPeripheral for embassy_rp::peripherals::PIO1 {
    fn spawn_task_sm0(
        receiver: NecReceiver<'static, Self, 0>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm0_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm1_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm2_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_sm3_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
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
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm1(
        receiver: NecReceiver<'static, Self, 1>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm1_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm2(
        receiver: NecReceiver<'static, Self, 2>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm2_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }

    fn spawn_task_sm3(
        receiver: NecReceiver<'static, Self, 3>,
        ir_static: &'static IrStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_sm3_task(receiver, ir_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }
}

/// A device abstraction for an infrared receiver for NEC protocol decoding.
///
/// This implementation uses the RP2040's PIO state machine to decode NEC IR signals in hardware,
/// making decoding reliable even when the CPU is busy with other tasks. Works with any PIO
/// peripheral (PIO0, PIO1, or PIO2 on Pico 2).
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::ir;
/// use device_envoy_rp::ir::{Ir as _, IrEvent};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// ir! {
///     Ir15: { pio: PIO0, pin: PIN_15 }
/// }
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     let ir15 = Ir15::new(p.PIO0, p.PIN_15, spawner)?;
///
///     loop {
///         let IrEvent::Press { addr, cmd } = ir15.wait_for_press().await;
///         defmt::info!("IR: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
///     }
/// }
/// ```
pub struct IrRp<'a> {
    ir_static: &'a IrStatic,
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

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __new_ir_on_sm0<P, PIO>(
    ir_static: &'static IrStatic,
    pin: Peri<'static, P>,
    pio: Peri<'static, PIO>,
    spawner: Spawner,
) -> Result<IrRp<'static>>
where
    P: Pin + PioPin,
    PIO: IrPioPeripheral,
{
    let pio_instance = embassy_rp::pio::Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let embassy_rp::pio::Pio {
        mut common, sm0, ..
    } = pio_instance;
    let nec_receiver = __new_receiver(&mut common, sm0, pin);
    PIO::spawn_task_sm0(nec_receiver, ir_static, spawner)?;
    Ok(IrRp { ir_static })
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __new_ir_on_sm1<P, PIO>(
    ir_static: &'static IrStatic,
    pin: Peri<'static, P>,
    pio: Peri<'static, PIO>,
    spawner: Spawner,
) -> Result<IrRp<'static>>
where
    P: Pin + PioPin,
    PIO: IrPioPeripheral,
{
    let pio_instance = embassy_rp::pio::Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let embassy_rp::pio::Pio {
        mut common, sm1, ..
    } = pio_instance;
    let nec_receiver = __new_receiver(&mut common, sm1, pin);
    PIO::spawn_task_sm1(nec_receiver, ir_static, spawner)?;
    Ok(IrRp { ir_static })
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __new_ir_on_sm2<P, PIO>(
    ir_static: &'static IrStatic,
    pin: Peri<'static, P>,
    pio: Peri<'static, PIO>,
    spawner: Spawner,
) -> Result<IrRp<'static>>
where
    P: Pin + PioPin,
    PIO: IrPioPeripheral,
{
    let pio_instance = embassy_rp::pio::Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let embassy_rp::pio::Pio {
        mut common, sm2, ..
    } = pio_instance;
    let nec_receiver = __new_receiver(&mut common, sm2, pin);
    PIO::spawn_task_sm2(nec_receiver, ir_static, spawner)?;
    Ok(IrRp { ir_static })
}

// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub fn __new_ir_on_sm3<P, PIO>(
    ir_static: &'static IrStatic,
    pin: Peri<'static, P>,
    pio: Peri<'static, PIO>,
    spawner: Spawner,
) -> Result<IrRp<'static>>
where
    P: Pin + PioPin,
    PIO: IrPioPeripheral,
{
    let pio_instance = embassy_rp::pio::Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let embassy_rp::pio::Pio {
        mut common, sm3, ..
    } = pio_instance;
    let nec_receiver = __new_receiver(&mut common, sm3, pin);
    PIO::spawn_task_sm3(nec_receiver, ir_static, spawner)?;
    Ok(IrRp { ir_static })
}

impl Ir for IrRp<'_> {
    async fn wait_for_press(&self) -> IrEvent {
        self.ir_static.receive().await
    }
}

/// Generate one or more typed IR receiver constructors sharing a single PIO resource.
///
/// See the [`IrRp`] struct example for base usage and the generated `new(...)` constructors.
#[doc(hidden)]
#[macro_export]
macro_rules! irs {
    (
        pio: $pio:ident,
        $group_name:ident {
            $first_name:ident : { pin: $first_pin:ident $(,)? }
            $(, $rest_name:ident : { pin: $rest_pin:ident $(,)? })* $(,)?
        }
    ) => {
        $crate::__irs_impl! {
            pio: $pio,
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
        pio: $pio:ident,
        $group_name:ident,
        [($name0:ident, $pin0:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::IrStatic = $crate::ir::IrStatic::new();
            static [<$name0:upper _IR_CELL>]: ::static_cell::StaticCell<$name0> = ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir_static: &'static $crate::ir::IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let _ = $crate::ir::__new_ir_on_sm0(&[<$name0:upper _IR_STATIC>], pin, pio, spawner)?;
                    Ok([<$name0:upper _IR_CELL>].init(Self {
                        ir_static: &[<$name0:upper _IR_STATIC>],
                    }))
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
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident)]
    ) => {
        $crate::ir::paste::paste! {
            static [<$name0:upper _IR_STATIC>]: $crate::ir::IrStatic = $crate::ir::IrStatic::new();
            static [<$name1:upper _IR_STATIC>]: $crate::ir::IrStatic = $crate::ir::IrStatic::new();

            static [<$name0:upper _IR_CELL>]: ::static_cell::StaticCell<$name0> = ::static_cell::StaticCell::new();
            static [<$name1:upper _IR_CELL>]: ::static_cell::StaticCell<$name1> = ::static_cell::StaticCell::new();

            pub struct $name0 {
                ir_static: &'static $crate::ir::IrStatic,
            }
            pub struct $name1 {
                ir_static: &'static $crate::ir::IrStatic,
            }

            impl $name0 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let _ = $crate::ir::__new_ir_on_sm0(&[<$name0:upper _IR_STATIC>], pin, pio, spawner)?;
                    Ok([<$name0:upper _IR_CELL>].init(Self {
                        ir_static: &[<$name0:upper _IR_STATIC>],
                    }))
                }
            }

            impl $name1 {
                pub fn new(
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let _ = $crate::ir::__new_ir_on_sm1(&[<$name1:upper _IR_STATIC>], pin, pio, spawner)?;
                    Ok([<$name1:upper _IR_CELL>].init(Self {
                        ir_static: &[<$name1:upper _IR_STATIC>],
                    }))
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
                    pio: embassy_rp::Peri<'static, embassy_rp::peripherals::$pio>,
                    pin0: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin0>,
                    pin1: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin1>,
                    spawner: embassy_executor::Spawner,
                ) -> $crate::Result<(&'static $name0, &'static $name1)> {
                    let pio_instance = embassy_rp::pio::Pio::new(
                        pio,
                        <embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                    let embassy_rp::pio::Pio {
                        mut common, sm0, sm1, ..
                    } = pio_instance;

                    let receiver0 = $crate::ir::__new_receiver(&mut common, sm0, pin0);
                    <embassy_rp::peripherals::$pio as $crate::ir::IrPioPeripheral>::spawn_task_sm0(
                        receiver0,
                        &[<$name0:upper _IR_STATIC>],
                        spawner,
                    )?;

                    let receiver1 = $crate::ir::__new_receiver(&mut common, sm1, pin1);
                    <embassy_rp::peripherals::$pio as $crate::ir::IrPioPeripheral>::spawn_task_sm1(
                        receiver1,
                        &[<$name1:upper _IR_STATIC>],
                        spawner,
                    )?;

                    let ir0 = [<$name0:upper _IR_CELL>].init($name0 {
                        ir_static: &[<$name0:upper _IR_STATIC>],
                    });
                    let ir1 = [<$name1:upper _IR_CELL>].init($name1 {
                        ir_static: &[<$name1:upper _IR_STATIC>],
                    });
                    Ok((ir0, ir1))
                }
            }
        }
    };
    (
        pio: $pio:ident,
        $group_name:ident,
        [($name0:ident, $pin0:ident), ($name1:ident, $pin1:ident), ($($tail:tt)+)]
    ) => {
        compile_error!("irs! currently supports up to 2 receivers in one group.");
    };
}

/// Generate one typed IR receiver constructor.
#[doc(hidden)]
#[macro_export]
macro_rules! ir {
    (
        $name:ident : { pio: $pio:ident, pin: $pin:ident $(,)? }
    ) => {
        $crate::ir::paste::paste! {
            $crate::irs! {
                pio: $pio,
                [<__ $name _GROUP>] {
                    $name: { pin: $pin }
                }
            }
        }
    };
}

/// Macro to generate one IR receiver type.
///
/// Use this when you need a single receiver.
#[allow(unused_imports)]
#[doc(inline)]
pub use ir;
/// Macro to generate multiple IR receiver types on one PIO resource.
///
/// Use this when you need more than one receiver sharing the same PIO resource.
#[allow(unused_imports)]
#[doc(inline)]
pub use irs;
/// Macro to generate one IR-mapping type.
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_mapping;
/// Macro to generate multiple IR-mapping types on one PIO resource.
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_mappings;
/// Macro to generate one Kepler IR type.
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_kepler;
/// Macro to generate multiple Kepler IR types on one PIO resource.
#[allow(unused_imports)]
#[doc(inline)]
pub use crate::ir_keplers;

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
