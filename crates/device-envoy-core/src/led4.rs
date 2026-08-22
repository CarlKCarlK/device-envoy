//! A device abstraction support module for 4-digit, 7-segment LED displays.
//!
//! This module provides platform-independent types and text-to-segment mapping
//! used by platform crates such as `device-envoy-rp` and `device-envoy-esp`.

use core::borrow::Borrow;
use core::num::NonZeroU8;
use core::ops::{BitOrAssign, Index, IndexMut};
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use embassy_time::Timer;
use heapless::{LinearMap, Vec};

/// The number of cells (digits) in a 4-digit display.
#[doc(hidden)] // Platform plumbing constant; user-facing APIs should use CELL_COUNT.
pub const CELL_COUNT_U8: u8 = 4;

/// The number of cells (digits) in a 4-digit display.
pub const CELL_COUNT: usize = CELL_COUNT_U8 as usize;

/// The number of segments per digit.
#[doc(hidden)] // Platform plumbing constant used by RP/ESP led4 internals.
pub const SEGMENT_COUNT: usize = 8;

/// Sleep duration between multiplexing updates.
#[doc(hidden)] // Platform plumbing timing constant used by shared loops.
pub const MULTIPLEX_SLEEP: Duration = Duration::from_millis(3);

/// Maximum number of animation frames accepted by led4-style APIs.
pub const ANIMATION_MAX_FRAMES: usize = 16;
/// Frame buffer type used by led4 text animations.
pub type Animation = Vec<AnimationFrame, ANIMATION_MAX_FRAMES>;

const BLINK_OFF_DELAY: Duration = Duration::from_millis(50);
const BLINK_ON_DELAY: Duration = Duration::from_millis(150);

/// Blinking behavior for 4-digit LED displays.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BlinkState {
    /// Display is always on (solid, no blinking).
    #[default]
    Solid,
    /// Display blinks; currently shows on.
    BlinkingAndOn,
    /// Display blinks; currently shows off.
    BlinkingButOff,
}

/// Frame of animated text for `Led4::animate_text`.
#[derive(Clone, Copy, Debug)]
pub struct AnimationFrame {
    /// Text to display (4 characters for a 4-digit display).
    pub text: [char; CELL_COUNT],
    /// Duration to display this frame. This uses [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html).
    pub duration: embassy_time::Duration,
}

impl AnimationFrame {
    /// Creates a new animation frame with text and duration.
    /// This method uses [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html) for frame timing.
    #[must_use]
    pub const fn new(text: [char; CELL_COUNT], duration: embassy_time::Duration) -> Self {
        Self { text, duration }
    }
}

/// Creates a circular outline animation that chases around display edges.
#[must_use]
pub fn circular_outline_animation(clockwise: bool) -> Animation {
    const FRAME_DURATION: Duration = Duration::from_millis(120);
    const CLOCKWISE: [[char; 4]; 8] = [
        ['\'', '\'', '\'', '\''],
        ['\'', '\'', '\'', '"'],
        [' ', ' ', ' ', '>'],
        [' ', ' ', ' ', ')'],
        ['_', '_', '_', '_'],
        ['*', '_', '_', '_'],
        ['<', ' ', ' ', ' '],
        ['(', '\'', '\'', '\''],
    ];
    const COUNTER: [[char; 4]; 8] = [
        ['(', '\'', '\'', '\''],
        ['<', ' ', ' ', ' '],
        ['*', '_', '_', '_'],
        ['_', '_', '_', '_'],
        [' ', ' ', ' ', ')'],
        [' ', ' ', ' ', '>'],
        ['\'', '\'', '\'', '"'],
        ['\'', '\'', '\'', '\''],
    ];

    let mut animation = Animation::new();
    let frames = if clockwise { &CLOCKWISE } else { &COUNTER };
    for text in frames {
        animation
            .push(AnimationFrame::new(*text, FRAME_DURATION))
            .expect("animation exceeds frame capacity");
    }
    animation
}

