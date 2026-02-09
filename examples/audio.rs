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
use device_envoy::audio_player::{AtEnd, audio_player};
use device_envoy::button::{Button, PressedTo};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/audio_data.rs"));
// Rebuild the source clip (s16le mono raw) with:
// ffmpeg -i input.wav -ac 1 -ar 22050 -f s16le examples/data/audio/computers_in_control_mono_s16le_22050.raw
// TODO00 rename audio player
// TODO00 min language of tones, silence, concatenation, volume???
// TODO00 do pio and dma in macro
// TODO00 use same AtEnd as servo_player
// TODO00 preprocess samples at compile time
// TODO00 think about moving some of the 4 constants (rate, bit depth, amplitude, buffer len) into the macro with defaults
// TODO00 be sure new play commands (and stop) stops current playback immediately and doesn't just queue at the end of the current sequence
// TODO00 make the macro documentation look good with a generated type.
// TODO00 does the macro support vis

audio_player! {
    AudioPlayer8 {
        din_pin: PIN_8,
        bclk_pin: PIN_9,
        lrc_pin: PIN_10,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // TODO0 should pins or PIO come first? (moved from previous audio.rs revision)
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH0, spawner)?;

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        AUDIO_SAMPLE_I16.len(),
        AUDIO_SAMPLE_I16.len() * 2
    );
    info!("Button on GP13 starts playback");

    loop {
        button.wait_for_press().await;
        audio_player8.play([AUDIO_SAMPLE_I16.as_slice()], AtEnd::AtEnd);
        info!("Started static slice playback");
        // wait for 1 second
        Timer::after(Duration::from_secs(1)).await;
        audio_player8.stop();
        Timer::after(Duration::from_secs(1)).await;
        audio_player8.play([AUDIO_SAMPLE_I16.as_slice()], AtEnd::AtEnd);
    }
}
