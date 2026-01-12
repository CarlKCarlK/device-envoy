//! A device abstraction for NeoPixel-style (WS2812) LED strips.
//!
//! - See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for LED-strip usage.
//! - See [`Led2dGenerated`](`crate::led2d::led2d_generated::Led2dGenerated`) for 2D panel usage.

/// Predefined RGB color constants from the `smart_leds` crate.
///
/// Common colors include `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`.
#[doc(inline)]
pub use smart_leds::colors;

// ============================================================================
// Gamma Correction
// ============================================================================

/// Gamma correction mode for LED strips.
///
/// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gamma {
    /// Linear gamma (no correction). Gamma = 1.0
    Linear,
    /// Standard gamma 2.2 correction for perceived brightness.
    Gamma2_2,
}

impl Default for Gamma {
    fn default() -> Self {
        Self::Gamma2_2
    }
}

/// Gamma 2.2 lookup table for 8-bit values.
/// Pre-computed to avoid floating point math: corrected = (value/255)^2.2 * 255
#[allow(dead_code)]
pub(crate) const GAMMA_2_2_TABLE: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2,
    3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11,
    11, 12, 12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 22, 22, 23,
    23, 24, 25, 25, 26, 26, 27, 28, 28, 29, 30, 30, 31, 32, 33, 33, 34, 35, 35, 36, 37, 38, 39, 39,
    40, 41, 42, 43, 43, 44, 45, 46, 47, 48, 49, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61,
    62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 81, 82, 83, 84, 85, 87, 88,
    89, 90, 91, 93, 94, 95, 97, 98, 99, 100, 102, 103, 105, 106, 107, 109, 110, 111, 113, 114, 116,
    117, 119, 120, 121, 123, 124, 126, 127, 129, 130, 132, 133, 135, 137, 138, 140, 141, 143, 145,
    146, 148, 149, 151, 153, 154, 156, 158, 159, 161, 163, 165, 166, 168, 170, 172, 173, 175, 177,
    179, 181, 182, 184, 186, 188, 190, 192, 194, 196, 197, 199, 201, 203, 205, 207, 209, 211, 213,
    215, 217, 219, 221, 223, 225, 227, 229, 231, 234, 236, 238, 240, 242, 244, 246, 248, 251, 253,
    255,
];

/// Linear lookup table (identity function).
#[allow(dead_code)]
const LINEAR_TABLE: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];

/// Generate a combined gamma correction and brightness scaling lookup table.
///
/// This combines two operations into a single table lookup for efficiency:
/// 1. Apply gamma correction based on the `gamma` parameter
/// 2. Scale by `max_brightness` for current limiting
///
/// The result is a table where `combo_table[input_value]` gives the final output value.
#[doc(hidden)] // Implementation detail used by macro-generated strip types
#[must_use]
#[allow(dead_code)]
pub const fn generate_combo_table(gamma: Gamma, max_brightness: u8) -> [u8; 256] {
    let gamma_table = match gamma {
        Gamma::Linear => &LINEAR_TABLE,
        Gamma::Gamma2_2 => &GAMMA_2_2_TABLE,
    };

    let mut result = [0u8; 256];
    let mut index = 0;
    while index < 256 {
        let gamma_corrected = gamma_table[index];
        // Apply brightness scaling: (value * brightness) / 255
        let scaled = ((gamma_corrected as u16 * max_brightness as u16) / 255) as u8;
        result[index] = scaled;
        index += 1;
    }
    result
}

use core::cell::RefCell;
use embassy_futures::select::{Either, select};
use embassy_rp::pio::{Common, Instance};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel as EmbassyChannel;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use smart_leds::RGB8;

use crate::Result;

// ============================================================================
// Submodules
// ============================================================================

pub mod led_strip_generated;

/// RGB color representation re-exported from the `smart_leds` crate.
pub type Rgb = RGB8;

/// Frame1d of [`Rgb`] values for an LED strip.
///
/// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
///
/// Frames deref to `[Rgb; N]`, so you can mutate pixels directly before passing them to the generated strip's `write_frame` method.
#[derive(Clone, Copy, Debug)]
pub struct Frame1d<const N: usize>(pub [Rgb; N]);

impl<const N: usize> Frame1d<N> {
    /// Create a new blank (all black) frame.
    ///
    /// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
    #[must_use]
    pub const fn new() -> Self {
        Self([Rgb::new(0, 0, 0); N])
    }

    /// Create a frame filled with a single color.
    ///
    /// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
    #[must_use]
    pub const fn filled(color: Rgb) -> Self {
        Self([color; N])
    }

    /// Get the number of LEDs in this frame.
    #[must_use]
    pub const fn len() -> usize {
        N
    }
}

impl<const N: usize> core::ops::Deref for Frame1d<N> {
    type Target = [Rgb; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> core::ops::DerefMut for Frame1d<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> From<[Rgb; N]> for Frame1d<N> {
    fn from(array: [Rgb; N]) -> Self {
        Self(array)
    }
}

impl<const N: usize> From<Frame1d<N>> for [Rgb; N] {
    fn from(frame: Frame1d<N>) -> Self {
        frame.0
    }
}

impl<const N: usize> Default for Frame1d<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PIO Bus - Shared PIO resource for multiple LED strips
// ============================================================================

/// Trait for PIO peripherals that can be used with LED strips.
///
/// This trait is automatically implemented by the `led_strips!` macro
/// for the PIO peripheral specified in the macro invocation.
#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub trait LedStripPio: Instance {
    /// The interrupt binding type for this PIO
    type Irqs: embassy_rp::interrupt::typelevel::Binding<
            <Self as Instance>::Interrupt,
            embassy_rp::pio::InterruptHandler<Self>,
        >;

    /// Get the interrupt configuration
    fn irqs() -> Self::Irqs;
}
/// A state machine bundled with its PIO bus.
///
/// This is returned by `pio_split!` and passed to strip constructors.
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioStateMachine<PIO: Instance + 'static, const SM: usize> {
    bus: &'static PioBus<'static, PIO>,
    sm: embassy_rp::pio::StateMachine<'static, PIO, SM>,
}
// cmk should spell out sm and name bus pio_bus, this this be PioBusStateMachine?ks

impl<PIO: Instance + 'static, const SM: usize> PioStateMachine<PIO, SM> {
    #[doc(hidden)]
    pub fn new(
        bus: &'static PioBus<'static, PIO>,
        sm: embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) -> Self {
        Self { bus, sm }
    }

    #[doc(hidden)]
    pub fn bus(&self) -> &'static PioBus<'static, PIO> {
        self.bus
    }

    #[doc(hidden)]
    pub fn into_parts(
        self,
    ) -> (
        &'static PioBus<'static, PIO>,
        embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) {
        (self.bus, self.sm)
    }
}
/// Shared PIO bus that manages the Common resource and WS2812 program
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioBus<'d, PIO: Instance> {
    common: Mutex<CriticalSectionRawMutex, RefCell<Common<'d, PIO>>>,
    ws2812_program: OnceLock<PioWs2812Program<'d, PIO>>,
}

