#![no_std]
#![no_main]

use core::{convert::Infallible, panic};
use device_kit::{Result, servo};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // Create a servo on GPIO 11.
    // GPIO 11 → (11/2) % 8 = 5 → PWM_SLICE5
    let mut servo = servo! {
        pin: p.PIN_11,
        slice: p.PWM_SLICE5,
    };

    servo.set_degrees(45);                          // Move to 45 degrees and hold.
    Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
    servo.set_degrees(90);                          // Move to 90 degrees and hold.
    Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
    servo.relax();                                  // Let the servo relax. It will re-enable on next set_degrees()

    core::future::pending().await
}
