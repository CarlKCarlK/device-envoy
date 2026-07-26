//! Compile raw audio without Device Envoy's audio helper macros.
//!
//! This is the expanded counterpart to `a1_audio`. It reads the source length,
//! derives the PCM and ADPCM storage sizes, parses signed 16-bit samples,
//! resamples them, applies gain, and compresses them entirely in const
//! evaluation.
//!
//! Press the button to hear the NASA clip.
//!
//! Wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - Button -> GP13 to GND

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy_rp::{
    Result,
    audio_player::{
        __adpcm_data_len_for_pcm_samples, __audio_player_play, __pcm_clip_from_samples,
        __resample_pcm_clip, __resampled_sample_count, AtEnd, AudioPlayerRp, AudioPlayerStatic,
        Gain, NARROWBAND_8000_HZ, PcmClipBuf, Playable, VOICE_22050_HZ, Volume, device_loop,
    },
    button::{Button as _, ButtonRp, PressedTo},
};
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    peripherals::{DMA_CH0, PIN_8, PIN_9, PIN_10, PIO0},
};
use {defmt_rtt as _, panic_probe as _};

const MAX_CLIPS: usize = 1;
const SAMPLE_RATE_HZ: u32 = NARROWBAND_8000_HZ;

static AUDIO_PLAYER_STATIC: AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> =
    AudioPlayerRp::new_static_with_max_volume(Volume::percent(50));

const NASA_BYTES: &[u8] =
    include_bytes!("../../../device-envoy-examples-rp/examples/data/audio/nasa_22k.s16");
const NASA_SOURCE_SAMPLE_COUNT: usize = NASA_BYTES.len() / 2;
const NASA_SAMPLE_COUNT: usize =
    __resampled_sample_count(NASA_SOURCE_SAMPLE_COUNT, VOICE_22050_HZ, SAMPLE_RATE_HZ);
const NASA_ADPCM_DATA_LEN: usize = __adpcm_data_len_for_pcm_samples(NASA_SAMPLE_COUNT);

const NASA: &dyn Playable<SAMPLE_RATE_HZ> = &__resample_pcm_clip::<
    VOICE_22050_HZ,
    NASA_SOURCE_SAMPLE_COUNT,
    SAMPLE_RATE_HZ,
    NASA_SAMPLE_COUNT,
>(read_s16le::<NASA_SOURCE_SAMPLE_COUNT>(NASA_BYTES))
.with_gain(Gain::percent(25))
.with_adpcm::<NASA_ADPCM_DATA_LEN>();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let peripherals = embassy_rp::init(Default::default());
    let mut button = ButtonRp::new(peripherals.PIN_13, PressedTo::Ground);

    let task_token = audio_player_task(
        peripherals.PIO0,
        peripherals.DMA_CH0,
        peripherals.PIN_8,
        peripherals.PIN_9,
        peripherals.PIN_10,
    )?;
    spawner.spawn(task_token);

    info!("Press GP13 to play NASA");

    loop {
        button.wait_for_press().await;
        __audio_player_play(&AUDIO_PLAYER_STATIC, [NASA], AtEnd::Stop);
    }
}

const fn read_s16le<const SAMPLE_COUNT: usize>(
    bytes: &[u8],
) -> PcmClipBuf<VOICE_22050_HZ, SAMPLE_COUNT> {
    assert!(
        bytes.len() == SAMPLE_COUNT * 2,
        "s16le requires exactly two bytes per sample"
    );

    let mut samples = [0_i16; SAMPLE_COUNT];
    let mut sample_index = 0;
    while sample_index < SAMPLE_COUNT {
        let byte_index = sample_index * 2;
        samples[sample_index] = i16::from_le_bytes([bytes[byte_index], bytes[byte_index + 1]]);
        sample_index += 1;
    }
    __pcm_clip_from_samples(samples)
}

#[embassy_executor::task]
async fn audio_player_task(
    pio: Peri<'static, PIO0>,
    dma: Peri<'static, DMA_CH0>,
    data_pin: Peri<'static, PIN_8>,
    bit_clock_pin: Peri<'static, PIN_9>,
    word_select_pin: Peri<'static, PIN_10>,
) -> ! {
    device_loop::<MAX_CLIPS, SAMPLE_RATE_HZ, PIO0, DMA_CH0, PIN_8, PIN_9, PIN_10>(
        &AUDIO_PLAYER_STATIC,
        pio,
        dma,
        data_pin,
        bit_clock_pin,
        word_select_pin,
    )
    .await
}
