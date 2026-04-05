//! Embedded compile-only test target for five ButtonEsp devices on distinct GPIOs.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);

    let button0 = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
    let button1 = ButtonEsp::new(p.GPIO1, PressedTo::Ground);
    let button2 = ButtonEsp::new(p.GPIO2, PressedTo::Ground);
    let button3 = ButtonEsp::new(p.GPIO3, PressedTo::Ground);
    let button4 = ButtonEsp::new(p.GPIO4, PressedTo::Ground);

    let _ = button0.is_pressed();
    let _ = button1.is_pressed();
    let _ = button2.is_pressed();
    let _ = button3.is_pressed();
    let _ = button4.is_pressed();

    core::future::pending().await
}