impl<'d, PIO: Instance> PioBus<'d, PIO> {
    /// Create a new PIO bus with the given Common resource
    pub fn new(common: Common<'d, PIO>) -> Self {
        Self {
            common: Mutex::new(RefCell::new(common)),
            ws2812_program: OnceLock::new(),
        }
    }

    /// Get or initialize the WS2812 program (only loaded once)
    pub fn get_program(&'static self) -> &'static PioWs2812Program<'d, PIO> {
        self.ws2812_program.get_or_init(|| {
            self.common.lock(|common_cell: &RefCell<Common<'d, PIO>>| {
                let mut common = common_cell.borrow_mut();
                PioWs2812Program::new(&mut *common)
            })
        })
    }

    /// Access the common resource for initializing a driver
    pub fn with_common<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Common<'d, PIO>) -> R,
    {
        self.common.lock(|common_cell: &RefCell<Common<'d, PIO>>| {
            let mut common = common_cell.borrow_mut();
            f(&mut *common)
        })
    }
}

// ============================================================================
// LED Strip Command Channel and Static
// ============================================================================

#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub type LedStripCommands<const N: usize> = EmbassyChannel<CriticalSectionRawMutex, Frame1d<N>, 2>;

#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub type LedStripCommandSignal<const N: usize, const MAX_FRAMES: usize> =
    Signal<CriticalSectionRawMutex, Command<N, MAX_FRAMES>>;

#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub type LedStripCompletionSignal = Signal<CriticalSectionRawMutex, ()>;

#[doc(hidden)]
// Command for the LED strip animation loop.
#[derive(Clone)]
pub enum Command<const N: usize, const MAX_FRAMES: usize> {
    DisplayStatic(Frame1d<N>),
    Animate(Vec<(Frame1d<N>, Duration), MAX_FRAMES>),
}

/// Static used to construct LED strip instances with animation support.
#[doc(hidden)] // Must be pub for method signatures and macro expansion in downstream crates
pub struct LedStripStatic<const N: usize, const MAX_FRAMES: usize> {
    command_signal: LedStripCommandSignal<N, MAX_FRAMES>,
    completion_signal: LedStripCompletionSignal,
    commands: LedStripCommands<N>,
}

impl<const N: usize, const MAX_FRAMES: usize> LedStripStatic<N, MAX_FRAMES> {
    /// Creates static resources.
    #[must_use]
    #[doc(hidden)]
    pub const fn new_static() -> Self {
        Self {
            command_signal: Signal::new(),
            completion_signal: Signal::new(),
            commands: LedStripCommands::new(),
        }
    }

