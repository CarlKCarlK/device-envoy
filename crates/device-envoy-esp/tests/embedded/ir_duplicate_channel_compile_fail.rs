//! Embedded compile-fail test target for duplicate IR channel usage.
//!
//! This intentionally reuses one `ChannelCreator` value twice. In safe Rust this
//! must fail to compile because the first IR constructor consumes the channel.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplersDuplicateChannelCompileFail {
        IrKepler7: { pin: GPIO7 },
        IrKepler6: { pin: GPIO6 }
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
    init_and_start!(p, rmt80, rmt_mode::Async);

    #[cfg(target_arch = "xtensa")]
    let channel_creator = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator = rmt80.channel2;

    let _ir_kepler7 = IrKepler7::new(p.GPIO7, channel_creator, spawner)?;
    let _ir_kepler6 = IrKepler6::new(p.GPIO6, channel_creator, spawner)?;

    core::future::pending().await
}
