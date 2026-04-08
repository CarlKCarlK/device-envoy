#![allow(missing_docs)]
//! audio20k_one: one short 22.05 kHz clip, then stop.
//!
//! Purpose:
//! - Tail-repeat verification for `AtEnd::Stop`
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10

#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use defmt::info;
use device_envoy_rp::Result;
use device_envoy_rp::audio_player::{
    AtEnd, AudioPlayer as _, Volume, VOICE_22050_HZ, audio_player, pcm_clip,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(40),
    }
}

pcm_clip! {
    Digit0 {
        file: "data/audio/0_22050.s16",
        source_sample_rate_hz: VOICE_22050_HZ,
    }
}

const CLIP: &AudioPlayer8Playable = &Digit0::pcm_clip();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    info!("phase: before_play sample_rate_hz={}", AudioPlayer8::SAMPLE_RATE_HZ);
    audio_player8.play([CLIP], AtEnd::Stop);
    info!("phase: after_play_call");
    audio_player8.wait_until_stopped().await;
    info!("phase: wait_until_stopped_returned");
    info!("If audio stopped cleanly with no repeated tail, this board is good.");

    pending().await
}
