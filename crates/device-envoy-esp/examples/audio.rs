#![allow(missing_docs)]
//! MAX98357A sample playback example using ESP32-C6 I2S.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GPIO21
//! - Bit clock pin (`BCLK`) -> GPIO22
//! - Word select pin (`LRC` / `LRCLK`) -> GPIO23
//! - Button -> GPIO6 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp32::{
    Result,
    audio_player::{
        AtEnd, Gain, SilenceClip, VOICE_22050_HZ, Volume, audio_player, pcm_clip,
    },
    button::{Button, PressedTo},
    init_and_start, tone,
};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayerGpio21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO22,
        word_select_pin: GPIO23,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(50),
        initial_volume: Volume::percent(100),
    }
}

pcm_clip! {
    Nasa {
        file: "data/audio/nasa_22k.s16",
        source_sample_rate_hz: VOICE_22050_HZ,
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

    const NASA: &AudioPlayerGpio21Playable = &Nasa::pcm_clip().with_gain(Gain::percent(25));
    const TONE_A4: &AudioPlayerGpio21Playable = &tone!(
        440,
        AudioPlayerGpio21::SAMPLE_RATE_HZ,
        StdDuration::from_millis(500)
    )
    .with_gain(Gain::percent(25));
    const SILENCE_100MS: &AudioPlayerGpio21Playable =
        &SilenceClip::new(StdDuration::from_millis(100));

    let mut button = Button::new(p.GPIO6, PressedTo::Ground);
    let audio_player_gpio21 = AudioPlayerGpio21::new(
        p.GPIO21, p.GPIO22, p.GPIO23, p.I2S0, p.DMA_CH0, spawner,
    )?;

    info!("I2S ready: GPIO21 DIN, GPIO22 BCLK, GPIO23 LRCLK");
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        Nasa::PCM_SAMPLE_COUNT,
        Nasa::PCM_SAMPLE_COUNT * 2
    );
    info!("Button on GPIO6 starts playback");

    loop {
        button.wait_for_press().await;
        audio_player_gpio21.play([TONE_A4, SILENCE_100MS, TONE_A4], AtEnd::Loop);
        info!("Started static slice playback");
        for percent in [80, 60, 40, 20, 200] {
            audio_player_gpio21.set_volume(Volume::percent(percent));
            info!("Runtime volume set to {}%", percent);
            Timer::after(Duration::from_secs(1)).await;
        }
        audio_player_gpio21.stop();
        Timer::after(Duration::from_secs(1)).await;
        audio_player_gpio21.set_volume(AudioPlayerGpio21::INITIAL_VOLUME);
        audio_player_gpio21.play([NASA], AtEnd::Stop);
    }
}
