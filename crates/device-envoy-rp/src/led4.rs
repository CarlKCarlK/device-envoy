//! A device abstraction for a 4-digit, 7-segment LED display for text with optional animation and blinking.
//!
//! See [`Led4Rp`] for the primary text/blinking example and [`Led4`] for trait methods.
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
pub use device_envoy_core::led4::{AnimationFrame, BlinkState, Led4, circular_outline_animation};
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
/// use embassy_time::{Duration, Timer};
/// use device_envoy_rp::{Error, led4::{BlinkState, Led4 as _, Led4Rp, Led4RpStatic, OutputArray, circular_outline_animation}};
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
///     static LED4_STATIC: Led4RpStatic = Led4Rp::new_static();
///     let display = Led4Rp::new(&LED4_STATIC, cells, segments, spawner)?;
///
///     // Blink "1234" for three seconds.
///     display.write_text(['1', '2', '3', '4'], BlinkState::BlinkingAndOn);
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Run the circular outline animation for three seconds.
///     display.animate_text(circular_outline_animation(true));
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Show "rUSt" solid forever.
///     display.write_text(['r', 'U', 'S', 't'], BlinkState::Solid);
///     core::future::pending().await
/// }
/// ```
///
/// Beyond simple text, the driver can loop animations via [`Led4::animate_text`].
/// The struct owns the background task and signal wiring; create it once with
/// [`Led4Rp::new`] and use the returned handle for all display updates.
pub struct Led4Rp<'a>(&'a Led4RpOuterStatic);

/// Signal for sending display commands to the [`Led4Rp`] device.
pub(crate) type Led4RpOuterStatic = device_envoy_core::led4::Led4CommandSignal;

/// Static for the [`Led4Rp`] device.
pub struct Led4RpStatic {
    outer: Led4RpOuterStatic,
    display: Led4SimpleStatic,
}

impl Led4RpStatic {
    /// Creates static resources for the 4-digit LED display device.
    pub(crate) const fn new() -> Self {
        Self {
            outer: device_envoy_core::led4::Led4CommandSignal::new(),
            display: Led4Simple::new_static(),
        }
    }

    fn split(&self) -> (&Led4RpOuterStatic, &Led4SimpleStatic) {
        (&self.outer, &self.display)
    }
}

impl Led4Rp<'_> {
    /// Creates the display device and spawns its background task; see [`Led4Rp`] docs.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led4_static: &'static Led4RpStatic,
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

    /// Creates static channel resources for [`Led4Rp::new`]; see [`Led4Rp`] docs.
    #[must_use]
    pub const fn new_static() -> Led4RpStatic {
        Led4RpStatic::new()
    }
}

impl device_envoy_core::led4::Led4 for Led4Rp<'_> {
    fn write_text(&self, text: [char; CELL_COUNT], blink_state: BlinkState) {
        #[cfg(feature = "display-trace")]
        info!("blink_state: {:?}, text: {:?}", blink_state, text);
        signal_text(self.0, text, blink_state);
    }

    fn animate_text<I>(&self, animation: I)
    where
        I: IntoIterator,
        I::Item: core::borrow::Borrow<AnimationFrame>,
    {
        signal_animation(self.0, animation);
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn device_loop(outer_static: &'static Led4RpOuterStatic, display: Led4Simple<'static>) -> ! {
    run_command_loop(outer_static, |text| display.write_text(text)).await
}
