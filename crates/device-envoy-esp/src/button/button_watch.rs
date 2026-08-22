//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`button_watch!`](crate::button_watch!) for usage.

#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use device_envoy_core::button::{__ButtonMonitor, Button};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(target_os = "none")]
use super::{PressDuration, PressedTo};
#[cfg(target_os = "none")]
use crate::{Error, Result};

/// Static resources for [`ButtonWatchEsp`].
#[cfg(target_os = "none")]
// Public only because `button_watch!` macro expansions reference this type in
// downstream crates.
#[doc(hidden)]
pub struct ButtonWatchStaticEsp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: Signal<CriticalSectionRawMutex, ()>,
    is_pressed: AtomicBool,
    initialized: AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchStaticEsp {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            state_changed_signal: Signal::new(),
            initialized_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }
}

/// A device abstraction for background button monitoring.
///
/// Use this type when you need press detection that is not disrupted by other
/// fast loops/tasks that repeatedly cancel futures.
#[cfg(target_os = "none")]
// Public only because `button_watch!` macro expansions reference this type in
// downstream crates.
#[doc(hidden)]
pub struct ButtonWatchEsp<'a> {
    signal: &'a Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'a Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'a Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'a Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'a AtomicBool,
    initialized: &'a AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchEsp<'_> {
    /// Create static resources for [`ButtonWatchEsp::new_from_pin`].
    #[must_use]
    pub const fn new_static() -> ButtonWatchStaticEsp {
        ButtonWatchStaticEsp::new()
    }

    /// Create a background button monitor directly from a pin.
    #[must_use = "Must be used to manage the spawned task"]
    pub async fn new_from_pin(
        button_watch_static: &'static ButtonWatchStaticEsp,
        button_pin: impl esp_hal::gpio::InputPin + 'static,
        pressed_to: PressedTo,
        spawner: Spawner,
    ) -> Result<Self> {
        let pull = match pressed_to {
            PressedTo::Voltage => esp_hal::gpio::Pull::Down,
            PressedTo::Ground => esp_hal::gpio::Pull::Up,
        };
        let input = esp_hal::gpio::Input::new(
            button_pin,
            esp_hal::gpio::InputConfig::default().with_pull(pull),
        );
        let token = button_watch_task_from_pin(
            input,
            pressed_to,
            &button_watch_static.signal,
            &button_watch_static.state_signal,
            &button_watch_static.state_changed_signal,
            &button_watch_static.initialized_signal,
            &button_watch_static.is_pressed,
            &button_watch_static.initialized,
        );
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        let button_watch = Self {
            signal: &button_watch_static.signal,
            state_signal: &button_watch_static.state_signal,
            state_changed_signal: &button_watch_static.state_changed_signal,
            initialized_signal: &button_watch_static.initialized_signal,
            is_pressed: &button_watch_static.is_pressed,
            initialized: &button_watch_static.initialized,
        };
        button_watch.wait_until_initialized().await;
        Ok(button_watch)
    }

    /// Wait until the first sampled state has been captured by the background task.
    pub async fn wait_until_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }
        self.initialized_signal.wait().await;
    }

    /// Wait for any button state transition (press or release).
    pub async fn wait_for_state_change(&self) {
        self.state_changed_signal.wait().await;
    }
}

#[cfg(target_os = "none")]
impl __ButtonMonitor for ButtonWatchEsp<'_> {
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

#[cfg(target_os = "none")]
impl Button for ButtonWatchEsp<'_> {
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.signal.wait().await
    }
}

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn button_watch_task_from_pin(
    mut input: esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
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

#[cfg(target_os = "none")]
struct InputButton<'a> {
    input: &'a mut esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
impl __ButtonMonitor for InputButton<'_> {
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

#[cfg(target_os = "none")]
async fn signal_press_durations<B: __ButtonMonitor>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
    let initial_pressed = B::is_pressed_raw(button);
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);
    initialized.store(true, Ordering::Release);
    initialized_signal.signal(());

    loop {
        B::wait_until_pressed_state(button, false).await;

        B::wait_until_pressed_state(button, true).await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);
        state_changed_signal.signal(());

        Timer::after(device_envoy_core::button::BUTTON_DEBOUNCE_DELAY).await;
        if !B::is_pressed_raw(button) {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            state_changed_signal.signal(());
            continue;
        }

        let press_duration = embassy_futures::select::select(
            B::wait_until_pressed_state(button, false),
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
                B::wait_until_pressed_state(button, false).await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                state_changed_signal.signal(());
            }
        }
    }
}

