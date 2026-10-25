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
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
    servo::servo_player,
};

esp_bootloader_esp_idf::esp_app_desc!();

/// Servo steps built with the crate-root `combine!` path.
const SERVO_STEPS: [(u16, embassy_time::Duration); 2] = device_envoy_esp::combine!(
    [(0, embassy_time::Duration::from_millis(500))],
    [(90, embassy_time::Duration::from_millis(500))]
);

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

/// The public panel's layout.
const PANEL_LAYOUT: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

led2d! {
    /// A public panel.
    pub PublicPanel { pin: GPIO5, led_layout: PANEL_LAYOUT, font: Led2dFont::Font3x4Trim }
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
    init_and_start!(p);
    let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
    let ledc = device_envoy_esp::new_ledc(p.LEDC);

    let status_led = StatusLed::new(p.GPIO0, OnLevel::High, spawner)?;
    let busy_led = BusyLed::new(p.GPIO1, OnLevel::High, spawner)?;
    let public_button = PublicButton::new(p.GPIO2, PressedTo::Ground, spawner).await?;
    let public_servo = PublicServo::new(p.GPIO3, &ledc, spawner)?;
    let public_strip = PublicStrip::new(p.GPIO4, rmt80.channel0, spawner)?;
    let public_panel = PublicPanel::new(p.GPIO5, rmt80.channel1, spawner)?;

    esp_println::println!(
        "{}",
        size_of_val(status_led)
            + size_of_val(busy_led)
            + size_of_val(public_button)
            + size_of_val(&public_servo)
            + size_of_val(public_strip)
            + size_of_val(public_panel)
            + size_of_val(public_panel.strip())
            + size_of_val(&SERVO_STEPS)
    );
    core::future::pending().await
}