    #[doc(hidden)]
    pub fn command_signal(&'static self) -> &'static LedStripCommandSignal<N, MAX_FRAMES> {
        &self.command_signal
    }

    #[doc(hidden)]
    pub fn completion_signal(&'static self) -> &'static LedStripCompletionSignal {
        &self.completion_signal
    }

    #[doc(hidden)]
    pub fn commands(&'static self) -> &'static LedStripCommands<N> {
        &self.commands
    }
}

// cmk0000 need to described this better. It is kind of a prototype.
/// Internal deref target for generated LED strip types.
///
/// All LED strip methods are available through macro-generated types.
/// See [`led_strip!`] macro documentation for usage.
pub struct LedStrip<const N: usize, const MAX_FRAMES: usize> {
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static LedStripCompletionSignal,
}

impl<const N: usize, const MAX_FRAMES: usize> LedStrip<N, MAX_FRAMES> {
    /// Creates LED strip resources.
    #[must_use]
    #[doc(hidden)]
    pub const fn new_static() -> LedStripStatic<N, MAX_FRAMES> {
        LedStripStatic::new_static()
    }

    // cmk0000 should hide this
    /// Creates a new LED strip controller bound to the given static resources.
    pub fn new(led_strip_static: &'static LedStripStatic<N, MAX_FRAMES>) -> Result<Self> {
        Ok(Self {
            command_signal: led_strip_static.command_signal(),
            completion_signal: led_strip_static.completion_signal(),
        })
    }

    /// Writes a full frame to the LED strip. It remains displayed until another command
    /// replaces it.
    ///
    /// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for example usage.
    pub async fn write_frame(&self, frame: Frame1d<N>) -> Result<()> {
        self.command_signal.signal(Command::DisplayStatic(frame));
        self.completion_signal.wait().await;
        Ok(())
    }

    /// Loop forever through a sequence of animation frames.
    /// They remain displayed until another command replaces them.
    ///
    /// Each frame is a tuple of `(Frame1d, Duration)`. Accepts arrays, `Vec`s, or any
    /// iterator that produces `(Frame1d, Duration)` tuples.
    ///
    /// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for example usage.
    pub async fn animate(
        &self,
        frames: impl IntoIterator<Item = (Frame1d<N>, Duration)>,
    ) -> Result<()> {
        assert!(
            MAX_FRAMES > 0,
            "max_frames must be positive for LED strip animations"
        );
        let mut sequence: Vec<(Frame1d<N>, Duration), MAX_FRAMES> = Vec::new();
        for (frame, duration) in frames {
            assert!(
                duration.as_micros() > 0,
                "animation frame duration must be positive"
            );
            sequence
                .push((frame, duration))
                .expect("animation sequence fits within MAX_FRAMES");
        }
        assert!(
            !sequence.is_empty(),
            "animation requires at least one frame"
        );
        self.command_signal.signal(Command::Animate(sequence));
        self.completion_signal.wait().await;
        Ok(())
    }
}

#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub async fn led_strip_animation_loop<
    PIO,
    const SM: usize,
    const N: usize,
    const MAX_FRAMES: usize,
    ORDER,
>(
    mut driver: PioWs2812<'static, PIO, SM, N, ORDER>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static LedStripCompletionSignal,
    combo_table: &'static [u8; 256],
) -> !
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    loop {
        let command = command_signal.wait().await;
        command_signal.reset();

        match command {
            Command::DisplayStatic(frame) => {
                let mut corrected_frame = frame;
                apply_correction(&mut corrected_frame, combo_table);
                driver.write(&corrected_frame).await;
                completion_signal.signal(());
            }
            Command::Animate(frames) => {
                let next_command = run_frame_animation(
                    &mut driver,
                    frames,
                    command_signal,
                    completion_signal,
                    combo_table,
                )
                .await;
                command_signal.reset();
                match next_command {
                    Command::DisplayStatic(frame) => {
                        let mut corrected_frame = frame;
                        apply_correction(&mut corrected_frame, combo_table);
                        driver.write(&corrected_frame).await;
                        completion_signal.signal(());
                    }
                    Command::Animate(_) => {
                        // Loop back to process new animation
                        continue;
                    }
                }
            }
        }
    }
}

async fn run_frame_animation<PIO, const SM: usize, const N: usize, const MAX_FRAMES: usize, ORDER>(
    driver: &mut PioWs2812<'static, PIO, SM, N, ORDER>,
    frames: Vec<(Frame1d<N>, Duration), MAX_FRAMES>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static LedStripCompletionSignal,
    combo_table: &'static [u8; 256],
) -> Command<N, MAX_FRAMES>
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    completion_signal.signal(());

    loop {
        for (frame, duration) in &frames {
            let mut corrected_frame = *frame;
            apply_correction(&mut corrected_frame, combo_table);
            driver.write(&corrected_frame).await;

            match select(command_signal.wait(), Timer::after(*duration)).await {
                Either::First(new_command) => {
                    return new_command;
                }
                Either::Second(()) => continue,
            }
        }
    }
}

fn apply_correction<const N: usize>(frame: &mut Frame1d<N>, combo_table: &[u8; 256]) {
    for color in frame.iter_mut() {
        *color = Rgb::new(
            combo_table[usize::from(color.r)],
            combo_table[usize::from(color.g)],
            combo_table[usize::from(color.b)],
        );
    }
}

/// Macro to generate multiple NeoPixel-style (WS2812) LED strips sharing one PIO resource.
///
/// Use this macro to drive multiple independent LED strips and 2D panels from a single shared [PIO](crate#glossary) resource.
/// This saves hardware resources compared to using separate [`led_strip!`] and [`led2d!`](macro@crate::led2d) invocations, each of which consumes a dedicated PIO resource.
///
/// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) and
/// [`Led2dGenerated`](crate::led2d::led2d_generated::Led2dGenerated) for details on using the generated types.
///
/// **Example:**
///
/// ```no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use core::convert::Infallible;
/// # use core::future;
/// # use defmt_rtt as _;
/// # use embassy_executor::Spawner;
/// # use defmt::info;
/// use device_kit::{Result, led2d::layout::LedLayout, led_strip::{Current, Gamma, colors, led_strips}};
/// use embassy_time::Duration;
///
/// // Our 2D panel is two 12x4 panels stacked vertically.
/// const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
/// const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
/// const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
///
/// led_strips! {
///     pio: PIO0,                          // Optional; defaults to PIO0.
///     LedStrips0 {
///         gpio0: {                        // Prefix used to name generated types.
///             pin: PIN_0,                 // GPIO pin for LED data signal.
///             len: 8,                     // 8 LEDs on this strip.
///             max_current: Current::Milliamps(25), // Optional; default 250 mA.
///         },
///         gpio3: {
///             pin: PIN_3,
///             len: 48,
///             max_current: Current::Milliamps(75),
///             gamma: Gamma::Gamma2_2,     // Optional; default Gamma2_2.
///             max_frames: 1,              // Optional; default 16.
///             dma: DMA_CH11,              // Optional; auto-assigned by strip order.
///         },
///         gpio4: {
///             pin: PIN_4,
///             len: 96,
///             max_frames: 2,
///             led2d: {                    // Optional panel configuration for 2D displays.
///                 width: 8,               // Panel width in LEDs.
///                 height: 12,             // Panel height in LEDs.
///                 led_layout: LED_LAYOUT_12X8_ROTATED, // Two 12×4 panels stacked and rotated.
///                 font: Font4x6Trim,      // 4x6 pixel font without the usual 1 pixel spacing.
///             }
///         },
///     }
/// }
///
/// # #[embassy_executor::main]
/// # async fn main(spawner: Spawner) -> ! {
/// #     let _ = example(spawner).await;
/// #     core::panic!("done");
/// # }
/// async fn example(spawner: Spawner) -> Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///
///     // Create the two LED strips and one panel on GPIO0, GPIO3, and GPIO4.
///     let (gpio0_led_strip, gpio3_led_strip, gpio4_led2d) = LedStrips0::new(
///         p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH11, p.PIN_4, p.DMA_CH2, spawner,
///     )?;
///
///     // Turn on all-white on GPIO0 strip.
///     let frame_gpio0 = Gpio0Frame::filled(colors::WHITE);
///     gpio0_led_strip.write_frame(frame_gpio0).await?;
///
///     // Turn on every other LED in blue on GPIO3 strip.
///     let mut frame_gpio3 = Gpio3Frame::new();
///     for pixel_index in (0..frame_gpio3.len()).step_by(2) {
///         frame_gpio3[pixel_index] = colors::BLUE;
///     }
///     gpio3_led_strip.write_frame(frame_gpio3).await?;
///
///     // Animate "Go Go" text on GPIO4 2D panel.
///     let mut frame_go_top = Gpio4Led2dFrame::new();
///     gpio4_led2d.write_text_to_frame("Go", &[], &mut frame_go_top)?;
///
///     let mut frame_go_bottom = Gpio4Led2dFrame::new();
///     gpio4_led2d.write_text_to_frame(
///         "\nGo",
///         &[colors::HOT_PINK, colors::LIME],
///         &mut frame_go_bottom,
///     )?;
///
///     let frame_duration = Duration::from_millis(400);
///     gpio4_led2d
///         .animate([
///             (frame_go_top, frame_duration),
///             (frame_go_bottom, frame_duration),
///         ])
///         .await?;
///
///     future::pending::<Result<Infallible>>().await // Run forever
/// }
/// ```
///
/// **Required fields per strip:**
///
/// - `pin` — GPIO pin for LED data
/// - `len` — Number of LEDs
///
/// **Optional fields per strip:**
///
/// - `dma` — DMA channel (default: auto-assigned by state machine index)
/// - `max_current` — Current budget (default: `250` mA via [`Current::Milliamps`])
/// - `gamma` — Color curve (default: [`Gamma::Gamma2_2`])
/// - `max_frames` — Maximum animation frames (default: `16`)
/// - `led2d` — 2D layout configuration (optional, for matrix-style displays)
/// // cmk0000 need more on 2d
///
/// // cmk00000 need to tell everything the macro defines
///
/// # Current Limiting cmk000 need to update this
///
/// The `max_current` field automatically scales brightness to stay within your power budget.
/// Each strip can have independent current limits, or they can share a single budget across
/// all strips in the group.
///
/// Each WS2812 LED is assumed to draw 60 mA at full brightness. For example:
/// - 8 LEDs × 60 mA = 480 mA at full brightness
/// - With `max_current: Current::Milliamps(500)` on a strip, all LEDs fit at 100% brightness
/// - With `max_current: Current::Milliamps(250)` (the default), the generated `MAX_BRIGHTNESS` limits LEDs to ~52% brightness
///
/// The current limit is baked into a compile-time lookup table, so it has no runtime cost.
///
/// **Powering LEDs from the Pico's pin 40 (VBUS):** Pin 40 is the USB 5 V rail
/// pass-through, but the Pico itself has practical current limits — the USB connector,
/// cable, and internal circuitry aren't designed for heavy loads. Small LED strips
/// (a few hundred mA total) can usually power from pin 40 with a decent USB supply; for
/// larger loads (1 A+), use a separate 5 V supply and share ground with the Pico.
///
/// # Color Correction (Gamma)
///
/// The `gamma` field applies a color response curve to make colors look more natural:
///
/// - [`Gamma::Linear`] — No correction (raw values)
/// - [`Gamma::Gamma2_2`] — Standard sRGB curve (default, most natural-looking)
///
/// The gamma curve is baked into a compile-time lookup table, so it has no runtime cost.
///
/// # When to Use This Macro
///
/// Use `led_strips!` when:
/// - You have **multiple independent LED strips** connected to separate GPIO pins
/// - They should **share the same PIO resource** (to save PIO resources)
/// - Each strip can have independent brightness, animation, and current limits
///
/// Each `led_strips!` invocation can drive up to 4 independent strips on one [PIO](crate#glossary) resource.
/// On a Pico 2 with 3 PIOs available, you can use **3 instances of `led_strips!`** to drive **up to 12 independent LED strips**.
///
/// If you only have **one strip** (or two), use [`led_strip!`] instead for simpler syntax.
#[macro_export]
macro_rules! led_strips {
    ($($tt:tt)*) => { $crate::__led_strips_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led_strips!`] instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strips_impl {
    // Internal: full expansion with all fields specified
    (@__expand
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips: [
            $(
                $label:ident {
                    sm: $sm_index:expr,
                    dma: $dma:ident,
                    pin: $pin:ident,
                    len: $len:expr,
                    max_current: $max_current:expr,
                    gamma: $gamma:expr,
                    max_frames: $max_frames:expr
                    $(,
                        led2d: {
                            width: $led2d_width:expr,
                            height: $led2d_height:expr,
                            led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                            max_frames: $led2d_max_frames:expr,
                            font: $led2d_font:ident $(,)?
                        }
                    )?
                }
            ),+ $(,)?
        ]
    ) => {
        // Use crate-level PIO interrupt bindings (Pio0Irqs, Pio1Irqs, Pio2Irqs)
        paste::paste! {
            // Create the PIO bus
            #[allow(non_upper_case_globals)]
            static [<$pio _BUS>]: ::static_cell::StaticCell<
                $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>
            > = ::static_cell::StaticCell::new();

            /// Split the PIO into bus and state machines.
            ///
            /// Returns 4 StateMachines (one for each SM)
            #[allow(dead_code)]
            pub fn [<$pio:lower _split>](
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
            ) -> (
                $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, 0>,
                $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, 1>,
                $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, 2>,
                $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, 3>,
            ) {
                let ::embassy_rp::pio::Pio { common, sm0, sm1, sm2, sm3, .. } =
                    ::embassy_rp::pio::Pio::new(pio, <::embassy_rp::peripherals::$pio as $crate::led_strip::LedStripPio>::irqs());
                let pio_bus = [<$pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                (
                    $crate::led_strip::PioStateMachine::new(pio_bus, sm0),
                    $crate::led_strip::PioStateMachine::new(pio_bus, sm1),
                    $crate::led_strip::PioStateMachine::new(pio_bus, sm2),
                    $crate::led_strip::PioStateMachine::new(pio_bus, sm3),
                )
            }


        }

        paste::paste! {
            // Create strip types
            $(
                #[doc = concat!(
                    "LED strip wrapper generated by [`led_strips!`].\n\n",
                    "Derefs to provide all LED control methods. ",
                    "Created with [`", stringify!($group), "::new`]. ",
                    "See [`led_strip_generated`] for a similar example."
                )]
                pub struct [<$label:camel LedStrip>] {
                    strip: $crate::led_strip::LedStrip<{ $len }, { $max_frames }>,
                }

                impl [<$label:camel LedStrip>] {
                    pub const LEN: usize = $len;
                    pub const MAX_FRAMES: usize = $max_frames;

                    // Calculate max brightness from current budget
                    // Each WS2812B LED draws ~60mA at full brightness
                    /// cmk00 OK to assume 60 mA per LED
                    const WORST_CASE_MA: u32 = ($len as u32) * 60;
                    pub const MAX_BRIGHTNESS: u8 =
                        $max_current.max_brightness(Self::WORST_CASE_MA);

                    // Combined gamma correction and brightness scaling table
                    const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                    pub(crate) const fn new_static() -> $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> {
                        $crate::led_strip::LedStrip::new_static()
                    }

                    pub fn new(
                        state_machine: $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                        pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                        dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                        spawner: ::embassy_executor::Spawner,
                    ) -> $crate::Result<&'static Self> {
                        static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> = [<$label:camel LedStrip>]::new_static();
                        static STRIP_CELL: ::static_cell::StaticCell<[<$label:camel LedStrip>]> = ::static_cell::StaticCell::new();
                        let pin = pin.into();
                        let dma = dma.into();

                        let (bus, sm) = state_machine.into_parts();
                        let token = [<$group:snake _ $label:snake _animation_task>](
                            bus,
                            sm,
                            dma,
                            pin,
                            STRIP_STATIC.command_signal(),
                            STRIP_STATIC.completion_signal(),
                        )
                        .map_err($crate::Error::TaskSpawn)?;
                        spawner.spawn(token);
                        let strip = $crate::led_strip::LedStrip::new(&STRIP_STATIC)?;
                        let instance = STRIP_CELL.init(Self { strip });
                        Ok(instance)
                    }
                }

                impl ::core::ops::Deref for [<$label:camel LedStrip>] {
                    type Target = $crate::led_strip::LedStrip<{ $len }, { $max_frames }>;

                    fn deref(&self) -> &Self::Target {
                        &self.strip
                    }
                }

                $crate::__led_strips_impl!(
                    @__maybe_strip_frame_alias
                    frame_alias: $frame_alias,
                    label: $label,
                    len: $len
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                );

                #[cfg(not(feature = "host"))]
                impl $crate::led2d::WriteFrame<{ $len }> for [<$label:camel LedStrip>] {
                    async fn write_frame(&self, frame: $crate::led_strip::Frame1d<{ $len }>) -> $crate::Result<()> {
                        self.strip.write_frame(frame).await
                    }
                }

                #[::embassy_executor::task]
                async fn [<$group:snake _ $label:snake _animation_task>](
                    bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                    sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, $sm_index>,
                    dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                    pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                    command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
                    completion_signal: &'static $crate::led_strip::LedStripCompletionSignal,
                ) -> ! {
                    let program = bus.get_program();
                    let driver = bus.with_common(|common| {
                        ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                            ::embassy_rp::peripherals::$pio,
                            $sm_index,
                            { $len },
                            _
                        >::new(common, sm, dma, pin, program)
                    });
                    $crate::led_strip::led_strip_animation_loop::<
                        ::embassy_rp::peripherals::$pio,
                        $sm_index,
                        { $len },
                        { $max_frames },
                        _
                    >(driver, command_signal, completion_signal, &[<$label:camel LedStrip>]::COMBO_TABLE).await
                }

                $(
                    paste::paste! {
                        #[cfg(not(feature = "host"))]
                        $crate::led2d::led2d_from_strip! {
                            pub [<$label:camel Led2d>],
                            strip_type: [<$label:camel LedStrip>],
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }

                        #[cfg(not(feature = "host"))]
                        impl [<$label:camel LedStrip>] {
                        pub fn new_led2d(
                            state_machine: $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                            pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                            dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                            spawner: ::embassy_executor::Spawner,
                        ) -> $crate::Result<[<$label:camel Led2d>]> {
                            let strip = Self::new(state_machine, pin, dma, spawner)?;
                                [<$label:camel Led2d>]::from_strip(strip, spawner)
                            }
                        }
                    }
                )?
            )+

            // Generate the group marker struct with new() constructor
            pub struct $group;

            impl $group {
                #[allow(clippy::too_many_arguments)]
                pub fn new(
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    $(
                        [<$label _pin>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                        [<$label _dma>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    )+
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<(
                    $(
                        $crate::__led_strips_impl!(
                            @__strip_return_type
                            $label
                            $(,
                                led2d: {
                                    width: $led2d_width,
                                    height: $led2d_height,
                                    led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                                    max_frames: $led2d_max_frames,
                                    font: $led2d_font,
                                }
                            )?
                        ),
                    )+
                )> {
                    // Inline PIO splitting
                    let pio_peri = pio.into();
                    let ::embassy_rp::pio::Pio { common, sm0, sm1, sm2, sm3, .. } =
                        ::embassy_rp::pio::Pio::new(pio_peri, <::embassy_rp::peripherals::$pio as $crate::led_strip::LedStripPio>::irqs());
                    let pio_bus = [<$pio _BUS>].init_with(|| {
                        $crate::led_strip::PioBus::new(common)
                    });

                    // Create individual state machine wrappers
                    #[allow(unused_variables)]
                    let sm0_wrapped = $crate::led_strip::PioStateMachine::new(pio_bus, sm0);
                    #[allow(unused_variables)]
                    let sm1_wrapped = $crate::led_strip::PioStateMachine::new(pio_bus, sm1);
                    #[allow(unused_variables)]
                    let sm2_wrapped = $crate::led_strip::PioStateMachine::new(pio_bus, sm2);
                    #[allow(unused_variables)]
                    let sm3_wrapped = $crate::led_strip::PioStateMachine::new(pio_bus, sm3);

                    // Construct each strip with the appropriate SM
                    Ok((
                        $(
                            $crate::__led_strips_impl!(
                                @__strip_return_value
                                label: $label,
                                state_machine: $crate::__led_strips_impl!(
                                    @__select_sm
                                    $sm_index,
                                    sm0_wrapped,
                                    sm1_wrapped,
                                    sm2_wrapped,
                                    sm3_wrapped
                                ),
                                pin: [<$label _pin>],
                                dma: [<$label _dma>],
                                spawner: spawner
                                $(,
                                    led2d: {
                                        width: $led2d_width,
                                        height: $led2d_height,
                                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                                        max_frames: $led2d_max_frames,
                                        font: $led2d_font,
                                    }
                                )?
                            ),
                        )+
                    ))
                }
            }
        }
    };

    // Helper to select the right SM based on index
    (@__select_sm 0, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm0 };
    (@__select_sm 1, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm1 };
    (@__select_sm 2, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm2 };
    (@__select_sm 3, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm3 };

    (@__strip_return_type $label:ident) => {
        paste::paste! { &'static [<$label:camel LedStrip>] }
    };
    (@__strip_return_type $label:ident, led2d: { $($led2d_fields:tt)* }) => {
        paste::paste! { [<$label:camel Led2d>] }
    };

    (@__strip_frame_alias
        label: $label:ident,
        len: $len:expr
    ) => {
        paste::paste! {
            pub type [<$label:camel Frame>] = $crate::led_strip::Frame1d<{ $len }>;
        }
    };

    (@__maybe_strip_frame_alias
        frame_alias: __WITH_FRAME_ALIAS__,
        label: $label:ident,
        len: $len:expr
        $(, led2d: { $($led2d_fields:tt)* } )?
    ) => {
        $crate::__led_strips_impl!(
            @__strip_frame_alias
            label: $label,
            len: $len
        );
    };
    (@__maybe_strip_frame_alias
        frame_alias: __SKIP_FRAME_ALIAS__,
        label: $label:ident,
        len: $len:expr
        $(, led2d: { $($led2d_fields:tt)* } )?
    ) => {};

    (@__strip_return_value
        label: $label:ident,
        state_machine: $state_machine:expr,
        pin: $pin:ident,
        dma: $dma:ident,
        spawner: $spawner:ident
    ) => {
        paste::paste! {
            [<$label:camel LedStrip>]::new($state_machine, $pin, $dma, $spawner)?
        }
    };
    (@__strip_return_value
        label: $label:ident,
        state_machine: $state_machine:expr,
        pin: $pin:ident,
        dma: $dma:ident,
        spawner: $spawner:ident,
        led2d: { $($led2d_fields:tt)* }
    ) => {
        paste::paste! {{
            let [<$label:snake _led_strip>] =
                [<$label:camel LedStrip>]::new($state_machine, $pin, $dma, $spawner)?;
            [<$label:camel Led2d>]::from_strip([<$label:snake _led_strip>], $spawner)?
        }}
    };

    // Entry point with explicit pio and group syntax
    (@__with_frame_alias
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    (
        pio: $pio:ident,
        $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: __WITH_FRAME_ALIAS__,
            pio: $pio,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    // Entry point without pio (defaults to PIO0) with group syntax
    (@__with_frame_alias
        frame_alias: $frame_alias:tt,
        $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: $frame_alias,
            pio: PIO0,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    (
        $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: __WITH_FRAME_ALIAS__,
            pio: PIO0,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    // Process strips one at a time, adding defaults
    (@__with_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_in: [ $label:ident: { $($fields:tt)* } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($($rest)*)? ],
            label: $label,
            group: $group,
            pin: __MISSING_PIN__,
            dma: __DEFAULT_DMA__,
            len: __MISSING_LEN__,
            max_current: $crate::led_strip::Current::Unlimited,
            gamma: $crate::led_strip::Gamma::Linear,
            max_frames: 32,
            led2d: __NONE__,
            fields: [ $($fields)* ]
        }
    };

    // All strips processed, call the main implementation
    (@__with_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_in: []
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [],
            strips_in: [ $($out)* ]
        }
    };

    // Resolve any __DEFAULT_DMA__ placeholders before expansion.
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: []
    ) => {
        $crate::__led_strips_impl! {
            @__expand
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips: [ $($out)* ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 0,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        width: $led2d_width:expr,
                        height: $led2d_height:expr,
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        max_frames: $led2d_max_frames:expr,
                        font: $led2d_font:ident $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 0,
                    dma: DMA_CH0,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 0 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 0,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    width: $led2d_width:expr,
                    height: $led2d_height:expr,
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:ident $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 0,
                    dma: DMA_CH0,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        width: $led2d_width,
                        height: $led2d_height,
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 1 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 1,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    width: $led2d_width:expr,
                    height: $led2d_height:expr,
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:ident $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 1,
                    dma: DMA_CH1,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        width: $led2d_width,
                        height: $led2d_height,
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 1 with led2d and explicit max_frames (original)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 1,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        width: $led2d_width:expr,
                        height: $led2d_height:expr,
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        max_frames: $led2d_max_frames:expr,
                        font: $led2d_font:ident $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 1,
                    dma: DMA_CH1,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 2 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 2,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    width: $led2d_width:expr,
                    height: $led2d_height:expr,
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:ident $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 2,
                    dma: DMA_CH2,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        width: $led2d_width,
                        height: $led2d_height,
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 3 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 3,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    width: $led2d_width:expr,
                    height: $led2d_height:expr,
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:ident $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 3,
                    dma: DMA_CH3,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        width: $led2d_width,
                        height: $led2d_height,
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 2,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        width: $led2d_width:expr,
                        height: $led2d_height:expr,
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        max_frames: $led2d_max_frames:expr,
                        font: $led2d_font:ident $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 2,
                    dma: DMA_CH2,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 3,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        width: $led2d_width:expr,
                        height: $led2d_height:expr,
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        max_frames: $led2d_max_frames:expr,
                        font: $led2d_font:ident $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 3,
                    dma: DMA_CH3,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: $sm_index:expr,
                dma: $dma:ident,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        width: $led2d_width:expr,
                        height: $led2d_height:expr,
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        max_frames: $led2d_max_frames:expr,
                        font: $led2d_font:ident $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm_index,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            width: $led2d_width,
                            height: $led2d_height,
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            max_frames: $led2d_max_frames,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };


    // Parse fields for a single strip
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ pin: $new_pin:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $new_pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ dma: $new_dma:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $new_dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ len: $new_len:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $new_len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ max_current: $new_max_current:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $new_max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ gamma: $new_gamma:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $new_gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ max_frames: $new_max_frames:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $new_max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __NONE__,
        fields: [ led2d: { $($led2d_fields:tt)* } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: __HAS_LED2D__ { $($led2d_fields)* },
            fields: [ $($($rest)*)? ]
        }
    };

    // Done parsing fields, add strip to output and continue
    // Special case: convert __DEFAULT_DMA__ to actual DMA channel based on sm
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: 0,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: 0,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH0,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: 1,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: 1,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH1,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: 2,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: 2,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH2,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: 3,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            sm_counter: 3,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH3,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };

    // Done parsing fields, add strip to output and continue
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __NONE__,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__inc_counter
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            sm: $sm,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                },
            ],
            strips_in: [ $($remaining)* ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __HAS_LED2D__ { $($led2d_fields:tt)* },
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__inc_counter
            frame_alias: $frame_alias,
            pio: $pio,
            group: $group,
            sm: $sm,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames,
                    led2d: { $($led2d_fields)* }
                },
            ],
            strips_in: [ $($remaining)* ]
        }
    };
    // Increment counter by expanding to literal numbers
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, group: $group:ident, sm: 0, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, group: $group, sm_counter: 1, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, group: $group:ident, sm: 1, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, group: $group, sm_counter: 2, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, group: $group:ident, sm: 2, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, group: $group, sm_counter: 3, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, group: $group:ident, sm: 3, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, group: $group, sm_counter: 4, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
}

/// Macro to generate a device abstraction for a NeoPixel-style (WS2812) LED strip.
///
/// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
///
/// **Required fields:**
///
/// - `pin` — GPIO pin for LED data
/// - `len` — Number of LEDs
///
/// **Optional fields:**
///
/// - `pio` — PIO block (default: `PIO0`)
/// - `dma` — DMA channel (default: `DMA_CH0`)
/// - `max_current` — Current budget (default: `250` mA via [`Current::Milliamps`])
/// - `gamma` — Color curve (default: [`Gamma::Gamma2_2`])
/// - `max_frames` — Maximum animation frames (default: `16`)
///
/// # Current Limiting
///
/// The `max_current` field automatically scales brightness to stay within your power budget.
///
/// Each WS2812 LED is assumed to draw 60 mA at full brightness. For example:
/// - 16 LEDs × 60 mA = 960 mA at full brightness
/// - With `max_current: Current::Milliamps(1000)`, all LEDs fit at 100% brightness
/// - With `max_current: Current::Milliamps(250)` (the default), the generated `MAX_BRIGHTNESS` limits LEDs to ~26% brightness
///
/// The current limit is baked into a compile-time lookup table, so it has no
/// runtime cost.
///
/// **Powering LEDs from the Pico's pin 40 (VBUS):** Pin 40 is the USB 5 V rail
/// pass-through, but the Pico itself has practical current limits — the USB connector,
/// cable, and internal circuitry aren't designed for heavy loads. Small LED strips
/// (a few hundred mA) can usually power from pin 40 with a decent USB supply; for
/// larger loads (1 A+), use a separate 5 V supply and share ground with the Pico.
///
/// # Color Correction (Gamma)
///
/// The `gamma` field applies a color response curve to make colors look more natural:
///
/// - [`Gamma::Linear`] — No correction (raw values)
/// - [`Gamma::Gamma2_2`] — Standard sRGB curve (default, most natural-looking)
///
/// The gamma curve is baked into a compile-time lookup table, so it has no
/// runtime cost.
///
/// # When to Use This Macro
///
/// Use `led_strip!` when you have a **single LED strip** (or, say, two).
///
/// If you have **multiple strips**, use [`led_strips!`] instead to save [`PIO`](embassy_rp::pio) resources.
#[macro_export]
macro_rules! led_strip {
    ($($tt:tt)*) => { $crate::__led_strip_impl! { $($tt)* } };
}

// cmk0000 must/should these be at the top level? (If so, mention that in the doc line)
/// Implementation macro. Not part of the public API; use [`led_strip!`] instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_impl {
    // Entry point - name and fields
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: PIO0,
            name: $name,
            pin: _UNSET_,
            dma: DMA_CH0,
            len: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::Gamma::Gamma2_2,
            max_frames: 16,
            fields: [ $($fields)* ]
        }
    };

    // Fill defaults: pio
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ pio: $new_pio:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $new_pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: pin
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ pin: $new_pin:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $new_pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: dma
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ dma: $new_dma:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $new_dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: len (expression in braces)
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ len: { $new_len:expr } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: { $new_len },
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: len (plain expression)
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ len: $new_len:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $new_len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_current
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ max_current: $new_max_current:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $new_max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: gamma
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ gamma: $new_gamma:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $new_gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_frames
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ max_frames: $new_max_frames:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $new_max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill default max_current if still unset
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: _UNSET_,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: []
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $crate::led_strip::Current::Milliamps(250),
            gamma: $gamma,
            max_frames: $max_frames,
            fields: []
        }
    };

    // All fields processed - expand the type
    (@__fill_defaults
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: []
    ) => {
        ::paste::paste! {
            // Create the PIO bus (shared across all SM0 strips using this PIO)
            #[allow(non_upper_case_globals)]
            static [<$pio _BUS>]: ::static_cell::StaticCell<
                $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>
            > = ::static_cell::StaticCell::new();

            /// Split the PIO into bus and state machines.
            ///
            /// Returns SM0 only for single-strip usage.
            #[allow(dead_code)]
            fn [<$pio:lower _split_sm0>](
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
            ) -> $crate::led_strip::PioStateMachine<::embassy_rp::peripherals::$pio, 0> {
                let ::embassy_rp::pio::Pio { common, sm0, .. } =
                    ::embassy_rp::pio::Pio::new(pio, <::embassy_rp::peripherals::$pio as $crate::led_strip::LedStripPio>::irqs());
                let pio_bus = [<$pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                $crate::led_strip::PioStateMachine::new(pio_bus, sm0)
            }

            #[doc = concat!(
                "A LED strip generated by the [`led_strip!`] or [`led_strips!`](crate::led_strips!) macro.\n\n",
                "**This is a concrete example.** When you write your own [`led_strip!`] or [`led_strips!`](crate::led_strips!) invocation, the compiler generates \n",
                "a struct with *your chosen name* (e.g., `LedStrip3`, `LedStripAnimated`) and your specified LED count. \n",
                "Any struct generated by [`led_strip!`] or [`led_strips!`](crate::led_strips!) has the same interface and methods you see documented here.\n\n",
                "## Configuration\n\n",
                "Generated structs are specialized for their exact use case:\n",
                "- **Number of LEDs** — Set via the `len` parameter in your [`led_strip!`] or [`led_strips!`](crate::led_strips!) invocation.\n",
                "- **[PIO](crate#glossary) and [DMA](crate#glossary) resources** — Customized to the pins and channels you specify.\n",
                "- **Power limiting, gamma correction, and max animation frames** — Configured via `max_current`, `gamma`, and `max_frames` parameters. See the [`led_strip!`] or [`led_strips!`](crate::led_strips!) macro documentation for complete details.\n\n",
                "# Example: Write a Single Frame1d\n\n",
                "In this example, we set every other LED to blue. Here, the generated struct is named `LedStripSimple`.\n\n",
                "```rust,no_run\n",
                "# #![no_std]\n",
                "# #![no_main]\n",
                "# use panic_probe as _;\n",
                "# use core::convert::Infallible;\n",
                "# use core::default::Default;\n",
                "# use core::future;\n",
                "# use core::result::Result::Ok;\n",
                "# use embassy_executor::Spawner;\n",
                "use device_kit::{\n",
                "    Result,\n",
                "    led_strip::{Frame1d, colors},\n",
                "};\n\n",
                "use device_kit::led_strip;\n\n",
                "led_strip! {\n",
                "    LedStripSimple {\n",
                "        pin: PIN_3,  // GPIO pin for LED data\n",
                "        len: 48,     // 48 LEDs\n",
                "        // other inputs set to their defaults\n",
                "    }\n",
                "}\n\n",
                "async fn example(spawner: Spawner) -> Result<Infallible> {\n",
                "    let p = embassy_rp::init(Default::default());\n",
                "    let led_strip = LedStripSimple::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;\n\n",
                "    let mut frame = Frame1d::new();\n",
                "    for pixel_index in (0..frame.len()).step_by(2) {\n",
                "        frame[pixel_index] = colors::BLUE;\n",
                "    }\n",
                "    led_strip.write_frame(frame).await?;\n",
                "    future::pending().await // run forever\n",
                "}\n",
                "```\n\n",
                "# Example: Animate a Sequence\n\n",
                "This example animates a 96-LED strip through red, green, and blue frames, cycling continuously. Here, the generated struct is `LedStripAnimated`.\n\n",
                "```rust,no_run\n",
                "# #![no_std]\n",
                "# #![no_main]\n",
                "# use panic_probe as _;\n",
                "# use core::convert::Infallible;\n",
                "# use core::default::Default;\n",
                "# use core::future;\n",
                "# use core::result::Result::Ok;\n",
                "# use embassy_executor::Spawner;\n",
                "use embassy_time::Duration;\n",
                "use device_kit::{\n",
                "    Result,\n",
                "    led_strip::{Current, Frame1d, Gamma, colors},\n",
                "};\n\n",
                "use device_kit::led_strip;\n\n",
                "led_strip! {\n",
                "    LedStripAnimated {\n",
                "        pin: PIN_4,                            // GPIO pin for LED data\n",
                "        len: 96,                               // 96 LEDs\n",
                "        pio: PIO1,                             // Use PIO block 1\n",
                "        dma: DMA_CH3,                          // Use DMA channel 3\n",
                "        max_current: Current::Milliamps(1000), // 1A power budget\n",
                "        gamma: Gamma::Linear,                  // No color correction\n",
                "        max_frames: 3,                         // Up to 3 animation frames\n",
                "    }\n",
                "}\n\n",
                "async fn example(spawner: Spawner) -> Result<Infallible> {\n",
                "    let p = embassy_rp::init(Default::default());\n",
                "    let led_strip = LedStripAnimated::new(p.PIN_4, p.PIO1, p.DMA_CH3, spawner)?;\n\n",
                "    let frame_duration = Duration::from_millis(300);\n",
                "    led_strip.animate([\n",
                "        (Frame1d::filled(colors::RED), frame_duration),\n",
                "        (Frame1d::filled(colors::GREEN), frame_duration),\n",
                "        (Frame1d::filled(colors::BLUE), frame_duration),\n",
                "    ]).await?;\n",
                "    future::pending().await // run forever\n",
                "}\n",
                "```"
            )]
            pub struct $name {
                strip: $crate::led_strip::LedStrip<{ $len }, { $max_frames }>,
            }

