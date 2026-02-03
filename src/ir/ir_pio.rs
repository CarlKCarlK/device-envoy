//! PIO-based infrared receiver using the NEC protocol.
//!
//! See [`IrPio`] for usage examples.

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Pin, Pull};
use embassy_rp::pio::{
    Common, Config, FifoJoin, Instance, PioPin, ShiftConfig, ShiftDirection, StateMachine,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel as EmbassyChannel;
use fixed::traits::ToFixed;

use crate::{Error, Result};

use super::IrEvent;

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
pub trait IrPioPeripheral: Instance {
    /// The interrupt binding type for this PIO
    type Irqs: embassy_rp::interrupt::typelevel::Binding<
            <Self as Instance>::Interrupt,
            embassy_rp::pio::InterruptHandler<Self>,
        >;

    /// Get the interrupt configuration
    fn irqs() -> Self::Irqs;

    /// Spawn the task for this PIO
    fn spawn_task(
        receiver: NecReceiver<'static, Self, 0>,
        ir_pio_static: &'static IrPioStatic,
        spawner: Spawner,
    ) -> Result<()>;
}

impl IrPioPeripheral for embassy_rp::peripherals::PIO0 {
    type Irqs = crate::pio_irqs::Pio0Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio0Irqs
    }

    fn spawn_task(
        receiver: NecReceiver<'static, Self, 0>,
        ir_pio_static: &'static IrPioStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio0_task(receiver, ir_pio_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }
}

impl IrPioPeripheral for embassy_rp::peripherals::PIO1 {
    type Irqs = crate::pio_irqs::Pio1Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio1Irqs
    }

    fn spawn_task(
        receiver: NecReceiver<'static, Self, 0>,
        ir_pio_static: &'static IrPioStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio1_task(receiver, ir_pio_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }
}

#[cfg(feature = "pico2")]
impl IrPioPeripheral for embassy_rp::peripherals::PIO2 {
    type Irqs = crate::pio_irqs::Pio2Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio2Irqs
    }

    fn spawn_task(
        receiver: NecReceiver<'static, Self, 0>,
        ir_pio_static: &'static IrPioStatic,
        spawner: Spawner,
    ) -> Result<()> {
        let token = ir_pio2_task(receiver, ir_pio_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)
    }
}

// ===== Public API ===========================================================

/// Static resources for the `IrPio` device abstraction.
///
/// See [`IrPio`] for usage examples.
pub struct IrPioStatic(EmbassyChannel<CriticalSectionRawMutex, IrEvent, 8>);

impl IrPioStatic {
    /// Creates static resources for the PIO-based infrared receiver device.
    #[must_use]
    const fn new() -> Self {
        Self(EmbassyChannel::new())
    }

    async fn send(&self, event: IrEvent) {
        self.0.send(event).await;
    }

    async fn receive(&self) -> IrEvent {
        self.0.receive().await
    }
}

/// A device abstraction for an infrared receiver using PIO hardware for NEC protocol decoding.
///
/// This implementation uses the RP2040's PIO state machine to decode NEC IR signals in hardware,
/// making decoding reliable even when the CPU is busy with other tasks. Works with any PIO
/// peripheral (PIO0, PIO1, or PIO2 on Pico 2).
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_kit::ir::{IrEvent, IrPio, IrPioStatic};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_kit::Result<()> {
///     static IR_PIO_STATIC: IrPioStatic = IrPio::new_static();
///     let ir_pio = IrPio::new(&IR_PIO_STATIC, p.PIN_15, p.PIO0, spawner)?;
///
///     loop {
///         let IrEvent::Press { addr, cmd } = ir_pio.wait_for_press().await;
///         defmt::info!("IR: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
///     }
/// }
/// ```
pub struct IrPio<'a> {
    ir_pio_static: &'a IrPioStatic,
}

