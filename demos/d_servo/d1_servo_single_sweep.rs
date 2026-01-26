#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, future, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    servo,
};
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt::info, defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a servo on GPIO 11.
    // We must also give the "PWM slice" resource for this pin.
    let mut servo = servo! {
        pin: p.PIN_11,
        slice: p.PWM_SLICE5,  // rule: slice = (gpio/2) % 8; GPIO11 -> 5
    };
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // Start a background *hardware* control signal that says:
    // "To as fast as you can to 0 degrees and hold"
    servo.set_degrees(0);
    // Give it a reasonable time to get there.
    Timer::after_millis(400).await;
    servo.set_degrees(180);
    Timer::after_millis(400).await;
    servo.set_degrees(90);

    // Every time you press the button, go to 0, 10, 20, ..., 180 degrees (repeat).
    // (Use a cyclic iterator)
    for degree in (0..=180).step_by(10).cycle() {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                // Move servo to next position
                info!("Servo -> {} degrees", degree);
                servo.set_degrees(degree);
            }
            PressDuration::Long => {
                // Relax the servo to make it quiet.
                servo.relax();
            }
        }
    }

    future::pending().await // Needed because compiler can't see that loop is infinite
}
