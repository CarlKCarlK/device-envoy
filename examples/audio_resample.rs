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
    AtEnd, AudioClipSource, Gain, NARROWBAND_8000_HZ, VOICE_22050_HZ, Volume, audio_player,
    pcm_clip, resampled_type,
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

pcm_clip! {
    Nasa {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/nasa_22k.s16",
    }
}

pcm_clip! {
    Digit0 {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/0_22050.s16",
    }
}

pcm_clip! {
    Digit1 {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/1_22050.s16",
    }
}

pcm_clip! {
    Digit2 {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/2_22050.s16",
    }
}

pcm_clip! {
    Digit3 {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/3_22050.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    // TODO00 can do static array again?
    static DIGIT0_8K: resampled_type!(Digit0, NARROWBAND_8000_HZ) =
        Digit0::pcm_clip().with_resampled();
    static DIGIT1_8K: resampled_type!(Digit1, NARROWBAND_8000_HZ) =
        Digit1::pcm_clip().with_resampled();
    static DIGIT2_8K: resampled_type!(Digit2, NARROWBAND_8000_HZ) =
        Digit2::pcm_clip().with_resampled();
    static DIGIT3_8K: resampled_type!(Digit3, NARROWBAND_8000_HZ) =
        Digit3::pcm_clip().with_resampled();
    // TODO00 shorten this type?
    let digits: [&'static dyn AudioClipSource<{ AudioResamplePlayer::SAMPLE_RATE_HZ }>; 4] =
        [&DIGIT0_8K, &DIGIT1_8K, &DIGIT2_8K, &DIGIT3_8K];
    static NASA_8K: resampled_type!(Nasa, NARROWBAND_8000_HZ) = Nasa::pcm_clip()
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
        AudioResamplePlayerPcmClip::SAMPLE_RATE_HZ,
        NASA_8K.sample_count()
    );
    info!("Press GP13 button to play countdown 3,2,1,0 then NASA (8 kHz)");

    loop {
        button.wait_for_press().await;
        audio_resample_player.play(
            [digits[3], digits[2], digits[1], digits[0], &NASA_8K],
            AtEnd::Stop,
        );
    }
}
