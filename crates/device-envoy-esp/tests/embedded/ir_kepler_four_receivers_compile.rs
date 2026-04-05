//! Embedded compile-only test target for four Kepler IR receivers.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplersFourCompileTest {
        IrKepler7: { pin: GPIO7 },
        IrKepler6: { pin: GPIO6 },
        IrKepler5: { pin: GPIO5 },
        IrKepler4: { pin: GPIO4 }
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
        let (_ir_kepler7, _ir_kepler6, _ir_kepler5, _ir_kepler4) = IrKeplersFourCompileTest::new(
            p.GPIO7,
            rmt80.channel0,
            p.GPIO6,
            rmt80.channel1,
            p.GPIO5,
            rmt80.channel2,
            p.GPIO4,
            rmt80.channel3,
            spawner,
        )?;
    }

    #[cfg(all(target_arch = "xtensa", not(feature = "esp32s2")))]
    {
        let (_ir_kepler7, _ir_kepler6, _ir_kepler5, _ir_kepler4) = IrKeplersFourCompileTest::new(
            p.GPIO7,
            rmt80.channel4,
            p.GPIO6,
            rmt80.channel5,
            p.GPIO5,
            rmt80.channel6,
            p.GPIO4,
            rmt80.channel7,
            spawner,
        )?;
    }

    #[cfg(not(target_arch = "xtensa"))]
    let _ = (rmt80, spawner);

    core::future::pending().await
}
