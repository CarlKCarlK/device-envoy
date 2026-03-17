//! Wi-Fi DNS demo: resolve `google.com` and display the last 4 hex digits.
//!
//! Display format is two lines (`AB\nCD`) on the 8x12 panel wired to GPIO18.
//!
//! Uses `WifiAuto` with flash-backed credentials and captive portal setup.
//! Hold the GPIO6 button low during boot to force setup mode.

#![no_std]
#![no_main]

extern crate alloc;

use core::fmt::Write;

use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::{info, warn};

use device_envoy_esp::{
    button::{ButtonEsp, PressedTo},
    flash_block::FlashBlockEsp,
    init_and_start, led2d,
    led2d::Led2d as _,
    led2d::{layout::LedLayout, Led2dFont},
    led_strip::{colors, Current, Gamma},
    wifi_auto::{WifiAuto as _, WifiAutoEsp, WifiAutoEvent},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const COLORS: &[smart_leds::RGB8] = &[colors::YELLOW, colors::LIME, colors::CYAN, colors::RED];
const DNS_PERIOD: Duration = Duration::from_secs(15);
const CAPTIVE_PORTAL_SSID: &str = "DeviceEnvoySetup";

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
const LED_LAYOUT_8X12_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led2d! {
    Led12x8Dns {
        pin: GPIO18,
        len: 96,
        led_layout: LED_LAYOUT_8X12_ROTATED,
        max_current: Current::Milliamps(300),
        font: Led2dFont::Font4x6Trim,
        gamma: Gamma::Linear,
        max_frames: 4,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let led12x8_dns = Led12x8Dns::new(p.GPIO18, rmt80.channel0, spawner)?;
    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        CAPTIVE_PORTAL_SSID,
        [],
        spawner,
    )?;

    let led12x8_dns_ref = &led12x8_dns;
    let stack = wifi_auto
        .connect(&mut button6, |wifi_auto_event| async move {
            match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => {
                    led12x8_dns_ref.write_text("JO\nIN", COLORS);
                }
                WifiAutoEvent::Connecting {
                    try_index,
                    try_count: _,
                } => {
                    info!("connect try {}", try_index + 1);
                    led12x8_dns_ref.write_text("CO\nNN", COLORS);
                }
                WifiAutoEvent::ConnectionFailed => {
                    warn!("wifi_auto connection failed");
                    led12x8_dns_ref.write_text("FA\nIL", COLORS);
                }
            }
            Ok(())
        })
        .await?;

    while !stack.is_link_up() {
        led12x8_dns.write_text("IP\n..", COLORS);
        Timer::after(Duration::from_millis(200)).await;
    }
    while stack.config_v4().is_none() {
        led12x8_dns.write_text("IP\n..", COLORS);
        Timer::after(Duration::from_millis(200)).await;
    }

    info!("Wi-Fi up with DHCP: {:?}", stack.config_v4());
    led12x8_dns.write_text("--\n--", COLORS);
    loop {
        let mut hex_text: heapless::String<6> = heapless::String::new();

        if let Ok(Some(embassy_net::IpAddress::Ipv4(ipv4))) = stack
            .dns_query("google.com", DnsQueryType::A)
            .await
            .map(|addresses| addresses.first().copied())
        {
            let ip_bytes = ipv4.octets();
            write!(&mut hex_text, "{:02X}\n{:02X}", ip_bytes[2], ip_bytes[3])
                .expect("formatting into fixed buffer must fit");
        } else {
            hex_text
                .push_str("--\n--")
                .expect("fallback text must fit into fixed buffer");
        }

        led12x8_dns.write_text(&hex_text, COLORS);
        Timer::after(DNS_PERIOD).await;
    }
}
