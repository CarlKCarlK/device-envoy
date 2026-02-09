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
use embassy_rp::dma::{AnyChannel, Channel};
#[cfg(feature = "pico2")]
use embassy_rp::peripherals::PIO2;
use embassy_rp::peripherals::{PIN_8, PIN_9, PIN_10, PIO0, PIO1};
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/audio_data.rs"));
// Rebuild the source clip (s16le mono raw) with:
// ffmpeg -i input.wav -ac 1 -ar 22050 -f s16le examples/data/audio/computers_in_control_mono_s16le_22050.raw

const SAMPLE_RATE_HZ: u32 = 22_050;
const BIT_DEPTH_BITS: u32 = 16;
const AMPLITUDE: i16 = 8_000;
const SAMPLE_BUFFER_LEN: usize = 256;

type AudioCommandSignal = Signal<CriticalSectionRawMutex, AudioCommand>;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // TODO0 should pins or PIO come first?
    static AUDIO_OUT_STATIC: AudioOutStatic = AudioOut::new_static();
    let audio_out = AudioOut::new(
        &AUDIO_OUT_STATIC,
        p.PIO1,
        p.DMA_CH7,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        spawner,
    )?;

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 22.05kHz mono s16le",
        AUDIO_SAMPLE_I16.len(),
        AUDIO_SAMPLE_I16.len() * 2
    );
    info!("Button on GP13 queues playback");

    loop {
        button.wait_for_press().await;
        audio_out.play(&AUDIO_SAMPLE_I16, AtEnd::AtEnd);
        info!("Queued static slice playback");
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

trait AudioOutPio: Instance {
    type Irqs: embassy_rp::interrupt::typelevel::Binding<
            <Self as Instance>::Interrupt,
            embassy_rp::pio::InterruptHandler<Self>,
        >;

    fn irqs() -> Self::Irqs;

    fn spawn_task(
        spawner: Spawner,
        audio_out_static: &'static AudioOutStatic,
        pio: Peri<'static, Self>,
        dma: Peri<'static, AnyChannel>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
    ) -> Result<()>;
}

impl AudioOutPio for PIO0 {
    type Irqs = device_envoy::pio_irqs::Pio0Irqs;

    fn irqs() -> Self::Irqs {
        device_envoy::pio_irqs::Pio0Irqs
    }

    fn spawn_task(
        spawner: Spawner,
        audio_out_static: &'static AudioOutStatic,
        pio: Peri<'static, Self>,
        dma: Peri<'static, AnyChannel>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
    ) -> Result<()> {
        let token = audio_out_pio0_task(audio_out_static, pio, dma, pin_8, pin_9, pin_10);
        spawner.spawn(token).map_err(device_envoy::Error::TaskSpawn)
    }
}

impl AudioOutPio for PIO1 {
    type Irqs = device_envoy::pio_irqs::Pio1Irqs;

    fn irqs() -> Self::Irqs {
        device_envoy::pio_irqs::Pio1Irqs
    }

    fn spawn_task(
        spawner: Spawner,
        audio_out_static: &'static AudioOutStatic,
        pio: Peri<'static, Self>,
        dma: Peri<'static, AnyChannel>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
    ) -> Result<()> {
        let token = audio_out_pio1_task(audio_out_static, pio, dma, pin_8, pin_9, pin_10);
        spawner.spawn(token).map_err(device_envoy::Error::TaskSpawn)
    }
}

#[cfg(feature = "pico2")]
impl AudioOutPio for PIO2 {
    type Irqs = device_envoy::pio_irqs::Pio2Irqs;

    fn irqs() -> Self::Irqs {
        device_envoy::pio_irqs::Pio2Irqs
    }

    fn spawn_task(
        spawner: Spawner,
        audio_out_static: &'static AudioOutStatic,
        pio: Peri<'static, Self>,
        dma: Peri<'static, AnyChannel>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
    ) -> Result<()> {
        let token = audio_out_pio2_task(audio_out_static, pio, dma, pin_8, pin_9, pin_10);
        spawner.spawn(token).map_err(device_envoy::Error::TaskSpawn)
    }
}

impl AudioOut<'_> {
    const fn new_static() -> AudioOutStatic {
        AudioOutStatic::new_static()
    }

    fn new<PIO: AudioOutPio, DMA: Channel>(
        audio_out_static: &'static AudioOutStatic,
        pio: Peri<'static, PIO>,
        dma: Peri<'static, DMA>,
        pin_8: Peri<'static, PIN_8>,
        pin_9: Peri<'static, PIN_9>,
        pin_10: Peri<'static, PIN_10>,
        spawner: Spawner,
    ) -> Result<Self> {
        PIO::spawn_task(
            spawner,
            audio_out_static,
            pio,
            dma.into(),
            pin_8,
            pin_9,
            pin_10,
        )?;

        Ok(Self { audio_out_static })
    }

    fn play(&self, audio_sample_i16: &'static [i16], at_end: AtEnd) {
        self.audio_out_static
            .command_signal
            .signal(AudioCommand::Play {
                audio_sample_i16,
                at_end,
            });
    }
}