            impl $name {
                /// The number of LEDs in this strip (determined by the `len` parameter in [`led_strip!`] or [`led_strips!`](crate::led_strips!)).
                pub const LEN: usize = $len;
                /// Maximum number of animation frames (determined by `max_frames` parameter).
                pub const MAX_FRAMES: usize = $max_frames;

                // Calculate max brightness from current budget
                const WORST_CASE_MA: u32 = ($len as u32) * 60;
                /// Maximum brightness level, automatically limited by the power budget specified in `max_current`.
                /// We assume, each LED draws 60 mA at full brightness.
                pub const MAX_BRIGHTNESS: u8 =
                    $max_current.max_brightness(Self::WORST_CASE_MA);

                // Combined gamma correction and brightness scaling table
                const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                /// Create a new LED strip instance of the struct type
                /// defined by [`led_strip!`] or [`led_strips!`](crate::led_strips!).
                ///
                /// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for example usage.
                ///
                /// The `pin`, `pio`, and `dma` parameters must correspond to the
                /// GPIO pin, PIO resource, and DMA channel specified in the macro.
                ///
                /// - The [`led_strip!`] macro defaults to `PIO0` and `DMA_CH0` if not specified.
                /// - The [`led_strips!`](crate::led_strips!) macro defaults to `PIO0` and
                /// automatically assigns consecutive DMA channels to each strip, starting at `DMA_CH0`.                 
                ///
                /// # Parameters
                ///
                /// - `pin`: GPIO pin for LED data signal
                /// - `pio`: PIO resource
                /// - `dma`: DMA channel for LED data transfer
                /// - `spawner`: Task spawner for background operations
                pub fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> =
                        $crate::led_strip::LedStrip::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<$name> = ::static_cell::StaticCell::new();

