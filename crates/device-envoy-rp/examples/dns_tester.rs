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

use core::{convert::Infallible, fmt::Write};

use defmt::{info, warn};
use device_envoy_core::cyd::display::{CydFrame, Orientation};
use device_envoy_core::cyd::touch::calibration::CalibrationConfig;
use device_envoy_core::cyd::touch::calibration::{CALIBRATION_MIN_PIXEL_COUNT, ensure_calibration};
use device_envoy_core::flash_block::FlashBlock as _;
use device_envoy_rp::{
    Error, Result,
    button::{Button as _, ButtonRp, PressedTo},
    cyd::{
        CydDisplay as _, CydRp, CydRpUncalibrated, CydStaticRp, CydTouch as _,
        DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT,
    },
    flash_block::FlashBlockRp,
    wifi_auto::{WifiAutoEvent, WifiAutoRp},
};
use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

const DNS_HOSTNAME: &str = "example.com";
const CAPTIVE_PORTAL_SSID: &str = "DeviceEnvoySetup";
const TOUCH_POLL_PERIOD: Duration = Duration::from_millis(16);

/// One text line across the top of the panel. Kept small on purpose: this
/// example runs alongside the Wi-Fi stack, and a full-screen frame buffer
/// risks overflowing RAM once Wi-Fi's own heap is added in (this bit ESP32
/// in practice; no RP board has been verified against a full-screen buffer
/// plus Wi-Fi, so the same small-buffer discipline is applied here too).
const STATUS_LINE_HEIGHT: u32 = 20;
const CONTROL_HEIGHT: u32 = 20;

