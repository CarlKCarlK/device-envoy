#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    audio_player::{
        audio_player, AtEnd, AudioPlayer, Playable, SilenceClip, Volume, VOICE_22050_HZ,
    },
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start, tone, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(any(
    feature = "esp32c2",
    feature = "esp32c3",
    feature = "esp32c6",
    feature = "esp32h2",
    feature = "esp32s3"
))]
audio_player! {
    AudioPlayer21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO11,
        word_select_pin: GPIO12,
        sample_rate_hz: VOICE_22050_HZ,
        dma: DMA_CH0,
        max_volume: Volume::percent(50),
    }
}

#[cfg(any(feature = "esp32", feature = "esp32s2"))]
audio_player! {
    AudioPlayer21 {
        data_pin: GPIO21,
        bit_clock_pin: GPIO4,
        word_select_pin: GPIO5,
        sample_rate_hz: VOICE_22050_HZ,
        dma: DMA_I2S0,
        max_volume: Volume::percent(50),
    }
}

const SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;

fn play_mary_phrase(audio_player: &impl AudioPlayer<SAMPLE_RATE_HZ>) {
    type PlayableRef = &'static dyn Playable<SAMPLE_RATE_HZ>;

    const REST: PlayableRef = &SilenceClip::new(StdDuration::from_millis(80));
    const NOTE_DURATION: StdDuration = StdDuration::from_millis(220);
    const NOTE_E4: PlayableRef = &tone!(330, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_D4: PlayableRef = &tone!(294, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_C4: PlayableRef = &tone!(262, SAMPLE_RATE_HZ, NOTE_DURATION);

    audio_player.play(
        [
            NOTE_E4, REST, NOTE_D4, REST, NOTE_C4, REST, NOTE_D4, REST, NOTE_E4, REST, NOTE_E4,
            REST, NOTE_E4,
        ],
        AtEnd::Stop,
    );
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

    #[cfg(any(
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s3"
    ))]
    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    #[cfg(any(feature = "esp32", feature = "esp32s2"))]
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);

    #[cfg(any(
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s3"
    ))]
    let audio_player21 =
        AudioPlayer21::new(p.GPIO21, p.GPIO11, p.GPIO12, p.I2S0, p.DMA_CH0, spawner)?;
    #[cfg(any(feature = "esp32", feature = "esp32s2"))]
    let audio_player21 =
        AudioPlayer21::new(p.GPIO21, p.GPIO4, p.GPIO5, p.I2S0, p.DMA_I2S0, spawner)?;

    loop {
        play_mary_phrase(audio_player21);
        info!("Press the button to play again.");
        button.wait_for_press().await;
    }
}
