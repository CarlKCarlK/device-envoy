#![allow(missing_docs)]
//! Direct RP CYD buffered frame/flush test.
//!
//! This isolates the `full_frame_mut() -> fill() -> flush()` path from touch,
//! calibration, and higher-level app logic.

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

    info!("Starting CYD frame flush test");

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
        show_step(&mut display, Rgb565::MAGENTA, "MAGENTA").await?;
        show_step(&mut display, Rgb565::YELLOW, "YELLOW").await?;
        show_step(&mut display, Rgb565::CYAN, "CYAN").await?;
        show_step(&mut display, Rgb565::WHITE, "WHITE").await?;
        show_step(&mut display, Rgb565::BLACK, "BLACK").await?;
    }
}

async fn show_step(
    display: &mut CydDisplayRp,
    color: Rgb565,
    label: &str,
) -> Result<(), device_envoy_rp::cyd::CydError> {
    info!("Frame flush step: {}", label);
    let mut frame = display.full_frame_mut();
    frame.fill(color).flush()?;
    Timer::after_secs(2).await;
    Ok(())
}