/// `ensure_calibration`'s own on-screen text banner needs a buffer at least
/// `CALIBRATION_MIN_PIXEL_COUNT` pixels; its crosshair/dot geometry streams
/// buffer-free. The DNS status line uses the same small-buffer budget.
const STATUS_PIXEL_COUNT: usize = CALIBRATION_MIN_PIXEL_COUNT;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
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
    let calibration_is_available = match calibration_flash_block.load::<CalibrationConfig>() {
        Ok(Some(_)) => true,
        Ok(None) | Err(_) => false,
    };
    let display_orientation = if calibration_is_available {
        orientation
    } else {
        // The shared calibration UI is always landscape. Apply a saved user
        // orientation only after calibration has already been completed.
        Orientation::Landscape
    };
    let mut button = ButtonRp::new(p.PIN_15, PressedTo::Ground);

    static CYD_STATIC: CydStaticRp<STATUS_PIXEL_COUNT> = CydRp::new_static();
    let CydRpUncalibrated { mut display, touch } = CydRpUncalibrated::new(
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
        display_orientation,
        embedded_graphics::pixelcolor::Rgb888::new(10, 10, 12), // near-black
        embedded_graphics::pixelcolor::Rgb888::new(230, 230, 230), // near-white
        &DEFAULT_FONT,
        p.SPI1,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.PIN_13,
        p.PIN_14,
    )?;
    info!("CYD display and touch initialized");

    let (mut touch, calibration_outcome) = ensure_calibration(
        &mut display,
        touch,
        &mut calibration_flash_block,
        &mut button,
        Some("recalibrating"),
    )
    .await?;
    if calibration_outcome.was_saved() {
        while button.is_pressed() {
            Timer::after(Duration::from_millis(10)).await;
        }
        info!("Calibration saved, restarting");
        cortex_m::peripheral::SCB::sys_reset();
    }
    info!("Touch calibrated");

    let rectangle = status_rectangle(&display);
    display
        .frame_mut(rectangle)
        .clear()
        .write_text("Starting DNS tester...")
        .flush()?;

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
            let message = match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => "WiFi setup: join DeviceEnvoySetup",
                WifiAutoEvent::Connecting { .. } => "Connecting to WiFi...",
                WifiAutoEvent::ConnectionFailed => "WiFi connect failed, retrying",
            };
            info!("Wi-Fi: {}", message);
            let rectangle = status_rectangle(&display);
            display
                .frame_mut(rectangle)
                .clear()
                .write_text(message)
                .flush()?;
            Ok(())
        })
        .await?;

    while !stack.is_link_up() || stack.config_v4().is_none() {
        Timer::after(Duration::from_millis(200)).await;
    }
    info!("Wi-Fi up with DHCP");

    let mut tap_count: u32 = 0;
    let mut success_count: u32 = 0;
    let mut failure_count: u32 = 0;
    let mut last_latency_millis: u64 = 0;
    let mut status_text: heapless::String<64> = heapless::String::new();
    let mut is_touch_down = false;

    draw_status(
        &mut display,
        &mut status_text,
        tap_count,
        success_count,
        failure_count,
        last_latency_millis,
    )?;
    draw_controls(&mut display)?;

    loop {
        if button.is_pressed() {
            while button.is_pressed() {
                Timer::after(Duration::from_millis(10)).await;
            }
            calibration_flash_block.clear()?;
            let rectangle = status_rectangle(&display);
            display
                .frame_mut(rectangle)
                .clear()
                .write_text("Recalibrating...")
                .flush()?;
            cortex_m::peripheral::SCB::sys_reset();
        }
        if let Some(touch_event) = touch.read()? {
            match touch_event {
                device_envoy_rp::cyd::touch::TouchEvent::Down { point } => {
                    if !is_touch_down {
                        is_touch_down = true;
                        let point = orientation.map_landscape_point(point);
                        if let Some(control) = control_at(point, display.screen_size()) {
                            while !matches!(
                                touch.read()?,
                                Some(device_envoy_rp::cyd::touch::TouchEvent::Up)
                            ) {
                                Timer::after(TOUCH_POLL_PERIOD).await;
                            }
                            match control {
                                Control::Calibration => {
                                    calibration_flash_block.clear()?;
                                    let rectangle = status_rectangle(&display);
                                    display
                                        .frame_mut(rectangle)
                                        .clear()
                                        .write_text("Recalibrating...")
                                        .flush()?;
                                    cortex_m::peripheral::SCB::sys_reset();
                                }
                                Control::Wifi => {
                                    wifi_auto.reset_to_captive_portal()?;
                                    let rectangle = status_rectangle(&display);
                                    display
                                        .frame_mut(rectangle)
                                        .clear()
                                        .write_text("Resetting WiFi...")
                                        .flush()?;
                                    cortex_m::peripheral::SCB::sys_reset();
                                }
                                Control::Orientation => {
                                    orientation_flash_block.save(&orientation.next())?;
                                    let rectangle = status_rectangle(&display);
                                    display
                                        .frame_mut(rectangle)
                                        .clear()
                                        .write_text("Changing orientation...")
                                        .flush()?;
                                    cortex_m::peripheral::SCB::sys_reset();
                                }
                            }
                        }
                        tap_count += 1;

                        let query_start = Instant::now();
                        let dns_result = stack.dns_query(DNS_HOSTNAME, DnsQueryType::A).await;
                        last_latency_millis = query_start.elapsed().as_millis();

                        match dns_result {
                            Ok(addresses) if !addresses.is_empty() => {
                                success_count += 1;
                                info!(
                                    "DNS ok in {}ms: {:?}",
                                    last_latency_millis,
                                    addresses.first()
                                );
                            }
                            Ok(_) => {
                                failure_count += 1;
                                warn!("DNS query returned no addresses");
                            }
                            Err(_) => {
                                failure_count += 1;
                                warn!("DNS query failed");
                            }
                        }

                        draw_status(
                            &mut display,
                            &mut status_text,
                            tap_count,
                            success_count,
                            failure_count,
                            last_latency_millis,
                        )?;
                        draw_controls(&mut display)?;
                    }
                }
                device_envoy_rp::cyd::touch::TouchEvent::Move { .. } => {}
                device_envoy_rp::cyd::touch::TouchEvent::Up => {
                    is_touch_down = false;
                }
            }
        }
        Timer::after(TOUCH_POLL_PERIOD).await;
    }
}

