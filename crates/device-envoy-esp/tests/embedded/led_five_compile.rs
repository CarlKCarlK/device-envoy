//! Embedded compile-only test target for five macro-generated single LEDs.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_core::led::Led as _;
use device_envoy_esp::{
    init_and_start,
    led::{LedLevel, OnLevel},
};

device_envoy_esp::led! {
    LedOne {
        pin: GPIO2
    }
}
device_envoy_esp::led! {
    LedTwo {
        pin: GPIO3,
        max_steps: 40
    }
}
device_envoy_esp::led! {
    LedThree {
        pin: GPIO4
    }
}
device_envoy_esp::led! {
    LedFour {
        pin: GPIO5
    }
}
device_envoy_esp::led! {
    LedFive {
        pin: GPIO6
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p);

    let led_one = LedOne::new(p.GPIO2, OnLevel::High, spawner)?;
    let led_two = LedTwo::new(p.GPIO3, OnLevel::High, spawner)?;
    let led_three = LedThree::new(p.GPIO4, OnLevel::High, spawner)?;
    let led_four = LedFour::new(p.GPIO5, OnLevel::High, spawner)?;
    let led_five = LedFive::new(p.GPIO6, OnLevel::High, spawner)?;

    led_one.set_level(LedLevel::On);
    led_two.set_level(LedLevel::Off);
    led_three.set_level(LedLevel::On);
    led_four.set_level(LedLevel::Off);
    led_five.set_level(LedLevel::On);

    core::future::pending().await
}
