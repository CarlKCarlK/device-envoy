//! Wi-Fi LCD clock using `WifiAuto`, `TimezoneField`, and `ClockSync`.
//!
//! Wiring:
//! - Force-portal button: GPIO6 -> button -> GND
//! - Character LCD I2C SDA: GPIO1
//! - Character LCD I2C SCL: GPIO2

#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, fmt};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::PressedTo,
    char_lcd::{CharLcd, CharLcdStatic},
    clock_sync::{ClockSync, ClockSyncStatic, ONE_SECOND},
    flash_block::FlashBlockEsp,
    init_and_start,
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto,
    },
    Error, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const CAPTIVE_PORTAL_SSID: &str = "EnvoyClockLcd";

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("Starting LCD clock with WifiAuto");

    let i2c = esp_hal::i2c::master::I2c::new(p.I2C0, esp_hal::i2c::master::Config::default())
        .expect("I2C0 config should be valid")
        .with_sda(p.GPIO1)
        .with_scl(p.GPIO2);

    static CHAR_LCD_STATIC: CharLcdStatic = CharLcd::new_static();
    let char_lcd = CharLcd::new(&CHAR_LCD_STATIC, i2c, spawner)?;

    let [wifi_auto_flash_block, timezone_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;

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

    let (stack, _button) = wifi_auto
        .connect(|_wifi_auto_event| async move { Ok(()) })
        .await?;

    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();
    let clock_sync = ClockSync::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_SECOND),
        spawner,
    );

    info!("Entering clock loop");

    loop {
        let clock_sync_tick = clock_sync.wait_for_tick().await;
        let local_time = clock_sync_tick.local_time;

        let mut text = heapless::String::<64>::new();
        let (hour12, am_pm) = if local_time.hour() == 0 {
            (12, "AM")
        } else if local_time.hour() < 12 {
            (local_time.hour(), "AM")
        } else if local_time.hour() == 12 {
            (12, "PM")
        } else {
            #[expect(clippy::arithmetic_side_effects, reason = "hour guaranteed 13-23")]
            {
                (local_time.hour() - 12, "PM")
            }
        };

        fmt::Write::write_fmt(
            &mut text,
            format_args!(
                "{:2}:{:02}:{:02} {}\n{:04}-{:02}-{:02}",
                hour12,
                local_time.minute(),
                local_time.second(),
                am_pm,
                local_time.year(),
                u8::from(local_time.month()),
                local_time.day()
            ),
        )
        .map_err(|_| Error::FormatError)?;

        char_lcd.write_text(text, 0).await;
    }
}
