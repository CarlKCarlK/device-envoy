#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use device_envoy::{
    Result,
    audio_player::{
        AtEnd, Gain, NARROWBAND_8000_HZ, VOICE_22050_HZ, Volume, audio_player, pcm_clip,
    },
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// To save memory, we use a lower sample rate.
audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: NARROWBAND_8000_HZ,
        max_volume: Volume::percent(50),
    }
}

// We resample each clip from the original 22KHz to the 8KHz sample rate of our audio player.
pcm_clip! {
    Digit0 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/0_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
    }
}

pcm_clip! {
    Digit1 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/1_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
    }
}

pcm_clip! {
    Digit2 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/2_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
    }
}

pcm_clip! {
    Nasa {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = example(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn example(spawner: Spawner) -> Result<Infallible> {
    // We read the compressed version of the digits.
    const DIGITS: [&AudioPlayer8Playable; 3] = [
        &Digit0::adpcm_clip(),
        &Digit1::adpcm_clip(),
        &Digit2::adpcm_clip(),
    ];

    // We read the uncompressed (PCM) NASA clip, change its loudness, and then convert it to compressed (ADPCM) format.
    const NASA: &AudioPlayer8Playable = &Nasa::pcm_clip()
        .with_gain(Gain::percent(25))
        .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();

    let p = embassy_rp::init(Default::default());
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    audio_player8.play([DIGITS[2], DIGITS[1], DIGITS[0], NASA], AtEnd::Stop);
    core::future::pending().await
}
