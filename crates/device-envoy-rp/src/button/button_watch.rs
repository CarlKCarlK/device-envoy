//! Background button monitoring with a spawned task.
//!
//! See the [`button_watch!`](crate::button_watch!) macro for usage and
//! [`ButtonWatchGenerated`](super::button_watch_generated::ButtonWatchGenerated) for a sample of a generated type.

use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

use super::{PressDuration, PressedTo};

// ============================================================================
// ButtonWatchStaticRp - Static resources for button monitoring
// ============================================================================

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub struct ButtonWatchStaticRp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
}

impl ButtonWatchStaticRp {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    #[must_use]
    pub const fn signal(&self) -> &Signal<CriticalSectionRawMutex, PressDuration> {
        &self.signal
    }
}

// ============================================================================
// ButtonWatchRp - Handle for background button monitoring
// ============================================================================

// Must be public for macro expansion in downstream crates, but not user-facing API.
// Users interact with the macro-generated structs (e.g., ButtonWatchGenerated), not this type directly.
#[doc(hidden)]
pub struct ButtonWatchRp {
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
}

impl ButtonWatchRp {
    #[must_use]
    pub fn new(button_watch_static: &'static ButtonWatchStaticRp) -> Self {
        Self {
            signal: button_watch_static.signal(),
        }
    }
}

impl device_envoy_core::button::ButtonWatch for ButtonWatchRp {
    async fn wait_for_press_duration(&self) -> PressDuration {
        self.signal.wait().await
    }
}

// ============================================================================
// Background task implementation
// ============================================================================

/// Background task that monitors button state and fires events.
///
/// Never call directly - spawned automatically by the [`button_watch!`](crate::button_watch!) macro.
#[doc(hidden)]
pub async fn button_watch_task<P: Pin>(
    pin: Peri<'static, P>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
) -> ! {
    let pull = match pressed_to {
        PressedTo::Voltage => Pull::Down,
        PressedTo::Ground => Pull::Up,
    };
    let mut input = Input::new(pin, pull);
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    signal_press_durations(&mut input_button, signal).await
}

/// Background task that monitors button state from an existing Input.
///
/// This variant is used when converting from a `ButtonRp` via `from_button()`.
/// Never call directly - spawned automatically by the [`button_watch!`](crate::button_watch!) macro.
#[doc(hidden)]
pub async fn button_watch_task_from_input(
    mut input: Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
) -> ! {
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    signal_press_durations(&mut input_button, signal).await
}

struct InputButton<'a> {
    input: &'a mut Input<'static>,
    pressed_to: PressedTo,
}

impl device_envoy_core::button::Button for InputButton<'_> {
    fn is_pressed(&self) -> bool {
        self.pressed_to.is_pressed(self.input.is_high())
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        match (pressed, self.pressed_to) {
            (true, PressedTo::Voltage) | (false, PressedTo::Ground) => {
                self.input.wait_for_high().await;
            }
            (true, PressedTo::Ground) | (false, PressedTo::Voltage) => {
                self.input.wait_for_low().await;
            }
        }
    }
}

async fn signal_press_durations<B: device_envoy_core::button::Button>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
) -> ! {
    loop {
        let press_duration =
            <B as device_envoy_core::button::Button>::wait_for_press_duration(button).await;
        signal.signal(press_duration);
    }
}

// ============================================================================
// button_watch! macro
// ============================================================================

/// Creates a button monitoring device abstraction with a background task.
///
/// This macro creates a button monitor that runs in a dedicated background task,
/// providing continuous monitoring without interruption.
///
/// See [`ButtonWatchGenerated`](crate::button::button_watch_generated::ButtonWatchGenerated) for a sample of what the macro generates.
///
/// # Constructors
///
/// - [`new()`](crate::button::button_watch_generated::ButtonWatchGenerated::new) — Create from a pin
/// - [`from_button()`](crate::button::button_watch_generated::ButtonWatchGenerated::from_button) — Convert from an existing `ButtonRp`
///
/// # Use Cases
///
/// Use `button_watch!` instead of [`ButtonRp`](super::ButtonRp) when you need continuous monitoring
/// that works even in fast loops or `select()` operations. [`ButtonRp`](super::ButtonRp) starts
/// fresh monitoring on each call to `wait_for_press()`, which can miss events in busy loops.
///
///  # Parameters
///
/// - `name`: The struct name for the button watch device
/// - `pin`: The GPIO pin connected to the button
///
/// Optional:
/// - `vis`: Visibility modifier (default: private)
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::button_watch;
/// use device_envoy_rp::button::PressDuration;
/// use device_envoy_rp::button::PressedTo;
/// use device_envoy_rp::button::ButtonWatch as _;
/// use embassy_executor::Spawner;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// button_watch! {
///     ButtonWatch13 {
///         pin: PIN_13,
///     }
/// }
///
/// async fn example(p: embassy_rp::Peripherals, spawner: Spawner) {
///     // Create the button monitor (spawns background task automatically)
///     let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner)
///         .expect("Failed to create button monitor");
///
///     loop {
///         // Wait for button press - never misses events even if this loop is slow
///         match button_watch13.wait_for_press_duration().await {
///             PressDuration::Short => {
///                 // Handle short press
/// #               break;
///             }
///             PressDuration::Long => {
///                 // Handle long press
/// #               break;
///             }
///         }
///     }
/// }
/// ```
///
/// **Syntax:**
///
/// ```text
/// button_watch! {
///     [<attributes>]
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///     }
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! button_watch {
    ($($tt:tt)*) => { $crate::__button_watch_impl! { $($tt)* } };
}

