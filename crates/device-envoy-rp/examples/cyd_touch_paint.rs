#![allow(missing_docs)]
//! Calibrates a standalone 320x240 ILI9341 + XPT2046 touch module once
//! (persisting the result to flash) and then paints a small filled circle at
//! every touch point — a minimal interactive demo of `device_envoy_rp::cyd`'s
//! calibration flow and calibrated touch event stream.
//!
//! Wiring:
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
//! - Recalibration button -> PIN_15 to GND (`PressedTo::Ground`)
//! - Plus 3.3V and GND

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_core::cyd::touch::calibration::ensure_calibration;
use device_envoy_rp::{
    Result,
    button::{ButtonRp, PressedTo},
    cyd::{
        Cyd as _, CydDisplay as _, CydRp, CydStaticRp, CydTouch as _, DEFAULT_FONT, Orientation,
    },
    flash_block::FlashBlockRp,
};
use embassy_executor::Spawner;
use embedded_graphics::{
    Drawable,
    pixelcolor::{Rgb565, Rgb888},
    prelude::Point,
    primitives::{Circle, Primitive, PrimitiveStyle},
};
use panic_probe as _;

const TOUCH_DOT_RADIUS: u32 = 5;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("Starting CYD touch-paint demo");

    let [mut calibration_flash_block] = FlashBlockRp::new_array::<1>(p.FLASH)?;
    let mut recalibration_button = ButtonRp::new(p.PIN_15, PressedTo::Ground);

    static CYD_STATIC: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
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
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),
        Rgb888::new(230, 230, 230),
        &DEFAULT_FONT,
        p.SPI1,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.PIN_13,
        p.PIN_14,
    )?;
    info!("CYD display and touch initialized");

    let calibration_outcome = ensure_calibration(
        &mut cyd,
        &mut calibration_flash_block,
        &mut recalibration_button,
        Some("recalibrating"),
    )
    .await?;
    cyd.set_calibration(calibration_outcome.calibration_config());
    if calibration_outcome.was_saved() {
        info!("Calibration saved, restarting");
        cortex_m::peripheral::SCB::sys_reset();
    }
    info!("Touch calibrated; tap the panel to paint");

    let (mut display, mut touch) = cyd.parts();
    let mut frame = display.full_frame_mut();
    frame.flush()?;

    loop {
        if let Some(touch_event) = touch.read()? {
            let point = match touch_event {
                device_envoy_rp::cyd::touch::TouchEvent::Down { point }
                | device_envoy_rp::cyd::touch::TouchEvent::Move { point } => Some(point),
                device_envoy_rp::cyd::touch::TouchEvent::Up => None,
            };
            if let Some(point) = point {
                Circle::with_center(Point::new(point.x, point.y), TOUCH_DOT_RADIUS * 2)
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::new(28, 50, 12)))
                    .draw(&mut frame)
                    .expect("drawing onto an Infallible CYD frame cannot fail");
                frame.flush()?;
            }
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(16)).await;
    }
}
