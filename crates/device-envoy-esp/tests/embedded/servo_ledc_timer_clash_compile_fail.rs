//! Embedded compile-fail test target for an LEDC timer claimed twice.
//!
//! Two servos in different modules choose `Timer0`. The generated claim symbol
//! `LEDC_TIMER0_CAN_BE_USED_BY_ONLY_ONE_SERVO_OR_SERVO_PLAYER` is then defined twice, so
//! the build must fail. `servo_ledc_distinct_compile` is the passing counterpart.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, servo::servo};

esp_bootloader_esp_idf::esp_app_desc!();

mod first {
    super::servo! { pub ServoA { pin: GPIO2, timer: Timer0, channel: Channel0 } }
}

mod second {
    super::servo! { pub ServoB { pin: GPIO3, timer: Timer0, channel: Channel1 } }
}

async fn inner_main(_spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    let _servo_a = first::ServoA::new(&ledc, p.GPIO2)?;
    let _servo_b = second::ServoB::new(&ledc, p.GPIO3)?;
    core::future::pending().await
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}
