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
use core::future::pending;

use defmt::info;
use device_envoy::audio_player::{scale, Volume};
use device_envoy::pio_irqs::Pio0Irqs;
use embassy_executor::Spawner;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use {defmt_rtt as _, panic_probe as _};

const BITS_PER_SAMPLE: u32 = 16;
const I2S_BUFFER_LEN: usize = 256;
type AdpcmResult<T> = core::result::Result<T, &'static str>;

const ADPCM_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

const ADPCM_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
    449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
    9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

struct AdpcmClip<T: ?Sized = [u8]> {
    sample_rate_hz: u32,
    block_align: usize,
    samples_per_block: usize,
    data: T,
}

type AdpcmClipBuf<const DATA_LEN: usize> = AdpcmClip<[u8; DATA_LEN]>;

impl<T: ?Sized> AdpcmClip<T> {
    const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    const fn block_align(&self) -> usize {
        self.block_align
    }

    const fn samples_per_block(&self) -> usize {
        self.samples_per_block
    }
}

impl AdpcmClip {
    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<const DATA_LEN: usize> AdpcmClipBuf<DATA_LEN> {
    const fn new(
        sample_rate_hz: u32,
        block_align: usize,
        samples_per_block: usize,
        data: [u8; DATA_LEN],
    ) -> Self {
        Self {
            sample_rate_hz,
            block_align,
            samples_per_block,
            data,
        }
    }

    const fn as_adpcm_clip(&self) -> &AdpcmClip {
        self
    }
}

struct ParsedAdpcmWav {
    sample_rate_hz: u32,
    block_align: usize,
    samples_per_block: usize,
    data_chunk_start: usize,
    data_chunk_len: usize,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let err = inner_main().await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main() -> AdpcmResult<Infallible> {
    static NASA_22K_ADPCM: AdpcmClipBuf<{ nasa_22k_adpcm_data_len() }> =
        nasa_22k_adpcm();

    let p = embassy_rp::init(Default::default());
    let adpcm_clip = NASA_22K_ADPCM.as_adpcm_clip();

    let mut pio = embassy_rp::pio::Pio::new(p.PIO0, Pio0Irqs);
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        p.DMA_CH0,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        adpcm_clip.sample_rate_hz(),
        BITS_PER_SAMPLE,
        &pio_i2s_out_program,
    );

    let _pio_i2s_out_program = pio_i2s_out_program;

    info!(
        "Starting ADPCM playback: {} Hz, block_align={}, samples_per_block={}",
        adpcm_clip.sample_rate_hz(),
        adpcm_clip.block_align(),
        adpcm_clip.samples_per_block()
    );

    play_adpcm_clip_once(&mut pio_i2s_out, adpcm_clip).await?;
    info!("ADPCM playback completed");

