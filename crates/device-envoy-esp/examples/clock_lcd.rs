#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, fmt};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{PressedTo},
    button_watch,
    clock_sync::{ClockSync as _, ClockSyncEsp, ClockSyncStatic, ONE_SECOND},
    flash_block::FlashBlockEsp,
    init_and_start, lcd_text,
    lcd_text::LcdText as _,
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto as _, WifiAutoEsp,
    },
    Error, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

lcd_text! {
    i2c: I2C0,
    sda_pin: GPIO16,
    scl_pin: GPIO17,
    LcdTextClock {
        width: 16,
        height: 2,
        address: 0x27
    }
}

button_watch! {
    ButtonWatch6 {
        pin: GPIO6,
    }
}

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

    info!("Starting LCD clock with WiFi");

    let lcd_text_clock = LcdTextClock::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;
    lcd_text_clock.write_text("Booting...\nLCD Clock");

    let [wifi_auto_flash_block, timezone_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        ButtonWatch6::new(p.GPIO6, PressedTo::Ground, spawner)?,
        "EnvoyClockLcd",
        [timezone_field],
        spawner,
    )?;

    let lcd_text_clock_ref = lcd_text_clock;
    let (stack, _button) = wifi_auto
        .connect(|wifi_auto_event| {
            let lcd_text_clock_ref = lcd_text_clock_ref;
            async move {
                match wifi_auto_event {
                    device_envoy_esp::wifi_auto::WifiAutoEvent::CaptivePortalReady => {
                        lcd_text_clock_ref.write_text("Join WiFi:\nEnvoyClockLcd");
                    }
                    device_envoy_esp::wifi_auto::WifiAutoEvent::Connecting { .. } => {
                        lcd_text_clock_ref.write_text("Connecting...\nPlease wait");
                    }
                    device_envoy_esp::wifi_auto::WifiAutoEvent::ConnectionFailed => {
                        lcd_text_clock_ref.write_text("WiFi failed\nRetry setup");
                    }
                }
                Ok(())
            }
        })
        .await?;

    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncEsp::new_static();
    let clock_sync = ClockSyncEsp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_SECOND),
        spawner,
    );

    info!("Entering main event loop");
    lcd_text_clock.write_text("WiFi connected\nWaiting NTP");
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
        lcd_text_clock.write_text(text.as_str());
    }
}
