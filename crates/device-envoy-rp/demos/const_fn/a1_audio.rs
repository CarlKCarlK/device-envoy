//! Compile raw audio into compact, playback-ready clips with `const fn` and macros.
//!
//! Each `pcm_clip!` invocation includes signed 16-bit PCM, derives its sample
//! count from the file length, resamples it from 22.05 kHz to 8 kHz, and
//! derives the corresponding ADPCM storage size. The `const` declarations
//! below materialize only the compressed clips used by the firmware.
//!
//! Press the button to hear the NASA clip.
//!
//! Wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - Button -> GP13 to GND
//todo0 add to Esp?

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy_rp::{
    Result,
    audio_player::{
        AtEnd, AudioPlayer as _, Gain, NARROWBAND_8000_HZ, VOICE_22050_HZ, Volume, audio_player,
        pcm_clip,
    },
    button::{Button as _, ButtonRp, PressedTo},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8K {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: NARROWBAND_8000_HZ,
        max_volume: Volume::percent(50),
    }
}

pcm_clip! {
    Nasa {
        file: "../../../device-envoy-examples-rp/examples/data/audio/nasa_22k.s16",
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: AudioPlayer8K::SAMPLE_RATE_HZ,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const NASA: &AudioPlayer8KPlayable = &Nasa::pcm_clip()
        .with_gain(Gain::percent(25))
        .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();

    let peripherals = embassy_rp::init(Default::default());
    let mut button = ButtonRp::new(peripherals.PIN_13, PressedTo::Ground);
    let audio_player8k = AudioPlayer8K::new(
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
        audio_player8k.play([NASA], AtEnd::Stop);
    }
}
