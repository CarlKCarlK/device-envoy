#![allow(missing_docs)]
//! WiFi auto-provisioning demo with LED display showing last 4 hex digits of DNS.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, fmt::Write, panic};
use device_kit::{
    Result,
    button::PressedTo,
    flash_array::{FlashArray, FlashArrayStatic},
    led_strip::colors,
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
    wifi_auto::{WifiAuto, WifiAutoEvent},
};
use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> =
    LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4).rotate_cw();

const COLORS: &[smart_leds::RGB8] = &[colors::YELLOW, colors::LIME, colors::CYAN, colors::RED];

led2d! {
    Led8x12 {
        pin: PIN_4,
        led_layout: LED_LAYOUT_8X12,
        font: Led2dFont::Font4x6Trim,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let led8x12 = Led8x12::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    // Flash stores WiFi credentials after first setup
    static FLASH_STATIC: FlashArrayStatic = FlashArray::<1>::new_static();
    let [wifi_credentials_flash_block] = FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    // Create a WifiAuto instance.
    // A button is used to force reconfiguration via setup web page.
    // Pico W uses the CYW43 chip wired to fixed GPIOs; we pass those here.
    let wifi_auto = WifiAuto::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,  // CYW43 pins (fixed)
        p.PIO1,    // Needs a PIO resource
        p.DMA_CH1, // Needs a DMA resource
        wifi_credentials_flash_block,
        p.PIN_15, // Button for forced reconfiguration
        PressedTo::Ground,
        "PicoDemo", // Setup SSID
        [],         // Any custom fields
        spawner,
    )?;

    // Try to connect. Will launch setup web page as needed.
    // Returns network stack and button.
    //
    // Borrow `led8x12` outside closure so the event handler can use it without owning it.
    let led8x12_ref = &led8x12;
    let (stack, _button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    led8x12_ref.write_text("JO\nIN", COLORS).await? // Join setup network
                }
                WifiAutoEvent::Connecting { .. } => show_animated_dots(led8x12_ref).await?,
                WifiAutoEvent::ConnectionFailed => led8x12_ref.write_text("FA\nIL", COLORS).await?,
            }
            Ok(())
        })
        .await?;

    // Show initial state with dashes until DNS is fetched.
    led8x12.write_text("--\n--", COLORS).await?;

    // Do DNS on google.com periodically. Display last 4 hex digits of IP address.
    loop {
        let mut hex_str: heapless::String<6> = heapless::String::new();
        if let Ok(Some(embassy_net::IpAddress::Ipv4(ipv4))) = stack
            .dns_query("google.com", DnsQueryType::A)
            .await
            .map(|results| results.first().copied())
        {
            let bytes = ipv4.octets();
            write!(&mut hex_str, "{:02X}\n{:02X}", bytes[2], bytes[3]).unwrap();
        } else {
            hex_str.push_str("--\n--").unwrap();
        }
        led8x12.write_text(&hex_str, COLORS).await?;

        Timer::after(Duration::from_secs(15)).await;
    }
}
// Not shown:
//  - You can define custom fields for the setup web page to collect extra
//    information from the user, such as text or a timezone. Custom HTML
//    snippets are supported.
//
// Limitations:
//  - Only standard SSID/password 2.4 Ghz WiFi networks are supported.
//  - Networks that require their own login web page after connecting
//    (for example, public WiFi with an acceptance form) are not supported.

async fn show_animated_dots(led8x12: &Led8x12) -> Result<()> {
    const FRAME_DURATION: Duration = Duration::from_millis(200);
    let mut frames = [(led2d::Frame2d::new(), FRAME_DURATION); 4];
    led8x12.write_text_to_frame(".\n ", &[COLORS[0]], &mut frames[0].0)?;
    led8x12.write_text_to_frame(" .\n ", &[COLORS[1]], &mut frames[1].0)?;
    led8x12.write_text_to_frame(" \n .", &[COLORS[2]], &mut frames[2].0)?;
    led8x12.write_text_to_frame(" \n. ", &[COLORS[3]], &mut frames[3].0)?;

    led8x12.animate(frames)
}
