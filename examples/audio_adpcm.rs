#![allow(missing_docs)]
//! Minimal ADPCM playback example using the unified `audio_player` module.
//!
//! Wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD -> 3V3

#![no_std]
#![no_main]

use core::convert::Infallible;
use core::future::pending;

use device_envoy::audio_player::{
    AtEnd, Gain, VOICE_22050_HZ, Volume, adpcm_clip, audio_player, pcm_clip,
};
use device_envoy::{Result, samples_ms_type};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: crate::VOICE_22050_HZ,
        pio: PIO0,
        dma: DMA_CH0,
        max_clips: 4,
        max_volume: Volume::MAX,
        initial_volume: Volume::percent(10),
    }
}

adpcm_clip! {
    Nasa22kAdpcm {
        file: "data/audio/nasa_22k_adpcm.wav",
    }
}

pcm_clip! {
    Nasa22kPcm {
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
    static NASA_22K_ADPCM: Nasa22kAdpcm::AdpcmClip = Nasa22kAdpcm::adpcm_clip();
    // todo00 shouldn't silence and tone be Adpcm Clips.
    // todo00 should samples_ms_type have a pcm in name
    static GAP_100MS: samples_ms_type! { AudioPlayer8, 100 } = AudioPlayer8::silence();

    // todo00 should we add a block_play
    // todo00 should clips know their duration.
    let p = embassy_rp::init(Default::default());
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;
    audio_player8.play([&NASA_22K_ADPCM, &GAP_100MS], AtEnd::Stop);
    Timer::after(Duration::from_secs(4)).await;

    static NASA_22K_ADPCM_CONVERTED: Nasa22kPcm::AdpcmClip =
        Nasa22kPcm::adpcm_clip_from(Nasa22kPcm::pcm_clip().with_gain(Gain::percent(100)));
    audio_player8.play([&NASA_22K_ADPCM_CONVERTED], AtEnd::Stop);
    Timer::after(Duration::from_secs(4)).await;

    static NASA_22K_PCM_FROM_EXTERNAL_ADPCM: Nasa22kAdpcm::PcmClip = Nasa22kAdpcm::pcm_clip();
    audio_player8.play([&NASA_22K_PCM_FROM_EXTERNAL_ADPCM], AtEnd::Stop);
    Timer::after(Duration::from_secs(4)).await;

    // read adpcm, convert to PCM, change volume, save as static adpcm (and play)
    // read adpcm, change volume in one step, save as static adpcm (and play)
    // read adpcm, change sample rate in one step, save as static adpcm (and play)

    pending().await
}
