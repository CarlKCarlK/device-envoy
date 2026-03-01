//! Basic NEC IR receiver demo.
//!
//! Wiring:
//! - IR receiver data output -> GPIO7
//! - IR receiver VCC -> 3.3V
//! - IR receiver GND -> GND

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp32::{
    init_and_start,
    ir::{Ir, IrEvent, IrStatic},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp32::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("ir example started: listening on GPIO7");

    static IR_STATIC: IrStatic = Ir::new_static();
    let ir = Ir::new(&IR_STATIC, p.GPIO7, rmt80.channel2, spawner)?;

    loop {
        let IrEvent::Press { addr, cmd } = ir.wait_for_press().await;
        info!("IR press: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
    }
}
