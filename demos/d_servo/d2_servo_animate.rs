#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    servo_player::{AtEnd, linear, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt::info, defmt_rtt as _, panic_probe as _};

servo_player! {
    // cmk000 good name?
    DemoServo {
        pin: PIN_11,
        max_steps: 64,
        // cmk000 what's optional here?
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a servo player on GPIO 11
    info!("Starting servo player demo (GPIO 11)");
    // cmk000 give the formuala agan here
    // cmk000 good name?
    let demo_servo = DemoServo::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // It's set here and set_degree in servero
    demo_servo.set(0);
    Timer::after_millis(400).await;
    demo_servo.set(180);
    Timer::after_millis(400).await;
    demo_servo.set(90);

    // cmk0000 this seems overly complex
    // Create a sweep animation: 0→180 (2s), hold (400ms), 180→0 (2s), hold (400ms)
    const SWEEP_DURATION: Duration = Duration::from_secs(2);
    const HOLD_DURATION: Duration = Duration::from_millis(400);

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                // Start the sweep animation (repeats until interrupted).
                info!("Servo animate sweep");
                // cmk000 understand the enum better
                let steps = linear(0, 180, SWEEP_DURATION, 19)
                    .chain([(180, HOLD_DURATION)])
                    .chain(linear(180, 0, SWEEP_DURATION, 19))
                    .chain([(0, HOLD_DURATION)]);
                demo_servo.animate(steps, AtEnd::Loop);
            }
            PressDuration::Long => {
                // Interrupt animation and move to 90 degrees.
                info!("Servo set to 90 degrees");
                demo_servo.set(90);
            }
        }
    }
}
