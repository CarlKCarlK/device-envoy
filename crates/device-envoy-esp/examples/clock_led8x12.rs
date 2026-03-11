//! Wi-Fi enabled 8x12 LED clock with captive-portal setup.
//!
//! Uses `WifiAuto` for setup and connection, including a timezone field.
//! The display shows hours on the top line and minutes on the bottom line.
//!
//! Hardware:
//! - 8x12 panel on GPIO18
//! - optional force-portal button on GPIO6 wired to GND

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{PressDuration, PressedTo},
    button_watch,
    clock_sync::{h12_m_s, ClockSync as _, ClockSyncEsp, ClockSyncStatic, ONE_MINUTE, ONE_SECOND},
    flash_block::FlashBlockEsp,
    init_and_start, led2d,
    led2d::Led2d as _,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::{colors, Current, Gamma},
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto as _, WifiAutoEsp, WifiAutoEvent,
    },
};

use device_envoy_esp::button::Button as _;

esp_bootloader_esp_idf::esp_app_desc!();

const CAPTIVE_PORTAL_SSID: &str = "EnvoyClock";
const DIGIT_COLORS: [smart_leds::RGB8; 4] =
    [colors::CYAN, colors::MAGENTA, colors::ORANGE, colors::LIME];

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

button_watch! {
    ForcePortalButtonWatch {
        pin: GPIO6,
    }
}

led2d! {
    Led8x12Clock {
        len: 96,
        led_layout: LED_LAYOUT_8X12,
        max_current: Current::Milliamps(250),
        font: Led2dFont::Font4x6Trim,
        gamma: Gamma::Linear,
        max_frames: 16,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("clock_led8x12 starting");

    let led8x12_clock = Led8x12Clock::new(p.GPIO18, rmt80.channel0, spawner)?;

    let [wifi_auto_flash_block, timezone_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);
    let button6 = ForcePortalButtonWatch::new(p.GPIO6, PressedTo::Ground, spawner).await?;

    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        CAPTIVE_PORTAL_SSID,
        [timezone_field],
        spawner,
    )?;

    let led8x12_clock_ref = &led8x12_clock;
    let stack = wifi_auto
        .connect(&mut *button6, |wifi_auto_event| async move {
            match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => {
                    led8x12_clock_ref.write_text("JO\nIN", &DIGIT_COLORS);
                }
                WifiAutoEvent::Connecting { .. } => {
                    led8x12_clock_ref.write_text("CO\nNN", &DIGIT_COLORS);
                }
                WifiAutoEvent::ConnectionFailed => {
                    led8x12_clock_ref.write_text("FA\nIL", &DIGIT_COLORS);
                }
            }
            Ok(())
        })
        .await?;

    led8x12_clock.write_text("IP\n..", &DIGIT_COLORS);
    stack.wait_config_up().await;

    info!("network up: {:?}", stack.config_v4());

    let mut offset_minutes = timezone_field.offset_minutes()?.unwrap_or(0);
    info!("timezone offset minutes: {}", offset_minutes);

    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncEsp::new_static();
    let clock_sync = ClockSyncEsp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    let mut display_mode = DisplayMode::HourMinute;
    render_clock_text(&led8x12_clock, display_mode, &clock_sync.now_local());

    loop {
        match select(button6.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
            Either::First(press_duration) => match press_duration {
                PressDuration::Short => {
                    display_mode = match display_mode {
                        DisplayMode::HourMinute => DisplayMode::MinuteSecond,
                        DisplayMode::MinuteSecond => DisplayMode::HourMinute,
                    };
                    clock_sync.set_tick_interval(Some(match display_mode {
                        DisplayMode::HourMinute => ONE_MINUTE,
                        DisplayMode::MinuteSecond => ONE_SECOND,
                    }));
                    info!("display mode changed: {:?}", display_mode);
                    render_clock_text(&led8x12_clock, display_mode, &clock_sync.now_local());
                }
                PressDuration::Long => {
                    offset_minutes = increment_timezone_offset_minutes(offset_minutes);
                    clock_sync.set_offset_minutes(offset_minutes);
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("timezone offset changed by button: {}", offset_minutes);
                    render_clock_text(&led8x12_clock, display_mode, &clock_sync.now_local());
                }
            },
            Either::Second(clock_sync_tick) => {
                render_clock_text(&led8x12_clock, display_mode, &clock_sync_tick.local_time);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayMode {
    HourMinute,
    MinuteSecond,
}

fn format_hhmm(hour12: u8, minute: u8) -> heapless::String<5> {
    let mut text = heapless::String::<5>::new();
    if hour12 >= 10 {
        text.push(char::from(b'0' + (hour12 / 10))).ok();
    } else {
        text.push(' ').ok();
    }
    text.push(char::from(b'0' + (hour12 % 10))).ok();
    text.push('\n').ok();
    text.push(char::from(b'0' + (minute / 10))).ok();
    text.push(char::from(b'0' + (minute % 10))).ok();
    text
}

fn format_mmss(minute: u8, second: u8) -> heapless::String<5> {
    let mut text = heapless::String::<5>::new();
    text.push(char::from(b'0' + (minute / 10))).ok();
    text.push(char::from(b'0' + (minute % 10))).ok();
    text.push('\n').ok();
    text.push(char::from(b'0' + (second / 10))).ok();
    text.push(char::from(b'0' + (second % 10))).ok();
    text
}

fn format_zero_as_o(text: &str) -> heapless::String<5> {
    let mut formatted_text = heapless::String::<5>::new();
    for display_character in text.chars() {
        if display_character == '0' {
            formatted_text.push('O').ok();
        } else {
            formatted_text.push(display_character).ok();
        }
    }
    formatted_text
}

fn render_clock_text(
    led8x12_clock: &'static Led8x12Clock,
    display_mode: DisplayMode,
    local_time: &time::OffsetDateTime,
) {
    let (hours, minutes, seconds) = h12_m_s(local_time);
    let display_text = match display_mode {
        DisplayMode::HourMinute => format_hhmm(hours, minutes),
        DisplayMode::MinuteSecond => format_mmss(minutes, seconds),
    };
    let display_text_with_o = format_zero_as_o(&display_text);
    led8x12_clock.write_text(display_text_with_o.as_str(), &DIGIT_COLORS);
}

fn increment_timezone_offset_minutes(offset_minutes: i32) -> i32 {
    let next_offset_minutes = offset_minutes + 60;
    if next_offset_minutes > 840 {
        -720
    } else {
        next_offset_minutes
    }
}
