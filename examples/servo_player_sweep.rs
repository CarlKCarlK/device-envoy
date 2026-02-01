#![allow(missing_docs)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use core::convert::Infallible;
use core::default::Default;
use device_kit::{
    Result,
    servo_player::{AtEnd, combine, linear, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::Duration;

// Define ServoSweep, a struct type for a servo on PIN_12.
servo_player! {
    ServoSweep {
        pin: PIN_12,
        max_steps: 40,          // Increase from default (16) to hold all segments

       // Optional
        min_us: 500,            // Minimum pulse width (µs) for 0° (default)
        max_us: 2500,           // Maximum pulse width (µs) for max_degrees (default)
        max_degrees: 180,       // Maximum servo angle (degrees) (default)
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = servo_sweep_example(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn servo_sweep_example(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let servo_sweep = ServoSweep::new(p.PIN_12, p.PWM_SLICE6, spawner)?;

    // Combine animation segments into one sequence.
    const STEPS: [(u16, Duration); 40] = combine!(
        linear::<19>(0, 180, Duration::from_secs(2)), // 19 steps from 0° to 180°
        [(180, Duration::from_millis(400))],          // Hold at 180° for 400 ms
        linear::<19>(180, 0, Duration::from_secs(2)), // 19 steps from 180° to 0°
        [(0, Duration::from_millis(400))]             // Hold at 0° for 400 ms
    );

    servo_sweep.animate(STEPS, AtEnd::Loop); // Loop the sweep animation

    // Let it run for 10 seconds, then relax.
    embassy_time::Timer::after(Duration::from_secs(10)).await;
    servo_sweep.relax();

    core::future::pending().await // run forever
}
