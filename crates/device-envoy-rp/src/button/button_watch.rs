//! Background button monitoring with a spawned task.
//!
//! See the [`button_watch!`](crate::button_watch!) macro for usage and
//! [`ButtonWatchGenerated`](super::button_watch_generated::ButtonWatchGenerated) for a sample of a generated type.

use core::sync::atomic::{AtomicBool, Ordering};
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use super::{PressDuration, PressedTo};

// ============================================================================
// ButtonWatchStaticRp - Static resources for button monitoring
// ============================================================================

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub struct ButtonWatchStaticRp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: Signal<CriticalSectionRawMutex, ()>,
    is_pressed: AtomicBool,
    initialized: AtomicBool,
}

impl ButtonWatchStaticRp {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            state_changed_signal: Signal::new(),
            initialized_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub const fn signal(&self) -> &Signal<CriticalSectionRawMutex, PressDuration> {
        &self.signal
    }

    #[must_use]
    pub const fn state_signal(&self) -> &Signal<CriticalSectionRawMutex, bool> {
        &self.state_signal
    }

    #[must_use]
    pub const fn state_changed_signal(&self) -> &Signal<CriticalSectionRawMutex, ()> {
        &self.state_changed_signal
    }

    #[must_use]
    pub const fn initialized_signal(&self) -> &Signal<CriticalSectionRawMutex, ()> {
        &self.initialized_signal
    }

    #[must_use]
    pub const fn is_pressed(&self) -> &AtomicBool {
        &self.is_pressed
    }

    #[must_use]
    pub const fn initialized(&self) -> &AtomicBool {
        &self.initialized
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
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
}

impl ButtonWatchRp {
    #[must_use]
    pub fn new(button_watch_static: &'static ButtonWatchStaticRp) -> Self {
        Self {
            signal: button_watch_static.signal(),
            state_signal: button_watch_static.state_signal(),
            state_changed_signal: button_watch_static.state_changed_signal(),
            initialized_signal: button_watch_static.initialized_signal(),
            is_pressed: button_watch_static.is_pressed(),
            initialized: button_watch_static.initialized(),
        }
    }

    pub async fn wait_until_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }
        self.initialized_signal.wait().await;
    }

    pub async fn wait_for_state_change(&self) {
        self.state_changed_signal.wait().await;
    }
}

impl device_envoy_core::button::__ButtonMonitor for ButtonWatchRp {
    fn is_pressed_raw(&self) -> bool {
        self.is_pressed.load(Ordering::Relaxed)
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        if self.is_pressed.load(Ordering::Relaxed) == pressed {
            return;
        }

        loop {
            let state = self.state_signal.wait().await;
            if state == pressed {
                return;
            }
        }
    }
}

impl device_envoy_core::button::Button for ButtonWatchRp {
    async fn wait_for_press_duration(&mut self) -> PressDuration {
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
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
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
    signal_press_durations(
        &mut input_button,
        signal,
        state_signal,
        state_changed_signal,
        initialized_signal,
        is_pressed,
        initialized,
    )
    .await
}

struct InputButton<'a> {
    input: &'a mut Input<'static>,
    pressed_to: PressedTo,
}

impl device_envoy_core::button::__ButtonMonitor for InputButton<'_> {
    fn is_pressed_raw(&self) -> bool {
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

async fn signal_press_durations<B: device_envoy_core::button::__ButtonMonitor>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
    let initial_pressed = <B as device_envoy_core::button::__ButtonMonitor>::is_pressed_raw(button);
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);
    initialized.store(true, Ordering::Release);
    initialized_signal.signal(());

    loop {
        <B as device_envoy_core::button::__ButtonMonitor>::wait_until_pressed_state(button, false)
            .await;

        <B as device_envoy_core::button::__ButtonMonitor>::wait_until_pressed_state(button, true)
            .await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);
        state_changed_signal.signal(());

        Timer::after(device_envoy_core::button::BUTTON_DEBOUNCE_DELAY).await;
        if !<B as device_envoy_core::button::__ButtonMonitor>::is_pressed_raw(button) {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            state_changed_signal.signal(());
            continue;
        }

        let press_duration = embassy_futures::select::select(
            <B as device_envoy_core::button::__ButtonMonitor>::wait_until_pressed_state(
                button, false,
            ),
            Timer::after(device_envoy_core::button::LONG_PRESS_DURATION),
        )
        .await;

        match press_duration {
            embassy_futures::select::Either::First(()) => {
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                state_changed_signal.signal(());
                signal.signal(PressDuration::Short);
            }
            embassy_futures::select::Either::Second(()) => {
                signal.signal(PressDuration::Long);
                <B as device_envoy_core::button::__ButtonMonitor>::wait_until_pressed_state(
                    button, false,
                )
                .await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                state_changed_signal.signal(());
            }
        }
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
/// use device_envoy_rp::button::Button as _;
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
///     let mut button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner)
///         .await
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
                pub async fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    pressed_to: $crate::button::PressedTo,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static mut Self> {
                    static BUTTON_WATCH_STATIC: $crate::button::ButtonWatchStaticRp =
                        $crate::button::ButtonWatchStaticRp::new();
                    static BUTTON_WATCH_CELL: ::static_cell::StaticCell<$name> =
                        ::static_cell::StaticCell::new();

                    let pin = pin.into();
                    let task_token = [<$name:snake _task>](
                        pin,
                        pressed_to,
                        BUTTON_WATCH_STATIC.signal(),
                        BUTTON_WATCH_STATIC.state_signal(),
                        BUTTON_WATCH_STATIC.state_changed_signal(),
                        BUTTON_WATCH_STATIC.initialized_signal(),
                        BUTTON_WATCH_STATIC.is_pressed(),
                        BUTTON_WATCH_STATIC.initialized(),
                    );
                    spawner.spawn(task_token).map_err($crate::Error::TaskSpawn)?;

                    let button_watch = $crate::button::ButtonWatchRp::new(
                        &BUTTON_WATCH_STATIC,
                    );
                    button_watch.wait_until_initialized().await;

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

            impl $crate::button::__ButtonMonitor for $name {
                fn is_pressed_raw(&self) -> bool {
                    <$crate::button::ButtonWatchRp as $crate::button::Button>::is_pressed(
                        &self.button_watch,
                    )
                }

                async fn wait_until_pressed_state(&mut self, pressed: bool) {
                    <$crate::button::ButtonWatchRp as $crate::button::__ButtonMonitor>::wait_until_pressed_state(
                        &mut self.button_watch,
                        pressed,
                    )
                    .await
                }
            }

            impl $crate::button::Button for $name {
                async fn wait_for_press_duration(&mut self) -> $crate::button::PressDuration {
                    <$crate::button::ButtonWatchRp as $crate::button::Button>::wait_for_press_duration(
                        &mut self.button_watch,
                    )
                    .await
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
                state_signal: &'static ::embassy_sync::signal::Signal<
                    ::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    bool
                >,
                state_changed_signal: &'static ::embassy_sync::signal::Signal<
                    ::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    ()
                >,
                initialized_signal: &'static ::embassy_sync::signal::Signal<
                    ::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    ()
                >,
                is_pressed: &'static ::core::sync::atomic::AtomicBool,
                initialized: &'static ::core::sync::atomic::AtomicBool,
            ) -> ! {
                $crate::button::button_watch_task(
                    pin,
                    pressed_to,
                    signal,
                    state_signal,
                    state_changed_signal,
                    initialized_signal,
                    is_pressed,
                    initialized,
                )
                .await
            }

        }
    };
}
