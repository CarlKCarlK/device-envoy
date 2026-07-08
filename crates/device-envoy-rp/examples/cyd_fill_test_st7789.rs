#![allow(missing_docs)]
//! Direct RP CYD display test using the ST7789 model.
//!
//! This is a panel-identification probe. If the standard ILI9341-based test
//! stays black but this one shows solid colors, the RP wiring is fine and the
//! remaining issue is the panel init/model selection.

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::Result;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config as SpiConfig, Phase, Polarity, Spi};
use embassy_time::Timer;
use embedded_graphics::{
    draw_target::DrawTarget, pixelcolor::Rgb565, prelude::RgbColor, primitives::Rectangle,
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ST7789,
    options::{ColorOrder, Orientation as MipiOrientation, Rotation},
};
use panic_probe as _;
use static_cell::StaticCell;

const DEFAULT_DISPLAY_SPI_HZ: u32 = 2_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("Starting CYD ST7789 fill test");

    let spi_config = {
        let mut spi_config = SpiConfig::default();
        spi_config.frequency = DEFAULT_DISPLAY_SPI_HZ;
        spi_config.polarity = Polarity::IdleLow;
        spi_config.phase = Phase::CaptureOnFirstTransition;
        spi_config
    };
    let spi = Spi::new_blocking_txonly(p.SPI0, p.PIN_18, p.PIN_19, spi_config);

    let cs = Output::new(p.PIN_17, Level::High);
    let dc = Output::new(p.PIN_20, Level::Low);
    let rst = Output::new(p.PIN_21, Level::High);
    let mut backlight = Output::new(p.PIN_22, Level::High);

    let spi_device =
        ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs).expect("CS pin is infallible");

    static SPI_BUFFER: StaticCell<[u8; DISPLAY_SPI_BUFFER_LEN]> = StaticCell::new();
    let spi_buffer = SPI_BUFFER.init([0u8; DISPLAY_SPI_BUFFER_LEN]);
    let interface = SpiInterface::new(spi_device, dc, spi_buffer);
    let mut delay = embassy_time::Delay;

    let display_orientation = MipiOrientation::new()
        .rotate(Rotation::Deg90)
        .flip_horizontal()
        .rotate(Rotation::Deg180);

    let mut display = Builder::new(ST7789, interface)
        .reset_pin(rst)
        .display_size(240, 320)
        .color_order(ColorOrder::Bgr)
        .orientation(display_orientation)
        .init(&mut delay)
        .map_err(|_| {
            device_envoy_rp::Error::CydDisplayInit(
                device_envoy_rp::cyd::CydDisplayRpInitError::InitDisplay,
            )
        })?;

    backlight.set_high();

    info!("CYD ST7789 display initialized");

    let full_screen = Rectangle::new(
        embedded_graphics::prelude::Point::new(0, 0),
        embedded_graphics::prelude::Size::new(320, 240),
    );

    loop {
        info!("Filling RED");
        display.fill_solid(&full_screen, Rgb565::RED).map_err(|_| {
            device_envoy_rp::Error::CydDisplayFlush(
                device_envoy_rp::cyd::CydDisplayRpFlushError::FlushFrameBuffer,
            )
        })?;
        Timer::after_secs(2).await;

        info!("Filling GREEN");
        display
            .fill_solid(&full_screen, Rgb565::GREEN)
            .map_err(|_| {
                device_envoy_rp::Error::CydDisplayFlush(
                    device_envoy_rp::cyd::CydDisplayRpFlushError::FlushFrameBuffer,
                )
            })?;
        Timer::after_secs(2).await;

        info!("Filling BLUE");
        display
            .fill_solid(&full_screen, Rgb565::BLUE)
            .map_err(|_| {
                device_envoy_rp::Error::CydDisplayFlush(
                    device_envoy_rp::cyd::CydDisplayRpFlushError::FlushFrameBuffer,
                )
            })?;
        Timer::after_secs(2).await;

        info!("Filling WHITE");
        display
            .fill_solid(&full_screen, Rgb565::WHITE)
            .map_err(|_| {
                device_envoy_rp::Error::CydDisplayFlush(
                    device_envoy_rp::cyd::CydDisplayRpFlushError::FlushFrameBuffer,
                )
            })?;
        Timer::after_secs(2).await;

        info!("Filling BLACK");
        display
            .fill_solid(&full_screen, Rgb565::BLACK)
            .map_err(|_| {
                device_envoy_rp::Error::CydDisplayFlush(
                    device_envoy_rp::cyd::CydDisplayRpFlushError::FlushFrameBuffer,
                )
            })?;
        Timer::after_secs(2).await;
    }
}
