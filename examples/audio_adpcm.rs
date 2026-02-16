#![allow(missing_docs)]
//! Minimal ADPCM playback example using direct PIO I2S output.
//!
//! Wiring (MAX98357A):
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD -> 3V3

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::audio_player::{Volume, scale};
use device_envoy::pio_irqs::Pio0Irqs;
use embassy_executor::Spawner;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

const BITS_PER_SAMPLE: u32 = 16;
const I2S_BUFFER_LEN: usize = 256;
const NASA_22K_ADPCM_IMA_WAV: &[u8] = include_bytes!("data/audio/nasa_22k_adpcm_ima.wav");
type AdpcmResult<T> = core::result::Result<T, &'static str>;

const IMA_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

const IMA_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

struct ImaAdpcmWav<'a> {
    sample_rate_hz: u32,
    block_align: usize,
    samples_per_block: usize,
    data_chunk: &'a [u8],
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let err = inner_main().await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main() -> AdpcmResult<Infallible> {
    let p = embassy_rp::init(Default::default());
    let ima_adpcm_wav = parse_ima_adpcm_wav(NASA_22K_ADPCM_IMA_WAV)?;

    let mut pio = embassy_rp::pio::Pio::new(p.PIO0, Pio0Irqs);
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        p.DMA_CH0,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        ima_adpcm_wav.sample_rate_hz,
        BITS_PER_SAMPLE,
        &pio_i2s_out_program,
    );

    let _pio_i2s_out_program = pio_i2s_out_program;

    info!(
        "Starting ADPCM playback: {} Hz, block_align={}, samples_per_block={}",
        ima_adpcm_wav.sample_rate_hz, ima_adpcm_wav.block_align, ima_adpcm_wav.samples_per_block
    );

    play_ima_adpcm_wav_once(&mut pio_i2s_out, &ima_adpcm_wav).await?;
    info!("ADPCM playback completed");

    loop {
        Timer::after_secs(1).await;
    }
}

async fn play_ima_adpcm_wav_once(
    pio_i2s_out: &mut PioI2sOut<'static, PIO0, 0>,
    ima_adpcm_wav: &ImaAdpcmWav<'_>,
) -> AdpcmResult<()> {
    let volume = Volume::percent(10);
    let mut sample_buffer = [0_u32; I2S_BUFFER_LEN];
    let mut sample_buffer_len = 0usize;

    for adpcm_block in ima_adpcm_wav
        .data_chunk
        .chunks_exact(ima_adpcm_wav.block_align)
    {
        if adpcm_block.len() < 4 {
            return Err("IMA ADPCM block too small");
        }

        let mut predictor_i32 = read_i16_le(adpcm_block, 0)? as i32;
        let mut step_index_i32 = adpcm_block[2] as i32;
        if !(0..=88).contains(&step_index_i32) {
            return Err("IMA ADPCM step index out of range");
        }

        let predictor_sample_i16 = scale(predictor_i32 as i16, volume);
        sample_buffer[sample_buffer_len] = stereo_sample(predictor_sample_i16);
        sample_buffer_len += 1;
        if sample_buffer_len == I2S_BUFFER_LEN {
            pio_i2s_out.write(&sample_buffer).await;
            sample_buffer_len = 0;
        }

        for adpcm_byte in &adpcm_block[4..] {
            let low_nibble = adpcm_byte & 0x0F;
            let high_nibble = adpcm_byte >> 4;

            for adpcm_nibble in [low_nibble, high_nibble] {
                let decoded_sample_i16 =
                    decode_ima_nibble(adpcm_nibble, &mut predictor_i32, &mut step_index_i32);
                let volume_adjusted_sample_i16 = scale(decoded_sample_i16, volume);
                sample_buffer[sample_buffer_len] = stereo_sample(volume_adjusted_sample_i16);
                sample_buffer_len += 1;

                if sample_buffer_len == I2S_BUFFER_LEN {
                    pio_i2s_out.write(&sample_buffer).await;
                    sample_buffer_len = 0;
                }
            }
        }
    }

    if sample_buffer_len != 0 {
        sample_buffer[sample_buffer_len..].fill(stereo_sample(0));
        pio_i2s_out.write(&sample_buffer).await;
    }

    Ok(())
}

