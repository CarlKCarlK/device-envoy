#![allow(missing_docs)]
//! MAX98357A sample playback example using PIO I²S.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use defmt::info;
use device_envoy::Result;
use device_envoy::audio_player::{AtEnd, Gain, VOICE_22050_HZ, Volume, audio_player, pcm_clip};
use device_envoy::button::{Button, PressedTo};
use device_envoy::{silence, tone};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// TODO in the future think about adding concatenation, fade in and out, trim, and resample.
audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
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

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const NASA: &AudioPlayer8Playable = &Nasa::pcm_clip().with_gain(Gain::percent(25));
    const TONE_A4: &AudioPlayer8Playable = &tone!(
        440,
        AudioPlayer8::SAMPLE_RATE_HZ,
        StdDuration::from_millis(500)
    )
    .with_gain(Gain::percent(25));
    const SILENCE_100MS: &AudioPlayer8Playable =
        &silence!(AudioPlayer8::SAMPLE_RATE_HZ, StdDuration::from_millis(100));

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    info!(
        "I²S ready: GP8 data pin (DIN), GP9 bit clock pin (BCLK), GP10 word select pin (LRC/LRCLK)"
    );
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        Nasa::PCM_SAMPLE_COUNT,
        Nasa::PCM_SAMPLE_COUNT * 2
    );
    info!("Button on GP13 starts playback");

    loop {
        button.wait_for_press().await;
        audio_player8.play([TONE_A4, SILENCE_100MS, TONE_A4], AtEnd::Loop);
        info!("Started static slice playback");
        for percent in [80, 60, 40, 20, 200] {
            audio_player8.set_volume(Volume::percent(percent));
            info!("Runtime volume set to {}%", percent);
            Timer::after(Duration::from_secs(1)).await;
        }
        audio_player8.stop();
        Timer::after(Duration::from_secs(1)).await;
        audio_player8.set_volume(AudioPlayer8::INITIAL_VOLUME);
        audio_player8.play([NASA], AtEnd::Stop);
    }
}