enum AudioCommand {
    Play {
        audio_sample_i16: &'static [i16],
        at_end: AtEnd,
    },
}

#[allow(dead_code)]
enum AtEnd {
    Loop,
    AtEnd,
}

#[embassy_executor::task]
async fn audio_out_pio0_task(
    audio_out_static: &'static AudioOutStatic,
    pio: Peri<'static, PIO0>,
    dma: Peri<'static, AnyChannel>,
    pin_8: Peri<'static, PIN_8>,
    pin_9: Peri<'static, PIN_9>,
    pin_10: Peri<'static, PIN_10>,
) -> ! {
    audio_out_task_impl::<PIO0>(audio_out_static, pio, dma, pin_8, pin_9, pin_10).await
}

#[embassy_executor::task]
async fn audio_out_pio1_task(
    audio_out_static: &'static AudioOutStatic,
    pio: Peri<'static, PIO1>,
    dma: Peri<'static, AnyChannel>,
    pin_8: Peri<'static, PIN_8>,
    pin_9: Peri<'static, PIN_9>,
    pin_10: Peri<'static, PIN_10>,
) -> ! {
    audio_out_task_impl::<PIO1>(audio_out_static, pio, dma, pin_8, pin_9, pin_10).await
}

#[cfg(feature = "pico2")]
#[embassy_executor::task]
async fn audio_out_pio2_task(
    audio_out_static: &'static AudioOutStatic,
    pio: Peri<'static, PIO2>,
    dma: Peri<'static, AnyChannel>,
    pin_8: Peri<'static, PIN_8>,
    pin_9: Peri<'static, PIN_9>,
    pin_10: Peri<'static, PIN_10>,
) -> ! {
    audio_out_task_impl::<PIO2>(audio_out_static, pio, dma, pin_8, pin_9, pin_10).await
}

async fn audio_out_task_impl<PIO: AudioOutPio>(
    audio_out_static: &'static AudioOutStatic,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, AnyChannel>,
    pin_8: Peri<'static, PIN_8>,
    pin_9: Peri<'static, PIN_9>,
    pin_10: Peri<'static, PIN_10>,
) -> ! {
    let mut pio = Pio::new(pio, PIO::irqs());
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        dma,
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
                audio_sample_i16,
                at_end,
            } => match at_end {
                AtEnd::AtEnd => {
                    play_full_sample_once(&mut pio_i2s_out, audio_sample_i16, &mut sample_buffer)
                        .await;
                }
                AtEnd::Loop => loop {
                    play_full_sample_once(&mut pio_i2s_out, audio_sample_i16, &mut sample_buffer)
                        .await;
                },
            },
        }
    }
}

async fn play_full_sample_once<PIO: Instance>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_sample_i16: &[i16],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    for audio_sample_chunk in audio_sample_i16.chunks(SAMPLE_BUFFER_LEN) {
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            // TODO0 should we preprocess all this?
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
