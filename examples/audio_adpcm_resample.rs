#![allow(missing_docs)]
//! ADPCM resample demo: decode external ADPCM, apply gain, resample to 8 kHz,
//! re-encode to ADPCM, and play at 8 kHz.
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
use device_envoy::audio_player::{
    AdpcmClipBuf, AtEnd, Gain, NARROWBAND_8000_HZ, Volume, adpcm_clip,
    adpcm_data_len_for_pcm_samples, audio_player, resampled_sample_count,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8k {
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
    Nasa22kAdpcm {
        file: "data/audio/nasa_22k_adpcm.wav",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static NASA_8K_ADPCM: Nasa8kAdpcm = Nasa22kAdpcm::pcm_clip()
        .with_gain(Gain::percent(60))
        .with_resampled::<NARROWBAND_8000_HZ, NASA_8K_SAMPLE_COUNT>()
        .with_adpcm::<NASA_8K_ADPCM_LEN>();

    let p = embassy_rp::init(Default::default());

    const NASA_8K_SAMPLE_COUNT: usize = resampled_sample_count(
        Nasa22kAdpcm::SAMPLE_COUNT,
        Nasa22kAdpcm::SAMPLE_RATE_HZ,
        NARROWBAND_8000_HZ,
    );
    const NASA_8K_ADPCM_LEN: usize = adpcm_data_len_for_pcm_samples(NASA_8K_SAMPLE_COUNT);
    type Nasa8kAdpcm = AdpcmClipBuf<NARROWBAND_8000_HZ, NASA_8K_ADPCM_LEN>;

    let audio_player8k =
        AudioPlayer8k::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    audio_player8k.play([&NASA_8K_ADPCM], AtEnd::Stop);

    pending().await
}
