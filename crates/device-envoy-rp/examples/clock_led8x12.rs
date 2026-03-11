#![allow(missing_docs)]
//! Wi-Fi enabled 4-digit LED panel clock (8x12 pixels rotated) with captive-portal setup.
//!
//! This example uses two stacked 12x4 LED panels rotated 90° clockwise to create an 8-wide
//! by 12-tall display. Uses Font4x6Trim for dense 2-line digit display ("12\n34").
//! The panel is on GPIO4.
//!
//! Button on GPIO13:
//! - During WiFi setup: Hold to force captive portal mode
//! - After WiFi connects: Background monitoring for clock mode changes (short/long press)

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::button_watch;
use device_envoy_rp::{
    Error, Result,
    button::PressedTo,
    clock_sync::{ClockSyncRp, ClockSyncStatic, ONE_MINUTE},
    flash_block::FlashBlockRp,
    led_strip::{Current, Gamma, colors},
    led2d,
    led2d::Frame2d,
    led2d::Led2d as _,
    led2d::Led2dFont,
    led2d::layout::LedLayout,
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

// Two 12x4 panels stacked vertically and rotated 90° CW → 8×12 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

led2d! {
    Led8x12 {
        pio: PIO1,
        pin: PIN_4,
        dma: DMA_CH1,
        led_layout: LED_LAYOUT_8X12,
        max_current: Current::Milliamps(250),
        gamma: Gamma::Linear,
        max_frames: 36,
        font: Led2dFont::Font4x6Trim,
    }
}

const CONNECTING_COLOR: RGB8 = colors::SADDLE_BROWN;
const DIGIT_COLORS: [RGB8; 4] = [colors::CYAN, colors::MAGENTA, colors::ORANGE, colors::LIME];
const EDIT_COLORS: [RGB8; 4] = [
    colors::FIREBRICK,
    colors::DARK_ORANGE,
    colors::TEAL,
    colors::MAROON,
];

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
    info!("Starting Wi-Fi 8x12 LED clock (rotated display)");
    let p = embassy_rp::init(Default::default());

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define HTML to ask for timezone on the captive portal.
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up Wifi via a captive portal.
    let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data pin
        p.PIO0,    // CYW43 PIO interface
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        "www.picoclock.net", // Captive-portal SSID
        [timezone_field],    // Custom fields to ask for
        spawner,
    )?;

    // Set up the 8x12 LED display on GPIO4.
    let led8x12 = Led8x12::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    // Connect Wi-Fi, using the LED panel for status.
    let led8x12_ref = &led8x12;
    // TODO00 verify startup ButtonWatch13 behavior still matches reset-button expectations.
    let stack = wifi_auto
        .connect(&mut *button_watch13, |event| {
            let led8x12_ref = led8x12_ref;
            async move {
                match event {
                    WifiAutoEvent::CaptivePortalReady => {
                        info!("WiFi: captive portal ready, displaying JOIN");
                        show_portal_ready(led8x12_ref).await?;
                    }
                    WifiAutoEvent::Connecting {
                        try_index,
                        try_count,
                    } => {
                        info!("WiFi: connecting (attempt {}/{})", try_index + 1, try_count);
                        show_connecting(led8x12_ref, try_index, try_count).await?;
                    }
                    WifiAutoEvent::ConnectionFailed => {
                        info!("WiFi: connection failed, displaying FAIL, device will reset");
                        show_connection_failed(led8x12_ref).await;
                    }
                }
                Ok(())
            }
        })
        .await?;

    info!("WiFi: connected successfully, displaying DONE");
    show_connected(&led8x12).await;

    // Read the timezone offset, an extra field that WiFi portal saved to flash.
    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    // Create a clock synced over WiFi.
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    let led8x12_ref = &led8x12;
    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        |clock_ui_event| async move {
            match clock_ui_event {
                ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                    show_hours_minutes(led8x12_ref, hours, minutes).await;
                }
                ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                    show_minutes_seconds(led8x12_ref, minutes, seconds).await;
                }
                ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                    show_hours_minutes_indicator(led8x12_ref, hours, minutes).await;
                }
                ClockUiEvent::OffsetPersistRequested { offset_minutes } => {
                    timezone_field.set_offset_minutes(offset_minutes)?;
                }
            }
            Ok(())
        },
    )
    .await
}

// Display helper functions for the 8x12 LED clock

async fn show_portal_ready(led8x12: &Led8x12) -> Result<()> {
    let on_frame = text_frame(led8x12, "JO\nIN", &DIGIT_COLORS)?;
    led8x12.animate([
        (on_frame, Duration::from_millis(700)),
        (Frame2d::new(), Duration::from_millis(300)),
    ]);
    Ok(())
}

