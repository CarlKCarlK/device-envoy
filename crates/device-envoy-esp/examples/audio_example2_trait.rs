#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;

use device_envoy_esp::{
    Result,
    audio_player::{
        AtEnd, AudioPlayer, Gain, Playable, SilenceClip, VOICE_22050_HZ, Volume, audio_player,
        pcm_clip,
    },
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start, tone,
};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayerGpio21 {
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

async fn play_nasa_with_runtime_volume<AudioPlayerType>(
    audio_player: &AudioPlayerType,
    button: &mut ButtonEsp<'_>,
) where
    AudioPlayerType: AudioPlayer<{ VOICE_22050_HZ }>,
{
    type PlayableRef = &'static dyn Playable<{ VOICE_22050_HZ }>;

    const fn ms(milliseconds: u64) -> StdDuration {
        StdDuration::from_millis(milliseconds)
    }

    const SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;
    const NASA: PlayableRef = &Nasa::adpcm_clip();
    const GAP: PlayableRef = &SilenceClip::new(ms(80));
    const CHIME: PlayableRef = &tone!(880, SAMPLE_RATE_HZ, ms(100)).with_gain(Gain::percent(20));
    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];

    loop {
        button.wait_for_press().await;
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
        audio_player
            .set_volume(<AudioPlayerType as AudioPlayer<{ VOICE_22050_HZ }>>::INITIAL_VOLUME);
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

    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let audio_player_gpio21 =
        AudioPlayerGpio21::new(p.GPIO21, p.GPIO11, p.GPIO12, p.I2S0, p.DMA_CH0, spawner)?;

    play_nasa_with_runtime_volume(audio_player_gpio21, &mut button).await;
    core::future::pending().await
}
