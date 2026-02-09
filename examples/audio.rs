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
use embassy_executor::Spawner;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use {defmt_rtt as _, panic_probe as _};

const SAMPLE_RATE_HZ: u32 = 44_100;
const BIT_DEPTH_BITS: u32 = 16;
const AUDIO_SAMPLE_BYTES: &[u8] =
    include_bytes!("data/audio/computers_in_control_mono_s16le_44100.raw");
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

    let mut pio = Pio::new(p.PIO1, device_envoy::pio_irqs::Pio1Irqs);
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

    if (AUDIO_SAMPLE_BYTES.len() % 2) != 0 {
        return Err(device_envoy::Error::FormatError);
    }
    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    info!("I2S ready on GP8 (DIN), GP9 (BCLK), GP10 (LRC)");
    info!("Loaded sample: {} bytes, 44.1kHz mono s16le", AUDIO_SAMPLE_BYTES.len());
    info!("Button on GP13 plays the clip");

    loop {
        button.wait_for_press().await;
        info!("Playing sample...");
        play_full_sample_once(&mut pio_i2s_out, &mut sample_buffer).await;
        info!("Playback finished");
    }
}

async fn play_full_sample_once(
    pio_i2s_out: &mut PioI2sOut<'static, embassy_rp::peripherals::PIO1, 0>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    let mut audio_sample_byte_index = 0_usize;

    let mut sample_index = 0;
    while audio_sample_byte_index < AUDIO_SAMPLE_BYTES.len() {
        while sample_index < SAMPLE_BUFFER_LEN && audio_sample_byte_index < AUDIO_SAMPLE_BYTES.len()
        {
            let sample_value = i16::from_le_bytes([
                AUDIO_SAMPLE_BYTES[audio_sample_byte_index],
                AUDIO_SAMPLE_BYTES[audio_sample_byte_index + 1],
            ]);
            audio_sample_byte_index += 2;
            let scaled_sample = ((i32::from(sample_value) * i32::from(AMPLITUDE)) / 32_767) as i16;
            sample_buffer[sample_index] = stereo_sample(scaled_sample);
            sample_index += 1;
        }

        while sample_index < SAMPLE_BUFFER_LEN {
            sample_buffer[sample_index] = stereo_sample(0);
            sample_index += 1;
        }

        pio_i2s_out.write(sample_buffer).await;
        sample_index = 0;
    }
}

#[inline]
fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = u32::from(sample as u16);
    (sample_bits << 16) | sample_bits
}
