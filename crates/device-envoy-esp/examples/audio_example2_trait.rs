#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_core::{
    audio_player::{AtEnd, AudioPlayer, Gain, Playable, SilenceClip, VOICE_22050_HZ, Volume},
    button::Button,
};
use device_envoy_esp::{
    Result,
    audio_player::{audio_player, pcm_clip},
    button::{ButtonEsp, PressedTo},
    init_and_start, tone,
};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayer21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO11,
        word_select_pin: GPIO12,
        sample_rate_hz: VOICE_22050_HZ,
        max_clips: 8,
        max_volume: Volume::spinal_tap(11),
        initial_volume: Volume::spinal_tap(5),
    }
}

pcm_clip! {
    Nasa {
        file: "data/audio/nasa_22k.s16",
        source_sample_rate_hz: VOICE_22050_HZ,
    }
}

async fn play_nasa_with_runtime_volume(
    audio_player: &impl AudioPlayer<VOICE_22050_HZ>,
    button: &mut impl Button,
) -> ! {
    const fn ms(milliseconds: u64) -> StdDuration {
        StdDuration::from_millis(milliseconds)
    }

    type PlayableRef = &'static dyn Playable<VOICE_22050_HZ>;
    const NASA: PlayableRef = &Nasa::adpcm_clip();
    const GAP: PlayableRef = &SilenceClip::new(ms(80));
    const CHIME: PlayableRef = &tone!(880, VOICE_22050_HZ, ms(100)).with_gain(Gain::percent(20));
    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];
    let initial_volume = audio_player.volume();

    loop {
        audio_player.play([CHIME, NASA, GAP], AtEnd::Loop);

        for volume_percent in VOLUME_STEPS_PERCENT {
            match select(
                button.wait_for_press(),
                Timer::after(Duration::from_secs(1)),
            )
            .await
            {
                Either::First(()) => break,
                Either::Second(()) => audio_player.set_volume(Volume::percent(volume_percent)),
            }
        }

        audio_player.stop();
        audio_player.set_volume(initial_volume);
        info!("Press the button to play again.");
        button.wait_for_press().await;
    }
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
    let audio_player21 = AudioPlayer21::new(p.GPIO21, p.GPIO11, p.GPIO12, p.I2S0, p.DMA_CH0, spawner)?;

    play_nasa_with_runtime_volume(audio_player21, &mut button).await
}
