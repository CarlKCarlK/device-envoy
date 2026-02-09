#![allow(missing_docs)]
//! MAX98357A sample playback example using PIO I2S.
//!
//! Wiring:
//! - DIN  -> GP8
//! - BCLK -> GP9
//! - LRC  -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (queues playback)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::Result;
use device_envoy::button::{Button, PressedTo};
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::{DMA_CH0, PIN_8, PIN_9, PIN_10, PIO0};
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/audio_data.rs"));

const SAMPLE_RATE_HZ: u32 = 44_100;
const BIT_DEPTH_BITS: u32 = 16;
const AMPLITUDE: i16 = 8_000;
const SAMPLE_BUFFER_LEN: usize = 256;
const AUDIO_VEC_CAPACITY: usize = 8_192;
const AUDIO_VEC_START_SAMPLE: usize = 22_050;

type AudioCommandSignal = Signal<CriticalSectionRawMutex, AudioCommand>;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    static AUDIO_OUT_STATIC: AudioOutStatic = AudioOut::new_static();
    let audio_out = AudioOut::new(
        &AUDIO_OUT_STATIC,
        p.PIO0,
        p.DMA_CH0,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        spawner,
    )?;

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 44.1kHz mono s16le",
        AUDIO_SAMPLE_I16.len(),
        AUDIO_SAMPLE_I16.len() * 2
    );
    info!("Button on GP13 queues playback");

    let mut play_from_vec_next = false;
    loop {
        button.wait_for_press().await;

        if play_from_vec_next {
            let audio_sample_i16_vec = audio_sample_i16_vec();
            audio_out.play(AudioSamples::Vec(audio_sample_i16_vec), AtEnd::AtEnd);
            info!(
                "Queued Vec<i16> playback (start={}, len={})",
                AUDIO_VEC_START_SAMPLE,
                AUDIO_VEC_CAPACITY
            );
        } else {
            audio_out.play(AudioSamples::Static(&AUDIO_SAMPLE_I16), AtEnd::AtEnd);
            info!("Queued static slice playback");
        }

        play_from_vec_next = !play_from_vec_next;
    }
}

struct AudioOutStatic {
    command_signal: AudioCommandSignal,
}

impl AudioOutStatic {
    const fn new_static() -> Self {
        Self {
            command_signal: Signal::new(),
        }
    }
}

struct AudioOut<'a> {
    audio_out_static: &'a AudioOutStatic,
}

impl AudioOut<'_> {
    const fn new_static() -> AudioOutStatic {
        AudioOutStatic::new_static()
    }

    fn new(
        audio_out_static: &'static AudioOutStatic,
        pio0: Peri<'static, PIO0>,
        dma_ch0: Peri<'static, DMA_CH0>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
        spawner: Spawner,
    ) -> Result<Self> {
        let token = audio_out_task(audio_out_static, pio0, dma_ch0, pin_8, pin_9, pin_10);
        spawner.spawn(token).map_err(device_envoy::Error::TaskSpawn)?;

        Ok(Self { audio_out_static })
    }

    fn play(&self, audio_samples: AudioSamples, at_end: AtEnd) {
        self.audio_out_static
            .command_signal
            .signal(AudioCommand::Play { audio_samples, at_end });
    }
}

enum AudioCommand {
    Play {
        audio_samples: AudioSamples,
        at_end: AtEnd,
    },
}

enum AudioSamples {
    Static(&'static [i16]),
    Vec(Vec<i16, AUDIO_VEC_CAPACITY>),
}

impl AudioSamples {
    fn as_slice(&self) -> &[i16] {
        match self {
            Self::Static(audio_sample_i16) => audio_sample_i16,
            Self::Vec(audio_sample_i16) => audio_sample_i16.as_slice(),
        }
    }
}

#[allow(dead_code)]
enum AtEnd {
    Loop,
    AtEnd,
}

#[embassy_executor::task]
async fn audio_out_task(
    audio_out_static: &'static AudioOutStatic,
    pio0: Peri<'static, PIO0>,
    dma_ch0: Peri<'static, DMA_CH0>,
    pin_8: Peri<'static, PIN_8>,
    pin_9: Peri<'static, PIN_9>,
    pin_10: Peri<'static, PIN_10>,
) {
    let mut pio = Pio::new(pio0, device_envoy::pio_irqs::Pio0Irqs);
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        dma_ch0,
        pin_8,
        pin_9,
        pin_10,
        SAMPLE_RATE_HZ,
        BIT_DEPTH_BITS,
        &pio_i2s_out_program,
    );

    let _pio_i2s_out_program = pio_i2s_out_program;
    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    loop {
        let audio_command = audio_out_static.command_signal.wait().await;
        audio_out_static.command_signal.reset();

        match audio_command {
            AudioCommand::Play {
                audio_samples,
                at_end,
            } => {
                let audio_sample_i16 = audio_samples.as_slice();
                match at_end {
                    AtEnd::AtEnd => {
                        play_full_sample_once(&mut pio_i2s_out, audio_sample_i16, &mut sample_buffer)
                            .await;
                    }
                    AtEnd::Loop => loop {
                        play_full_sample_once(&mut pio_i2s_out, audio_sample_i16, &mut sample_buffer)
                            .await;
                    },
                }
            }
        }
    }
}

fn audio_sample_i16_vec() -> Vec<i16, AUDIO_VEC_CAPACITY> {
    let mut audio_sample_i16_vec = Vec::<i16, AUDIO_VEC_CAPACITY>::new();

    for sample_value_ref in AUDIO_SAMPLE_I16
        .iter()
        .skip(AUDIO_VEC_START_SAMPLE)
        .take(AUDIO_VEC_CAPACITY)
    {
        let sample_value = *sample_value_ref;
        assert!(audio_sample_i16_vec.push(sample_value).is_ok());
    }

    audio_sample_i16_vec
}

async fn play_full_sample_once(
    pio_i2s_out: &mut PioI2sOut<'static, PIO0, 0>,
    audio_sample_i16: &[i16],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    for audio_sample_chunk in audio_sample_i16.chunks(SAMPLE_BUFFER_LEN) {
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            let scaled_sample = ((i32::from(sample_value) * i32::from(AMPLITUDE)) / 32_768) as i16;
            *sample_buffer_slot = stereo_sample(scaled_sample);
        }

        sample_buffer[audio_sample_chunk.len()..].fill(stereo_sample(0));
        pio_i2s_out.write(sample_buffer).await;
    }
}

#[inline]
const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = sample as u16 as u32;
    (sample_bits << 16) | sample_bits
}
