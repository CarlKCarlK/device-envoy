//! Compile raw audio into playback-ready PCM with `const fn` and macros.
//!
//! Each `pcm_clip!` invocation includes signed 16-bit PCM, derives its sample
//! count from the file length, validates the input, and constructs exactly
//! sized clip storage.
//!
//! Press the button to hear the NASA clip.
//!
//! Wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - Button -> GP13 to GND
//todo add to Esp?

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy_rp::{
    Result,
    audio_player::{AtEnd, AudioPlayer as _, Volume, audio_player, pcm_clip},
    button::{Button as _, ButtonRp, PressedTo},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer22K {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: 22_050,
        max_volume: Volume::percent(50),
    }
}

pcm_clip! {
    Nasa {
        file: "../../../device-envoy-examples-rp/examples/data/audio/nasa_22k.s16",
        source_sample_rate_hz: 22_050,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const NASA: &AudioPlayer22KPlayable = &Nasa::pcm_clip();

    let peripherals = embassy_rp::init(Default::default());
    let mut button = ButtonRp::new(peripherals.PIN_13, PressedTo::Ground);
    let audio_player22k = AudioPlayer22K::new(
        peripherals.PIN_8,
        peripherals.PIN_9,
        peripherals.PIN_10,
        peripherals.PIO0,
        peripherals.DMA_CH0,
        spawner,
    )?;

    info!("Press GP13 to play NASA");

    loop {
        button.wait_for_press().await;
        audio_player22k.play([NASA], AtEnd::Stop);
    }
}