    pending().await
}

async fn play_adpcm_clip_once(
    pio_i2s_out: &mut PioI2sOut<'static, PIO0, 0>,
    adpcm_clip: &AdpcmClip,
) -> AdpcmResult<()> {
    let volume = Volume::percent(10);
    let mut sample_buffer = [0_u32; I2S_BUFFER_LEN];
    let mut sample_buffer_len = 0usize;

    for adpcm_block in adpcm_clip.data().chunks_exact(adpcm_clip.block_align()) {
        if adpcm_block.len() < 4 {
            return Err("ADPCM block too small");
        }

        let mut predictor_i32 = read_i16_le(adpcm_block, 0)? as i32;
        let mut step_index_i32 = adpcm_block[2] as i32;
        if !(0..=88).contains(&step_index_i32) {
            return Err("ADPCM step index out of range");
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
                    decode_adpcm_nibble(adpcm_nibble, &mut predictor_i32, &mut step_index_i32);
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

const fn nasa_22k_adpcm_data_len() -> usize {
    let wav_bytes = include_bytes!("data/audio/nasa_22k_adpcm.wav");
    parse_adpcm_wav_header(wav_bytes).data_chunk_len
}

const fn nasa_22k_adpcm() -> AdpcmClipBuf<{ nasa_22k_adpcm_data_len() }> {
    let wav_bytes = include_bytes!("data/audio/nasa_22k_adpcm.wav");
    let parsed_adpcm_wav = parse_adpcm_wav_header(wav_bytes);

    let mut data = [0u8; nasa_22k_adpcm_data_len()];
    let mut data_index = 0usize;
    while data_index < data.len() {
        data[data_index] = wav_bytes[parsed_adpcm_wav.data_chunk_start + data_index];
        data_index += 1;
    }

    AdpcmClip::new(
        parsed_adpcm_wav.sample_rate_hz,
        parsed_adpcm_wav.block_align,
        parsed_adpcm_wav.samples_per_block,
        data,
    )
}

const fn parse_adpcm_wav_header(wav_bytes: &[u8]) -> ParsedAdpcmWav {
    if wav_bytes.len() < 12 {
        panic!("WAV file too small");
    }
    if !wav_tag_eq(wav_bytes, 0, *b"RIFF") {
        panic!("Missing RIFF header");
    }
    if !wav_tag_eq(wav_bytes, 8, *b"WAVE") {
        panic!("Missing WAVE header");
    }

    let mut chunk_offset = 12usize;
    let mut sample_rate_hz = 0u32;
    let mut block_align = 0usize;
    let mut samples_per_block = 0usize;
    let mut fmt_found = false;
    let mut data_chunk_start = 0usize;
    let mut data_chunk_end = 0usize;
    let mut data_found = false;

    while chunk_offset + 8 <= wav_bytes.len() {
        let chunk_size = read_u32_le_const(wav_bytes, chunk_offset + 4) as usize;
        let chunk_data_start = chunk_offset + 8;
        if chunk_data_start > wav_bytes.len() || chunk_size > wav_bytes.len() - chunk_data_start {
            panic!("WAV chunk overruns file");
        }
        let chunk_data_end = chunk_data_start + chunk_size;

        if wav_tag_eq(wav_bytes, chunk_offset, *b"fmt ") {
            if chunk_size < 16 {
                panic!("fmt chunk too small");
            }

            let audio_format = read_u16_le_const(wav_bytes, chunk_data_start);
            let channels = read_u16_le_const(wav_bytes, chunk_data_start + 2);
            sample_rate_hz = read_u32_le_const(wav_bytes, chunk_data_start + 4);
            block_align = read_u16_le_const(wav_bytes, chunk_data_start + 12) as usize;
            let bits_per_sample = read_u16_le_const(wav_bytes, chunk_data_start + 14);

            if audio_format != 0x0011 {
                panic!("Expected ADPCM WAV format");
            }
            if channels != 1 {
                panic!("Expected mono ADPCM WAV");
            }
            if bits_per_sample != 4 {
                panic!("Expected 4-bit ADPCM");
            }
            if block_align < 5 {
                panic!("ADPCM block_align too small");
            }

            let derived_samples_per_block = derive_adpcm_samples_per_block_const(block_align);
            samples_per_block = if chunk_size >= 22 {
                read_u16_le_const(wav_bytes, chunk_data_start + 18) as usize
            } else {
                derived_samples_per_block
            };

            if samples_per_block != derived_samples_per_block {
                panic!("Unexpected ADPCM samples_per_block");
            }
            fmt_found = true;
        } else if wav_tag_eq(wav_bytes, chunk_offset, *b"data") {
            data_chunk_start = chunk_data_start;
            data_chunk_end = chunk_data_end;
            data_found = true;
        }

        let padded_chunk_size = chunk_size + (chunk_size & 1);
        if chunk_data_start > usize::MAX - padded_chunk_size {
            panic!("WAV chunk traversal overflow");
        }
        chunk_offset = chunk_data_start + padded_chunk_size;
    }

    if !fmt_found {
        panic!("Missing fmt chunk");
    }
    if !data_found {
        panic!("Missing data chunk");
    }
    let data_chunk_len = data_chunk_end - data_chunk_start;
    if data_chunk_len % block_align != 0 {
        panic!("data chunk is not block aligned");
    }

    ParsedAdpcmWav {
        sample_rate_hz,
        block_align,
        samples_per_block,
        data_chunk_start,
        data_chunk_len,
    }
}

const fn wav_tag_eq(wav_bytes: &[u8], byte_offset: usize, tag_bytes: [u8; 4]) -> bool {
    if byte_offset > wav_bytes.len().saturating_sub(4) {
        return false;
    }
    wav_bytes[byte_offset] == tag_bytes[0]
        && wav_bytes[byte_offset + 1] == tag_bytes[1]
        && wav_bytes[byte_offset + 2] == tag_bytes[2]
        && wav_bytes[byte_offset + 3] == tag_bytes[3]
}

const fn derive_adpcm_samples_per_block_const(block_align: usize) -> usize {
    if block_align < 4 {
        panic!("ADPCM block_align underflow");
    }
    ((block_align - 4) * 2) + 1
}

fn decode_adpcm_nibble(adpcm_nibble: u8, predictor_i32: &mut i32, step_index_i32: &mut i32) -> i16 {
    let step = ADPCM_STEP_TABLE[*step_index_i32 as usize];
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
    *step_index_i32 += ADPCM_INDEX_TABLE[adpcm_nibble as usize];
    *step_index_i32 = (*step_index_i32).clamp(0, 88);

    *predictor_i32 as i16
}

const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits_u16 = sample as u16 as u32;
    (sample_bits_u16 << 16) | sample_bits_u16
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

const fn read_u16_le_const(bytes: &[u8], byte_offset: usize) -> u16 {
    if byte_offset > bytes.len().saturating_sub(2) {
        panic!("read_u16_le_const out of bounds");
    }
    u16::from_le_bytes([bytes[byte_offset], bytes[byte_offset + 1]])
}

const fn read_u32_le_const(bytes: &[u8], byte_offset: usize) -> u32 {
    if byte_offset > bytes.len().saturating_sub(4) {
        panic!("read_u32_le_const out of bounds");
    }
    u32::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
        bytes[byte_offset + 2],
        bytes[byte_offset + 3],
    ])
}