fn parse_ima_adpcm_wav(wav_bytes: &[u8]) -> AdpcmResult<ImaAdpcmWav<'_>> {
    if wav_bytes.len() < 12 {
        return Err("WAV file too small");
    }
    if &wav_bytes[0..4] != b"RIFF" {
        return Err("Missing RIFF header");
    }
    if &wav_bytes[8..12] != b"WAVE" {
        return Err("Missing WAVE header");
    }

    let mut chunk_offset = 12usize;
    let mut sample_rate_hz = None;
    let mut block_align = None;
    let mut samples_per_block = None;
    let mut data_chunk = None;

    while chunk_offset + 8 <= wav_bytes.len() {
        let chunk_id = &wav_bytes[chunk_offset..chunk_offset + 4];
        let chunk_size = read_u32_le(wav_bytes, chunk_offset + 4)? as usize;
        let chunk_data_start = chunk_offset + 8;
        let Some(chunk_data_end) = chunk_data_start.checked_add(chunk_size) else {
            return Err("WAV chunk size overflow");
        };
        if chunk_data_end > wav_bytes.len() {
            return Err("WAV chunk overruns file");
        }

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err("fmt chunk too small");
            }

            let audio_format = read_u16_le(wav_bytes, chunk_data_start)?;
            let channels = read_u16_le(wav_bytes, chunk_data_start + 2)?;
            let parsed_sample_rate_hz = read_u32_le(wav_bytes, chunk_data_start + 4)?;
            let parsed_block_align = read_u16_le(wav_bytes, chunk_data_start + 12)? as usize;
            let bits_per_sample = read_u16_le(wav_bytes, chunk_data_start + 14)?;

            if audio_format != 0x0011 {
                return Err("Expected IMA ADPCM WAV format");
            }
            if channels != 1 {
                return Err("Expected mono IMA ADPCM WAV");
            }
            if bits_per_sample != 4 {
                return Err("Expected 4-bit IMA ADPCM");
            }
            if parsed_block_align < 5 {
                return Err("IMA ADPCM block_align too small");
            }

            let parsed_samples_per_block = if chunk_size >= 22 {
                read_u16_le(wav_bytes, chunk_data_start + 18)? as usize
            } else {
                derive_ima_samples_per_block(parsed_block_align)?
            };

            if parsed_samples_per_block != derive_ima_samples_per_block(parsed_block_align)? {
                return Err("Unexpected IMA ADPCM samples_per_block");
            }

            sample_rate_hz = Some(parsed_sample_rate_hz);
            block_align = Some(parsed_block_align);
            samples_per_block = Some(parsed_samples_per_block);
        } else if chunk_id == b"data" {
            data_chunk = Some(&wav_bytes[chunk_data_start..chunk_data_end]);
        }

        let padded_chunk_size = chunk_size + (chunk_size & 1);
        let Some(next_chunk_offset) = chunk_data_start.checked_add(padded_chunk_size) else {
            return Err("WAV chunk traversal overflow");
        };
        chunk_offset = next_chunk_offset;
    }

    let sample_rate_hz = sample_rate_hz.ok_or("Missing fmt chunk")?;
    let block_align = block_align.ok_or("Missing fmt chunk block_align")?;
    let samples_per_block = samples_per_block.ok_or("Missing fmt chunk samples_per_block")?;
    let data_chunk = data_chunk.ok_or("Missing data chunk")?;

    if data_chunk.len() % block_align != 0 {
        return Err("data chunk is not block aligned");
    }

    Ok(ImaAdpcmWav {
        sample_rate_hz,
        block_align,
        samples_per_block,
        data_chunk,
    })
}

fn derive_ima_samples_per_block(block_align: usize) -> AdpcmResult<usize> {
    let Some(codes_per_block) = block_align.checked_sub(4) else {
        return Err("IMA ADPCM block_align underflow");
    };
    Ok((codes_per_block * 2) + 1)
}

fn decode_ima_nibble(adpcm_nibble: u8, predictor_i32: &mut i32, step_index_i32: &mut i32) -> i16 {
    let step = IMA_STEP_TABLE[*step_index_i32 as usize];
    let mut delta = step >> 3;

    if (adpcm_nibble & 0x01) != 0 {
        delta += step >> 2;
    }
    if (adpcm_nibble & 0x02) != 0 {
        delta += step >> 1;
    }
    if (adpcm_nibble & 0x04) != 0 {
        delta += step;
    }

    if (adpcm_nibble & 0x08) != 0 {
        *predictor_i32 -= delta;
    } else {
        *predictor_i32 += delta;
    }

    *predictor_i32 = (*predictor_i32).clamp(i16::MIN as i32, i16::MAX as i32);
    *step_index_i32 += IMA_INDEX_TABLE[adpcm_nibble as usize];
    *step_index_i32 = (*step_index_i32).clamp(0, 88);

    *predictor_i32 as i16
}

const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits_u16 = sample as u16 as u32;
    (sample_bits_u16 << 16) | sample_bits_u16
}

fn read_u16_le(bytes: &[u8], byte_offset: usize) -> AdpcmResult<u16> {
    let Some(end_offset) = byte_offset.checked_add(2) else {
        return Err("read_u16_le offset overflow");
    };
    if end_offset > bytes.len() {
        return Err("read_u16_le out of bounds");
    }
    Ok(u16::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
    ]))
}

fn read_i16_le(bytes: &[u8], byte_offset: usize) -> AdpcmResult<i16> {
    let Some(end_offset) = byte_offset.checked_add(2) else {
        return Err("read_i16_le offset overflow");
    };
    if end_offset > bytes.len() {
        return Err("read_i16_le out of bounds");
    }
    Ok(i16::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
    ]))
}

fn read_u32_le(bytes: &[u8], byte_offset: usize) -> AdpcmResult<u32> {
    let Some(end_offset) = byte_offset.checked_add(4) else {
        return Err("read_u32_le offset overflow");
    };
    if end_offset > bytes.len() {
        return Err("read_u32_le out of bounds");
    }
    Ok(u32::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
        bytes[byte_offset + 2],
        bytes[byte_offset + 3],
    ]))
}