async fn show_connecting(led8x12: &Led8x12, try_index: u8, _try_count: u8) -> Result<()> {
    // Delay animation start to avoid wifi initialization glitches
    embassy_time::Timer::after(Duration::from_secs(1)).await;

    let clockwise = try_index % 2 == 0;
    const FRAME_DURATION: Duration = Duration::from_millis(90);
    let animation = perimeter_chase_animation(clockwise, CONNECTING_COLOR, FRAME_DURATION)?;
    led8x12.animate(animation);
    Ok(())
}

async fn show_connected(led8x12: &Led8x12) {
    led8x12.write_text("DO\nNE", &DIGIT_COLORS);
}

async fn show_connection_failed(led8x12: &Led8x12) {
    led8x12.write_text("FA\nIL", &DIGIT_COLORS);
}

async fn show_hours_minutes(led8x12: &Led8x12, hours: u8, minutes: u8) {
    let (hours_tens, hours_ones) = hours_digits(hours);
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let text = two_line_text([hours_tens, hours_ones], [minutes_tens, minutes_ones]);
    led8x12.write_text(text.as_str(), &DIGIT_COLORS);
}

async fn show_hours_minutes_indicator(led8x12: &Led8x12, hours: u8, minutes: u8) {
    let (hours_tens, hours_ones) = hours_digits(hours);
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let text = two_line_text([hours_tens, hours_ones], [minutes_tens, minutes_ones]);
    led8x12.write_text(text.as_str(), &EDIT_COLORS);
}

async fn show_minutes_seconds(led8x12: &Led8x12, minutes: u8, seconds: u8) {
    let (minutes_tens, minutes_ones) = two_digit_chars(minutes);
    let (seconds_tens, seconds_ones) = two_digit_chars(seconds);
    let text = two_line_text([minutes_tens, minutes_ones], [seconds_tens, seconds_ones]);
    led8x12.write_text(text.as_str(), &DIGIT_COLORS);
}

const PERIMETER_LENGTH: usize = (Led8x12::WIDTH * 2) + ((Led8x12::HEIGHT - 2) * 2);

fn two_line_text(top_chars: [char; 2], bottom_chars: [char; 2]) -> String<5> {
    let mut text = String::new();
    for ch in top_chars {
        text.push(ch).expect("text buffer has capacity");
    }
    text.push('\n').expect("text buffer has capacity");
    for ch in bottom_chars {
        text.push(ch).expect("text buffer has capacity");
    }
    text
}

fn text_frame(led8x12: &Led8x12, text: &str, colors: &[RGB8]) -> Result<Frame2d<8, 12>> {
    let mut frame = Frame2d::new();
    led8x12.write_text_to_frame(text, colors, &mut frame);
    Ok(frame)
}

fn perimeter_chase_animation(
    clockwise: bool,
    color: RGB8,
    duration: Duration,
) -> Result<heapless::Vec<(Frame2d<8, 12>, Duration), PERIMETER_LENGTH>> {
    assert!(
        duration.as_micros() > 0,
        "perimeter animation duration must be positive"
    );
    const SNAKE_LENGTH: usize = 4;
    assert!(
        SNAKE_LENGTH <= PERIMETER_LENGTH,
        "snake length must fit inside the perimeter"
    );
    let coordinates = perimeter_coordinates(clockwise);
    let mut frames = heapless::Vec::new();
    for head_index in 0..PERIMETER_LENGTH {
        let mut frame = Frame2d::new();
        for segment_offset in 0..SNAKE_LENGTH {
            let coordinate_index =
                (head_index + PERIMETER_LENGTH - segment_offset) % PERIMETER_LENGTH;
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

    for x_index in 0..Led8x12::WIDTH {
        push(x_index, 0);
    }
    for y_index in 1..Led8x12::HEIGHT {
        push(Led8x12::WIDTH - 1, y_index);
    }
    for x_index in (0..(Led8x12::WIDTH - 1)).rev() {
        push(x_index, Led8x12::HEIGHT - 1);
    }
    for y_index in (1..(Led8x12::HEIGHT - 1)).rev() {
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
    let digit = value / 10;
    if digit == 0 {
        'O'
    } else {
        (digit + b'0') as char
    }
}

#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    reason = "Value < 100 ensures division is safe"
)]
fn ones_digit(value: u8) -> char {
    let digit = value % 10;
    if digit == 0 {
        'O'
    } else {
        (digit + b'0') as char
    }
}
