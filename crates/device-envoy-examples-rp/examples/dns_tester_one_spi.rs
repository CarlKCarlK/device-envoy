#![allow(missing_docs)]
//! Touch-triggered Wi-Fi/DNS reliability tester using one shared CYD SPI bus.
//!
//! The display and touch controller share SPI0. The CYW43 Wi-Fi connection uses
//! the Pico W / Pico 2 W radio pins separately.

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
    Error, Result,
    button::{ButtonRp, PressedTo},
    cyd::{CydRpOneSpi, CydRpOneSpiStatic, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT},
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
        Err(MainError::Platform(error)) => panic!("{error:?}"),
        Err(MainError::Core(error)) => panic!("{error:?}"),
        Err(MainError::DnsTester(error)) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible, MainError> {
    let p = embassy_rp::init(Default::default());
    info!("Starting one-SPI CYD DNS tester");

    let [
        wifi_flash_block,
        mut calibration_flash_block,
        mut orientation_flash_block,
    ] = FlashBlockRp::new_array::<3>(p.FLASH)?;
    let orientation = orientation_flash_block
        .load::<Orientation>()?
        .unwrap_or(Orientation::Landscape);
    let mut button = ButtonRp::new(p.PIN_15, PressedTo::Ground);

    static CYD_STATIC: CydRpOneSpiStatic<embassy_rp::peripherals::SPI0, STATUS_PIXEL_COUNT> =
        CydRpOneSpi::new_static();
    let mut cyd = CydRpOneSpi::new(
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
        p.PIN_13,
        p.PIN_14,
        orientation,
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &DEFAULT_FONT,
        &mut calibration_flash_block,
        &mut button,
    )
    .await?;
    info!("CYD display and touch initialized and calibrated");
    dns_tester::splash(&mut cyd).await?;

    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH0,
        wifi_flash_block,
        CAPTIVE_PORTAL_SSID,
        [],
        spawner,
    )?;
    let stack = wifi_auto
        .connect(
            &mut button,
            async |wifi_auto_event| -> Result<(), MainError> {
                dns_tester::wifi_status(&mut cyd, wifi_auto_event).await?;
                Ok(())
            },
        )
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
enum MainError {
    Platform(Error),
    Core(dns_tester::Error<device_envoy_rp::cyd::CydError, Infallible>),
    DnsTester(dns_tester::Error<device_envoy_rp::cyd::CydError, embassy_net::dns::Error>),
}
