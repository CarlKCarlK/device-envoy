#![allow(missing_docs)]
//! Clock Audio - WiFi-synced console clock with audio state/tick cues.
//!
//! It starts in `hh:mm` mode (minute ticks). Press the button on GP13 to
//! toggle to `mm:ss` mode (second ticks), then press again to switch back.
//!
//! Audio wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::audio_player::SilenceClip;
use device_envoy_rp::audio_player::{
    AtEnd, AudioPlayer as _, Gain, VOICE_22050_HZ, Volume, audio_player,
};
use device_envoy_rp::button::PressedTo;
use device_envoy_rp::button_watch;
use device_envoy_rp::clock_sync::{
    ClockSync as _, ClockSyncRp, ClockSyncStaticRp, ONE_MINUTE, h12_m_s,
};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::wifi_auto::{WifiAutoEvent, WifiAutoRp};
use device_envoy_rp::{Error, Result, tone};
use embassy_executor::Spawner;
use panic_probe as _;

audio_player! {
    AudioPlayer10 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        pio: PIO1,
        dma: DMA_CH1,
        max_volume: Volume::percent(10),
    }
}

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
    const CAPTIVE_PORTAL_TONE: &AudioPlayer10Playable = &tone!(
        330,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(120)
    )
    .with_gain(Gain::percent(20));
    const CONNECTING_TONE: &AudioPlayer10Playable = &tone!(
        550,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(90)
    )
    .with_gain(Gain::percent(20));
    const CONNECTION_FAILED_TONE: &AudioPlayer10Playable = &tone!(
        220,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(150)
    )
    .with_gain(Gain::percent(25));
    const WIFI_CONNECTED_TONE: &AudioPlayer10Playable = &tone!(
        880,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(140)
    )
    .with_gain(Gain::percent(20));
    const TIME_SYNCED_TONE: &AudioPlayer10Playable = &tone!(
        1047,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(90)
    )
    .with_gain(Gain::percent(20));
    const MODE_MM_SS_TONE: &AudioPlayer10Playable = &tone!(
        988,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(100)
    )
    .with_gain(Gain::percent(18));
    const HH_MM_TICK_TONE: &AudioPlayer10Playable = &tone!(
        784,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(70)
    )
    .with_gain(Gain::percent(15));
    const MM_SS_TICK_TONE: &AudioPlayer10Playable = &tone!(
        523,
        AudioPlayer10::SAMPLE_RATE_HZ,
        StdDuration::from_millis(40)
    )
    .with_gain(Gain::percent(12));
    const SILENCE_40MS: &AudioPlayer10Playable = &SilenceClip::new(StdDuration::from_millis(40));

    info!("Starting Clock Audio with WiFi");

    let p = embassy_rp::init(Default::default());

    let audio_player8 = AudioPlayer10::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH1, spawner)?;

    let [wifi_credentials_flash_block, mut timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH0,
        wifi_credentials_flash_block,
        "DeviceEnvoyClock",
        [timezone_field],
        spawner,
    )?;

    let audio_player10_ref = audio_player8;
    let stack = wifi_auto
        .connect(&mut *button_watch13, |event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    info!("Captive portal ready");
                    audio_player10_ref.play(
                        [CAPTIVE_PORTAL_TONE, SILENCE_40MS, CAPTIVE_PORTAL_TONE],
                        AtEnd::Stop,
                    );
                }
                WifiAutoEvent::Connecting {
                    try_index,
                    try_count,
                } => {
                    info!("Connecting (attempt {} of {})", try_index + 1, try_count);
                    audio_player10_ref.play([CONNECTING_TONE], AtEnd::Stop);
                }
                WifiAutoEvent::ConnectionFailed => {
                    info!("WiFi connection failed");
                    audio_player10_ref.play(
                        [CONNECTION_FAILED_TONE, SILENCE_40MS, CONNECTION_FAILED_TONE],
                        AtEnd::Stop,
                    );
                }
            }
            Ok(())
        })
        .await?;

    info!("WiFi connected");
    audio_player8.play(
        [WIFI_CONNECTED_TONE, SILENCE_40MS, WIFI_CONNECTED_TONE],
        AtEnd::Stop,
    );

    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;
    static CLOCK_SYNC_STATIC: ClockSyncStaticRp = ClockSyncRp::new_static();

    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    // First tick confirms successful time sync.
    let first_tick = clock_sync.wait_for_tick().await;
    let (first_hours, first_minutes, first_seconds) = h12_m_s(&first_tick.local_time);
    audio_player8.play([TIME_SYNCED_TONE], AtEnd::Stop);
    info!(
        "Time synced: {:02}:{:02}:{:02}",
        first_hours, first_minutes, first_seconds
    );

    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        &mut timezone_flash_block,
        |clock_ui_event| async move {
            match clock_ui_event {
                ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                    audio_player8.play([HH_MM_TICK_TONE], AtEnd::Stop);
                    info!("hh:mm {:02}:{:02}", hours, minutes);
                }
                ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                    audio_player8.play([MM_SS_TICK_TONE], AtEnd::Stop);
                    info!("mm:ss {:02}:{:02}", minutes, seconds);
                }
                ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                    audio_player8.play([MODE_MM_SS_TONE], AtEnd::Stop);
                    info!("edit offset {:02}:{:02}", hours, minutes);
                }
            }
            Ok(())
        },
    )
    .await
}
