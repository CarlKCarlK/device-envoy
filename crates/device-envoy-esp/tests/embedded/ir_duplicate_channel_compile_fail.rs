//! Embedded compile-fail test target for duplicate IR channel usage.
//!
//! This intentionally reuses one `ChannelCreator` value twice. In safe Rust this
//! must fail to compile because the first IR constructor consumes the channel.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplersDuplicateChannelCompileFail {
        IrKepler0: { pin: GPIO0 },
        IrKepler1: { pin: GPIO1 }
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
    let channel_creator = rmt80.channel0;
    #[cfg(all(target_arch = "xtensa", not(feature = "esp32s2")))]
    let channel_creator = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator = rmt80.channel2;

    let _ir_kepler0 = IrKepler0::new(p.GPIO0, channel_creator, spawner)?;
    let _ir_kepler1 = IrKepler1::new(p.GPIO1, channel_creator, spawner)?;

    core::future::pending().await
}
