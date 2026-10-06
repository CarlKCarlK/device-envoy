//! Embedded compile-fail test target for an LEDC channel claimed twice.
//!
//! A servo and a servo player both choose `Channel0`. The claim is shared by both macros,
//! so `LEDC_CHANNEL0_CAN_BE_USED_BY_ONLY_ONE_SERVO_OR_SERVO_PLAYER` is defined twice and
//! the build must fail. `servo_ledc_distinct_compile` is the passing counterpart.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    servo::{servo, servo_player},
};

esp_bootloader_esp_idf::esp_app_desc!();

servo! { pub ServoA { pin: GPIO2, timer: Timer0, channel: Channel0 } }

servo_player! { pub PlayerB { pin: GPIO3, timer: Timer1, channel: Channel0 } }

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    let _servo_a = ServoA::new(&ledc, p.GPIO2)?;
    let _player_b = PlayerB::new(&ledc, p.GPIO3, spawner)?;
    core::future::pending().await
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}
