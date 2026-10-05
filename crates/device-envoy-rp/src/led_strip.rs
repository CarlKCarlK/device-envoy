#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_simple",
        "docs/assets/led_strip_simple.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gpio0",
        "docs/assets/led_strip_gpio0.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gogo",
        "docs/assets/led2d2.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_animated",
        "docs/assets/led_strip_animated.png"
    )
)]
//! A device abstraction for 1-dimensional NeoPixel-style (WS2812) LED strips. For 2-dimensional
//! panels, see the [`led2d`](mod@crate::led2d) module.
//!
//! This page provides the primary documentation and examples for programming LED strips.
//! The device abstraction supports pixel patterns and animation on the LED strip.
//!
//! **After reading the examples below, see also:**
//!
//! - [`led_strip!`](macro@crate::led_strip) — Macro to generate an LED strip struct type (includes syntax details).
//! - [`LedStrip`](`crate::led_strip::LedStrip`) — Core trait defining the LED strip API surface.
//! - [`LedStripGenerated`](led_strip_generated::LedStripGenerated) — Sample generated strip type showing the constructor path.
//! - [`Frame1d`] — 1D pixel array used to describe LED strip patterns.
//! - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other strips or panels (includes examples).
//!
//! # Example: Write a Single 1-Dimensional Frame
//!
//! In this example, we set every other LED to blue and gray. Here, the generated struct type is
//! named `LedStripSimple`.
//!
//! ![LED strip preview][led_strip_simple]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{Result, led_strip::{LedStrip as _, Frame1d, colors}};
//! use device_envoy_rp::led_strip;
//!
//! // Define LedStripSimple, a struct type for an 8-LED strip on PIN_0.
//! led_strip! {
//!     LedStripSimple {
//!         pin: PIN_0,  // GPIO pin for LED data
//!         len: 8,      // 8 LEDs
//!         // other inputs set to their defaults
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     // Create a LedStripSimple instance.
//!     let led_strip_simple = LedStripSimple::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     // Create and write a frame with alternating blue and gray pixels.
//!     let mut frame = Frame1d::new();
//!     for pixel_index in 0..LedStripSimple::LEN {
//!         // Directly index into the frame buffer.
//!         frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
//!     }
//!
//!     // Display the frame on the LED strip (until replaced).
//!     led_strip_simple.write_frame(frame);
//!
//!     pending().await // run forever
//! }
//! ```
//!
//! # Example: Animate a Sequence
//!
//! This example animates a 96-LED strip through red, green, and blue frames, cycling continuously.
//! Here, the generated struct type is named `LedStripAnimated`.
//!
//! ![LED strip preview][led_strip_animated]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{Result, led_strip::{LedStrip as _, Current, Frame1d, Gamma, colors}};
//! use device_envoy_rp::led_strip;
//!
//! // Define LedStripAnimated, a struct type for a 96-LED strip on PIN_4.
//! // We change some defaults including setting a 1A power budget and disabling gamma correction.
//! led_strip! {
//!     pub(self) LedStripAnimated {               // Can provide a visibility modifier
//!         pin: PIN_4,                            // GPIO pin for LED data
//!         len: 96,                               // 96 LEDs
//!         pio: PIO1,                             // Use PIO resource 1
//!         dma: DMA_CH3,                          // Use DMA channel 3
//!         max_current: Current::Milliamps(1000), // 1A power budget
//!         gamma: Gamma::Linear,                  // No color correction
//!         max_frames: 3,                         // Up to 3 animation frames
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let led_strip_animated = LedStripAnimated::new(p.PIN_4, p.PIO1, p.DMA_CH3, spawner)?;
//!
//!     // Create a sequence of frames and durations and then animate them (looping, until replaced).
//!     let frame_duration = embassy_time::Duration::from_millis(300);
//!     led_strip_animated.animate([
//!         (Frame1d::filled(colors::RED), frame_duration),
//!         (Frame1d::filled(colors::GREEN), frame_duration),
//!         (Frame1d::filled(colors::BLUE), frame_duration),
//!     ]);
//!
//!     pending().await // run forever
//! }
//! ```

