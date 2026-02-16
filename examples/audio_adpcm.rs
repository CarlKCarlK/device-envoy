#![allow(missing_docs)]
//! Minimal ADPCM playback example using the `adpcm_player` module.
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

use device_envoy::adpcm_player::{AtEnd, Volume, adpcm_clip, adpcm_player};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

adpcm_player! {
    AdpcmPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: device_envoy::audio_player::VOICE_22050_HZ,
        pio: PIO0,
        dma: DMA_CH0,
        max_clips: 4,
        max_volume: Volume::MAX,
        initial_volume: Volume::percent(10),
    }
}

adpcm_clip! {
    Nasa22kAdpcm {
        sample_rate_hz: device_envoy::audio_player::VOICE_22050_HZ,
        file: "data/audio/nasa_22k_adpcm.wav",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> device_envoy::Result<Infallible> {
    static NASA_22K_ADPCM: Nasa22kAdpcm::AdpcmClip = Nasa22kAdpcm::adpcm_clip();

    let p = embassy_rp::init(Default::default());
    let adpcm_player8 = AdpcmPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;
    adpcm_player8.play([&NASA_22K_ADPCM], AtEnd::Stop);

    pending().await
}
