#![allow(missing_docs)]
//! Wi-Fi enabled 4-character LED panel clock (12x4 pixels) with captive-portal setup.
//!
//! This example mirrors the WiFi/clock state machine from `clock_servos.rs` but drives a
//! 12x4 LED panel on GPIO3 instead of servos. The reset button is on GPIO13.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::{
    Error, Result,
    button::PressedTo,
    button_watch,
    clock_sync::{ClockSyncRp, ClockSyncStaticRp, ONE_MINUTE},
    flash_block::FlashBlockRp,
    led_strip::{Current, Gamma, colors},
    led2d,
    led2d::{Frame2d, Led2d as _, Led2dFont, layout::LedLayout},
    wifi_auto::{
        WifiAutoEvent, WifiAutoRp,
        fields::{TimezoneField, TimezoneFieldStatic},
    },
};
use embassy_executor::Spawner;
use embassy_time::Duration;
use heapless::String;
use panic_probe as _;
use smart_leds::RGB8;

// Single 12x4 panel wired serpentine column-major.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();

led2d! {
    Led12x4 {
        pio: PIO0,
        pin: PIN_3,
        dma: DMA_CH1,
        led_layout: LED_LAYOUT_12X4,
        max_current: Current::Milliamps(1000),
        gamma: Gamma::Linear,
        max_frames: 32,
        font: Led2dFont::Font3x4Trim,
    }
}

const CONNECTING_COLOR: RGB8 = colors::WHITE;
const DIGIT_COLORS: [RGB8; 4] = [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW];
const EDIT_COLORS: [RGB8; 4] = [colors::MAGENTA, colors::ORANGE, colors::CYAN, colors::WHITE];

button_watch! {
    ButtonWatch13 {
        pin: PIN_13,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Wi-Fi 12x4 LED clock (WifiAutoRp)");
    let p = embassy_rp::init(Default::default());

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, mut timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define HTML to ask for timezone on the captive portal.
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up Wifi via a captive portal. The button pin is used to reset stored credentials.
    let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 data
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 clock
        p.PIO1,    // CYW43 PIO interface (swapped to show PIO not hardcoded)
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        "DeviceEnvoyClock", // Captive-portal SSID
        [timezone_field],   // Custom fields to ask for
        spawner,
    )?;

    // Set up the 12x4 LED display on GPIO3.
    let led12x4 = Led12x4::new(p.PIN_3, p.PIO0, p.DMA_CH1, spawner)?;

    // Connect Wi-Fi, using the LED panel for status.
    let led12x4_ref = &led12x4;
    let stack = wifi_auto
        .connect(
            &mut *button_watch13,
            async |event| -> Result<(), device_envoy_rp::Error> {
                match event {
                    WifiAutoEvent::CaptivePortalReady => {
                        info!("WiFi: captive portal ready, displaying JOIN");
                        show_portal_ready(led12x4_ref).await?;
                    }
                    WifiAutoEvent::Connecting {
                        try_index,
                        try_count,
                    } => {
                        info!("WiFi: connecting (attempt {}/{})", try_index + 1, try_count);
                        show_connecting(led12x4_ref, try_index, try_count).await?;
                    }
                    WifiAutoEvent::ConnectionFailed => {
                        info!("WiFi: connection failed, displaying FAIL, device will reset");
                        show_connection_failed(led12x4_ref).await;
                    }
                }
                Ok(())
            },
        )
        .await?;

    info!("WiFi: connected successfully, displaying DONE");
    show_connected(&led12x4).await;

    // Read the timezone offset, an extra field that WiFi portal saved to flash.
    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    // Create a ClockSync device that knows its timezone offset.
    static CLOCK_SYNC_STATIC: ClockSyncStaticRp = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    )?;

    let led12x4_ref = &led12x4;
    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        &mut timezone_flash_block,
        |clock_ui_event| async move {
            match clock_ui_event {
                ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                    show_hours_minutes(led12x4_ref, hours, minutes).await;
                }
                ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                    show_minutes_seconds(led12x4_ref, minutes, seconds).await;
                }
                ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                    show_hours_minutes_indicator(led12x4_ref, hours, minutes).await;
                }
            }
            Ok(())
        },
    )
    .await
}

// Display helper functions for the 12x4 LED clock

async fn show_portal_ready(led12x4: &Led12x4) -> Result<()> {
    let on_frame = text_frame(led12x4, "JOIN", &DIGIT_COLORS)?;
    led12x4.animate([
        (on_frame, Duration::from_millis(700)),
        (Frame2d::new(), Duration::from_millis(300)),
    ]);
    Ok(())
}

async fn show_connecting(led12x4: &Led12x4, try_index: u8, _try_count: u8) -> Result<()> {
    let clockwise = try_index % 2 == 0;
    const FRAME_DURATION: Duration = Duration::from_millis(90);
    let animation = perimeter_chase_animation(clockwise, CONNECTING_COLOR, FRAME_DURATION)?;
    led12x4.animate(animation);
    Ok(())
}

