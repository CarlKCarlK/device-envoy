#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_core::audio_player::{
    AtEnd, AudioPlayer, Gain, Playable, Volume, NARROWBAND_8000_HZ, VOICE_22050_HZ,
};
use device_envoy_esp::{
    audio_player::{audio_player, pcm_clip},
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start,
    init_and_start::rmt_mode,
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayer21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO11,
        word_select_pin: GPIO12,
        sample_rate_hz: NARROWBAND_8000_HZ,
        max_volume: Volume::percent(50),
    }
}

pcm_clip! {
    Digit0 {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../device-envoy-rp/examples/data/audio/0_22050.s16"
        ),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Digit1 {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../device-envoy-rp/examples/data/audio/1_22050.s16"
        ),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Digit2 {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../device-envoy-rp/examples/data/audio/2_22050.s16"
        ),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Nasa {
        file: "data/audio/nasa_22k.s16",
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

fn play_resampled_countdown(audio_player: &impl AudioPlayer<NARROWBAND_8000_HZ>) {
    type PlayableRef = &'static dyn Playable<NARROWBAND_8000_HZ>;

    const DIGITS: [PlayableRef; 3] = [
        &Digit0::adpcm_clip(),
        &Digit1::adpcm_clip(),
        &Digit2::adpcm_clip(),
    ];
    const NASA: PlayableRef = &Nasa::pcm_clip()
        .with_gain(Gain::percent(25))
        .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();

    audio_player.play([DIGITS[2], DIGITS[1], DIGITS[0], NASA], AtEnd::Stop);
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);

    let audio_player21 =
        AudioPlayer21::new(p.GPIO21, p.GPIO11, p.GPIO12, p.I2S0, p.DMA_CH0, spawner)?;

    loop {
        play_resampled_countdown(audio_player21);
        info!("Press the button to play again.");
        button.wait_for_press().await;
    }
}
