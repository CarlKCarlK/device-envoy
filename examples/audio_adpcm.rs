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
use core::time::Duration;

use device_envoy::audio_player::{
    AtEnd, Gain, VOICE_22050_HZ, Volume, adpcm_clip, audio_player, pcm_clip,
};
use device_envoy::{Result, silence};
use embassy_executor::Spawner;
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
    Jabber22kAdpcm {
        file: "data/audio/jabberwocky_22k_adpcm.wav",
    }
}

pcm_clip! {
    Jabber22kPcm {
        source_sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/jabberwocky_22k.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const JABBER_ADPCM: &AudioPlayer8Playable = &Jabber22kAdpcm::adpcm_clip();
    // todo00 shouldn't silence and tone be Adpcm Clips.
    // todo00 should samples_ms_type have a pcm in name (may no longer apply)
    const GAP_100MS: &AudioPlayer8Playable =
        &silence!(AudioPlayer8::SAMPLE_RATE_HZ, Duration::from_millis(100));

    // todo00 should we add a block_play
    // todo00 should clips know their duration.
    let p = embassy_rp::init(Default::default());
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    // Section 1: Play external ADPCM clip directly.
    audio_player8.play([JABBER_ADPCM, GAP_100MS], AtEnd::Stop);
    audio_player8.wait_until_stopped().await;

    // Section 2: Read external PCM clip, change gain, encode to ADPCM, and play.
    const JABBER_ADPCM256: &AudioPlayer8Playable =
        &Jabber22kPcm::adpcm256_clip_from(Jabber22kPcm::pcm_clip().with_gain(Gain::percent(50)));
    audio_player8.play([JABBER_ADPCM256], AtEnd::Stop);
    audio_player8.wait_until_stopped().await;

    // Section 3: Read external ADPCM clip, decode to PCM, and play.
    const JABBER_PCM: &AudioPlayer8Playable = &Jabber22kAdpcm::pcm_clip();
    audio_player8.play([JABBER_PCM], AtEnd::Stop);
    audio_player8.wait_until_stopped().await;

    // Section 4: Read external ADPCM clip, decode to PCM, change gain, encode to ADPCM, and play.
    const JABBER_ADPCM_GAIN: &AudioPlayer8Playable =
        &Jabber22kAdpcm::adpcm_clip_from(Jabber22kAdpcm::pcm_clip().with_gain(Gain::percent(60)));
    audio_player8.play([JABBER_ADPCM_GAIN], AtEnd::Stop);
    audio_player8.wait_until_stopped().await;

    // Section 5: Read ADPCM, change volume in one step, save as static ADPCM, and play.
    const JABBER_ADPCM_GAIN_STEP: &AudioPlayer8Playable =
        &Jabber22kAdpcm::with_gain(Gain::percent(35));
    audio_player8.play([JABBER_ADPCM_GAIN_STEP], AtEnd::Stop);
    audio_player8.wait_until_stopped().await;

    // read adpcm, convert to pcm, change gain and sample rate to 8K,   convert back to adpcm, and play.

    pending().await
}
