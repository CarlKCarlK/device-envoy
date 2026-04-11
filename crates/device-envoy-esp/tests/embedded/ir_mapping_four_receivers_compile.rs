//! Embedded compile-only test target for four mapped IR receivers.

#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-only verification target")]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir_mappings};

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy, Eq, PartialEq)]
enum TestButton {
    One,
}

const BUTTON_MAP: [(u16, u8, TestButton); 1] = [(0x0000, 0x45, TestButton::One)];

ir_mappings! {
    button: TestButton,
    capacity: 1,
    IrMappingsFourCompileTest {
        IrMapping3: { pin: GPIO3 },
        IrMapping2: { pin: GPIO2 },
        IrMapping1: { pin: GPIO1 },
        IrMapping0: { pin: GPIO0 }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    #[cfg(feature = "esp32s2")]
    {
        let (_ir_mapping3, _ir_mapping2, _ir_mapping1, _ir_mapping0) =
            IrMappingsFourCompileTest::new(
                p.GPIO3,
                rmt80.channel0,
                p.GPIO2,
                rmt80.channel1,
                p.GPIO1,
                rmt80.channel2,
                p.GPIO0,
                rmt80.channel3,
                &BUTTON_MAP,
                &BUTTON_MAP,
                &BUTTON_MAP,
                &BUTTON_MAP,
                spawner,
            )?;
    }

    #[cfg(all(target_arch = "xtensa", not(feature = "esp32s2")))]
    {
        let (_ir_mapping3, _ir_mapping2, _ir_mapping1, _ir_mapping0) =
            IrMappingsFourCompileTest::new(
                p.GPIO3,
                rmt80.channel4,
                p.GPIO2,
                rmt80.channel5,
                p.GPIO1,
                rmt80.channel6,
                p.GPIO0,
                rmt80.channel7,
                &BUTTON_MAP,
                &BUTTON_MAP,
                &BUTTON_MAP,
                &BUTTON_MAP,
                spawner,
            )?;
    }

    #[cfg(not(target_arch = "xtensa"))]
    let _ = (rmt80, spawner);

    core::future::pending().await
}
