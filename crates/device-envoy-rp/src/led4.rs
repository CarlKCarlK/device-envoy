//! A device abstraction for a 4-digit, 7-segment LED display for text with optional animation and blinking.
//!
//! See [`Led4`] for the primary text/blinking example and [`Led4::animate_text`] for the animation example.
//!
//! This module provides device abstraction for controlling common-cathode
//! 4-digit 7-segment LED displays. Supports displaying text and numbers with
//! optional blinking.

use embassy_executor::Spawner;

use crate::{Error, Result};
use device_envoy_core::led4::{run_command_loop, signal_animation, signal_text};

#[cfg(feature = "display-trace")]
use defmt::info;

// ============================================================================
// Led4Simple Submodule (internal helper)
// ============================================================================

pub(crate) mod led4_simple;
use self::led4_simple::{Led4Simple, Led4SimpleStatic};

// ============================================================================
// OutputArray Submodule
// ============================================================================

mod output_array;
pub use device_envoy_core::led4::{AnimationFrame, BlinkState, circular_outline_animation};
pub use output_array::OutputArray;

// ============================================================================
// Constants
// ============================================================================

/// The number of cells (digits) in the display.
pub(crate) const CELL_COUNT_U8: u8 = 4;
pub(crate) const CELL_COUNT: usize = CELL_COUNT_U8 as usize;

/// The number of segments per digit in the display.
pub(crate) const SEGMENT_COUNT: usize = 8;

// ============================================================================
// Led4 Virtual Device
// ============================================================================

/// A device abstraction for a 4-digit, 7-segment LED display with blinking support.
///
/// # Hardware Requirements
///
/// This abstraction is designed for common-cathode 7-segment displays where:
/// - Cell pins control which digit is active (LOW = on, HIGH = off)
/// - Segment pins control which segments light up (HIGH = on, LOW = off)
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::{Error, led4::{BlinkState, Led4, Led4Static, OutputArray}};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(p: embassy_rp::Peripherals, spawner: embassy_executor::Spawner) -> Result<(), Error> {
///     // Set up cell pins (control which digit is active)
///     let cells = OutputArray::new([
///         embassy_rp::gpio::Output::new(p.PIN_1, embassy_rp::gpio::Level::High),
///         embassy_rp::gpio::Output::new(p.PIN_2, embassy_rp::gpio::Level::High),
///         embassy_rp::gpio::Output::new(p.PIN_3, embassy_rp::gpio::Level::High),
///         embassy_rp::gpio::Output::new(p.PIN_4, embassy_rp::gpio::Level::High),
///     ]);
///
///     // Set up segment pins (control which segments light up)
///     let segments = OutputArray::new([
///         embassy_rp::gpio::Output::new(p.PIN_5, embassy_rp::gpio::Level::Low),  // Segment A
///         embassy_rp::gpio::Output::new(p.PIN_6, embassy_rp::gpio::Level::Low),  // Segment B
///         embassy_rp::gpio::Output::new(p.PIN_7, embassy_rp::gpio::Level::Low),  // Segment C
///         embassy_rp::gpio::Output::new(p.PIN_8, embassy_rp::gpio::Level::Low),  // Segment D
///         embassy_rp::gpio::Output::new(p.PIN_9, embassy_rp::gpio::Level::Low),  // Segment E
///         embassy_rp::gpio::Output::new(p.PIN_10, embassy_rp::gpio::Level::Low), // Segment F
///         embassy_rp::gpio::Output::new(p.PIN_11, embassy_rp::gpio::Level::Low), // Segment G
///         embassy_rp::gpio::Output::new(p.PIN_12, embassy_rp::gpio::Level::Low), // Decimal point
///     ]);
///
///     // Create the display
///     static LED4_STATIC: Led4Static = Led4::new_static();
///     let display = Led4::new(&LED4_STATIC, cells, segments, spawner)?;
///
///     // Display "1234" (solid)
///     display.write_text(['1', '2', '3', '4'], BlinkState::Solid);
///     
///     // Display "rUSt" blinking
///     display.write_text(['r', 'U', 'S', 't'], BlinkState::BlinkingAndOn);
///     
///     Ok(())
/// }
/// ```
///
/// Beyond simple text, the driver can loop animations via [`Led4::animate_text`].
/// The struct owns the background task and signal wiring; create it once with
/// [`Led4::new`] and use the returned handle for all display updates.
pub struct Led4<'a>(&'a Led4OuterStatic);

