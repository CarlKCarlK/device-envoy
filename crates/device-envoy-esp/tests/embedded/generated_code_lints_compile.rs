//! Embedded compile-only check that the code the declaration macros generate
//! builds cleanly in a user crate that denies warnings and missing docs.
//!
//! Each macro is declared once publicly and once privately where pins allow,
//! and each device is constructed, as a typical application would.

#![no_std]
#![no_main]
#![deny(warnings, missing_docs)]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    button::PressedTo,
    button_watch, init_and_start,
    led::{OnLevel, led},
    led_strip,
    led_strip::Current,
    servo::servo_player,
};

esp_bootloader_esp_idf::esp_app_desc!();

led! {
    /// A public status LED.
    pub StatusLed { pin: GPIO0 }
}

led! {
    /// A private busy LED.
    BusyLed { pin: GPIO1 }
}

button_watch! {
    /// A public button.
    pub PublicButton { pin: GPIO2 }
}

servo_player! {
    /// A public servo player.
    pub PublicServo { pin: GPIO3, timer: Timer0, channel: Channel0 }
}

led_strip! {
    /// A public LED strip.
    pub PublicStrip { pin: GPIO4, len: 8, max_current: Current::Milliamps(100) }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking, ledc: ledc);

    let status_led = StatusLed::new(p.GPIO0, OnLevel::High, spawner)?;
    let busy_led = BusyLed::new(p.GPIO1, OnLevel::High, spawner)?;
    let public_button = PublicButton::new(p.GPIO2, PressedTo::Ground, spawner).await?;
    let public_servo = PublicServo::new(&ledc, p.GPIO3, spawner)?;
    let public_strip = PublicStrip::new(p.GPIO4, rmt80.channel0, spawner)?;

    esp_println::println!(
        "{}",
        size_of_val(&status_led)
            + size_of_val(&busy_led)
            + size_of_val(public_button)
            + size_of_val(&public_servo)
            + size_of_val(public_strip)
    );
    core::future::pending().await
}
