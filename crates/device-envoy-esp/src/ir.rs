//! A device abstraction for infrared receivers using the NEC protocol.
//!
//! See [`Ir`], [`IrMapping`], and [`IrKepler`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

mod kepler;
mod mapping;

pub use kepler::{IrKepler, IrKeplerStatic, KeplerButton};
pub use mapping::{IrMapping, IrMappingStatic};

#[cfg(target_os = "none")]
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel as EmbassyChannel;
#[cfg(target_os = "none")]
use log::info;

#[cfg(target_os = "none")]
use crate::Result;

/// Events received from the infrared receiver.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IrEvent {
    /// Button press with 16-bit address and 8-bit command.
    /// Supports both standard NEC (8-bit address) and extended NEC (16-bit address).
    Press {
        /// 16-bit device address (or 8-bit address in low byte for standard NEC).
        addr: u16,
        /// 8-bit command code.
        cmd: u8,
    },
}

/// Static resources for the [`Ir`] device abstraction.
pub struct IrStatic(EmbassyChannel<CriticalSectionRawMutex, IrEvent, 8>);

impl IrStatic {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(EmbassyChannel::new())
    }

    #[cfg(target_os = "none")]
    pub(crate) async fn send(&self, event: IrEvent) {
        self.0.send(event).await;
    }

    pub(crate) async fn receive(&self) -> IrEvent {
        self.0.receive().await
    }
}

/// A device abstraction for an infrared receiver for NEC protocol decoding.
///
/// The caller owns RMT lifecycle and passes an owned RX channel.
pub struct Ir<'a> {
    ir_static: &'a IrStatic,
}

impl Ir<'_> {
    /// Create static channel resources for IR events.
    #[must_use]
    pub const fn new_static() -> IrStatic {
        IrStatic::new()
    }

    /// Create a new RMT-based IR receiver from an RX channel creator and GPIO pin.
    ///
    /// This configures an async RX channel using [`crate::rmt::nec_rx_config`].
    ///
    /// # Errors
    /// Returns an error if the background task cannot be spawned.
    #[cfg(target_os = "none")]
    pub fn new(
        ir_static: &'static IrStatic,
        pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        channel_creator: impl esp_hal::rmt::RxChannelCreator<'static, esp_hal::Async>,
        spawner: Spawner,
    ) -> Result<Self> {
        let channel = channel_creator
            .configure_rx(pin, crate::rmt::nec_rx_config())
            .map_err(crate::Error::Rmt)?;
        info!("IR: receiver task started");
        spawner
            .spawn(ir_receiver_task(channel, ir_static))
            .map_err(crate::Error::TaskSpawn)?;

        Ok(Self { ir_static })
    }

    /// Wait for the next IR event.
    pub async fn wait_for_press(&self) -> IrEvent {
        self.ir_static.receive().await
    }
}

#[cfg(target_os = "none")]
#[embassy_executor::task]
async fn ir_receiver_task(
    mut channel: esp_hal::rmt::Channel<'static, esp_hal::Async, esp_hal::rmt::Rx>,
    ir_static: &'static IrStatic,
) -> ! {
    let mut pulse_codes = [esp_hal::rmt::PulseCode::default(); 96];

    loop {
        for pulse_code in &mut pulse_codes {
            pulse_code.reset();
        }

        match channel.receive(&mut pulse_codes).await {
            Ok(symbol_count) => {
                if let Some((addr, cmd)) = decode_nec_from_pulses(&pulse_codes[..symbol_count]) {
                    ir_static.send(IrEvent::Press { addr, cmd }).await;
                }
            }
            Err(_) => {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(2)).await;
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

    // Typical demodulated NEC from IR receiver:
    // - Leader mark: low ~9000us
    // - Leader space: high ~4500us
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

    // TODO0 Handle NEC repeat frames explicitly (leader + 2.25ms + 560us pattern).
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

    // NEC repeat frame: 9ms leader mark + 2.25ms space (+ trailing 560us mark).
    level0 == Level::Low
        && level1 == Level::High
        && within(duration0, 9000, 2200)
        && within(duration1, 2250, 1000)
}

/// Decode and validate a 32-bit NEC frame.
///
/// NEC protocol structure (32 bits, LSB first):
/// - Byte 0: Address (8 bits)
/// - Byte 1: Address inverse (~Address)
/// - Byte 2: Command (8 bits)
/// - Byte 3: Command inverse (~Command)
///
/// Extended NEC uses 16-bit address (bytes 0-1) without inversion check.
#[cfg(target_os = "none")]
fn decode_nec_frame(frame: u32) -> Option<(u16, u8)> {
    let byte0 = (frame & 0xFF) as u8;
    let byte1 = ((frame >> 8) & 0xFF) as u8;
    let byte2 = ((frame >> 16) & 0xFF) as u8;
    let byte3 = ((frame >> 24) & 0xFF) as u8;

    if (byte2 ^ byte3) != 0xFF {
        return None;
    }

    if (byte0 ^ byte1) == 0xFF {
        return Some((u16::from(byte0), byte2));
    }

    let addr16 = ((u16::from(byte1)) << 8) | u16::from(byte0);
    Some((addr16, byte2))
}