fn draw_status(
    display: &mut device_envoy_rp::cyd::CydDisplayRp,
    status_text: &mut heapless::String<64>,
    tap_count: u32,
    success_count: u32,
    failure_count: u32,
    last_latency_millis: u64,
) -> Result<()> {
    for (line_index, line) in [
        "Tap Screen",
        DNS_HOSTNAME,
        "DNS Queries:",
        "DNS Successes:",
        "DNS Failures:",
        "Last latency:",
    ]
    .into_iter()
    .enumerate()
    {
        status_text.clear();
        match line_index {
            0 => status_text
                .push_str(line)
                .expect("status text exceeds fixed buffer"),
            1 => write!(status_text, "DNS: {line}").expect("status text exceeds fixed buffer"),
            2 => write!(status_text, "DNS Queries: {tap_count}")
                .expect("status text exceeds fixed buffer"),
            3 => write!(status_text, "DNS Successes: {success_count}")
                .expect("status text exceeds fixed buffer"),
            4 => write!(status_text, "DNS Failures: {failure_count}")
                .expect("status text exceeds fixed buffer"),
            5 => write!(status_text, "Last latency: {last_latency_millis}ms")
                .expect("status text exceeds fixed buffer"),
            _ => unreachable!(),
        }
        let rectangle = status_line_rectangle(display, line_index as u32);
        display
            .frame_mut(rectangle)
            .clear()
            .write_text(status_text.as_str())
            .flush()?;
    }
    status_text.clear();
    status_text
        .push_str("Tap Settings:")
        .expect("status text exceeds fixed buffer");
    let settings_line_index = display.screen_size().height / STATUS_LINE_HEIGHT - 2;
    let rectangle = status_line_rectangle(display, settings_line_index);
    display
        .frame_mut(rectangle)
        .clear()
        .write_text(status_text.as_str())
        .flush()?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Control {
    Orientation,
    Calibration,
    Wifi,
}

fn status_rectangle(display: &device_envoy_rp::cyd::CydDisplayRp) -> Rectangle {
    status_line_rectangle(display, 0)
}

fn status_line_rectangle(
    display: &device_envoy_rp::cyd::CydDisplayRp,
    line_index: u32,
) -> Rectangle {
    Rectangle::new(
        Point::new(0, (line_index * STATUS_LINE_HEIGHT) as i32),
        Size::new(display.screen_size().width, STATUS_LINE_HEIGHT),
    )
}

fn control_rectangle(display: &device_envoy_rp::cyd::CydDisplayRp, control: Control) -> Rectangle {
    let index = match control {
        Control::Orientation => 0,
        Control::Calibration => 1,
        Control::Wifi => 2,
    };
    Rectangle::new(
        Point::new(
            display.screen_size().width as i32
                - display.screen_size().width as i32 / 3 * (3 - index),
            display.screen_size().height as i32 - CONTROL_HEIGHT as i32,
        ),
        Size::new(display.screen_size().width / 3, CONTROL_HEIGHT),
    )
}

fn control_at(point: Point, screen_size: Size) -> Option<Control> {
    [Control::Orientation, Control::Calibration, Control::Wifi]
        .into_iter()
        .find(|control| {
            let index = match control {
                Control::Orientation => 0,
                Control::Calibration => 1,
                Control::Wifi => 2,
            };
            Rectangle::new(
                Point::new(
                    screen_size.width as i32 - screen_size.width as i32 / 3 * (3 - index),
                    screen_size.height as i32 - CONTROL_HEIGHT as i32,
                ),
                Size::new(screen_size.width / 3, CONTROL_HEIGHT),
            )
            .contains(point)
        })
}

fn draw_controls(display: &mut device_envoy_rp::cyd::CydDisplayRp) -> Result<()> {
    for (control, label) in [
        (Control::Orientation, "ROT"),
        (Control::Calibration, "CAL"),
        (Control::Wifi, "WiFi"),
    ] {
        let rectangle = control_rectangle(display, control);
        display
            .frame_mut(rectangle)
            .clear()
            .write_text(label)
            .flush()?;
    }
    Ok(())
}
