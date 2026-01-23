//! Dual servo control example.
//! Moves two servos in opposite directions for 2 seconds.
//! Connect servos to GPIO 11 and GPIO 12.

#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use device_kit::servo;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
pub async fn main(_spawner: Spawner) -> ! {
    let p = embassy_rp::init(Default::default());

    info!("Starting dual servo example");
    // cmk00 in future, create macro servos! that can share PWM resources.

    // Create servos on GPIO 11 and GPIO 12
    // GPIO 11 → PWM_SLICE5 (channel B)
    // GPIO 12 → PWM_SLICE6 (channel A)
    let some_pin = p.PIN_11;
    let some_slice = p.PWM_SLICE5;
    let mut servo11 = servo! {
        pin: some_pin,
        slice: some_slice,
    };
    let mut servo12 = servo! {
        pin: p.PIN_12,
        slice: p.PWM_SLICE6,
    };

    info!("Moving servos in opposite directions for 2 seconds");

    let start = embassy_time::Instant::now();
    let duration = Duration::from_secs(2);

    loop {
        let elapsed = start.elapsed();
        if elapsed > duration {
            break;
        }

        // Move servos in opposite directions
        info!("Position: servo11=0°, servo12=180°");
        servo11.set_degrees(0);
        servo12.set_degrees(180);
        Timer::after_millis(500).await;

        // Move servos in opposite directions (swapped)
        info!("Position: servo11=180°, servo12=0°");
        servo11.set_degrees(180);
        servo12.set_degrees(0);
        Timer::after_millis(500).await;
    }

    info!("Done! Centering servos");
    servo11.center();
    servo12.center();

    Timer::after_millis(500).await;

    info!("Relaxing servos");
    servo11.disable();
    servo12.disable();

    Timer::after_secs(5).await;

    loop {
        info!("Sleeping");
        Timer::after_secs(5).await;
    }
}
