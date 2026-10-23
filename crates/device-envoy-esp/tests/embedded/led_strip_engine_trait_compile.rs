//! Embedded compile-only test target for `led_strip!` engine selection by type: the
//! chip default, an explicit `Rmt`, and an explicit `Spi` with a custom reset interval.

#![no_std]
#![no_main]
#![deny(warnings, missing_docs)]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{Current, Frame1d, LedStrip as _, Rmt, Spi, colors},
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    /// A strip on the chip's default engine.
    pub DefaultStrip { pin: GPIO0, len: 8, max_current: Current::Milliamps(100) }
}

led_strip! {
    /// A strip on RMT.
    pub RmtStrip { pin: GPIO1, len: 4, engine: Rmt }
}

led_strip! {
    /// A strip on SPI with a longer reset gap.
    pub SpiStrip { pin: GPIO2, len: 16, engine: Spi, reset_us: 80 }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);

    let default_strip = DefaultStrip::new(p.GPIO0, rmt80.channel0, spawner)?;
    let rmt_strip = RmtStrip::new(p.GPIO1, rmt80.channel1, spawner)?;
    let spi_strip = SpiStrip::new(p.GPIO2, p.SPI2, spawner)?;

    default_strip.write_frame(Frame1d::filled(colors::RED));
    rmt_strip.write_frame(Frame1d::filled(colors::GREEN));
    spi_strip.write_frame(Frame1d::filled(colors::BLUE));

    core::future::pending().await
}
