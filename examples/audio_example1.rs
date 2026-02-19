#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use device_envoy::{
    Result,
    audio_player::{AtEnd, SilenceClip, VOICE_22050_HZ, Volume, audio_player},
    tone,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

// Generate `AudioPlayer8`, a struct type with the specified configuration.
audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(25),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = example(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn example(spawner: Spawner) -> Result<Infallible> {
    // Keep original note/rest timing. tone! applies internal attack/release shaping.
    const REST: &AudioPlayer8Playable = &SilenceClip::new(StdDuration::from_millis(80));
    // Define each note as a static clip of a sine wave.
    const SAMPLE_RATE_HZ: u32 = AudioPlayer8::SAMPLE_RATE_HZ;
    const NOTE_DURATION: StdDuration = StdDuration::from_millis(220);
    const NOTE_E4: &AudioPlayer8Playable = &tone!(330, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_D4: &AudioPlayer8Playable = &tone!(294, SAMPLE_RATE_HZ, NOTE_DURATION);
    const NOTE_C4: &AudioPlayer8Playable = &tone!(262, SAMPLE_RATE_HZ, NOTE_DURATION);

    let p = embassy_rp::init(Default::default());
    // Create an `AudioPlayer8` instance with the specified pins and resources.
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    audio_player8.play(
        [
            NOTE_E4, REST, NOTE_D4, REST, NOTE_C4, REST, NOTE_D4, REST, NOTE_E4, REST, NOTE_E4,
            REST, NOTE_E4,
        ],
        AtEnd::Stop,
    );

    // Audio plays in the background while we can do other things here, like blink an LED or read a button.
    core::future::pending().await
}
