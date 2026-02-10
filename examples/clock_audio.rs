#![allow(missing_docs)]
//! Clock Audio - WiFi-synced console clock with audio state/tick cues.
//!
//! It starts in `hh:mm` mode (minute ticks). Press the button on GP13 to
//! toggle to `mm:ss` mode (second ticks), then press again to switch back.
//!
//! Audio wiring (MAX98357A):
//! - DIN  -> GP8
//! - BCLK -> GP9
//! - LRC  -> GP10

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy::audio_player::{AtEnd, Volume, audio_player};
use device_envoy::button::PressedTo;
use device_envoy::clock_sync::{ClockSync, ClockSyncStatic, ONE_MINUTE, ONE_SECOND, h12_m_s};
use device_envoy::flash_array::FlashArray;
use device_envoy::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy::wifi_auto::{WifiAuto, WifiAutoEvent};
use device_envoy::{Error, Result};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::Duration;
use panic_probe as _;

audio_player! {
    AudioPlayer10 {
        din_pin: PIN_8,
        bclk_pin: PIN_9,
        lrc_pin: PIN_10,
        pio: PIO1,
        dma: DMA_CH1,
        volume: Volume::percent(10),
    }
}

const CAPTIVE_PORTAL_TONE: [i16; AudioPlayer10::samples_ms(120)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(330), Volume::percent(20));
const CONNECTING_TONE: [i16; AudioPlayer10::samples_ms(90)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(550), Volume::percent(20));
const CONNECTION_FAILED_TONE: [i16; AudioPlayer10::samples_ms(150)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(220), Volume::percent(25));
const WIFI_CONNECTED_TONE: [i16; AudioPlayer10::samples_ms(140)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(880), Volume::percent(20));
const TIME_SYNCED_TONE: [i16; AudioPlayer10::samples_ms(90)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(1047), Volume::percent(20));
const MODE_HH_MM_TONE: [i16; AudioPlayer10::samples_ms(100)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(698), Volume::percent(18));
const MODE_MM_SS_TONE: [i16; AudioPlayer10::samples_ms(100)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(988), Volume::percent(18));
const HH_MM_TICK_TONE: [i16; AudioPlayer10::samples_ms(70)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(784), Volume::percent(15));
const MM_SS_TICK_TONE: [i16; AudioPlayer10::samples_ms(40)] =
    AudioPlayer10::with_volume(&AudioPlayer10::tone(523), Volume::percent(12));
const SILENCE_40MS: [i16; AudioPlayer10::samples_ms(40)] = AudioPlayer10::silence();

#[derive(Clone, Copy)]
enum ClockAudioMode {
    HoursMinutes,
    MinutesSeconds,
}

impl ClockAudioMode {
    fn tick_interval(self) -> Duration {
        match self {
            Self::HoursMinutes => ONE_MINUTE,
            Self::MinutesSeconds => ONE_SECOND,
        }
    }

    fn toggled(self) -> Self {
        match self {
            Self::HoursMinutes => Self::MinutesSeconds,
            Self::MinutesSeconds => Self::HoursMinutes,
        }
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Clock Audio with WiFi");

    let p = embassy_rp::init(Default::default());

    let audio_player8 = AudioPlayer10::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH1, spawner)?;

    let [wifi_credentials_flash_block, timezone_flash_block] = FlashArray::<2>::new(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    let wifi_auto = WifiAuto::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH0,
        wifi_credentials_flash_block,
        p.PIN_13,
        PressedTo::Ground,
        "www.picoclock.net",
        [timezone_field],
        spawner,
    )?;

    let audio_player10_ref = audio_player8;
    let (stack, mut button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    info!("Captive portal ready");
                    audio_player10_ref.play(
                        [&CAPTIVE_PORTAL_TONE, &SILENCE_40MS, &CAPTIVE_PORTAL_TONE],
                        AtEnd::Stop,
                    );
                }
                WifiAutoEvent::Connecting {
                    try_index,
                    try_count,
                } => {
                    info!("Connecting (attempt {} of {})", try_index + 1, try_count);
                    audio_player10_ref.play([&CONNECTING_TONE], AtEnd::Stop);
                }
                WifiAutoEvent::ConnectionFailed => {
                    info!("WiFi connection failed");
                    audio_player10_ref.play(
                        [
                            &CONNECTION_FAILED_TONE,
                            &SILENCE_40MS,
                            &CONNECTION_FAILED_TONE,
                        ],
                        AtEnd::Stop,
                    );
                }
            }
            Ok(())
        })
        .await?;

    info!("WiFi connected");
    audio_player8.play(
        [&WIFI_CONNECTED_TONE, &SILENCE_40MS, &WIFI_CONNECTED_TONE],
        AtEnd::Stop,
    );

    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();

    let mut clock_audio_mode = ClockAudioMode::HoursMinutes;
    let clock_sync = ClockSync::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(clock_audio_mode.tick_interval()),
        spawner,
    );

    // First tick confirms successful time sync.
    let first_tick = clock_sync.wait_for_tick().await;
    let (first_hours, first_minutes, first_seconds) = h12_m_s(&first_tick.local_time);
    audio_player8.play([&TIME_SYNCED_TONE], AtEnd::Stop);
    info!(
        "Time synced: {:02}:{:02}:{:02} (button toggles mode)",
        first_hours, first_minutes, first_seconds
    );

    loop {
        match select(button.wait_for_press(), clock_sync.wait_for_tick()).await {
            Either::First(()) => {
                clock_audio_mode = clock_audio_mode.toggled();
                clock_sync
                    .set_tick_interval(Some(clock_audio_mode.tick_interval()))
                    .await;

                match clock_audio_mode {
                    ClockAudioMode::HoursMinutes => {
                        audio_player8.play([&MODE_HH_MM_TONE], AtEnd::Stop);
                        info!("Mode changed: hh:mm (minute tick)");
                    }
                    ClockAudioMode::MinutesSeconds => {
                        audio_player8.play([&MODE_MM_SS_TONE], AtEnd::Stop);
                        info!("Mode changed: mm:ss (second tick)");
                    }
                }
            }
            Either::Second(tick) => {
                let (hours, minutes, seconds) = h12_m_s(&tick.local_time);
                match clock_audio_mode {
                    ClockAudioMode::HoursMinutes => {
                        audio_player8.play([&HH_MM_TICK_TONE], AtEnd::Stop);
                        info!(
                            "hh:mm {:02}:{:02} (since sync: {}s)",
                            hours,
                            minutes,
                            tick.since_last_sync.as_secs()
                        );
                    }
                    ClockAudioMode::MinutesSeconds => {
                        audio_player8.play([&MM_SS_TICK_TONE], AtEnd::Stop);
                        info!(
                            "mm:ss {:02}:{:02} (since sync: {}s)",
                            minutes,
                            seconds,
                            tick.since_last_sync.as_secs()
                        );
                    }
                }
            }
        }
    }
}