pub use device_envoy_core::led_strip::*;
pub mod led_strip_generated;

#[cfg(not(feature = "host"))]
use core::cell::RefCell;
#[cfg(not(feature = "host"))]
use embassy_futures::select::{Either, select};
#[cfg(not(feature = "host"))]
use embassy_rp::pio::{Common, Instance};
#[cfg(not(feature = "host"))]
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
#[cfg(not(feature = "host"))]
use embassy_sync::blocking_mutex::Mutex;
#[cfg(not(feature = "host"))]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(not(feature = "host"))]
#[cfg(not(feature = "host"))]
use embassy_sync::once_lock::OnceLock;
#[cfg(not(feature = "host"))]
use embassy_time::{Duration, Timer};
#[cfg(not(feature = "host"))]
use heapless::Vec;

/// Internal runtime handle for macro-generated LED strip types.
///
/// `#[doc(hidden)]` because this is implementation detail used by macro output.
#[doc(hidden)]
pub struct LedStripRp<const N: usize, const MAX_FRAMES: usize> {
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
}

impl<const N: usize, const MAX_FRAMES: usize> LedStripRp<N, MAX_FRAMES> {
    #[doc(hidden)]
    pub const fn new_static() -> LedStripStatic<N, MAX_FRAMES> {
        LedStripStatic::new_static()
    }

    #[doc(hidden)]
    pub fn new(led_strip_static: &'static LedStripStatic<N, MAX_FRAMES>) -> Self {
        Self {
            command_signal: led_strip_static.command_signal(),
        }
    }

    // Must be `pub` for macro expansion at foreign call sites — not user-facing.
    #[doc(hidden)]
    pub fn __command_signal(&self) -> &'static LedStripCommandSignal<N, MAX_FRAMES> {
        self.command_signal
    }
}

// ============================================================================
// Submodules
// ============================================================================

// ============================================================================
// PIO Bus - Shared PIO resource for multiple LED strips
// ============================================================================

/// A state machine bundled with its PIO bus.
///
/// This is returned by `pio_split!` and passed to strip constructors.
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioBusStateMachine<PIO: Instance + 'static, const SM: usize> {
    pio_bus: &'static PioBus<'static, PIO>,
    state_machine: embassy_rp::pio::StateMachine<'static, PIO, SM>,
}

#[cfg(not(feature = "host"))]
impl<PIO: Instance + 'static, const SM: usize> PioBusStateMachine<PIO, SM> {
    #[doc(hidden)]
    pub fn new(
        pio_bus: &'static PioBus<'static, PIO>,
        state_machine: embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) -> Self {
        Self {
            pio_bus,
            state_machine,
        }
    }

    #[doc(hidden)]
    pub fn pio_bus(&self) -> &'static PioBus<'static, PIO> {
        self.pio_bus
    }

    #[doc(hidden)]
    pub fn into_parts(
        self,
    ) -> (
        &'static PioBus<'static, PIO>,
        embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) {
        (self.pio_bus, self.state_machine)
    }
}

/// Shared PIO bus that manages the Common resource and WS2812 program.
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioBus<'d, PIO: Instance> {
    common: Mutex<CriticalSectionRawMutex, RefCell<Common<'d, PIO>>>,
    ws2812_program: OnceLock<PioWs2812Program<'d, PIO>>,
}

#[cfg(not(feature = "host"))]
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

#[cfg(not(feature = "host"))]
#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub async fn led_strip_device_loop<
    PIO,
    const SM: usize,
    const N: usize,
    const MAX_FRAMES: usize,
    ORDER,
>(
    mut driver: PioWs2812<'static, PIO, SM, N, ORDER>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> !
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    loop {
        let mut command = command_signal.wait().await;
        command_signal.reset();

        loop {
            match command {
                Command::DisplayStatic(mut frame) => {
                    apply_correction(&mut frame, combo_table);
                    driver.write(&frame).await;
                    break;
                }
                Command::Animate(frames) => {
                    command =
                        run_frame_animation(&mut driver, frames, command_signal, combo_table).await;
                }
            }
        }
    }
}

