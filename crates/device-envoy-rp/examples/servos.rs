//! Dual servo control example.
//! Moves two servos in opposite directions for 2 seconds.
//! Connect servos to GPIO 11 and GPIO 12.

#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{servo, servo::Servo as _};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;

#[embassy_executor::main]
pub async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    info!("Starting dual servo example");
    // TODO in future, create macro servos! (plural) that can share PWM resources.

    // Create servos on GPIO 11 and GPIO 12
    // GPIO 11 → PWM_SLICE5 (channel B)
    // GPIO 12 → PWM_SLICE6 (channel A)
    let servo11 = servo! {
        pin: p.PIN_11,
        slice: p.PWM_SLICE5,
    };
    let servo12 = servo! {
        pin: p.PIN_12,
        slice: p.PWM_SLICE6,
    };

    info!("Moving servos in opposite directions for 2 seconds");

    let start = Instant::now();
    let duration = Duration::from_secs(2);

    loop {
        let elapsed = start.elapsed();
        if elapsed > duration {
            break;
        }

        // Move servos in opposite directions
        servo11.set_degrees(0);
        servo12.set_degrees(180);
        Timer::after_millis(500).await;

        // Move servos in opposite directions (swapped)
        servo11.set_degrees(180);
        servo12.set_degrees(0);
        Timer::after_millis(500).await;
    }

    info!("Done! Centering servos");
    servo11.set_degrees(90);
    servo12.set_degrees(90);

    Timer::after_millis(500).await;

    info!("Relaxing servos");
    servo11.relax();
    servo12.relax();

    pending().await
}
