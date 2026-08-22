#![allow(missing_docs)]
//! Touch-triggered Wi-Fi/DNS reliability tester for a CYD-style touchscreen
//! wired to a Pico W / Pico 2 W.
//!
//! Tap anywhere on the panel to fire one DNS query and update a running tally
//! of hits/misses and the last round-trip latency on a single status line.
//! Meant to run for hours unattended on hardware with no reachable physical
//! reset button, repeatedly exercising the touch driver and the Wi-Fi stack
//! together so long-run failures (socket exhaustion, dropped connections,
//! stuck touch reads) show up on screen instead of silently wedging the
//! board. The status line stays deliberately small rather than using a
//! full-screen frame buffer, mirroring the ESP32 port of this example, where
//! a full-screen buffer plus the Wi-Fi stack's own heap overflowed DRAM.
//!
//! Wiring: a standalone 320x240 ILI9341 + XPT2046 module wired to spare
//! SPI-capable GPIOs (see `device_envoy_rp::cyd` module docs for the CYD
//! abstraction). The CYW43 Wi-Fi pins are fixed by the Pico W / Pico 2 W
//! module itself.
//!
//! - Display SPI0 SCK  -> PIN_18
//! - Display SPI0 MOSI -> PIN_19
//! - Display SPI0 MISO -> PIN_16
//! - Display CS        -> PIN_17
//! - Display DC        -> PIN_20
//! - Display RST       -> PIN_21
//! - Display backlight -> PIN_22
//! - Touch SPI1 SCK    -> PIN_10
//! - Touch SPI1 MOSI   -> PIN_11
//! - Touch SPI1 MISO   -> PIN_12
//! - Touch CS          -> PIN_13
//! - Touch IRQ         -> PIN_14
//! - Button (Wi-Fi reset while connecting; calibration backup afterward) -> PIN_15 to GND
//!   (`PressedTo::Ground`)
//! - Plus 3.3V and GND

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use core::convert::Infallible;

use defmt::info;
use device_envoy_core::cyd::display::Orientation;
use device_envoy_core::dns::DnsWithStack;
use device_envoy_core::flash_block::FlashBlock as _;
use device_envoy_core::wifi_auto::WifiAuto as _;
use device_envoy_examples_core::dns_tester;
use device_envoy_rp::{
    Error as DeviceEnvoyError, Result,
    button::{ButtonRp, PressedTo},
    cyd::{CydRp, CydStaticRp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT},
    flash_block::FlashBlockRp,
    wifi_auto::WifiAutoRp,
};
use embassy_executor::Spawner;
use embedded_graphics::pixelcolor::Rgb888;

const CAPTIVE_PORTAL_SSID: &str = "DeviceEnvoySetup";
const STATUS_PIXEL_COUNT: usize = device_envoy_examples_core::dns_tester::FRAME_PIXEL_COUNT;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(never) => match never {},
        Err(Error::Platform(error)) => panic!("{error:?}"),
        Err(Error::Core(error)) => panic!("{error:?}"),
        Err(Error::DnsTester(error)) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible, Error> {
    let p = embassy_rp::init(Default::default());
    info!("Starting CYD DNS tester");

    let [
        wifi_flash_block,
        mut calibration_flash_block,
        mut orientation_flash_block,
    ] = FlashBlockRp::new_array::<3>(p.FLASH)?;
    let orientation = orientation_flash_block
        .load::<Orientation>()?
        .unwrap_or(Orientation::Landscape);
    let mut button = ButtonRp::new(p.PIN_15, PressedTo::Ground);

    static CYD_STATIC: CydStaticRp<STATUS_PIXEL_COUNT> = CydRp::new_static();
    let mut cyd = CydRp::new(
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
        orientation,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &DEFAULT_FONT,
        p.SPI1,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.PIN_13,
        p.PIN_14,
        &mut calibration_flash_block,
        &mut button,
    )
    .await?;
    info!("CYD display and touch initialized");
    info!("Touch calibrated");
    dns_tester::splash(&mut cyd).await?;

    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 data
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 clock
        p.PIO0,    // WiFi PIO
        p.DMA_CH0, // WiFi DMA
        wifi_flash_block,
        CAPTIVE_PORTAL_SSID,
        [],
        spawner,
    )?;
    let stack = wifi_auto
        .connect(&mut button, async |wifi_auto_event| -> Result<(), Error> {
            dns_tester::wifi_status(&mut cyd, wifi_auto_event).await?;
            Ok(())
        })
        .await?;

    info!("Wi-Fi up with DHCP");

    let mut dns = DnsWithStack::new(*stack);
    match dns_tester::run(&mut cyd, &mut button, &mut dns).await? {
        dns_tester::Exit::Calibrate => calibration_flash_block.clear()?,
        dns_tester::Exit::ResetWifi => wifi_auto.reset_to_captive_portal()?,
        dns_tester::Exit::Reorientate(next_orientation) => {
            orientation_flash_block.save(&next_orientation)?;
        }
    }
    cortex_m::peripheral::SCB::sys_reset();
}

#[derive(Debug, derive_more::From)]
enum Error {
    Platform(DeviceEnvoyError),
    Core(dns_tester::Error<device_envoy_rp::cyd::Error, Infallible>),
    DnsTester(dns_tester::Error<device_envoy_rp::cyd::Error, embassy_net::dns::Error>),
}
