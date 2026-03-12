//! Embedded compile-only test target for two Led4Esp displays with independent statics.

#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};

use device_envoy_esp::{
    init_and_start,
    led4::{BlinkState, Led4 as _, Led4Esp, Led4EspStatic, OutputArray},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);

    let cell_pins0 = OutputArray::new([
        Output::new(p.GPIO0, Level::High, OutputConfig::default()),
        Output::new(p.GPIO1, Level::High, OutputConfig::default()),
        Output::new(p.GPIO2, Level::High, OutputConfig::default()),
        Output::new(p.GPIO3, Level::High, OutputConfig::default()),
    ]);
    let segment_pins0 = OutputArray::new([
        Output::new(p.GPIO4, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO5, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO6, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO7, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO8, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO9, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO10, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO11, Level::Low, OutputConfig::default()),
    ]);

    let cell_pins1 = OutputArray::new([
        Output::new(p.GPIO12, Level::High, OutputConfig::default()),
        Output::new(p.GPIO13, Level::High, OutputConfig::default()),
        Output::new(p.GPIO14, Level::High, OutputConfig::default()),
        Output::new(p.GPIO15, Level::High, OutputConfig::default()),
    ]);
    let segment_pins1 = OutputArray::new([
        Output::new(p.GPIO16, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO17, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO18, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO19, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO20, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO21, Level::Low, OutputConfig::default()),
        #[cfg(target_arch = "xtensa")]
        Output::new(p.GPIO46, Level::Low, OutputConfig::default()),
        #[cfg(not(target_arch = "xtensa"))]
        Output::new(p.GPIO22, Level::Low, OutputConfig::default()),
        #[cfg(target_arch = "xtensa")]
        Output::new(p.GPIO48, Level::Low, OutputConfig::default()),
        #[cfg(not(target_arch = "xtensa"))]
        Output::new(p.GPIO23, Level::Low, OutputConfig::default()),
    ]);

    static LED4_0_STATIC: Led4EspStatic = Led4Esp::new_static();
    static LED4_1_STATIC: Led4EspStatic = Led4Esp::new_static();

    let led4_0 = Led4Esp::new(&LED4_0_STATIC, cell_pins0, segment_pins0, spawner)?;
    let led4_1 = Led4Esp::new(&LED4_1_STATIC, cell_pins1, segment_pins1, spawner)?;

    led4_0.write_text(['1', '2', '3', '4'], BlinkState::Solid);
    led4_1.write_text(['a', 'b', 'c', 'd'], BlinkState::Solid);

    pending().await
}