                    let pin = pin.into();
                    let dma = dma.into();

                    let sm0 = [<$pio:lower _split_sm0>](pio);
                    let (bus, sm) = sm0.into_parts();

                    let token = [<$name:snake _animation_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                        STRIP_STATIC.completion_signal(),
                    )
                    .map_err($crate::Error::TaskSpawn)?;
                    spawner.spawn(token);

                    let strip = $crate::led_strip::LedStrip::new(&STRIP_STATIC)?;
                    let instance = STRIP_CELL.init($name { strip });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::led_strip::LedStrip<{ $len }, { $max_frames }>;

                fn deref(&self) -> &Self::Target {
                    &self.strip
                }
            }

            #[cfg(not(feature = "host"))]
            impl $crate::led2d::WriteFrame<{ $len }> for $name {
                async fn write_frame(&self, frame: $crate::led_strip::Frame1d<{ $len }>) -> $crate::Result<()> {
                    self.strip.write_frame(frame).await
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _animation_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, 0>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
                completion_signal: &'static $crate::led_strip::LedStripCompletionSignal,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        0,
                        { $len },
                        _
                    >::new(common, sm, dma, pin, program)
                });
                $crate::led_strip::led_strip_animation_loop::<
                    ::embassy_rp::peripherals::$pio,
                    0,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, completion_signal, &$name::COMBO_TABLE).await
            }
        }
    };
}

