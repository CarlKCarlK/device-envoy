#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    servo,
    servo_animate::{concat_steps, linear, ServoAnimate, ServoAnimateStatic, Step},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt::info, defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a servo animator on GPIO 11
    info!("Starting servo animate demo (GPIO 11)");
    static SERVO_ANIMATE_STATIC: ServoAnimateStatic = ServoAnimate::new_static();
    let servo_animate = ServoAnimate::new(
        &SERVO_ANIMATE_STATIC,
        servo! {
            pin: p.PIN_11,
            slice: p.PWM_SLICE5,  // rule: slice = (gpio/2) % 8; GPIO11 -> 5
        },
        spawner,
    )?;
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    servo_animate.set(0).await;
    Timer::after_millis(400).await;
    servo_animate.set(180).await;
    Timer::after_millis(400).await;
    servo_animate.set(90).await;

    const SWEEP_SECONDS: Duration = Duration::from_secs(2);
    const HOLD_MILLIS: Duration = Duration::from_millis(400);
    let sweep_up = linear::<19>(0, 180, SWEEP_SECONDS);
    let sweep_down = linear::<19>(180, 0, SWEEP_SECONDS);
    let hold_high = [Step {
        degrees: 180,
        duration: HOLD_MILLIS,
    }];
    let hold_low = [Step {
        degrees: 0,
        duration: HOLD_MILLIS,
    }];
    let animate_sequence = concat_steps::<64>(&[&sweep_up, &hold_high, &sweep_down, &hold_low]);

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                // Start the sweep animation (repeats until interrupted).
                info!("Servo animate sweep");
                servo_animate.animate(&animate_sequence).await;
            }
            PressDuration::Long => {
                // Interrupt animation and move to 90 degrees.
                info!("Servo set to 90 degrees");
                servo_animate.set(90).await;
            }
        }
    }
}
