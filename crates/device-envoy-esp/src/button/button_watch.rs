//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`button_watch!`](crate::button_watch!) for usage.
//!
//! The monitoring loop and the handle live in core; this module supplies the ESP pin sampling.

use device_envoy_core::button::{__ButtonMonitor, __run_button_watch, ButtonWatchStatic};
use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

use super::PressedTo;

/// Body of each generated button's background task: samples `pin` forever.
///
/// Never call directly - each [`button_watch!`](crate::button_watch!) declaration spawns its
/// own task that calls this, so every button has its own task.
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub async fn __button_watch_task(
    pin: impl InputPin + 'static,
    pressed_to: PressedTo,
    button_watch_static: &'static ButtonWatchStatic,
) -> ! {
    let pull = match pressed_to {
        PressedTo::Voltage => Pull::Down,
        PressedTo::Ground => Pull::Up,
    };
    let mut input = Input::new(pin, InputConfig::default().with_pull(pull));
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    __run_button_watch(&mut input_button, button_watch_static).await
}

struct InputButton<'a> {
    input: &'a mut Input<'static>,
    pressed_to: PressedTo,
}

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