impl IrPio<'_> {
    /// Create static channel resources for IR events.
    ///
    /// See [`IrPio`] for usage examples.
    #[must_use]
    pub const fn new_static() -> IrPioStatic {
        IrPioStatic::new()
    }

    /// Create a new PIO-based IR receiver on the specified pin.
    ///
    /// See [`IrPio`] for usage examples.
    ///
    /// # Errors
    /// Returns an error if the background task cannot be spawned.
    pub fn new<P, PIO>(
        ir_pio_static: &'static IrPioStatic,
        pin: Peri<'static, P>,
        pio: Peri<'static, PIO>,
        spawner: Spawner,
    ) -> Result<Self>
    where
        P: Pin + PioPin,
        PIO: IrPioPeripheral,
    {
        // Set up PIO in the generic context where we have the concrete pin type
        let pio_instance = embassy_rp::pio::Pio::new(pio, PIO::irqs());
        let embassy_rp::pio::Pio {
            mut common, sm0, ..
        } = pio_instance;

        // Configure pin for IR receiver input with pull-up
        // IR receivers idle HIGH and pull LOW when detecting carrier
        let mut ir_pin = common.make_pio_pin(pin);
        ir_pin.set_pull(Pull::Up);

        // Load and configure the PIO program
        let nec_receiver = NecReceiver::new(&mut common, sm0, ir_pin);

        // Spawn the task with the configured receiver (dispatch to PIO-specific task)
        PIO::spawn_task(nec_receiver, ir_pio_static, spawner)?;

        Ok(Self { ir_pio_static })
    }

    /// Wait for the next IR event.
    ///
    /// See [`IrPio`] for usage examples.
    pub async fn wait_for_press(&self) -> IrEvent {
        self.ir_pio_static.receive().await
    }
}

#[embassy_executor::task]
async fn ir_pio0_task(
    mut nec_receiver: NecReceiver<'static, embassy_rp::peripherals::PIO0, 0>,
    ir_pio_static: &'static IrPioStatic,
) -> ! {
    loop {
        // Wait for a frame from the PIO FIFO
        let raw_frame = nec_receiver.receive_frame().await;

        // Decode and validate the frame
        if let Some((addr, cmd)) = decode_nec_frame(raw_frame) {
            ir_pio_static.send(IrEvent::Press { addr, cmd }).await;
        }
    }
}

#[embassy_executor::task]
async fn ir_pio1_task(
    mut nec_receiver: NecReceiver<'static, embassy_rp::peripherals::PIO1, 0>,
    ir_pio_static: &'static IrPioStatic,
) -> ! {
    loop {
        // Wait for a frame from the PIO FIFO
        let raw_frame = nec_receiver.receive_frame().await;

        // Decode and validate the frame
        if let Some((addr, cmd)) = decode_nec_frame(raw_frame) {
            ir_pio_static.send(IrEvent::Press { addr, cmd }).await;
        }
    }
}

#[cfg(feature = "pico2")]
#[embassy_executor::task]
async fn ir_pio2_task(
    mut nec_receiver: NecReceiver<'static, embassy_rp::peripherals::PIO2, 0>,
    ir_pio_static: &'static IrPioStatic,
) -> ! {
    loop {
        // Wait for a frame from the PIO FIFO
        let raw_frame = nec_receiver.receive_frame().await;

        // Decode and validate the frame
        if let Some((addr, cmd)) = decode_nec_frame(raw_frame) {
            ir_pio_static.send(IrEvent::Press { addr, cmd }).await;
        }
    }
}

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

/// Decode and validate a 32-bit NEC frame
///
/// NEC protocol structure (32 bits, LSB first):
/// - Byte 0: Address (8 bits)
/// - Byte 1: Address inverse (~Address)
/// - Byte 2: Command (8 bits)
/// - Byte 3: Command inverse (~Command)
///
/// Extended NEC uses 16-bit address (bytes 0-1) without inversion check
///
/// Returns `Some((address, command))` if valid, `None` if checksum fails
fn decode_nec_frame(frame: u32) -> Option<(u16, u8)> {
    let byte0 = (frame & 0xFF) as u8;
    let byte1 = ((frame >> 8) & 0xFF) as u8;
    let byte2 = ((frame >> 16) & 0xFF) as u8;
    let byte3 = ((frame >> 24) & 0xFF) as u8;

    // Validate command bytes (required in both standard and extended NEC)
    if (byte2 ^ byte3) != 0xFF {
        return None;
    }

    // Standard NEC: 8-bit address with inverse validation
    if (byte0 ^ byte1) == 0xFF {
        return Some((u16::from(byte0), byte2));
    }

    // Extended NEC: 16-bit address (no inversion check on address)
    let addr16 = ((u16::from(byte1)) << 8) | u16::from(byte0);
    Some((addr16, byte2))
}
