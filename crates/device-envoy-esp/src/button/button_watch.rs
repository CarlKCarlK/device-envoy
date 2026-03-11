//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`ButtonWatchEsp`] for usage.

#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(target_os = "none")]
use super::{ButtonEsp, PressDuration, PressedTo};
#[cfg(target_os = "none")]
use crate::{Error, Result};

/// Static resources for [`ButtonWatchEsp`].
#[cfg(target_os = "none")]
pub struct ButtonWatchStaticEsp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    is_pressed: AtomicBool,
    press_latch: AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchStaticEsp {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            press_latch: AtomicBool::new(false),
        }
    }
}

/// A device abstraction for background button monitoring.
///
/// Use this type when you need press detection that is not disrupted by other
/// fast loops/tasks that repeatedly cancel futures.
#[cfg(target_os = "none")]
pub struct ButtonWatchEsp<'a> {
    signal: &'a Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'a Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'a AtomicBool,
    press_latch: &'a AtomicBool,
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
    pub fn new_from_pin(
        button_watch_static: &'static ButtonWatchStaticEsp,
        button_pin: impl esp_hal::gpio::InputPin + 'static,
        pressed_to: PressedTo,
        spawner: Spawner,
    ) -> Result<Self> {
        let button = ButtonEsp::new(button_pin, pressed_to);
        let initial_pressed =
            <ButtonEsp<'_> as device_envoy_core::button::Button>::is_pressed(&button);
        button_watch_static
            .is_pressed
            .store(initial_pressed, Ordering::Relaxed);
        button_watch_static.state_signal.signal(initial_pressed);
        if initial_pressed {
            button_watch_static
                .press_latch
                .store(true, Ordering::Relaxed);
        }
        let (input, pressed_to) = button.into_parts();
        let token = button_watch_task_from_input(
            input,
            pressed_to,
            &button_watch_static.signal,
            &button_watch_static.state_signal,
            &button_watch_static.is_pressed,
            &button_watch_static.press_latch,
        );
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self {
            signal: &button_watch_static.signal,
            state_signal: &button_watch_static.state_signal,
            is_pressed: &button_watch_static.is_pressed,
            press_latch: &button_watch_static.press_latch,
        })
    }
}

#[cfg(target_os = "none")]
impl device_envoy_core::button::Button for ButtonWatchEsp<'_> {
    fn is_pressed(&self) -> bool {
        self.is_pressed.load(Ordering::Relaxed)
    }

    fn take_press_latch(&mut self) -> bool {
        let latched = self.press_latch.load(Ordering::Relaxed);
        if latched {
            self.press_latch.store(false, Ordering::Relaxed);
        }
        latched
    }

    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.signal.wait().await
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        if self.is_pressed() == pressed {
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

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn button_watch_task_from_input(
    mut input: esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'static AtomicBool,
    press_latch: &'static AtomicBool,
) -> ! {
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    signal_press_durations(&mut input_button, signal, state_signal, is_pressed, press_latch).await
}

#[cfg(target_os = "none")]
struct InputButton<'a> {
    input: &'a mut esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
async fn signal_press_durations<B: device_envoy_core::button::Button>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'static AtomicBool,
    press_latch: &'static AtomicBool,
) -> ! {
    let initial_pressed = button.is_pressed();
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);
    if initial_pressed {
        press_latch.store(true, Ordering::Relaxed);
    }

    loop {
        <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false).await;

        <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, true).await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);
        press_latch.store(true, Ordering::Relaxed);

        Timer::after(device_envoy_core::button::BUTTON_DEBOUNCE_DELAY).await;
        if !button.is_pressed() {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            continue;
        }

        let press_duration = embassy_futures::select::select(
            <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false),
            Timer::after(device_envoy_core::button::LONG_PRESS_DURATION),
        )
        .await;

        match press_duration {
            embassy_futures::select::Either::First(()) => {
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                signal.signal(PressDuration::Short);
            }
            embassy_futures::select::Either::Second(()) => {
                signal.signal(PressDuration::Long);
                <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false)
                    .await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
            }
        }
    }
}

/// Creates a button monitoring device abstraction with a background task.
///
/// This macro creates a generated wrapper type around [`ButtonWatchEsp`] with
/// RP-style constructor ergonomics.
///
/// See [`ButtonWatchGenerated`](crate::button::button_watch_generated::ButtonWatchGenerated)
/// for a sample of what the macro generates.
///
/// # Constructors
///
/// - [`new()`](crate::button::button_watch_generated::ButtonWatchGenerated::new) — Create from a pin
///
/// Syntax:
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

/// Implementation macro for [`button_watch!`].
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
                /// # Errors
                ///
                /// Returns an error if the background task cannot be spawned.
                pub fn new(
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
                    )?;

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

            impl $crate::button::Button for $name {
                fn is_pressed(&self) -> bool {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::is_pressed(
                        &self.button_watch,
                    )
                }

                async fn wait_for_press_duration(&mut self) -> $crate::button::PressDuration {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::wait_for_press_duration(
                        &mut self.button_watch,
                    )
                    .await
                }

                async fn wait_until_pressed_state(&mut self, pressed: bool) {
                    <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::wait_until_pressed_state(
                        &mut self.button_watch,
                        pressed,
                    )
                    .await
                }
            }
        }
    };
}
