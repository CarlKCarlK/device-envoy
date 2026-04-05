//! Embedded compile-only test target for five ButtonWatchEsp devices on distinct GPIOs.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    button::{Button as _, PressedTo},
    button_watch, init_and_start,
};

esp_bootloader_esp_idf::esp_app_desc!();

button_watch! {
    ButtonWatch0 {
        pin: GPIO0,
    }
}

button_watch! {
    ButtonWatch1 {
        pin: GPIO1,
    }
}

button_watch! {
    ButtonWatch2 {
        pin: GPIO2,
    }
}

button_watch! {
    ButtonWatch3 {
        pin: GPIO3,
    }
}

button_watch! {
    ButtonWatch4 {
        pin: GPIO4,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);

    let button_watch0 = ButtonWatch0::new(p.GPIO0, PressedTo::Ground, spawner).await?;
    let button_watch1 = ButtonWatch1::new(p.GPIO1, PressedTo::Ground, spawner).await?;
    let button_watch2 = ButtonWatch2::new(p.GPIO2, PressedTo::Ground, spawner).await?;
    let button_watch3 = ButtonWatch3::new(p.GPIO3, PressedTo::Ground, spawner).await?;
    let button_watch4 = ButtonWatch4::new(p.GPIO4, PressedTo::Ground, spawner).await?;

    let _ = button_watch0.is_pressed();
    let _ = button_watch1.is_pressed();
    let _ = button_watch2.is_pressed();
    let _ = button_watch3.is_pressed();
    let _ = button_watch4.is_pressed();

    core::future::pending().await
}