/// Signal for sending display commands to the [`Led4`] device.
pub(crate) type Led4OuterStatic = device_envoy_core::led4::Led4CommandSignal;

/// Static for the [`Led4`] device.
pub struct Led4Static {
    outer: Led4OuterStatic,
    display: Led4SimpleStatic,
}

impl Led4Static {
    /// Creates static resources for the 4-digit LED display device.
    pub(crate) const fn new() -> Self {
        Self {
            outer: device_envoy_core::led4::Led4CommandSignal::new(),
            display: Led4Simple::new_static(),
        }
    }

    fn split(&self) -> (&Led4OuterStatic, &Led4SimpleStatic) {
        (&self.outer, &self.display)
    }
}

impl Led4<'_> {
    /// Creates the display device and spawns its background task; see [`Led4`] docs.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led4_static: &'static Led4Static,
        cell_pins: OutputArray<'static, CELL_COUNT>,
        segment_pins: OutputArray<'static, SEGMENT_COUNT>,
        spawner: Spawner,
    ) -> Result<Self> {
        let (outer_static, display_static) = led4_static.split();
        let display = Led4Simple::new(display_static, cell_pins, segment_pins, spawner)?;
        let token = device_loop(outer_static, display);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(outer_static))
    }

    /// Creates static channel resources for [`Led4::new`]; see [`Led4`] docs.
    #[must_use]
    pub const fn new_static() -> Led4Static {
        Led4Static::new()
    }

    /// Sends text to the display with optional blinking.
    ///
    /// See the main [`Led4`] example for end-to-end usage.
    pub fn write_text(&self, text: [char; CELL_COUNT], blink_state: BlinkState) {
        #[cfg(feature = "display-trace")]
        info!("blink_state: {:?}, text: {:?}", blink_state, text);
        signal_text(self.0, text, blink_state);
    }

    /// Plays a looped text animation using the provided frames.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// # use panic_probe as _;
    /// # use embassy_rp::gpio::{Level, Output};
    /// # use embassy_executor::Spawner;
    /// use device_envoy_rp::{Result, led4::{AnimationFrame, Led4, Led4Static, OutputArray}};
    /// use embassy_time::Duration;
    /// async fn demo(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    ///     let cells = OutputArray::new([
    ///         Output::new(p.PIN_1, Level::High),
    ///         Output::new(p.PIN_2, Level::High),
    ///         Output::new(p.PIN_3, Level::High),
    ///         Output::new(p.PIN_4, Level::High),
    ///     ]);
    ///     let segments = OutputArray::new([
    ///         Output::new(p.PIN_5, Level::Low),
    ///         Output::new(p.PIN_6, Level::Low),
    ///         Output::new(p.PIN_7, Level::Low),
    ///         Output::new(p.PIN_8, Level::Low),
    ///         Output::new(p.PIN_9, Level::Low),
    ///         Output::new(p.PIN_10, Level::Low),
    ///         Output::new(p.PIN_11, Level::Low),
    ///         Output::new(p.PIN_12, Level::Low),
    ///     ]);
    ///     static LED4_STATIC: Led4Static = Led4::new_static();
    ///     let display = Led4::new(&LED4_STATIC, cells, segments, spawner)?;
    ///     const FRAME_DURATION: Duration = Duration::from_millis(120);
    ///     let animation = [
    ///         AnimationFrame::new(['-', '-', '-', '-'], FRAME_DURATION),
    ///         AnimationFrame::new([' ', ' ', ' ', ' '], FRAME_DURATION),
    ///         AnimationFrame::new(['1', '2', '3', '4'], FRAME_DURATION),
    ///     ];
    ///     display.animate_text(animation);
    ///     Ok(())
    /// }
    /// ```
    /// See the example below for how to build animations.
    pub fn animate_text<I>(&self, animation: I)
    where
        I: IntoIterator,
        I::Item: core::borrow::Borrow<AnimationFrame>,
    {
        signal_animation(self.0, animation);
    }
}

#[embassy_executor::task]
async fn device_loop(outer_static: &'static Led4OuterStatic, display: Led4Simple<'static>) -> ! {
    run_command_loop(outer_static, |text| display.write_text(text)).await
}
