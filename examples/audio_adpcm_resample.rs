#![allow(missing_docs)]
//! ADPCM demo using external Jabberwocky audio at 8 kHz.
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

use device_envoy::Result;
use device_envoy::audio_player::{AtEnd, NARROWBAND_8000_HZ, Volume, adpcm_clip, audio_player};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8K {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: NARROWBAND_8000_HZ,
        pio: PIO0,
        dma: DMA_CH0,
        max_clips: 2,
        max_volume: Volume::MAX,
        initial_volume: Volume::percent(25),
    }
}

adpcm_clip! {
    Jabber8kAdpcm {
        file: "data/audio/jabberwocky_22k_adpcm.wav",
        target_sample_rate_hz: AudioPlayer8K::SAMPLE_RATE_HZ,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const JABBER_8K_ADPCM: &AudioPlayer8KPlayable = &Jabber8kAdpcm::adpcm_clip();

    let p = embassy_rp::init(Default::default());

    let audio_player8k =
        AudioPlayer8K::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    audio_player8k.play([JABBER_8K_ADPCM], AtEnd::Stop);

    pending().await
}