/// Implementation macro for `button_watch!`.
///
/// Do not call directly - use [`button_watch!`](crate::button_watch!) instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __button_watch_impl {
    // Entry point with optional visibility
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident {
            pin: $pin:ident,
        }
    ) => {
        $crate::__button_watch_impl! {
            @impl
            meta: [$(#[$meta])*],
            vis: $vis,
            name: $name,
            pin: $pin
        }
    };

    // Entry point with default (private) visibility
    (
        $(#[$meta:meta])*
        $name:ident {
            pin: $pin:ident,
        }
    ) => {
        $crate::__button_watch_impl! {
            @impl
            meta: [$(#[$meta])*],
            vis: ,
            name: $name,
            pin: $pin
        }
    };

    // Internal implementation
    (
        @impl
        meta: [$(#[$meta:meta])*],
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:ident
    ) => {
        ::paste::paste! {
            $(#[$meta])*
            #[doc = concat!(
                "Button monitor generated by [`button_watch!`].\n\n",
                "Monitors button presses in a background task. ",
                "See the [button_watch module documentation](mod@$crate::button) for usage."
            )]
            $vis struct $name {
                button_watch: $crate::button::ButtonWatchRp,
            }

            impl $name {
                /// Creates a new button monitor and spawns its background task.
                ///
                /// # Parameters
                ///
                /// - `pin`: GPIO pin for the button
                /// - `pressed_to`: How the button is wired ([`PressedTo::Ground`] or [`PressedTo::Voltage`])
                /// - `spawner`: Task spawner for background operations
                ///
                /// # Errors
                ///
                /// Returns an error if the background task cannot be spawned.
                pub fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    pressed_to: $crate::button::PressedTo,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static BUTTON_WATCH_STATIC: $crate::button::ButtonWatchStaticRp =
                        $crate::button::ButtonWatchStaticRp::new();
                    static BUTTON_WATCH_CELL: ::static_cell::StaticCell<$name> =
                        ::static_cell::StaticCell::new();

                    let pin = pin.into();
                    let task_token = [<$name:snake _task>](
                        pin,
                        pressed_to,
                        BUTTON_WATCH_STATIC.signal(),
                    );
                    spawner.spawn(task_token).map_err($crate::Error::TaskSpawn)?;

                    let button_watch = $crate::button::ButtonWatchRp::new(
                        &BUTTON_WATCH_STATIC,
                    );

                    let instance = BUTTON_WATCH_CELL.init($name { button_watch });
                    Ok(instance)
                }

                /// Creates a button monitor from an existing `ButtonRp` and spawns its background task.
                ///
                /// This is useful for converting a `ButtonRp` returned from `WifiAuto::connect()`
                /// into a `ButtonWatchRp` for background monitoring.
                ///
                /// # Parameters
                ///
                /// - `button`: An existing button (e.g., from `WifiAuto::connect()`)
                /// - `spawner`: Task spawner for background operations
                ///
                /// # Errors
                ///
                /// Returns an error if the background task cannot be spawned.
                ///
                /// # Example
                ///
                /// ```rust,no_run
                /// # #![no_std]
                /// # #![no_main]
                /// # use device_envoy_rp::button_watch;
                /// # use device_envoy_rp::button::ButtonWatch as _;
                /// # use embassy_executor::Spawner;
                /// # #[panic_handler]
                /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
                /// button_watch! {
                ///     ButtonWatch13 {
                ///         pin: PIN_13,
                ///     }
                /// }
                ///
                /// async fn example(
                ///     button: device_envoy_rp::button::ButtonRp<'static>,
                ///     spawner: Spawner,
                /// ) -> device_envoy_rp::Result<()> {
                ///     // Convert ButtonRp from WifiAuto into ButtonWatchRp
                ///     let button_watch13 = ButtonWatch13::from_button(button, spawner)?;
                ///
                ///     // Now button monitoring happens in background
                ///     loop {
                ///         let press = button_watch13.wait_for_press_duration().await;
                ///         // Handle press...
                /// #       break;
                ///     }
                /// #   Ok(())
                /// }
                /// ```
                pub fn from_button(
                    button: $crate::button::ButtonRp<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static BUTTON_WATCH_STATIC: $crate::button::ButtonWatchStaticRp =
                        $crate::button::ButtonWatchStaticRp::new();
                    static BUTTON_WATCH_CELL: ::static_cell::StaticCell<$name> =
                        ::static_cell::StaticCell::new();

                    let (input, pressed_to) = button.into_parts();
                    let task_token = [<$name:snake _task_from_input>](
                        input,
                        pressed_to,
                        BUTTON_WATCH_STATIC.signal(),
                    );
                    spawner.spawn(task_token).map_err($crate::Error::TaskSpawn)?;

                    let button_watch = $crate::button::ButtonWatchRp::new(
                        &BUTTON_WATCH_STATIC,
                    );

                    let instance = BUTTON_WATCH_CELL.init($name { button_watch });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::button::ButtonWatchRp;

                fn deref(&self) -> &Self::Target {
                    &self.button_watch
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _task>](
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                pressed_to: $crate::button::PressedTo,
                signal: &'static ::embassy_sync::signal::Signal<
                    ::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    $crate::button::PressDuration
                >,
            ) -> ! {
                $crate::button::button_watch_task(pin, pressed_to, signal).await
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _task_from_input>](
                input: ::embassy_rp::gpio::Input<'static>,
                pressed_to: $crate::button::PressedTo,
                signal: &'static ::embassy_sync::signal::Signal<
                    ::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    $crate::button::PressDuration
                >,
            ) -> ! {
                $crate::button::button_watch_task_from_input(input, pressed_to, signal).await
            }
        }
    };
}
