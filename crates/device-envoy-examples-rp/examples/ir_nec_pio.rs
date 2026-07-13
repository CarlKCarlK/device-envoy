//! NEC IR receiver using PIO for hardware-based decoding.
//!
//! This example demonstrates using the RP2040's PIO state machine to decode NEC IR
//! remote control signals. The PIO handles tight timing constraints in hardware,
//! making decoding reliable even when the CPU is busy with other tasks.
//!
//! Hardware setup:
//! - Connect an active-low IR receiver (e.g., VS1838B) to PIN_15
//! - IR receiver power to 3.3V and GND
//! - PIN_15 should be pulled up (IR receiver outputs LOW when carrier detected)
//!
//! The PIO program:
//! - Measures burst widths to distinguish sync pulses from data bits
//! - Samples gap length after each burst to decode 0 vs 1
//! - Auto-pushes 32-bit frames to the RX FIFO
//! - Handles all timing in hardware with no CPU intervention needed
//!
//! Ported from the official Raspberry Pi Pico SDK example:
//! https://github.com/raspberrypi/pico-examples/tree/master/pio/ir_nec

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Common, Config, FifoJoin, Instance, InterruptHandler, Pio, ShiftConfig, ShiftDirection,
    StateMachine,
};
use embassy_time::Timer;
use fixed::traits::ToFixed;
use panic_probe as _;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("NEC IR PIO decoder example starting");

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    // Configure PIN_15 for IR receiver input with pull-up
    // IR receivers idle HIGH and pull LOW when detecting carrier
    let mut ir_pin = common.make_pio_pin(p.PIN_15);
    ir_pin.set_pull(Pull::Up);

    // Load and configure the PIO program
    let mut nec_receiver = NecReceiver::new(&mut common, sm0, ir_pin);

    info!("PIO NEC IR receiver initialized on PIN_15");
    info!("Point your NEC remote at the IR receiver and press buttons");

    loop {
        // Wait for a frame from the PIO FIFO
        let raw_frame = nec_receiver.receive_frame().await;

        // Decode and validate the frame
        match decode_nec_frame(raw_frame) {
            Some((addr, cmd)) => {
                info!("✓ Valid NEC frame: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
            }
            None => {
                info!("✗ Invalid frame (checksum failed): 0x{:08X}", raw_frame);
            }
        }

        // Small delay to avoid flooding the console
        Timer::after_millis(50).await;
    }
}

/// NEC IR receiver using PIO
struct NecReceiver<'d, PIO: Instance, const SM: usize> {
    sm: StateMachine<'d, PIO, SM>,
}

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