/// Commands sent to the shared led4 command loop.
#[derive(Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "the bounded inline animation keeps this no-alloc command signal last-command-wins"
)]
#[doc(hidden)] // Platform plumbing command type; not part of trait-facing API.
pub enum Led4Command {
    /// Display static text with selected blink behavior.
    Text {
        /// Blink behavior for the text.
        blink_state: BlinkState,
        /// Text to display.
        text: [char; CELL_COUNT],
    },
    /// Display a looping text animation.
    Animation(Animation),
}

/// Signal used to send [`Led4Command`] values.
#[doc(hidden)] // Platform plumbing signal type used by RP/ESP implementations.
pub type Led4CommandSignal = Signal<CriticalSectionRawMutex, Led4Command>;

/// Signals static text to a led4 command loop.
#[doc(hidden)] // Platform plumbing helper used by RP/ESP implementations.
pub fn signal_text(
    led4_command_signal: &Led4CommandSignal,
    text: [char; CELL_COUNT],
    blink_state: BlinkState,
) {
    led4_command_signal.signal(Led4Command::Text { blink_state, text });
}

/// Signals an animation to a led4 command loop.
#[doc(hidden)] // Platform plumbing helper used by RP/ESP implementations.
pub fn signal_animation<I>(led4_command_signal: &Led4CommandSignal, animation: I)
where
    I: IntoIterator,
    I::Item: Borrow<AnimationFrame>,
{
    let mut frames: Animation = Animation::new();
    for animation_frame in animation {
        let animation_frame = *animation_frame.borrow();
        frames
            .push(animation_frame)
            .expect("animation fits within ANIMATION_MAX_FRAMES");
    }
    led4_command_signal.signal(Led4Command::Animation(frames));
}

/// Platform-agnostic 4-digit display contract.
///
/// Platform crates implement this trait for their concrete runtime handles.
/// Constructors (`new`, `new_static`) remain inherent on platform types.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::led4::{AnimationFrame, BlinkState, Led4, circular_outline_animation};
/// use embassy_time::{Duration, Timer};
///
/// async fn show_status(led4: &impl Led4) -> ! {
///     // Blink "1234" for three seconds.
///     led4.write_text(['1', '2', '3', '4'], BlinkState::BlinkingAndOn);
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Run the circular outline animation for three seconds.
///     led4.animate_text(circular_outline_animation(true));
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Show "rUSt" solid forever.
///     led4.write_text(['r', 'U', 'S', 't'], BlinkState::Solid);
///     core::future::pending().await
/// }
///
/// # struct DemoLed4;
/// # impl Led4 for DemoLed4 {
/// #     fn write_text(&self, _text: [char; 4], _blink_state: BlinkState) {}
/// #     fn animate_text<I>(&self, _animation: I)
/// #     where
/// #         I: IntoIterator,
/// #         I::Item: core::borrow::Borrow<AnimationFrame>,
/// #     {
/// #     }
/// # }
/// # let led4 = DemoLed4;
/// # let _future = show_status(&led4);
/// ```
pub trait Led4 {
    /// Send text to the display with optional blinking.
    ///
    /// See the [Led4 trait documentation](Self) for usage examples.
    fn write_text(&self, text: [char; CELL_COUNT], blink_state: BlinkState);

    /// Play a looped text animation from the provided frames.
    ///
    /// See the [Led4 trait documentation](Self) for usage examples.
    fn animate_text<I>(&self, animation: I)
    where
        I: IntoIterator,
        I::Item: Borrow<AnimationFrame>;
}

/// Shared command loop for blinking/animated led4 text.
#[doc(hidden)] // Platform plumbing loop used by RP/ESP device tasks.
pub async fn run_command_loop<F>(
    led4_command_signal: &'static Led4CommandSignal,
    mut write_text: F,
) -> !
where
    F: FnMut([char; CELL_COUNT]),
{
    let mut command = Led4Command::Text {
        blink_state: BlinkState::default(),
        text: [' '; CELL_COUNT],
    };

    loop {
        command = match command {
            Led4Command::Text { blink_state, text } => {
                run_text_loop(blink_state, text, led4_command_signal, &mut write_text).await
            }
            Led4Command::Animation(animation) => {
                run_animation_loop(animation, led4_command_signal, &mut write_text).await
            }
        };
    }
}

