//! Embedded compile-pass test target for LEDC claims that don't clash.
//!
//! A servo and a servo player on distinct timers and channels must build. This is the
//! control for `servo_ledc_timer_clash_compile_fail` and
//! `servo_ledc_channel_clash_compile_fail`: those differ only in reusing a timer or channel.

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

servo_player! { pub PlayerB { pin: GPIO3, timer: Timer1, channel: Channel1 } }

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
