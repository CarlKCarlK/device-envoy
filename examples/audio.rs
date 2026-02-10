#![allow(missing_docs)]
//! MAX98357A sample playback example using PIO I2S.
//!
//! Wiring:
//! - DIN  -> GP8
//! - BCLK -> GP9
//! - LRC  -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::Result;
use device_envoy::audio_player::{AtEnd, AudioClipN, Gain, Volume, audio_player};
use device_envoy::button::{Button, PressedTo};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// TODO00 rename nasa clip (may no longer apply)
include!(concat!(env!("OUT_DIR"), "/nasa_clip.rs"));
// Rebuild the source clip (s16le mono raw) with:
// ffmpeg -i input.wav -ac 1 -ar 22050 -f s16le examples/data/audio/computers_in_control_mono_s16le_22050.raw
// TODO00 min language of concatenation, fade in and out?
// TODO00 think about moving some of the 3 constants (rate, bit depth, buffer len) into the macro with defaults
// TODO00 make the macro documentation look good with a generated type.
// TODO00 does the macro support vis
// TODO00 If you want one small extra “pro” touch: add a fade-out on stop (even 5–10 ms) to avoid clicks when you stop mid-waveform. But that’s optional.
// TODO00 should with_gain be an extension method?
//
audio_player! {
    AudioPlayer8 {
        din_pin: PIN_8,
        bclk_pin: PIN_9,
        lrc_pin: PIN_10,
        max_volume: Volume::percent(25),
        initial_volume: Volume::percent(100),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static NASA_CLIP: NasaClip = nasa_clip().with_gain(Gain::percent(25));
    static TONE_A4: AudioClipN<{ AudioPlayer8::samples_ms(500) }> =
        AudioPlayer8::tone(440).with_gain(Gain::percent(25));
    static SILENCE_100MS: AudioClipN<{ AudioPlayer8::samples_ms(100) }> = AudioPlayer8::silence();

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // TODO0 should pins or PIO come first? (moved from previous audio.rs revision)
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH0, spawner)?;

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        NASA_CLIP.sample_count(),
        NASA_CLIP.sample_count() * 2
    );
    info!("Button on GP13 starts playback");

    // TODO0 amplitude 8_000 is arbitrary (may no longer apply)
    loop {
        button.wait_for_press().await;
        audio_player8.play(
            [
                &TONE_A4,
                &SILENCE_100MS,
                &TONE_A4,
            ],
            AtEnd::Loop,
        );
        info!("Started static slice playback");
        for percent in [80, 60, 40, 20, 200] {
            audio_player8.set_volume(Volume::percent(percent));
            info!("Runtime volume set to {}%", percent);
            Timer::after(Duration::from_secs(1)).await;
        }
        audio_player8.stop();
        Timer::after(Duration::from_secs(1)).await;
        audio_player8.set_volume(AudioPlayer8::INITIAL_VOLUME);
        audio_player8.play([&NASA_CLIP], AtEnd::Stop);
    }
}