async fn run_text_loop<F>(
    mut blink_state: BlinkState,
    text: [char; CELL_COUNT],
    led4_command_signal: &'static Led4CommandSignal,
    write_text: &mut F,
) -> Led4Command
where
    F: FnMut([char; CELL_COUNT]),
{
    loop {
        match blink_state {
            BlinkState::Solid => {
                write_text(text);
                return led4_command_signal.wait().await;
            }
            BlinkState::BlinkingAndOn => {
                write_text(text);
                match select(led4_command_signal.wait(), Timer::after(BLINK_ON_DELAY)).await {
                    Either::First(command) => return command,
                    Either::Second(()) => blink_state = BlinkState::BlinkingButOff,
                }
            }
            BlinkState::BlinkingButOff => {
                write_text([' '; CELL_COUNT]);
                match select(led4_command_signal.wait(), Timer::after(BLINK_OFF_DELAY)).await {
                    Either::First(command) => return command,
                    Either::Second(()) => blink_state = BlinkState::BlinkingAndOn,
                }
            }
        }
    }
}

async fn run_animation_loop<F>(
    animation: Animation,
    led4_command_signal: &'static Led4CommandSignal,
    write_text: &mut F,
) -> Led4Command
where
    F: FnMut([char; CELL_COUNT]),
{
    if animation.is_empty() {
        return led4_command_signal.wait().await;
    }

    let frames = animation;
    let len = frames.len();
    let mut index = 0;
    loop {
        let frame = frames[index];
        write_text(frame.text);
        match select(led4_command_signal.wait(), Timer::after(frame.duration)).await {
            Either::First(command) => return command,
            Either::Second(()) => index = (index + 1) % len,
        }
    }
}

/// Error returned when building [`BitsToIndexes`] exceeds preallocated capacity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[doc(hidden)] // Platform plumbing error for shared multiplex internals.
pub enum Led4BitsToIndexesError {
    /// `BitsToIndexes` does not have enough preallocated space.
    Full,
}

/// Internal type for multiplex optimization.
///
/// Maps segment bit patterns to the indexes of digits sharing that pattern.
#[doc(hidden)] // Platform plumbing map type used by shared multiplex loop.
pub type BitsToIndexes = LinearMap<NonZeroU8, Vec<u8, CELL_COUNT>, CELL_COUNT>;

/// Output adapter used by the shared led4 multiplex loop.
#[doc(hidden)] // Platform plumbing adapter trait implemented in platform crates.
pub trait Led4OutputAdapter {
    /// Platform-specific error returned by GPIO writes.
    type Error;

    /// Applies segment bits to the segment output pins.
    fn set_segments_from_nonzero_bits(&mut self, bits: NonZeroU8);

    /// Enables or disables the selected cells.
    ///
    /// When `active` is `true`, cells should turn on. When `active` is
    /// `false`, cells should turn off.
    fn set_cells_active(&mut self, indexes: &[u8], active: bool) -> Result<(), Self::Error>;
}

/// Errors produced by [`run_simple_loop`].
#[derive(Debug)]
#[doc(hidden)] // Platform plumbing error used by platform task wiring.
pub enum Led4SimpleLoopError<E> {
    /// Failed while grouping bit patterns for multiplexing.
    BitsToIndexes(Led4BitsToIndexesError),
    /// Failed while writing the platform output pins.
    Output(E),
}