#[cfg(not(feature = "host"))]
async fn run_frame_animation<PIO, const SM: usize, const N: usize, const MAX_FRAMES: usize, ORDER>(
    driver: &mut PioWs2812<'static, PIO, SM, N, ORDER>,
    mut frames: Vec<(Frame1d<N>, Duration), MAX_FRAMES>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> Command<N, MAX_FRAMES>
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    frames
        .iter_mut()
        .for_each(|(frame, _)| apply_correction(frame, combo_table));

    loop {
        for (frame, duration) in &frames {
            driver.write(frame).await;

            match select(command_signal.wait(), Timer::after(*duration)).await {
                Either::First(new_command) => {
                    command_signal.reset();
                    return new_command;
                }
                Either::Second(()) => continue,
            }
        }
    }
}

/// Code generator for [`led_strips!`](crate::led_strip::led_strips).
///
/// Called only by `led_strips!` after `device-envoy-macros` has validated the
/// input and filled defaults. Must be public for macro expansion in downstream
/// crates, but not user-facing API.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strips_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $group:ident,
        doc: $doc:literal,
        pio: $pio:ident,
        member_count: $member_count:literal,
        members: [$({
            index: $sm_index:literal,
            attrs: [$(#[$member_attr:meta])*],
            vis: [$member_vis:vis],
            name: $label:ident,
            doc: $member_doc:literal,
            pin: $pin:ident,
            len: $len:expr,
            max_current: $max_current:expr,
            dma: $dma:ident,
            gamma: $gamma:expr,
            max_frames: $max_frames:expr,
            led2d: $led2d:tt,
        },)*],
    ) => {
        $crate::__paste! {
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
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 0>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 1>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 2>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 3>,
            ) {
                let ::embassy_rp::pio::Pio { common, sm0, sm1, sm2, sm3, .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio,
                        <::embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                let pio_bus = [<$pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                (
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm0),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm1),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm2),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm3),
                )
            }

            $(
                $crate::__led_strips_member! {
                    @define
                    group: $group,
                    pio: $pio,
                    sm: $sm_index,
                    attrs: [$(#[$member_attr])*],
                    vis: [$member_vis],
                    name: $label,
                    doc: $member_doc,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    dma: $dma,
                    gamma: $gamma,
                    max_frames: $max_frames,
                    led2d: $led2d,
                }
            )*

            $(#[$attr])*
            #[doc = $doc]
            $vis struct $group;

            impl $group {
                /// Creates every strip and panel in the group and spawns their background tasks.
                ///
                /// Takes the PIO resource, then a pin and DMA channel per member in declaration
                /// order, then the spawner.
                #[allow(clippy::too_many_arguments)]
                pub fn new(
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    $(
                        [<$label:snake _pin>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                        [<$label:snake _dma>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    )*
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<($($crate::__led_strips_member!(@return_type $label, $led2d),)*)> {
                    #[allow(unused_variables)]
                    let (sm0, sm1, sm2, sm3) = [<$pio:lower _split>](pio.into());
                    Ok(($(
                        $crate::__led_strips_member!(
                            @new $label,
                            [<sm $sm_index>],
                            [<$label:snake _pin>],
                            [<$label:snake _dma>],
                            spawner,
                            $led2d
                        ),
                    )*))
                }
            }
        }
    };
}

/// Per-member code for [`__led_strips_generate!`]: a 1D strip, or a 2D panel
/// wrapping a private strip, depending on whether `led2d` was given.
/// Must be public for macro expansion in downstream crates, but not user-facing API.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strips_member {
    (
        @define
        group: $group:ident,
        pio: $pio:ident,
        sm: $sm_index:literal,
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $label:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        dma: $dma:ident,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: [],
    ) => {
        $crate::__led_strips_member! {
            @strip
            group: $group,
            pio: $pio,
            sm: $sm_index,
            attrs: [$(#[$attr])*],
            vis: [$vis],
            name: $label,
            doc: $doc,
            pin: $pin,
            len: $len,
            max_current: $max_current,
            dma: $dma,
            gamma: $gamma,
            max_frames: $max_frames,
        }
    };

    (
        @define
        group: $group:ident,
        pio: $pio:ident,
        sm: $sm_index:literal,
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $label:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        dma: $dma:ident,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: [{ led_layout: $led_layout:expr, font: $font:expr, }],
    ) => {
        $crate::__paste! {
            $crate::__led_strips_member! {
                @strip
                group: $group,
                pio: $pio,
                sm: $sm_index,
                attrs: [],
                vis: [pub(self)],
                name: [<$label LedStrip>],
                doc: "LED strip behind a 2D panel generated by `led_strips!`.",
                pin: $pin,
                len: $len,
                max_current: $max_current,
                dma: $dma,
                gamma: $gamma,
                max_frames: $max_frames,
            }

            #[cfg(not(feature = "host"))]
            $crate::led2d::led2d_from_strip! {
                $(#[$attr])*
                #[doc = $doc]
                $vis $label,
                strip_type: [<$label LedStrip>],
                width: $led_layout.width(),
                height: $led_layout.height(),
                led_layout: $led_layout,
                font: $font,
            }
        }
    };

    (
        @strip
        group: $group:ident,
        pio: $pio:ident,
        sm: $sm_index:literal,
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $label:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        dma: $dma:ident,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
    ) => {
        $crate::__paste! {
            $(#[$attr])*
            #[doc = $doc]
            #[doc = concat!(
                "\n\nImplements the [`LedStrip`](crate::led_strip::LedStrip) trait for LED control methods. ",
                "Created with [`", stringify!($group), "::new`]."
            )]
            $vis struct $label {
                strip: $crate::led_strip::LedStripRp<{ $len }, { $max_frames }>,
            }

            #[allow(missing_docs)]
            impl $label {
                pub const LEN: usize = $len;
                pub const MAX_FRAMES: usize = $max_frames;

                // Each WS2812B LED draws ~60mA at full brightness.
                const WORST_CASE_MA: u32 = ($len as u32) * 60;
                pub const MAX_BRIGHTNESS: u8 =
                    $max_current.max_brightness(Self::WORST_CASE_MA);

                // Combined gamma correction and brightness scaling table
                const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                pub(crate) const fn new_static() -> $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> {
                    $crate::led_strip::LedStripRp::new_static()
                }

                pub fn new(
                    state_machine: $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> =
                        $label::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<$label> = ::static_cell::StaticCell::new();
                    let pin = pin.into();
                    let dma = dma.into();

                    let (bus, sm) = state_machine.into_parts();
                    let token = [<$label:snake _device_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                    );
                    spawner.spawn(token.map_err($crate::Error::TaskSpawn)?);
                    let strip = $crate::led_strip::LedStripRp::new(&STRIP_STATIC);
                    let instance = STRIP_CELL.init(Self { strip });
                    Ok(instance)
                }
            }

            #[cfg(not(feature = "host"))]
            impl AsRef<$crate::led_strip::LedStripRp<{ $len }, { $max_frames }>> for $label {
                fn as_ref(&self) -> &$crate::led_strip::LedStripRp<{ $len }, { $max_frames }> {
                    &self.strip
                }
            }

            impl $crate::led_strip::LedStrip<{ $len }> for $label {
                const MAX_FRAMES: usize = $max_frames;
                const MAX_BRIGHTNESS: u8 = Self::MAX_BRIGHTNESS;

                fn write_frame(&self, frame: $crate::led_strip::Frame1d<{ $len }>) {
                    $crate::led_strip::__write_frame(self.strip.__command_signal(), frame);
                }

                fn animate<I>(&self, frames: I)
                where
                    I: IntoIterator,
                    I::Item: ::core::borrow::Borrow<(
                        $crate::led_strip::Frame1d<{ $len }>,
                        embassy_time::Duration,
                    )>,
                {
                    $crate::led_strip::__animate(self.strip.__command_signal(), frames);
                }
            }

            #[::embassy_executor::task]
            async fn [<$label:snake _device_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, $sm_index>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        $sm_index,
                        { $len },
                        _
                    >::new(
                        common,
                        sm,
                        dma,
                        <::embassy_rp::peripherals::$dma as $crate::pio_irqs::DmaIrqMap>::irqs(),
                        pin,
                        program,
                    )
                });
                $crate::led_strip::led_strip_device_loop::<
                    ::embassy_rp::peripherals::$pio,
                    $sm_index,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, &$label::COMBO_TABLE).await
            }
        }
    };

    (@return_type $label:ident, []) => {
        &'static $label
    };
    (@return_type $label:ident, [$($led2d:tt)*]) => {
        $label
    };

    (@new $label:ident, $state_machine:ident, $pin:ident, $dma:ident, $spawner:ident, []) => {
        $label::new($state_machine, $pin, $dma, $spawner)?
    };
    (@new $label:ident, $state_machine:ident, $pin:ident, $dma:ident, $spawner:ident, [$($led2d:tt)*]) => {
        $crate::__paste! {
            $label::from_strip([<$label LedStrip>]::new($state_machine, $pin, $dma, $spawner)?)?
        }
    };
}

/// Macro to generate an LED-strip struct type (includes syntax details).
///
/// **See the [led_strip module documentation](mod@crate::led_strip) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// led_strip! {
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///         len: <usize_expr>,
///         pio: <pio_ident>,               // optional
///         dma: <dma_ident>,               // optional
///         max_current: <Current_expr>,    // optional
///         gamma: <Gamma_expr>,            // optional
///         max_frames: <usize_expr>,       // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO pin for LED data
/// - `len` — Number of LEDs
///
/// **Optional fields:**
///
/// - `pio` — PIO resource (default: `PIO0`)
/// - `dma` — DMA channel (default: `DMA_CH0`)
/// - `max_current` — Electrical current budget (default: 250 mA)
/// - `gamma` — Color curve (default: `Gamma::Srgb`)
/// - `max_frames` — Maximum number of animation frames (default: 16 frames)
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other strips or panels (includes examples)
/// - [`led2d!`](mod@crate::led2d) — For 2-dimensional LED panels
///
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! led_strip {
    // TODO_NIGHTLY When nightly feature `decl_macro` becomes stable, change this
    // code by replacing `#[macro_export] macro_rules!` with module-scoped `pub macro`
    // so macro visibility and helper exposure can be controlled more precisely.
    ($($tt:tt)*) => { $crate::__led_strip_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led_strip!`] instead.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_impl {
    // Entry point - name without visibility defaults to public
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__validate_keyword_fields_expr! {
            macro_name: "led_strip!",
            allowed_macro: $crate::__led_strip_allowed_field,
            fields: [ $($fields)* ]
        }
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: pub(self),
            pio: PIO0,
            name: $name,
            pin: _UNSET_,
            dma: DMA_CH0,
            len: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            fields: [ $($fields)* ]
        }
    };

    // Entry point - name with explicit visibility
    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__validate_keyword_fields_expr! {
            macro_name: "led_strip!",
            allowed_macro: $crate::__led_strip_allowed_field,
            fields: [ $($fields)* ]
        }
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: PIO0,
            name: $name,
            pin: _UNSET_,
            dma: DMA_CH0,
            len: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            fields: [ $($fields)* ]
        }
    };

    // Fill defaults: pio
    (@__fill_defaults
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
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
        vis: $vis:vis,
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
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $crate::led_strip::MAX_CURRENT_DEFAULT,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: []
        }
    };

    // All fields processed - expand the type
    (@__fill_defaults
        vis: $vis:vis,
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
            static [<$name:snake _ $pio _BUS>]: ::static_cell::StaticCell<
                $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>
            > = ::static_cell::StaticCell::new();

            /// Split the PIO into bus and state machines.
            ///
            /// Returns SM0 only for single-strip usage.
            #[allow(dead_code)]
            fn [<$name:snake _split_sm0>](
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
            ) -> $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 0> {
                let ::embassy_rp::pio::Pio { common, sm0, .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio,
                        <::embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                let pio_bus = [<$name:snake _ $pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                $crate::led_strip::PioBusStateMachine::new(pio_bus, sm0)
            }

            #[doc = concat!(
                "LED strip generated by [`led_strip!`] or [`led_strips!`](crate::led_strips!).\n\n",
                "See the [led_strip module documentation](mod@crate::led_strip) for usage and examples."
            )]
            $vis struct $name {
                strip: $crate::led_strip::LedStripRp<{ $len }, { $max_frames }>,
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
                /// See the [led_strip module documentation](mod@crate::led_strip) for example usage.
                ///
                /// The `pin`, `pio`, and `dma` parameters must correspond to the
                /// GPIO pin, PIO resource, and DMA channel specified in the macro.
                ///
                /// - The [`led_strip!`] macro defaults to `PIO0` and `DMA_CH0` if not specified.
                /// - The [`led_strips!`](crate::led_strips!) macro defaults to `PIO0` and
                /// automatically assigns consecutive DMA channels to each strip, starting at `DMA_CH0`.
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
                        $crate::led_strip::LedStripRp::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<$name> = ::static_cell::StaticCell::new();

                    let pin = pin.into();
                    let dma = dma.into();

                    let sm0 = [<$name:snake _split_sm0>](pio);
                    let (bus, sm) = sm0.into_parts();

                    let token = [<$name:snake _device_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                    );
                    spawner.spawn(token.map_err($crate::Error::TaskSpawn)?);

                    let strip = $crate::led_strip::LedStripRp::new(&STRIP_STATIC);
                    let instance = STRIP_CELL.init($name { strip });
                    Ok(instance)
                }
            }

            #[cfg(not(feature = "host"))]
            impl AsRef<$crate::led_strip::LedStripRp<{ $len }, { $max_frames }>> for $name {
                fn as_ref(&self) -> &$crate::led_strip::LedStripRp<{ $len }, { $max_frames }> {
                    &self.strip
                }
            }

            impl $crate::led_strip::LedStrip<{ $len }> for $name {
                const MAX_FRAMES: usize = $max_frames;
                const MAX_BRIGHTNESS: u8 = Self::MAX_BRIGHTNESS;

                fn write_frame(&self, frame: $crate::led_strip::Frame1d<{ $len }>) {
                    $crate::led_strip::__write_frame(self.strip.__command_signal(), frame);
                }

                fn animate<I>(&self, frames: I)
                where
                    I: IntoIterator,
                    I::Item: ::core::borrow::Borrow<(
                        $crate::led_strip::Frame1d<{ $len }>,
                        embassy_time::Duration,
                    )>,
                {
                    $crate::led_strip::__animate(self.strip.__command_signal(), frames);
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _device_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, 0>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        0,
                        { $len },
                        _
                    >::new(
                        common,
                        sm,
                        dma,
                        <::embassy_rp::peripherals::$dma as $crate::pio_irqs::DmaIrqMap>::irqs(),
                        pin,
                        program,
                    )
                });
                $crate::led_strip::led_strip_device_loop
                ::<
                    ::embassy_rp::peripherals::$pio,
                    0,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, &$name::COMBO_TABLE).await
            }
        }
    };
}

/// Public for macro expansion in downstream crates.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_allowed_field {
    (pin, $macro_name:literal) => {};
    (len, $macro_name:literal) => {};
    (pio, $macro_name:literal) => {};
    (dma, $macro_name:literal) => {};
    (max_current, $macro_name:literal) => {};
    (gamma, $macro_name:literal) => {};
    (max_frames, $macro_name:literal) => {};
    ($field:ident, $macro_name:literal) => {
        compile_error!(concat!(
            $macro_name,
            " unknown field; expected `pin`, `len`, `pio`, `dma`, `max_current`, `gamma`, or `max_frames`"
        ));
    };
}

/// Macro for advanced PIO splitting (rarely needed).
///
/// **See the module docs.** Most users should use the group constructor instead.
#[doc(hidden)]
#[cfg(not(feature = "host"))]
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

#[cfg(not(feature = "host"))]
pub use pio_split;

/// Macro to generate multiple LED strip and panel struct types that share a single
/// [PIO resource](crate#glossary).
///
/// This page provides the primary documentation and examples for configuring strip/panel
/// groups that share a PIO resource.
///
/// **After reading the examples below, see also:**
///
/// - [`LedStrip`](`crate::led_strip::LedStrip`) — LED **strip** trait defining methods and associated constants
/// - [`Led2d`](crate::led2d::Led2d) — LED **panel** trait defining methods and associated constants
/// - [`led_strip!`](macro@crate::led_strip) — Alternative macro to generate a single LED strip type. Consumes a PIO resource.
/// - [`led2d!`](mod@crate::led2d) — Alternative macro to generate a single LED panel type. Consumes a PIO resource.
///
/// Use this macro when your project has multiple LED strips or panels
/// that should share a single PIO resource.
/// If you only need a single strip or panel, prefer [`led_strip!`](macro@crate::led_strip)
/// or [`led2d!`](macro@crate::led2d) for simpler configuration.
///
/// We’ll start with a complete example below. The syntax and field reference are at
/// the end of this page.
///
/// # Example: Connect Three LED Strips/Panels to One PIO Resource
///
/// This example creates three LED strips/panels on GPIO0, GPIO3, and GPIO4,
/// all sharing PIO0. It demonstrates showing a pattern on the first two strips
/// and animating text on the 2D panel.
///
/// ![GPIO0 strip preview][led_strip_gpio0]
///
/// ![GPIO3 strip preview][led_strip_simple]
///
/// ![GPIO4 panel preview][led_strip_gogo]
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use core::{convert::Infallible, future::pending};
/// # use defmt_rtt as _;
/// # use embassy_executor::Spawner;
/// # use defmt::info;
/// use device_envoy_rp::{Result, led2d::Frame2d, led2d::Led2dFont, led2d::layout::LedLayout, led_strip::{LedStrip as _, Current, Frame1d, Gamma, colors, led_strips}};
/// use device_envoy_rp::led2d::Led2d as _;
/// use embassy_time::Duration;
///
/// // Our 2D panel is two 12x4 panels stacked vertically.
/// const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
/// const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
/// const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
///
/// led_strips! {
///     LedStrips0 {                        // Name for this group of LED strips/panels. Can provide visibility modifier
///         pio: PIO0,                      // Optional; defaults to PIO0.
///         // 1. a 8-LED strip on GPIO0
///         Gpio0LedStrip {                 // Exact struct name for this strip. Can provide visibility modifier
///             pin: PIN_0,                 // GPIO pin for LED data signal.
///             len: 8,                     // 8 LEDs on this strip.
///             max_current: Current::Milliamps(25), // Every strip/panel requires an electrical current budget.
///         },
///         // 2. a 48-LED strip on GPIO3
///         Gpio3LedStrip {
///             pin: PIN_3,
///             len: 48,
///             max_current: Current::Milliamps(75),
///             gamma: Gamma::Srgb,        // Optional; color correction (default, Gamma::Srgb).
///             max_frames: 1,              // Optional; default 16 frames.
///             dma: DMA_CH11,              // Optional; auto-assigned by strip order.
///         },
///         // 3. a 96-LED 2D panel on GPIO4
///         Gpio4Led2d {
///             pin: PIN_4,
///             len: 96,
///             max_current: Current::Milliamps(250),
///             max_frames: 2,
///             led2d: {                    // Optional panel configuration for 2D displays.
///                 led_layout: LED_LAYOUT_12X8_ROTATED, // Two 12×4 panels stacked and rotated.
///                 font: Led2dFont::Font4x6Trim, // 4x6 pixel font without the usual 1 pixel spacing.
///             },
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
///     // Create instances of two LED strips and one panel.
///     let (gpio0_led_strip, gpio3_led_strip, gpio4_led2d) = LedStrips0::new(
///         p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH11, p.PIN_4, p.DMA_CH2, spawner,
///     )?;
///
///     info!("Setting GPIO0 to white, GPIO3 to alternating blue/gray, GPIO4 to Go Go animation");
///
///     // Turn on all-white on GPIO0 strip.
///     let frame_gpio0 = Frame1d::filled(colors::WHITE);
///     gpio0_led_strip.write_frame(frame_gpio0); // Display the frame (until replaced)
///
///     // Alternate blue/gray on GPIO3 strip.
///     let mut frame_gpio3 = Frame1d::new();
///     for pixel_index in 0..Gpio3LedStrip::LEN {
///         frame_gpio3[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
///     }
///     gpio3_led_strip.write_frame(frame_gpio3);  // Display the frame (until replaced)
///
///     // Animate "Go Go" text on GPIO4 2D panel.
///     let mut frame_go_top = Frame2d::new();
///     gpio4_led2d.write_text_to_frame("Go", &[], &mut frame_go_top);
///
///     let mut frame_go_bottom = Frame2d::new();
///     gpio4_led2d.write_text_to_frame(
///         "\nGo",
///         &[colors::HOT_PINK, colors::LIME],
///         &mut frame_go_bottom,
///     );
///
///     let frame_duration = Duration::from_secs(1);
///     gpio4_led2d
///         .animate([
///             (frame_go_top, frame_duration),
///             (frame_go_bottom, frame_duration),
///         ]); // Loop animation (until replaced)
///
///     pending().await // run forever
/// }
/// ```
///
/// # 2D Panels
///
/// If a member is a rectangular LED panel rather than a linear strip, add a `led2d`
/// block to describe its geometry. That member's type then implements
/// [`Led2d`](crate::led2d::Led2d) instead of [`LedStrip`](crate::led_strip::LedStrip).
/// The `led_layout` value must be a `const` [`LedLayout`](crate::led2d::layout::LedLayout)
/// so its dimensions are known at compile time, and `font` is a
/// [`Led2dFont`](crate::led2d::Led2dFont). Detailed 2D rendering and animation support is
/// documented in the [`led2d` module](mod@crate::led2d).
///
/// Set `max_frames: 0` to disable animation and allocate no frame storage; `write_frame()`
/// is still supported.
///
/// # Capacity and Board Capabilities
///
/// The `led_strips!` macro is designed to **fully utilize the PIO resources**
/// of supported Pico boards.
///
/// Each `led_strips!` invocation can drive up to **4 LED strips or panels**
/// while sharing a single PIO resource. This lets you consolidate multiple
/// LED outputs efficiently instead of consuming one PIO per strip.
///
/// Each invocation consumes exactly one PIO resource.
///
/// On supported boards, this enables the maximum practical LED capacity:
///
/// - **Pico 1** provides **2 PIO resources**, allowing up to **8 LED strips or panels**
/// - **Pico 2** provides **3 PIO resources**, allowing up to **12 LED strips or panels**
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led_strip!`](macro@crate::led_strip) — For a single 1-dimensional LED strip (includes examples)
/// - [`led2d!`](mod@crate::led2d) — For 2-dimensional LED panels
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gpio0",
        "docs/assets/led_strip_gpio0.png"
    )
)]
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_simple",
        "docs/assets/led_strip_simple.png"
    )
)]
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gogo",
        "docs/assets/led2d2.png"
    )
)]
#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use device_envoy_macros::rp_led_strips as led_strips;
#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use led_strip;

// Public so led_strip!/led_strips! expansions in downstream crates can reference it.
#[doc(hidden)]
/// Default electrical current budget for generated LED devices (`Current::Milliamps(250)`).
pub const MAX_CURRENT_DEFAULT: Current = Current::Milliamps(250);
