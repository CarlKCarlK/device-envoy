//! Embedded compile-only test target for four mapped IR receivers.

#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-only verification target")]

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
        IrMapping7: { pin: GPIO7 },
        IrMapping6: { pin: GPIO6 },
        IrMapping5: { pin: GPIO5 },
        IrMapping4: { pin: GPIO4 }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    #[cfg(feature = "esp32s2")]
    {
        let (_ir_mapping7, _ir_mapping6, _ir_mapping5, _ir_mapping4) =
            IrMappingsFourCompileTest::new(
                p.GPIO7,
                rmt80.channel0,
                p.GPIO6,
                rmt80.channel1,
                p.GPIO5,
                rmt80.channel2,
                p.GPIO4,
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
        let (_ir_mapping7, _ir_mapping6, _ir_mapping5, _ir_mapping4) =
            IrMappingsFourCompileTest::new(
                p.GPIO7,
                rmt80.channel4,
                p.GPIO6,
                rmt80.channel5,
                p.GPIO5,
                rmt80.channel6,
                p.GPIO4,
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