/// Shared multiplex loop used by platform led4 drivers.
///
/// Waits for new [`BitMatrixLed4`] values on `bit_matrix_signal`, then drives
/// the attached output adapter.
///
/// # Errors
///
/// Returns [`Led4SimpleLoopError`] when either bit grouping or output writes
/// fail.
#[doc(hidden)] // Platform plumbing loop used by RP/ESP implementations.
pub async fn run_simple_loop<T>(
    led4_output_adapter: &mut T,
    bit_matrix_signal: &'static Signal<CriticalSectionRawMutex, BitMatrixLed4>,
) -> Result<core::convert::Infallible, Led4SimpleLoopError<T::Error>>
where
    T: Led4OutputAdapter,
{
    let mut bit_matrix_led4 = BitMatrixLed4::default();
    let mut bits_to_indexes = BitsToIndexes::default();
    'outer: loop {
        bit_matrix_led4
            .bits_to_indexes(&mut bits_to_indexes)
            .map_err(Led4SimpleLoopError::BitsToIndexes)?;

        match bits_to_indexes.iter().next() {
            None => bit_matrix_led4 = bit_matrix_signal.wait().await,
            Some((&bits, indexes)) if bits_to_indexes.len() == 1 => {
                led4_output_adapter.set_segments_from_nonzero_bits(bits);
                led4_output_adapter
                    .set_cells_active(indexes, true)
                    .map_err(Led4SimpleLoopError::Output)?;
                bit_matrix_led4 = bit_matrix_signal.wait().await;
                led4_output_adapter
                    .set_cells_active(indexes, false)
                    .map_err(Led4SimpleLoopError::Output)?;
            }
            _ => loop {
                for (bits, indexes) in &bits_to_indexes {
                    led4_output_adapter.set_segments_from_nonzero_bits(*bits);
                    led4_output_adapter
                        .set_cells_active(indexes, true)
                        .map_err(Led4SimpleLoopError::Output)?;
                    let timeout_or_signal =
                        select(Timer::after(MULTIPLEX_SLEEP), bit_matrix_signal.wait()).await;
                    led4_output_adapter
                        .set_cells_active(indexes, false)
                        .map_err(Led4SimpleLoopError::Output)?;
                    if let Either::Second(notification) = timeout_or_signal {
                        bit_matrix_led4 = notification;
                        continue 'outer;
                    }
                }
            },
        }
    }
}

/// LED segment state for a 4-digit 7-segment display.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[doc(hidden)] // Platform plumbing frame type used by shared multiplex internals.
pub struct BitMatrixLed4([u8; CELL_COUNT]);

impl BitMatrixLed4 {
    #[must_use]
    pub const fn new(bits: [u8; CELL_COUNT]) -> Self {
        Self(bits)
    }

    #[must_use]
    pub fn from_text(text: &[char; CELL_COUNT]) -> Self {
        let bytes = text.map(|char| Leds::ASCII_TABLE.get(char as usize).copied().unwrap_or(0));
        Self::new(bytes)
    }

    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, u8> {
        self.0.iter_mut()
    }

