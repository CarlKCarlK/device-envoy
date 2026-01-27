//! WiFi auto-provisioning demo with LED display showing Unix time.

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

// Set up LED layout for 12x8 panel (two 12x4 panels stacked)
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);

// Color palette for display
const COLORS: &[smart_leds::RGB8] = &[
    colors::YELLOW,
    colors::LIME,
    colors::CYAN,
    colors::RED,
    colors::WHITE,
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

    // Create device name field (max 4 characters)
    static DEVICE_NAME_STATIC: TextFieldStatic<4> = TextField::new_static();
    let device_name_field = TextField::new(
        &DEVICE_NAME_STATIC,
        device_name_flash_block,
        "name",
        "Name",
        "PICO",
    );

    // Initialize WifiAuto
    let wifi_auto = WifiAuto::new(
        p.PIN_23,  // CYW43 power
        p.PIN_25,  // CYW43 chip select
        p.PIO0,    // CYW43 PIO interface (required)
        p.PIN_24,  // CYW43 clock
        p.PIN_29,  // CYW43 data
        p.DMA_CH0, // CYW43 DMA (required)
        wifi_credentials_flash_block,
        p.PIN_13, // Button for forced reconfiguration
        PressedTo::Ground,
        "PicoTime", // Captive-portal SSID
        [device_name_field],
        spawner,
    )?;

    // Set up LED display (PIO1/DMA_CH1 to avoid conflict with WiFi)
    let led12x8 = Led12x8::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;

    // Connect with status on display
    let led12x8_ref = &led12x8;
    let (stack, _button) = wifi_auto
        .connect(spawner, move |event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    led12x8_ref.write_text("CONN", COLORS).await.ok();
                }
                WifiAutoEvent::Connecting { .. } => {
                    led12x8_ref.write_text("...", COLORS).await.ok();
                }
                WifiAutoEvent::Connected => {
                    led12x8_ref.write_text("DONE", COLORS).await.ok();
                }
                WifiAutoEvent::ConnectionFailed => {
                    led12x8_ref.write_text("FAIL", COLORS).await.ok();
                }
            }
        })
        .await?;

    // Get device name from field
    let device_name = device_name_field.text()?.unwrap_or_default();
    info!("Device name: {}", device_name.as_str());

    // Show initial state with dashes until time arrives
    let initial_display = format_two_lines("----", device_name.as_str());
    led12x8.write_text(&initial_display, COLORS).await?;

    // Main loop: fetch and display time every minute
    loop {
        match fetch_ntp_time(stack).await {
            Ok(unix_seconds) => {
                // Get last 4 digits of unix timestamp
                let last_4_digits = (unix_seconds % 10000) as u16;
                let time_str = format_4_digits(last_4_digits);

                // Display: time on line 1, name on line 2
                let display_text = format_two_lines(&time_str, device_name.as_str());
                led12x8.write_text(&display_text, COLORS).await?;

                info!("Time: {} | Name: {}", time_str, device_name.as_str());
            }
            Err(msg) => {
                warn!("NTP fetch failed: {}", msg);
                // Keep showing dashes with device name on error
                let error_display = format_two_lines("----", device_name.as_str());
                led12x8.write_text(&error_display, COLORS).await?;
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
    const NTP_SERVER: &str = "pool.ntp.org";
    const NTP_PORT: u16 = 123;

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
