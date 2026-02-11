#![allow(missing_docs)]
//! MAX98357A sample playback example using PIO I2S.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::Result;
use device_envoy::audio_player::{AtEnd, Gain, VOICE_22050_HZ, Volume, audio_clip, audio_player};
use device_envoy::button::{Button, PressedTo};
use device_envoy::samples_ms;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// Rebuild the source clip (s16le mono raw) with:
// ffmpeg -i input.wav -ac 1 -ar 22050 -f s16le examples/data/audio/computers_in_control_mono_s16le_22050.raw
// TODO00 min language of concatenation, fade in and out?
// TODO00 think about moving some of the 3 constants (rate, bit depth, buffer len) into the macro with defaults
// TODO00 make the macro documentation look good with a generated type.
// TODO00 does the macro support vis
// TODO00 If you want one small extra “pro” touch: add a fade-out on stop (even 5–10 ms) to avoid clicks when you stop mid-waveform. But that’s optional.
// TODO00 verify that it can play sound while doing other things (like blinking an LED or reading a button) without stuttering
audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(50),
        initial_volume: Volume::percent(100),
    }
}

audio_clip! {
    Nasa {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/nasa_22k.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static NASA: Nasa::AudioClip = Nasa::audio_clip().with_gain(Gain::percent(25));
    static TONE_A4: samples_ms! { AudioPlayer8, 500 } =
        AudioPlayer8::tone(440).with_gain(Gain::percent(25));
    static SILENCE_100MS: samples_ms! { AudioPlayer8, 100 } = AudioPlayer8::silence();

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    info!(
        "I2S ready: GP8 data pin (DIN), GP9 bit clock pin (BCLK), GP10 word select pin (LRC/LRCLK)"
    );
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        Nasa::AudioClip::SAMPLE_COUNT,
        Nasa::AudioClip::SAMPLE_COUNT * 2
    );
    info!("Button on GP13 starts playback");

    loop {
        button.wait_for_press().await;
        audio_player8.play([&TONE_A4, &SILENCE_100MS, &TONE_A4], AtEnd::Loop);
        info!("Started static slice playback");
        for percent in [80, 60, 40, 20, 200] {
            audio_player8.set_volume(Volume::percent(percent));
            info!("Runtime volume set to {}%", percent);
            Timer::after(Duration::from_secs(1)).await;
        }
        audio_player8.stop();
        Timer::after(Duration::from_secs(1)).await;
        audio_player8.set_volume(AudioPlayer8::INITIAL_VOLUME);
        audio_player8.play([&NASA], AtEnd::Stop);
    }
}