    /// Converts to optimized index mapping for multiplexing.
    ///
    /// # Errors
    ///
    /// Returns [`Led4BitsToIndexesError::Full`] when the preallocated mapping
    /// storage is exhausted.
    pub fn bits_to_indexes(
        &self,
        bits_to_indexes: &mut BitsToIndexes,
    ) -> Result<(), Led4BitsToIndexesError> {
        bits_to_indexes.clear();
        for (&bits, index) in self.iter().zip(0..CELL_COUNT_U8) {
            if let Some(nonzero_bits) = NonZeroU8::new(bits) {
                if let Some(indexes) = bits_to_indexes.get_mut(&nonzero_bits) {
                    indexes
                        .push(index)
                        .map_err(|_| Led4BitsToIndexesError::Full)?;
                } else {
                    let indexes =
                        Vec::from_slice(&[index]).map_err(|_| Led4BitsToIndexesError::Full)?;
                    bits_to_indexes
                        .insert(nonzero_bits, indexes)
                        .map_err(|_| Led4BitsToIndexesError::Full)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for BitMatrixLed4 {
    fn default() -> Self {
        Self([0; CELL_COUNT])
    }
}

impl BitOrAssign<u8> for BitMatrixLed4 {
    fn bitor_assign(&mut self, rhs: u8) {
        self.iter_mut().for_each(|bits| *bits |= rhs);
    }
}

impl Index<usize> for BitMatrixLed4 {
    type Output = u8;

    #[expect(clippy::indexing_slicing, reason = "Caller validates indexing")]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for BitMatrixLed4 {
    #[expect(clippy::indexing_slicing, reason = "Caller validates indexing")]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl IntoIterator for BitMatrixLed4 {
    type Item = u8;
    type IntoIter = core::array::IntoIter<u8, CELL_COUNT>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BitMatrixLed4 {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut BitMatrixLed4 {
    type Item = &'a mut u8;
    type IntoIter = core::slice::IterMut<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

struct Leds;

impl Leds {
    const SEG_A: u8 = 0b_0000_0001;
    const SEG_B: u8 = 0b_0000_0010;
    const SEG_C: u8 = 0b_0000_0100;
    const SEG_D: u8 = 0b_0000_1000;
    const SEG_E: u8 = 0b_0001_0000;
    const SEG_F: u8 = 0b_0010_0000;

    const ASCII_TABLE: [u8; 128] = [
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_1000_0110,
        Self::SEG_A | Self::SEG_B,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        0b_0000_0000,
        Self::SEG_A,
        Self::SEG_A | Self::SEG_F,
        Self::SEG_C | Self::SEG_D,
        Self::SEG_D | Self::SEG_E,
        0b_0000_0000,
        0b_0000_0000,
        0b_0100_0000,
        0b_1000_0000,
        0b_0000_0000,
        0b_0011_1111,
        0b_0000_0110,
        0b_0101_1011,
        0b_0100_1111,
        0b_0110_0110,
        0b_0110_1101,
        0b_0111_1101,
        0b_0000_0111,
        0b_0111_1111,
        0b_0110_1111,
        0b_0000_0000,
        0b_0000_0000,
        Self::SEG_E | Self::SEG_F,
        0b_0000_0000,
        Self::SEG_B | Self::SEG_C,
        0b_0000_0000,
        0b_0000_0000,
        0b_0111_0111,
        0b_0111_1100,
        0b_0011_1001,
        0b_0101_1110,
        0b_0111_1001,
        0b_0111_0001,
        0b_0011_1101,
        0b_0111_0110,
        0b_0000_0110,
        0b_0001_1110,
        0b_0111_0110,
        0b_0011_1000,
        0b_0001_0101,
        0b_0101_0100,
        0b_0011_1111,
        0b_0111_0011,
        0b_0110_0111,
        0b_0101_0000,
        0b_0110_1101,
        0b_0111_1000,
        0b_0011_1110,
        0b_0010_1010,
        0b_0001_1101,
        0b_0111_0110,
        0b_0110_1110,
        0b_0101_1011,
        0b_0011_1001,
        0b_0000_0000,
        0b_0000_1111,
        0b_0000_0000,
        0b_0000_1000,
        0b_0000_0000,
        0b_0111_0111,
        0b_0111_1100,
        0b_0011_1001,
        0b_0101_1110,
        0b_0111_1001,
        0b_0111_0001,
        0b_0011_1101,
        0b_0111_0100,
        0b_0001_0000,
        0b_0001_1110,
        0b_0111_0110,
        0b_0011_1000,
        0b_0001_0101,
        0b_0101_0100,
        0b_0101_1100,
        0b_0111_0011,
        0b_0110_0111,
        0b_0101_0000,
        0b_0110_1101,
        0b_0111_1000,
        0b_0011_1110,
        0b_0010_1010,
        0b_0001_1101,
        0b_0111_0110,
        0b_0110_1110,
        0b_0101_1011,
        0b_0011_1001,
        0b_0000_0110,
        0b_0000_1111,
        0b_0100_0000,
        0b_0000_0000,
    ];
}

#[cfg(test)]
mod tests {
    use super::{BitMatrixLed4, CELL_COUNT};

    #[test]
    fn from_text_maps_known_characters() {
        let bit_matrix_led4 = BitMatrixLed4::from_text(&['1', '2', '3', '4']);
        assert_eq!(bit_matrix_led4.iter().count(), CELL_COUNT);
    }
}
