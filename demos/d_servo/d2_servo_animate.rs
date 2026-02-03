#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_envoy::{
    Result,
    button::{Button, PressDuration, PressedTo},
    servo_player::{AtEnd, combine, linear, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// Define ServoPlayer11 with up to 40 animation steps.
servo_player! {
    ServoPlayer11 {pin: PIN_11, max_steps: 40}
}

// Define a 2nd ServoPlayer. (Can define up to 8).
servo_player! {
    ServoPlayer12 {
        pin: PIN_12, max_steps: 40,
        // Optional servo parameters; these are the defaults.
        min_us: 500, max_us: 2500, max_degrees: 180,
    }
}

// Create a const array of (degrees, duration) steps for sweeping the servo.
// Compiler will catch size mismatches.
const STEPS: [(u16, Duration); 40] = combine!(
    linear::<19>(0, 180, Duration::from_secs(2)), // sweep up in 19 steps
    [(180, Duration::from_millis(400))],          // hold
    linear::<19>(180, 0, Duration::from_secs(2)), // sweep down in 19 steps
    [(0, Duration::from_millis(400))]             // hold
);

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // Create the servo player on GPIO 11. (11/2) % 8 = 5 → PWM_SLICE5
    let servo_player_11 = ServoPlayer11::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
    // GPIO 12 → (12/2) % 8 = 6 → PWM_SLICE6
    let servo_player_12 = ServoPlayer12::new(p.PIN_12, p.PWM_SLICE6, spawner)?;

    loop {
        // Put the two servos in a "ready" position.
        servo_player_11.set_degrees(90);
        servo_player_12.set_degrees(90);
        Timer::after_millis(500).await;
        servo_player_11.relax(); // make the servos quiet.
        servo_player_12.relax();

        loop {
            // On short press, (re)start the sweep animation on both servos.
            match button.wait_for_press_duration().await {
                PressDuration::Short => {
                    // Play the sweep animation with two endings.
                    servo_player_11.animate(STEPS, AtEnd::Relax);
                    servo_player_12.animate(STEPS, AtEnd::Loop);
                }
                // On long press, exit inner loop and go back to "ready" position.
                PressDuration::Long => break,
            }
        }
    }
}
