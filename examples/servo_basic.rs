#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use core::convert::Infallible;
use core::default::Default;
use device_kit::{
    Result,
    servo_player::{AtEnd, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

// Define ServoBasic, a struct type for a servo on PIN_11.
servo_player! {
    ServoBasic {
        pin: PIN_11,  // GPIO pin for servo PWM signal
        // other inputs set to their defaults
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = servo_basic_example(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn servo_basic_example(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // PIN_11 uses PWM_SLICE5 (slice = pin / 2 = 11 / 2 = 5)
    let servo_basic = ServoBasic::new(p.PIN_11, p.PWM_SLICE5, spawner)?;

    // Move to 90°, wait 1 second, then relax.
    servo_basic.set_degrees(90);
    Timer::after(Duration::from_secs(1)).await;
    servo_basic.relax();

    // Animate: hold at 180° for 1 second, then 0° for 1 second, then relax.
    const STEPS: [(u16, Duration); 2] =
        [(180, Duration::from_secs(1)), (0, Duration::from_secs(1))];
    servo_basic.animate(STEPS, AtEnd::Relax);

    core::future::pending().await // run forever
}