/// Creates a button monitoring device abstraction with a background task.
///
/// This macro creates a button monitor that runs in a dedicated background task,
/// providing continuous monitoring without interruption.
///
/// See [`ButtonWatchGenerated`](crate::button::button_watch_generated::ButtonWatchGenerated)
/// for a sample of what the macro generates.
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
///
/// # Constructors
///
/// - [`new()`](crate::button::button_watch_generated::ButtonWatchGenerated::new) — Create from a pin
///
/// # Use Cases
///
/// Use `button_watch!` instead of [`ButtonEsp`](super::ButtonEsp) when you need continuous monitoring
/// that works even in fast loops or `select()` operations. [`ButtonEsp`](super::ButtonEsp) starts
/// fresh monitoring on each call to `wait_for_press()`, which can miss events in busy loops.
///
/// # Parameters
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
/// use device_envoy_esp::{
///     Result,
///     button::{Button as _, PressDuration, PressedTo},
///     button_watch,
/// };
/// use embassy_executor::Spawner;
/// # use esp_backtrace as _;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// button_watch! {
///     ButtonWatch13 {
///         pin: GPIO13,
///     }
/// }
///
/// async fn example(
///     p: esp_hal::peripherals::Peripherals,
///     spawner: Spawner,
/// ) -> Result<()> {
///     // Create the button monitor (spawns background task automatically)
///     let mut button_watch13 = ButtonWatch13::new(p.GPIO13, PressedTo::Ground, spawner)
///         .await?;
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
///     Ok(())
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! button_watch {
    ($($tt:tt)*) => { $crate::__button_watch_impl! { $($tt)* } };
}

/// Implementation macro for [`button_watch!`].
///
/// Do not call directly - use [`button_watch!`](crate::button_watch!) instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __button_watch_impl {
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
                "See the [button module documentation](mod@$crate::button) for usage."
            )]
            $vis struct $name {
                button_watch: $crate::button::ButtonWatchEsp<'static>,
            }

            impl $name {
                /// Creates a new button monitor and spawns its background task.
                ///
                /// # Parameters
                ///
                /// - `button_pin`: GPIO pin for the button
                /// - `pressed_to`: How the button is wired ([`PressedTo::Ground`] or [`PressedTo::Voltage`])
                /// - `spawner`: Task spawner for background operations
                ///
                /// # Errors
                ///
                /// Returns an error if the background task cannot be spawned.
                pub async fn new(
                    button_pin: $crate::esp_hal::peripherals::$pin<'static>,
                    pressed_to: $crate::button::PressedTo,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static mut Self> {
                    static BUTTON_WATCH_STATIC: $crate::button::ButtonWatchStaticEsp =
                        $crate::button::ButtonWatchEsp::new_static();
                    static BUTTON_WATCH_CELL: ::static_cell::StaticCell<$name> =
                        ::static_cell::StaticCell::new();

                    let button_watch = $crate::button::ButtonWatchEsp::new_from_pin(
                        &BUTTON_WATCH_STATIC,
                        button_pin,
                        pressed_to,
                        spawner,
                    )
                    .await?;

                    let instance = BUTTON_WATCH_CELL.init($name { button_watch });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::button::ButtonWatchEsp<'static>;

                fn deref(&self) -> &Self::Target {
                    &self.button_watch
                }
            }

            impl $crate::button::__ButtonMonitor for $name {
                fn is_pressed_raw(&self) -> bool {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::is_pressed(
                        &self.button_watch,
                    )
                }

                async fn wait_until_pressed_state(&mut self, pressed: bool) {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::__ButtonMonitor>::wait_until_pressed_state(
                        &mut self.button_watch,
                        pressed,
                    )
                    .await
                }
            }

            impl $crate::button::Button for $name {
                async fn wait_for_press_duration(&mut self) -> $crate::button::PressDuration {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::wait_for_press_duration(
                        &mut self.button_watch,
                    )
                    .await
                }
            }
        }
    };
}
