#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    combine,
    servo_player::{AtEnd, linear, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// Define a struct `ServoPlayer11` to servo player on PIN_11
servo_player! {
    ServoPlayer11 {
        pin: PIN_11,
        max_steps: 40,
    }
}

// Can control up to 8 servos.
servo_player! {
    ServoPlayer12 {
        pin: PIN_12,
        max_steps: 40,

        // Optional servo parameters; these are the defaults.
        min_us: 500,
        max_us: 2500,
        max_degrees: 180,
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
    // GPIO 11 → (11/2) % 8 = 5 → PWM_SLICE5
    let servo_player_11 = ServoPlayer11::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
    // GPIO 12 → (12/2) % 8 = 6 → PWM_SLICE6
    let servo_player_12 = ServoPlayer12::new(p.PIN_12, p.PWM_SLICE6, spawner)?;
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // But the two servos in the center position to start.
    servo_player_11.set_degrees(0);
    servo_player_12.set_degrees(180);
    Timer::after_millis(400).await;
    servo_player_11.set_degrees(90);
    servo_player_12.set_degrees(90);

    // Create a sweep animation: 0→180 (2s), hold (400ms), 180→0 (2s), hold (400ms)
    // Build animation using const arrays that concatenate at compile time.
    const SWEEP_UP: [(u16, Duration); 19] = linear(0, 180, Duration::from_secs(2));
    const SWEEP_DN: [(u16, Duration); 19] = linear(180, 0, Duration::from_secs(2));
    const STEPS: [(u16, Duration); 40] = combine!(
        SWEEP_UP,
        [(180, Duration::from_millis(400))],
        SWEEP_DN,
        [(0, Duration::from_millis(400))]
    );

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                //Play the sweep animation.
                servo_player_11.animate(STEPS, AtEnd::Relax);
                servo_player_12.animate(STEPS, AtEnd::Loop);
            }
            PressDuration::Long => {
                // Interrupt animation and move to 90 degrees.
                servo_player_11.animate([(90, Duration::from_millis(500))], AtEnd::Relax);
                servo_player_12.animate([(90, Duration::from_millis(500))], AtEnd::Relax);
            }
        }
    }
}
