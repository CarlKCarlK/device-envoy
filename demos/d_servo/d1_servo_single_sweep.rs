#![allow(missing_docs)]
#![no_std]
#![no_main]
#![cfg(not(feature = "host"))]

use core::{convert::Infallible, panic};
use device_kit::{
    Result,
    button::{Button, PressDuration, PressedTo},
    servo,
};
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // Create a servo on GPIO 11. Must also give "PWM slice".
    // rule: slice = (gpio/2) % 8; GPIO11 -> 5
    let mut servo = servo! { pin: p.PIN_11, slice: p.PWM_SLICE5 };

    // Start a background *hardware* control signal that says:
    // "To as fast as you can to 180 degrees and hold"
    servo.set_degrees(180);
    // Give it a reasonable time to get there.
    Timer::after_millis(400).await;
    servo.set_degrees(90);

    // Short press: move to next position. Long press: reverse direction.
    let mut degree = 0;
    let mut direction: i16 = 1;
    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                // Because 180 degrees is allowed, we wrap around at 190.
                degree = (degree as i16 + direction * 10).rem_euclid(190) as u16;
                servo.set_degrees(degree);
            }
            PressDuration::Long => {
                direction = -direction;
            }
        }
    }
}
