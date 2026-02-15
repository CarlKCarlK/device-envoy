#![allow(missing_docs)]
//! Resample a compiled-in clip from 22.05 kHz to narrowband 8 kHz.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - Button -> GP13 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::Result;
use device_envoy::audio_player::{
    AtEnd, Gain, NARROWBAND_8000_HZ, VOICE_22050_HZ, Volume, audio_clip, audio_player,
    resampled_type,
};
use device_envoy::button::{Button, PressedTo};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioResamplePlayer {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: NARROWBAND_8000_HZ,
        max_volume: Volume::percent(50),
    }
}

audio_clip! {
    Nasa {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/nasa_22k.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static NASA_8K: resampled_type!(Nasa, NARROWBAND_8000_HZ) = Nasa::audio_clip()
        .with_resampled()
        .with_gain(Gain::percent(25));

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);
    let audio_resample_player =
        AudioResamplePlayer::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    info!(
        "NASA source clip: {} Hz, {} samples",
        Nasa::SAMPLE_RATE_HZ,
        Nasa::SAMPLE_COUNT
    );
    info!(
        "NASA resampled clip: {} Hz, {} samples",
        NASA_8K.sample_rate_hz(),
        NASA_8K.sample_count()
    );
    info!("Press GP13 button to play the resampled narrowband clip");

    loop {
        button.wait_for_press().await;
        audio_resample_player.play([&NASA_8K], AtEnd::Stop);
    }
}
