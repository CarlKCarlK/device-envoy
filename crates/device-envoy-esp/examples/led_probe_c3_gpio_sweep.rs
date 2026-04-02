#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use log::info;

use device_envoy_esp::{init_and_start, Result};

esp_bootloader_esp_idf::esp_app_desc!();

const ON_HOLD: Duration = Duration::from_millis(900);
const OFF_HOLD: Duration = Duration::from_millis(500);

macro_rules! probe_pin {
    ($pin_var:ident, $pin_label:literal) => {{
        info!("probe {}: HIGH", $pin_label);
        $pin_var.set_high();
        Timer::after(ON_HOLD).await;

        info!("probe {}: LOW", $pin_label);
        $pin_var.set_low();
        Timer::after(OFF_HOLD).await;
    }};
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("ESP32-C3 GPIO LED probe sweep starting");
    info!("Using a safe pin set that avoids likely UART/USB console pins");

    let mut gpio0 = Output::new(p.GPIO0, Level::Low, OutputConfig::default());
    let mut gpio1 = Output::new(p.GPIO1, Level::Low, OutputConfig::default());
    let mut gpio2 = Output::new(p.GPIO2, Level::Low, OutputConfig::default());
    let mut gpio3 = Output::new(p.GPIO3, Level::Low, OutputConfig::default());
    let mut gpio4 = Output::new(p.GPIO4, Level::Low, OutputConfig::default());
    let mut gpio5 = Output::new(p.GPIO5, Level::Low, OutputConfig::default());
    let mut gpio6 = Output::new(p.GPIO6, Level::Low, OutputConfig::default());
    let mut gpio7 = Output::new(p.GPIO7, Level::Low, OutputConfig::default());
    let mut gpio8 = Output::new(p.GPIO8, Level::Low, OutputConfig::default());
    let mut gpio9 = Output::new(p.GPIO9, Level::Low, OutputConfig::default());
    let mut gpio10 = Output::new(p.GPIO10, Level::Low, OutputConfig::default());

    probe_pin!(gpio4, "GPIO4 (possible D4 label)");
    probe_pin!(gpio6, "GPIO6 (possible D6 label)");

    probe_pin!(gpio0, "GPIO0");
    probe_pin!(gpio1, "GPIO1");
    probe_pin!(gpio2, "GPIO2");
    probe_pin!(gpio3, "GPIO3");
    probe_pin!(gpio5, "GPIO5");
    probe_pin!(gpio7, "GPIO7");
    probe_pin!(gpio8, "GPIO8");
    probe_pin!(gpio9, "GPIO9");
    probe_pin!(gpio10, "GPIO10");

    info!("GPIO sweep complete. Leaving all probe pins LOW.");
    core::future::pending().await
}
