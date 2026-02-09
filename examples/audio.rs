#![allow(missing_docs)]
//! MAX98357A sample playback example using PIO I2S.
//!
//! Wiring:
//! - DIN  -> GP8
//! - BCLK -> GP9
//! - LRC  -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (toggles tone on/off)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::Result;
use device_envoy::button::{Button, PressedTo};
use device_envoy::led_strip::LedStripPio;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use {defmt_rtt as _, panic_probe as _};

include!(concat!(env!("OUT_DIR"), "/audio_data.rs"));

const SAMPLE_RATE_HZ: u32 = 44_100;
const BIT_DEPTH_BITS: u32 = 16;
const AMPLITUDE: i16 = 8_000;
const SAMPLE_BUFFER_LEN: usize = 256;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);
    let _ = spawner;

    let pio = p.PIO0;
    let mut pio = pio_new(pio);
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        p.DMA_CH0,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        SAMPLE_RATE_HZ,
        BIT_DEPTH_BITS,
        &pio_i2s_out_program,
    );

    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!(
        "Loaded sample: {} samples ({} bytes), 44.1kHz mono s16le",
        AUDIO_SAMPLE_I16.len(),
        AUDIO_SAMPLE_I16.len() * 2
    );
    info!("Button on GP13 plays the clip");

    loop {
        button.wait_for_press().await;
        info!("Playing sample...");
        play_full_sample_once(&mut pio_i2s_out, &AUDIO_SAMPLE_I16, &mut sample_buffer).await;
        info!("Playback finished");
    }
}

fn pio_new<PioInstance: LedStripPio>(pio: Peri<'static, PioInstance>) -> Pio<'static, PioInstance> {
    Pio::new(pio, PioInstance::irqs())
}

async fn play_full_sample_once<PioInstance: Instance>(
    pio_i2s_out: &mut PioI2sOut<'static, PioInstance, 0>,
    audio_sample_i16: &[i16],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    let mut audio_sample_i16 = audio_sample_i16.iter().copied();

    loop {
        let mut written_samples = 0_usize;

        for sample_buffer_slot in sample_buffer.iter_mut() {
            if let Some(sample_value) = audio_sample_i16.next() {
                let scaled_sample =
                    ((i32::from(sample_value) * i32::from(AMPLITUDE)) / 32_767) as i16;
                *sample_buffer_slot = stereo_sample(scaled_sample);
                written_samples += 1;
            } else {
                *sample_buffer_slot = stereo_sample(0);
            }
        }

        if written_samples == 0 {
            break;
        }

        pio_i2s_out.write(sample_buffer).await;
    }
}

#[inline]
fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = u32::from(sample as u16);
    (sample_bits << 16) | sample_bits
}