/// Macro for advanced PIO splitting (rarely needed).
///
/// **See the module docs.** Most users should use the group constructor instead.
#[doc(hidden)]
#[macro_export]
macro_rules! pio_split {
    ($p:ident . PIO0) => {
        pio0_split($p.PIO0)
    };
    ($p:ident . PIO1) => {
        pio1_split($p.PIO1)
    };
    ($p:ident . PIO2) => {
        pio2_split($p.PIO2)
    };
}

pub use pio_split;

// Implement LedStripPio for all PIO peripherals
impl LedStripPio for embassy_rp::peripherals::PIO0 {
    type Irqs = crate::pio_irqs::Pio0Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio0Irqs
    }
}

impl LedStripPio for embassy_rp::peripherals::PIO1 {
    type Irqs = crate::pio_irqs::Pio1Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio1Irqs
    }
}

#[cfg(feature = "pico2")]
impl LedStripPio for embassy_rp::peripherals::PIO2 {
    type Irqs = crate::pio_irqs::Pio2Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio2Irqs
    }
}

pub use led_strip;
pub use led_strips;

/// Used by [`led_strip!`] and [`led_strips!`] to budget current for LED strips.
///
/// See [`LedStripGenerated`](crate::led_strip::led_strip_generated::LedStripGenerated) for usage examples.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Current {
    /// Limit brightness to stay within a specific milliamp budget.
    ///
    /// The `max_brightness` is automatically calculated to ensure the worst-case current
    /// (all LEDs at full brightness) does not exceed this limit. For example, a 16-LED strip
    /// draws 960 mA at full brightness (60 mA per LED); with `Milliamps(250)`, brightness is
    /// capped at ~26%.
    Milliamps(u16),
    /// No limit — brightness stays at 100% (subject to practical hardware constraints like
    /// USB power delivery and the Pico's circuitry).
    Unlimited,
}

impl Default for Current {
    fn default() -> Self {
        Self::Milliamps(250)
    }
}

impl Current {
    /// Calculate maximum brightness based on current budget and worst-case current draw.
    ///
    /// Returns 255 (full brightness) for Unlimited, or a scaled value for Milliamps.
    #[doc(hidden)] // Called by macro-generated code; not part of public API
    #[must_use]
    pub const fn max_brightness(self, worst_case_ma: u32) -> u8 {
        assert!(worst_case_ma > 0, "worst_case_ma must be positive");
        match self {
            Self::Milliamps(ma) => {
                let scale = (ma as u32 * 255) / worst_case_ma;
                if scale > 255 { 255 } else { scale as u8 }
            }
            Self::Unlimited => 255,
        }
    }
}
