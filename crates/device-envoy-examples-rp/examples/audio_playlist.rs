#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use defmt::info;
use device_envoy_rp::{
    Result,
    audio_player::{
        AtEnd, AudioPlayer as _, Gain, NARROWBAND_8000_HZ, SilenceClip, VOICE_22050_HZ,
        audio_player, pcm_clip,
    },
    button::{ButtonRp, PressedTo},
    tone,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayerPin8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Nasa {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayerPin8::SAMPLE_RATE_HZ,
    }
}

use device_envoy_rp::button::Button as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);

    const CHIME: &AudioPlayerPin8Playable =
        &tone!(880, AudioPlayerPin8::SAMPLE_RATE_HZ, ms(100)).with_gain(Gain::percent(20));
    const GAP: &AudioPlayerPin8Playable = &SilenceClip::new(ms(500));
    const NASA: &AudioPlayerPin8Playable = &Nasa::adpcm_clip();

    let audio_player_pin8 =
        AudioPlayerPin8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    loop {
        button.wait_for_press().await;
        info!("Button pressed; playing playlist in background");
        audio_player_pin8.play([CHIME, GAP, NASA], AtEnd::Stop);
    }
}

const fn ms(milliseconds: u64) -> StdDuration {
    StdDuration::from_millis(milliseconds)
}
