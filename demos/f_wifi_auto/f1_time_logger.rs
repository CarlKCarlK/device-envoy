//! WiFi auto-provisioning demo with LED display showing time and device name.
//!
//! This demo shows `WifiAuto` usage with a custom text field. It provisions WiFi
//! credentials and a short device name through a captive portal, then displays:
//! - Line 1: Last 4 digits of Unix timestamp (updates every minute, cyan)
//! - Line 2: The device name entered during provisioning (magenta)

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, panic};
use defmt::{info, warn};
use device_kit::{
    Result,
    button::PressedTo,
    flash_array::{FlashArray, FlashArrayStatic},
    led_strip::colors,
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
    wifi_auto::fields::{TextField, TextFieldStatic},
    wifi_auto::{WifiAuto, WifiAutoEvent},
};
use embassy_executor::Spawner;
use embassy_net::{
    Stack,
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const NTP_SERVER: &str = "pool.ntp.org";
const NTP_PORT: u16 = 123;

// Set up LED layout for 12x8 panel (two 12x4 panels stacked)
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

// Color palettes for display
const COLORS_WIFI: &[smart_leds::RGB8] = &[colors::YELLOW, colors::CYAN, colors::WHITE];
const COLORS_SUCCESS: &[smart_leds::RGB8] = &[colors::GREEN, colors::CYAN];
const COLORS_ERROR: &[smart_leds::RGB8] = &[colors::RED, colors::ORANGE];
const COLORS_MAIN: &[smart_leds::RGB8] = &[
    colors::CYAN,
    colors::YELLOW,
    colors::GREEN,
    colors::BLUE,
    colors::MAGENTA,
    colors::RED,
    colors::ORANGE,
    colors::PURPLE,
];

led2d! {
    Led12x8 {
        pin: PIN_4,
        pio: PIO1,
        dma: DMA_CH1,
        led_layout: LED_LAYOUT_12X8,
        font: Led2dFont::Font3x4Trim,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("WiFi Auto LED Display - Starting");
    let p = embassy_rp::init(Default::default());

    // Set up flash storage for WiFi credentials and device name
    static FLASH_STATIC: FlashArrayStatic = FlashArray::<2>::new_static();
    let [wifi_credentials_flash_block, device_name_flash_block] =
        FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    // Create device name field (max 4 characters for the display)
    static DEVICE_NAME_STATIC: TextFieldStatic<4> = TextField::new_static();
    let device_name_field = TextField::new(
        &DEVICE_NAME_STATIC,
        device_name_flash_block,
        "name",
        "Name",
        "PICO",
    );

    // Initialize WifiAuto with device name field
    let wifi_auto = WifiAuto::new(
        p.PIN_23,  // CYW43 power
        p.PIN_25,  // CYW43 chip select
        p.PIO0,    // CYW43 PIO interface
        p.PIN_24,  // CYW43 clock
        p.PIN_29,  // CYW43 data
        p.DMA_CH0, // CYW43 DMA
        wifi_credentials_flash_block,
        p.PIN_13, // Button for forced reconfiguration
        PressedTo::Ground,
        "PicoTime",          // Captive-portal SSID
        [device_name_field], // Custom field for device name
        spawner,
    )?;

    // Set up LED display
    let led12x8 = Led12x8::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    // Connect with status on display
    let led12x8_ref = &led12x8;
    let (stack, _button) = wifi_auto
        .connect(spawner, move |event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    info!("Captive portal ready - connect to 'PicoTime' WiFi network");
                    // Don't show anything - portal runs in background for reconfiguration
                }
                WifiAutoEvent::Connecting { try_index, .. } => {
                    info!("Connecting to WiFi");
                    // Show connecting animation
                    let dots = match try_index {
                        0 => "WIFI",
                        1 => "WIFI\n.",
                        2 => "WIFI\n..",
                        _ => "WIFI\n...",
                    };
                    led12x8_ref.write_text(dots, COLORS_WIFI).await.ok();
                }
                WifiAutoEvent::Connected => {
                    info!("WiFi connected");
                    led12x8_ref.write_text("DONE", COLORS_SUCCESS).await.ok();
                }
                WifiAutoEvent::ConnectionFailed => {
                    info!("WiFi connection failed");
                    led12x8_ref.write_text("FAIL", COLORS_ERROR).await.ok();
                }
            }
        })
        .await?;

    // Get device name from the field
    let device_name = device_name_field.text()?.unwrap_or_default();
    info!("Device name: {}", device_name.as_str());

    // Show initial state with dashes until time arrives
    let initial_display = format_two_lines("----", device_name.as_str());
    led12x8.write_text(&initial_display, COLORS_MAIN).await?;

    // Main loop: fetch and display time every minute
    loop {
        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                // Get last 4 digits of unix timestamp
                let last_4_digits = (unix_seconds % 10000) as u16;
                let time_str = format_4_digits(last_4_digits);

                // Display: time on line 1, name on line 2
                let display_text = format_two_lines(&time_str, device_name.as_str());
                led12x8.write_text(&display_text, COLORS_MAIN).await?;

                info!("Time: {} | Name: {}", time_str, device_name.as_str());
            }
            Err(msg) => {
                warn!("NTP fetch failed: {}", msg);
                // Keep showing dashes with device name on error
                let error_display = format_two_lines("----", device_name.as_str());
                led12x8.write_text(&error_display, COLORS_MAIN).await?;
            }
        }

        Timer::after(Duration::from_secs(60)).await;
    }
}

/// Format a number as a 4-digit string with leading zeros
fn format_4_digits(num: u16) -> heapless::String<4> {
    use core::fmt::Write;
    let mut s = heapless::String::new();
    write!(&mut s, "{:04}", num).unwrap();
    s
}

/// Format two lines of text separated by newline
fn format_two_lines(line1: &str, line2: &str) -> heapless::String<9> {
    use core::fmt::Write;
    let mut s = heapless::String::new();
    write!(&mut s, "{}\n{}", line1, line2).unwrap();
    s
}

/// Fetch current time from NTP server and return Unix timestamp.
async fn fetch_ntp_time(stack: &Stack<'static>) -> core::result::Result<i64, &'static str> {
    // DNS lookup
    let dns_result = stack
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
        .map_err(|_| "DNS lookup failed")?;
    let server_addr = dns_result.first().ok_or("No DNS results")?;

    // Create UDP socket
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0; 128];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0; 128];
    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    socket.bind(0).map_err(|_| "Socket bind failed")?;

    // Build NTP request (48 bytes, version 3, client mode)
    let mut ntp_request = [0u8; 48];
    ntp_request[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)

    // Send request
    socket
        .send_to(&ntp_request, (*server_addr, NTP_PORT))
        .await
        .map_err(|_| "NTP send failed")?;

    // Receive response with timeout
    let mut response = [0u8; 48];
    embassy_time::with_timeout(Duration::from_secs(5), socket.recv_from(&mut response))
        .await
        .map_err(|_| "NTP receive timeout")?
        .map_err(|_| "NTP receive failed")?;

    // Extract transmit timestamp from response (bytes 40-43)
    let ntp_seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]);

    // Convert NTP time (seconds since 1900) to Unix time (seconds since 1970)
    const NTP_TO_UNIX_OFFSET: i64 = 2_208_988_800;
    let unix_seconds = (ntp_seconds as i64) - NTP_TO_UNIX_OFFSET;

    Ok(unix_seconds)
}
