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
use embassy_net::{dns::DnsQueryType, udp, Stack};
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use log::{info, warn};

use device_envoy_esp::{
    button::PressedTo,
    flash_array::FlashArray,
    init_and_start, led2d,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::{colors, Current, Gamma},
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto, WifiAutoEvent,
    },
};

esp_bootloader_esp_idf::esp_app_desc!();

const CAPTIVE_PORTAL_SSID: &str = "EnvoyClock";
const DIGIT_COLORS: [smart_leds::RGB8; 4] =
    [colors::CYAN, colors::MAGENTA, colors::ORANGE, colors::LIME];

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

const NTP_SERVER: &str = "pool.ntp.org";
const NTP_PORT: u16 = 123;
const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
const BUTTON_POLL_INTERVAL: Duration = Duration::from_millis(200);
const LONG_PRESS_THRESHOLD: Duration = Duration::from_millis(900);

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

    let [wifi_auto_flash_block, timezone_flash_block] = FlashArray::<2>::new(p.FLASH)?;
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    let wifi_auto = WifiAuto::new(
        p.WIFI,
        wifi_auto_flash_block,
        p.GPIO6,
        PressedTo::Ground,
        CAPTIVE_PORTAL_SSID,
        [timezone_field],
        spawner,
    )?;

    let led8x12_clock_ref = &led8x12_clock;
    let (stack, force_portal_button) = wifi_auto
        .connect(|wifi_auto_event| async move {
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

    while !stack.is_link_up() || stack.config_v4().is_none() {
        led8x12_clock.write_text("IP\n..", &DIGIT_COLORS);
        Timer::after(Duration::from_millis(200)).await;
    }

    info!("network up: {:?}", stack.config_v4());

    let mut offset_minutes = timezone_field.offset_minutes()?.unwrap_or(0);
    info!("timezone offset minutes: {}", offset_minutes);

    let mut synced_unix_seconds = fetch_ntp_unix_seconds(&stack).await;
    let mut synced_at = Instant::now();
    let mut display_mode = DisplayMode::HourMinute;
    let mut was_pressed = false;
    let mut pressed_since: Option<Instant> = None;

    loop {
        if synced_unix_seconds.is_none() || synced_at.elapsed() >= Duration::from_secs(3600) {
            synced_unix_seconds = fetch_ntp_unix_seconds(&stack).await;
            synced_at = Instant::now();
        }

        let is_pressed = force_portal_button.is_pressed();
        if is_pressed && !was_pressed {
            pressed_since = Some(Instant::now());
        } else if !is_pressed && was_pressed {
            if let Some(press_start) = pressed_since {
                let held_for = Instant::now() - press_start;
                if held_for >= LONG_PRESS_THRESHOLD {
                    offset_minutes = increment_timezone_offset_minutes(offset_minutes);
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("timezone offset changed by button: {}", offset_minutes);
                } else {
                    display_mode = match display_mode {
                        DisplayMode::HourMinute => DisplayMode::MinuteSecond,
                        DisplayMode::MinuteSecond => DisplayMode::HourMinute,
                    };
                    info!("display mode changed: {:?}", display_mode);
                }
            }
            pressed_since = None;
        }
        was_pressed = is_pressed;

        if let Some(base_unix_seconds) = synced_unix_seconds {
            let current_unix_seconds = base_unix_seconds + (synced_at.elapsed().as_secs() as i64);
            let display_text = match display_mode {
                DisplayMode::HourMinute => format_hhmm(current_unix_seconds, offset_minutes),
                DisplayMode::MinuteSecond => format_mmss(current_unix_seconds, offset_minutes),
            };
            let display_text_with_o = format_zero_as_o(&display_text);
            led8x12_clock.write_text(display_text_with_o.as_str(), &DIGIT_COLORS);
        } else {
            led8x12_clock.write_text("--\n--", &DIGIT_COLORS);
        }

        Timer::after(BUTTON_POLL_INTERVAL).await;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DisplayMode {
    HourMinute,
    MinuteSecond,
}

async fn fetch_ntp_unix_seconds(stack: &Stack<'static>) -> Option<i64> {
    let dns_results = match stack.dns_query(NTP_SERVER, DnsQueryType::A).await {
        Ok(dns_results) => dns_results,
        Err(error) => {
            warn!("NTP DNS failed: {:?}", error);
            return None;
        }
    };
    let server_ip = match dns_results.first().copied() {
        Some(server_ip) => server_ip,
        None => return None,
    };

    let mut rx_metadata = [udp::PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0u8; 128];
    let mut tx_metadata = [udp::PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 128];
    let mut socket = udp::UdpSocket::new(
        *stack,
        &mut rx_metadata,
        &mut rx_buffer,
        &mut tx_metadata,
        &mut tx_buffer,
    );

    if socket.bind(0).is_err() {
        return None;
    }

    let mut ntp_request = [0u8; 48];
    ntp_request[0] = 0x1B;
    if socket
        .send_to(&ntp_request, (server_ip, NTP_PORT))
        .await
        .is_err()
    {
        return None;
    }

    let mut ntp_response = [0u8; 48];
    let received =
        embassy_time::with_timeout(Duration::from_secs(5), socket.recv_from(&mut ntp_response))
            .await;
    let (received_len, _) = match received {
        Ok(Ok(data)) => data,
        _ => return None,
    };
    if received_len < 48 {
        return None;
    }

    let ntp_seconds = u32::from_be_bytes([
        ntp_response[40],
        ntp_response[41],
        ntp_response[42],
        ntp_response[43],
    ]);
    let unix_seconds = (ntp_seconds as i64) - NTP_TO_UNIX_SECONDS;
    if unix_seconds <= 0 {
        return None;
    }

    info!("NTP sync ok: unix={}", unix_seconds);
    Some(unix_seconds)
}

fn format_hhmm(unix_seconds: i64, offset_minutes: i32) -> heapless::String<5> {
    let local_seconds = unix_seconds + (offset_minutes as i64 * 60);
    let seconds_in_day = 24 * 60 * 60;
    let day_seconds = local_seconds.rem_euclid(seconds_in_day);
    let hour24 = (day_seconds / 3600) as u8;
    let minute = ((day_seconds % 3600) / 60) as u8;
    let hour12 = match hour24 {
        0 => 12,
        1..=12 => hour24,
        _ => hour24 - 12,
    };

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

fn format_mmss(unix_seconds: i64, offset_minutes: i32) -> heapless::String<5> {
    let local_seconds = unix_seconds + (offset_minutes as i64 * 60);
    let seconds_in_hour = 60 * 60;
    let hour_seconds = local_seconds.rem_euclid(seconds_in_hour);
    let minute = (hour_seconds / 60) as u8;
    let second = (hour_seconds % 60) as u8;

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

fn increment_timezone_offset_minutes(offset_minutes: i32) -> i32 {
    let next_offset_minutes = offset_minutes + 60;
    if next_offset_minutes > 840 {
        -720
    } else {
        next_offset_minutes
    }
}
