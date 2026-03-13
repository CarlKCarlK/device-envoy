//! Embedded compile-only test target for IR single-item macro visibility.

#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{ir, ir_kepler, ir_mapping};

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppButton {
    Power,
}

ir! {
    pub IrPublic { pin: GPIO7 }
}

ir! {
    IrPrivate { pin: GPIO6 }
}

ir_kepler! {
    pub IrKeplerPublic { pin: GPIO5 }
}

ir_kepler! {
    IrKeplerPrivate { pin: GPIO4 }
}

ir_mapping! {
    pub IrMappingPublic {
        pin: GPIO3,
        button: AppButton,
        capacity: 1,
    }
}

ir_mapping! {
    IrMappingPrivate {
        pin: GPIO2,
        button: AppButton,
        capacity: 1,
    }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    core::future::pending().await
}
