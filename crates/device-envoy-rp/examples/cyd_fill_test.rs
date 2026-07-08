#![allow(missing_docs)]
//! Direct RP CYD display test.
//!
//! This bypasses touch, calibration, and higher-level app logic and only tests
//! low-level LCD initialization plus immediate whole-screen fills.

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_core::cyd::CydDisplay as _;
use device_envoy_rp::{
    Result,
    cyd::{CydDisplayRp, CydRp, CydStaticRp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation},
};
use embassy_executor::Spawner;
use embassy_time::Timer;
use embedded_graphics::{
    pixelcolor::{Rgb565, Rgb888},
    prelude::RgbColor,
};
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("Starting CYD fill test");

    static CYD_STATIC: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
    let mut display = CydDisplayRp::new(
        &CYD_STATIC,
        p.SPI0,
        p.PIN_18,
        p.PIN_19,
        p.PIN_16,
        p.PIN_17,
        p.PIN_20,
        p.PIN_21,
        p.PIN_22,
        DEFAULT_DISPLAY_SPI_HZ,
        Orientation::Landscape,
        Rgb888::new(0, 0, 0),
        Rgb888::new(255, 255, 255),
        &DEFAULT_FONT,
    )?;

    info!("CYD display initialized");

    loop {
        info!("Filling RED");
        display.fill(Rgb565::RED)?;
        Timer::after_secs(2).await;

        info!("Filling GREEN");
        display.fill(Rgb565::GREEN)?;
        Timer::after_secs(2).await;

        info!("Filling BLUE");
        display.fill(Rgb565::BLUE)?;
        Timer::after_secs(2).await;

        info!("Filling WHITE");
        display.fill(Rgb565::WHITE)?;
        Timer::after_secs(2).await;

        info!("Filling BLACK");
        display.fill(Rgb565::BLACK)?;
        Timer::after_secs(2).await;
    }
}