async fn show_connected(led12x4: &Led12x4) {
    led12x4.write_text("DONE", &DIGIT_COLORS);
}

async fn show_connection_failed(led12x4: &Led12x4) {
    led12x4.write_text("FAIL", &DIGIT_COLORS);
}

async fn show_hours_minutes(led12x4: &Led12x4, hours: u8, minutes: u8) {
    let (hours_tens, hours_ones) = hours_digits(hours);
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let text = chars_to_text([hours_tens, hours_ones, minutes_tens, minutes_ones]);
    led12x4.write_text(text.as_str(), &DIGIT_COLORS);
}

async fn show_hours_minutes_indicator(led12x4: &Led12x4, hours: u8, minutes: u8) {
    let (hours_tens, hours_ones) = hours_digits(hours);
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let text = chars_to_text([hours_tens, hours_ones, minutes_tens, minutes_ones]);
    led12x4.write_text(text.as_str(), &EDIT_COLORS);
}

async fn show_minutes_seconds(led12x4: &Led12x4, minutes: u8, seconds: u8) {
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let (seconds_tens, seconds_ones) = two_digit_chars(seconds);
    let text = chars_to_text([minutes_tens, minutes_ones, seconds_tens, seconds_ones]);
    led12x4.write_text(text.as_str(), &DIGIT_COLORS);
}

const PERIMETER_LENGTH: usize = (Led12x4::WIDTH * 2) + ((Led12x4::HEIGHT - 2) * 2);

fn chars_to_text(chars: [char; 4]) -> String<4> {
    let mut text = String::new();
    for ch in chars {
        text.push(ch).expect("text buffer has capacity");
    }
    text
}

fn text_frame(led12x4: &Led12x4, text: &str, colors: &[RGB8]) -> Result<Frame2d<12, 4>> {
    let mut frame = Frame2d::new();
    led12x4.write_text_to_frame(text, colors, &mut frame);
    Ok(frame)
}

fn perimeter_chase_animation(
    clockwise: bool,
    color: RGB8,
    duration: Duration,
) -> Result<heapless::Vec<(Frame2d<12, 4>, Duration), PERIMETER_LENGTH>> {
    assert!(
        duration.as_micros() > 0,
        "perimeter animation duration must be positive"
    );
    let coordinates = perimeter_coordinates(clockwise);
    let mut frames = heapless::Vec::new();
    for frame_index in 0..PERIMETER_LENGTH {
        let mut frame = Frame2d::new();
        for tail_offset in 0..3 {
            let coordinate_index = (frame_index + tail_offset) % PERIMETER_LENGTH;
            let (x_index, y_index) = coordinates[coordinate_index];
            frame[(x_index, y_index)] = color;
        }
        frames
            .push((frame, duration))
            .map_err(|_| Error::FormatError)?;
    }
    Ok(frames)
}

fn perimeter_coordinates(clockwise: bool) -> [(usize, usize); PERIMETER_LENGTH] {
    let mut coordinates = [(0_usize, 0_usize); PERIMETER_LENGTH];
    let mut write_index = 0;
    let mut push = |x_index: usize, y_index: usize| {
        coordinates[write_index] = (x_index, y_index);
        write_index += 1;
    };

    for x_index in 0..Led12x4::WIDTH {
        push(x_index, 0);
    }
    for y_index in 1..Led12x4::HEIGHT {
        push(Led12x4::WIDTH - 1, y_index);
    }
    for x_index in (0..(Led12x4::WIDTH - 1)).rev() {
        push(x_index, Led12x4::HEIGHT - 1);
    }
    for y_index in (1..(Led12x4::HEIGHT - 1)).rev() {
        push(0, y_index);
    }

    debug_assert_eq!(write_index, PERIMETER_LENGTH);

    if clockwise {
        coordinates
    } else {
        let mut reversed = [(0_usize, 0_usize); PERIMETER_LENGTH];
        for (reverse_index, &(x_index, y_index)) in coordinates.iter().enumerate() {
            reversed[PERIMETER_LENGTH - 1 - reverse_index] = (x_index, y_index);
        }
        reversed
    }
}

#[inline]
fn two_digit_chars(value: u8) -> (char, char) {
    assert!(value < 100);
    (tens_digit(value), ones_digit(value))
}

#[inline]
fn hours_digits(hours: u8) -> (char, char) {
    assert!(hours >= 1 && hours <= 12);
    if hours >= 10 {
        ('1', ones_digit(hours))
    } else {
        (' ', ones_digit(hours))
    }
}

#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    reason = "Value < 100 ensures division is safe"
)]
fn tens_digit(value: u8) -> char {
    ((value / 10) + b'0') as char
}

#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    reason = "Value < 100 ensures division is safe"
)]
fn ones_digit(value: u8) -> char {
    ((value % 10) + b'0') as char
}
