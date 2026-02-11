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
use device_envoy::audio_player::{AtEnd, AudioClipBuf, Gain, Volume, audio_player, VOICE_22050_HZ};
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
        din_pin: PIN_8,
        bclk_pin: PIN_9,
        lrc_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        max_volume: Volume::percent(50),
        initial_volume: Volume::percent(100),
    }
}

const NASA_SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;
const NASA_BYTES: usize = 184_320;
const NASA_SAMPLES: usize = NASA_BYTES / 2;

const fn nasa_clip_s16le() -> AudioClipBuf<NASA_SAMPLE_RATE_HZ, NASA_SAMPLES> {
    assert!(NASA_BYTES % 2 == 0, "nasa clip byte length must be even");
    assert!(
        NASA_SAMPLES * 2 == NASA_BYTES,
        "nasa sample count must match byte length"
    );
    let bytes: &[u8; NASA_BYTES] = include_bytes!("../deldir/nasa_22k.s16");
    AudioClipBuf::from_s16le_bytes(bytes)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    // todo0 we shouldn't use "clip" it should be audio_clip
    static NASA: AudioClipBuf<NASA_SAMPLE_RATE_HZ, NASA_SAMPLES> =
        nasa_clip_s16le().with_gain(Gain::percent(25));
    static TONE_A4: samples_ms! { AudioPlayer8, 500 } =
        AudioPlayer8::tone(440).with_gain(Gain::percent(25));
    static SILENCE_100MS: samples_ms! { AudioPlayer8, 100 } = AudioPlayer8::silence();

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // TODO0 should pins or PIO come first? (moved from previous audio.rs revision)
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH0, spawner)?;

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        NASA.sample_count(),
        NASA.sample_count() * 2
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
