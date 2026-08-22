//! Embassy scheduler for the shared [`ConwayApp`](crate::conway_app::ConwayApp).

use crate::conway_app::{ConwayApp, ConwayInput};
use device_envoy_core::{
    ir::kepler::{IrKepler, KeplerKeys},
    led2d::{Frame2d, Led2d},
};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};

/// Run Conway on an LED panel and IR remote using the shared application state.
pub async fn conway_with_led2d_ir_kepler<L, I>(led2d: L, ir_kepler: I) -> !
where
    L: Led2d<16, 16>,
    I: IrKepler,
{
    let mut app = ConwayApp::new();
    loop {
        if !app.display_powered() {
            if ir_kepler.wait_for_press().await == KeplerKeys::Power {
                app.input(ConwayInput::Power);
                led2d.write_frame(app.frame());
            }
            continue;
        }

        led2d.write_frame(app.frame());
        let frame_duration = Duration::from_millis(app.tick_interval_ms() as u64);
        match select(Timer::after(frame_duration), ir_kepler.wait_for_press()).await {
            Either::First(_) => {
                app.tick();
            }
            Either::Second(key) => {
                if let Some(input) = input_for_key(key) {
                    app.input(input);
                    if !app.display_powered() {
                        led2d.write_frame(Frame2d::<16, 16>::new());
                    }
                }
            }
        }
    }
}

fn input_for_key(key: KeplerKeys) -> Option<ConwayInput> {
    Some(match key {
        KeplerKeys::Prev => ConwayInput::Previous,
        KeplerKeys::Next => ConwayInput::Next,
        KeplerKeys::PlayPause => ConwayInput::PlayPause,
        KeplerKeys::Power => ConwayInput::Power,
        KeplerKeys::Minus => ConwayInput::SpeedDown,
        KeplerKeys::Plus => ConwayInput::SpeedUp,
        KeplerKeys::Mode => ConwayInput::Mode,
        KeplerKeys::Repeat => ConwayInput::Repeat,
        KeplerKeys::USd => ConwayInput::UndoSymmetry,
        KeplerKeys::Num(number) if number < 10 => ConwayInput::Pattern(number),
        _ => return None,
    })
}
