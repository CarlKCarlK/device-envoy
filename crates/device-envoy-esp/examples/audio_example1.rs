#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    Result, audio_player::AtEnd, audio_player::SilenceClip, audio_player::VOICE_22050_HZ,
    audio_player::Volume, audio_player::audio_player, init_and_start, tone,
};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayerGpio21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO22,
        word_select_pin: GPIO23,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(25),
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

    const REST: &AudioPlayerGpio21Playable = &SilenceClip::new(StdDuration::from_millis(80));
    const SAMPLE_RATE_HZ: u32 = AudioPlayerGpio21::SAMPLE_RATE_HZ;
    const NOTE_DURATION: StdDuration = StdDuration::from_millis(220);
    const NOTE_E4: &AudioPlayerGpio21Playable = &tone!(330, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_D4: &AudioPlayerGpio21Playable = &tone!(294, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_C4: &AudioPlayerGpio21Playable = &tone!(262, SAMPLE_RATE_HZ, NOTE_DURATION);

    let audio_player_gpio21 = AudioPlayerGpio21::new(
        p.GPIO21, p.GPIO22, p.GPIO23, p.I2S0, p.DMA_CH0, spawner,
    )?;
    audio_player_gpio21.play(
        [
            NOTE_E4, REST, NOTE_D4, REST, NOTE_C4, REST, NOTE_D4, REST, NOTE_E4, REST, NOTE_E4,
            REST, NOTE_E4,
        ],
        AtEnd::Stop,
    );

    core::future::pending().await
}
