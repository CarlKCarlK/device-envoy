#![allow(missing_docs)]
//! RP CYD probe for `fill_contiguous`.
//!
//! Direct `fill_solid` is known to work on the tested panel. This example
//! alternates between a solid full-screen fill and a small contiguous patch so
//! the `mipidsi::Display::fill_contiguous` path can be tested in isolation.

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
    prelude::{Point, RgbColor, Size},
    primitives::Rectangle,
};
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("Starting CYD contiguous probe");

    static CYD_STATIC: CydStaticRp<0> = CydRp::new_static();
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

    let patch = Rectangle::new(Point::new(40, 40), Size::new(80, 80));

    loop {
        info!("Solid white background");
        display.fill(Rgb565::WHITE)?;
        Timer::after_secs(2).await;

        info!("Contiguous magenta patch");
        display.fill_contiguous(patch, core::iter::repeat_n(Rgb565::MAGENTA, 80 * 80))?;
        Timer::after_secs(2).await;

        info!("Solid black background");
        display.fill(Rgb565::BLACK)?;
        Timer::after_secs(2).await;

        info!("Contiguous yellow patch");
        display.fill_contiguous(patch, core::iter::repeat_n(Rgb565::YELLOW, 80 * 80))?;
        Timer::after_secs(2).await;
    }
}
