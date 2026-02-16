//! A device abstraction for playing ADPCM WAV clips over PIO I2S.
//!
//! Start with [`AdpcmPlayerGenerated`](adpcm_player_generated::AdpcmPlayerGenerated)
//! and [`AdpcmClipGenerated`](adpcm_clip_generated::AdpcmClipGenerated) to see
//! the generated API from [`adpcm_player!`](macro@crate::adpcm_player::adpcm_player)
//! and [`adpcm_clip!`](macro@crate::adpcm_player::adpcm_clip).

#![cfg_attr(all(test, feature = "host"), allow(dead_code))]

use core::cell::Cell;

#[cfg(target_os = "none")]
use embassy_rp::Peri;
#[cfg(target_os = "none")]
use embassy_rp::dma::Channel;
#[cfg(target_os = "none")]
use embassy_rp::gpio::Pin;
#[cfg(target_os = "none")]
use embassy_rp::pio::{Instance, Pio, PioPin};
#[cfg(target_os = "none")]
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

pub mod adpcm_clip_generated;
pub mod adpcm_player_generated;

const BIT_DEPTH_BITS: u32 = 16;
#[cfg(target_os = "none")]
const SAMPLE_BUFFER_LEN: usize = 256;

/// Relative runtime volume for playback.
///
/// Re-exported from the [`audio_player`](mod@crate::audio_player) module.
pub use crate::audio_player::Volume;

/// Scales a signed 16-bit PCM sample by [`Volume`].
///
/// Re-exported from the [`audio_player`](mod@crate::audio_player) module.
pub use crate::audio_player::scale;

/// End-of-sequence behavior for playback.
pub enum AtEnd {
    /// Repeat the full clip sequence forever.
    Loop,
    /// Stop after one full clip sequence pass.
    Stop,
}

/// Unsized view of ADPCM clip data.
///
/// For fixed-size, const-friendly storage, see [`AdpcmClipBuf`].
pub struct AdpcmClip<const SAMPLE_RATE_HZ: u32, T: ?Sized = [u8]> {
    block_align: u16,
    samples_per_block: u16,
    data: T,
}

impl<const SAMPLE_RATE_HZ: u32, T: ?Sized> AdpcmClip<SAMPLE_RATE_HZ, T> {
    /// Clip sample rate in hertz.
    pub const SAMPLE_RATE_HZ: u32 = SAMPLE_RATE_HZ;
}

impl<const SAMPLE_RATE_HZ: u32> AdpcmClip<SAMPLE_RATE_HZ> {
    /// Returns ADPCM bytes.
    #[must_use]
    pub const fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns ADPCM block size in bytes.
    #[must_use]
    pub const fn block_align(&self) -> usize {
        self.block_align as usize
    }

    /// Returns decoded sample count per ADPCM block.
    #[must_use]
    pub const fn samples_per_block(&self) -> usize {
        self.samples_per_block as usize
    }

    /// Returns decoded sample count for this clip.
    #[must_use]
    pub const fn sample_count(&self) -> usize {
        (self.data.len() / self.block_align as usize) * self.samples_per_block as usize
    }
}

/// Sized, const-friendly storage for ADPCM clip data.
pub type AdpcmClipBuf<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> =
    AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]>;

impl<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]> {
    /// Creates a fixed-size ADPCM clip.
    #[must_use]
    pub const fn new(block_align: u16, samples_per_block: u16, data: [u8; DATA_LEN]) -> Self {
        assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
        assert!(block_align >= 5, "block_align must be >= 5");
        assert!(samples_per_block > 0, "samples_per_block must be > 0");
        assert!(
            DATA_LEN % block_align as usize == 0,
            "adpcm data length must be block aligned"
        );
        Self {
            block_align,
            samples_per_block,
            data,
        }
    }

    /// Returns an unsized clip view.
    #[must_use]
    pub const fn as_adpcm_clip(&self) -> &AdpcmClip<SAMPLE_RATE_HZ> {
        self
    }
}

/// Parsed ADPCM WAV metadata used by [`adpcm_clip!`](macro@crate::adpcm_player::adpcm_clip).
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct ParsedAdpcmWavHeader {
    /// WAV sample rate.
    pub sample_rate_hz: u32,
    /// ADPCM block size in bytes.
    pub block_align: usize,
    /// Decoded samples per ADPCM block.
    pub samples_per_block: usize,
    /// Byte offset of the `data` chunk payload.
    pub data_chunk_start: usize,
    /// Byte length of the `data` chunk payload.
    pub data_chunk_len: usize,
    /// Total decoded sample count from all ADPCM blocks.
    pub sample_count: usize,
}

/// Parses ADPCM WAV header metadata in a `const` context.
#[must_use]
#[doc(hidden)]
pub const fn parse_adpcm_wav_header(wav_bytes: &[u8]) -> ParsedAdpcmWavHeader {
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

            let derived_samples_per_block = derive_samples_per_block_const(block_align);
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

    ParsedAdpcmWavHeader {
        sample_rate_hz,
        block_align,
        samples_per_block,
        data_chunk_start,
        data_chunk_len,
        sample_count: (data_chunk_len / block_align) * samples_per_block,
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

const fn derive_samples_per_block_const(block_align: usize) -> usize {
    if block_align < 4 {
        panic!("ADPCM block_align underflow");
    }
    ((block_align - 4) * 2) + 1
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

enum AudioCommand<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    Play {
        adpcm_clips: Vec<&'static AdpcmClip<SAMPLE_RATE_HZ>, MAX_CLIPS>,
        at_end: AtEnd,
    },
    Stop,
}

#[doc(hidden)]
pub struct AdpcmPlayerStatic<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    command_signal: Signal<CriticalSectionRawMutex, AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>>,
    runtime_volume: Mutex<CriticalSectionRawMutex, Cell<Volume>>,
    max_volume: Volume,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
    pub const fn new_static_with_max_volume_and_initial_volume(
        max_volume: Volume,
        initial_volume: Volume,
    ) -> Self {
        Self {
            command_signal: Signal::new(),
            runtime_volume: Mutex::new(Cell::new(initial_volume)),
            max_volume,
        }
    }

    async fn wait(&self) -> AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ> {
        self.command_signal.wait().await
    }

    fn play(&self, adpcm_clips: Vec<&'static AdpcmClip<SAMPLE_RATE_HZ>, MAX_CLIPS>, at_end: AtEnd) {
        self.command_signal
            .signal(AudioCommand::Play { adpcm_clips, at_end });
    }

    fn stop(&self) {
        self.command_signal.signal(AudioCommand::Stop);
    }

    fn set_volume(&self, volume: Volume) {
        self.runtime_volume.lock(|runtime_volume| runtime_volume.set(volume));
    }

    fn volume(&self) -> Volume {
        self.runtime_volume.lock(Cell::get)
    }

    fn max_volume(&self) -> Volume {
        self.max_volume
    }
}

/// Runtime ADPCM player device handle.
pub struct AdpcmPlayer<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    adpcm_player_static: &'static AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> AdpcmPlayer<MAX_CLIPS, SAMPLE_RATE_HZ> {
    /// Creates an `AdpcmPlayer` handle from static resources.
    #[must_use]
    pub const fn new(
        adpcm_player_static: &'static AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    ) -> Self {
        Self { adpcm_player_static }
    }

    /// Starts playback of one or more static ADPCM clips.
    pub fn play<const CLIP_COUNT: usize>(
        &self,
        adpcm_clips: [&'static AdpcmClip<SAMPLE_RATE_HZ>; CLIP_COUNT],
        at_end: AtEnd,
    ) {
        assert!(CLIP_COUNT <= MAX_CLIPS, "too many clips for this player");
        let mut adpcm_clips_vec = Vec::new();
        for adpcm_clip in adpcm_clips {
            assert!(
                adpcm_clips_vec.push(adpcm_clip).is_ok(),
                "clip list exceeds max_clips"
            );
        }
        self.adpcm_player_static.play(adpcm_clips_vec, at_end);
    }

    /// Stops current playback as soon as possible.
    pub fn stop(&self) {
        self.adpcm_player_static.stop();
    }

    /// Sets runtime playback volume.
    pub fn set_volume(&self, volume: Volume) {
        self.adpcm_player_static.set_volume(volume);
    }

    /// Returns the current runtime playback volume.
    #[must_use]
    pub fn volume(&self) -> Volume {
        self.adpcm_player_static.volume()
    }

    /// Returns the configured runtime volume ceiling.
    #[must_use]
    pub fn max_volume(&self) -> Volume {
        self.adpcm_player_static.max_volume()
    }
}

#[cfg(target_os = "none")]
#[doc(hidden)]
pub trait AdpcmPlayerPio: crate::pio_irqs::PioIrqMap {}

#[cfg(target_os = "none")]
impl<PioResource: crate::pio_irqs::PioIrqMap> AdpcmPlayerPio for PioResource {}

/// Device task loop for ADPCM playback.
#[cfg(target_os = "none")]
#[doc(hidden)]
pub async fn device_loop<
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
    PIO: AdpcmPlayerPio,
    DMA: Channel,
    DinPin: Pin + PioPin,
    BclkPin: Pin + PioPin,
    LrcPin: Pin + PioPin,
>(
    adpcm_player_static: &'static AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, DMA>,
    data_pin: Peri<'static, DinPin>,
    bit_clock_pin: Peri<'static, BclkPin>,
    word_select_pin: Peri<'static, LrcPin>,
) -> ! {
    let mut pio = Pio::new(pio, <PIO as crate::pio_irqs::PioIrqMap>::irqs());
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        dma,
        data_pin,
        bit_clock_pin,
        word_select_pin,
        SAMPLE_RATE_HZ,
        BIT_DEPTH_BITS,
        &pio_i2s_out_program,
    );

    let _pio_i2s_out_program = pio_i2s_out_program;
    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    loop {
        let mut audio_command = adpcm_player_static.wait().await;

        loop {
            match audio_command {
                AudioCommand::Play { adpcm_clips, at_end } => {
                    let next_audio_command = match at_end {
                        AtEnd::Loop => loop {
                            if let Some(next_audio_command) = play_clip_sequence_once(
                                &mut pio_i2s_out,
                                &adpcm_clips,
                                &mut sample_buffer,
                                adpcm_player_static,
                            )
                            .await
                            {
                                break Some(next_audio_command);
                            }
                        },
                        AtEnd::Stop => {
                            play_clip_sequence_once(
                                &mut pio_i2s_out,
                                &adpcm_clips,
                                &mut sample_buffer,
                                adpcm_player_static,
                            )
                            .await
                        }
                    };

                    if let Some(next_audio_command) = next_audio_command {
                        audio_command = next_audio_command;
                        continue;
                    }
                }
                AudioCommand::Stop => {}
            }

            break;
        }
    }
}

#[cfg(target_os = "none")]
async fn play_clip_sequence_once<PIO: Instance, const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    adpcm_clips: &[&'static AdpcmClip<SAMPLE_RATE_HZ>],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    adpcm_player_static: &'static AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> Option<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>> {
    for adpcm_clip in adpcm_clips {
        if let Some(next_audio_command) =
            play_full_clip_once(pio_i2s_out, adpcm_clip, sample_buffer, adpcm_player_static).await
        {
            return Some(next_audio_command);
        }
    }
    None
}

#[cfg(target_os = "none")]
async fn play_full_clip_once<PIO: Instance, const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    adpcm_clip: &AdpcmClip<SAMPLE_RATE_HZ>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    adpcm_player_static: &'static AdpcmPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> Option<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>> {
    let mut sample_buffer_len = 0usize;

    for adpcm_block in adpcm_clip.data().chunks_exact(adpcm_clip.block_align()) {
        if adpcm_block.len() < 4 {
            return None;
        }

        let runtime_volume = adpcm_player_static.volume();
        let mut predictor_i32 = match read_i16_le(adpcm_block, 0) {
            Ok(value) => value as i32,
            Err(_) => return None,
        };
        let mut step_index_i32 = adpcm_block[2] as i32;
        if !(0..=88).contains(&step_index_i32) {
            return None;
        }

        sample_buffer[sample_buffer_len] = stereo_sample(scale(predictor_i32 as i16, runtime_volume));
        sample_buffer_len += 1;

        let mut samples_decoded_in_block = 1usize;
        let samples_per_block = adpcm_clip.samples_per_block();

        for adpcm_byte in &adpcm_block[4..] {
            for adpcm_nibble in [adpcm_byte & 0x0F, adpcm_byte >> 4] {
                if samples_decoded_in_block >= samples_per_block {
                    break;
                }

                let decoded_sample_i16 =
                    decode_adpcm_nibble(adpcm_nibble, &mut predictor_i32, &mut step_index_i32);
                sample_buffer[sample_buffer_len] = stereo_sample(scale(decoded_sample_i16, runtime_volume));
                sample_buffer_len += 1;
                samples_decoded_in_block += 1;

                if sample_buffer_len == SAMPLE_BUFFER_LEN {
                    pio_i2s_out.write(sample_buffer).await;
                    sample_buffer_len = 0;
                    if let Some(next_audio_command) = adpcm_player_static.command_signal.try_take() {
                        return Some(next_audio_command);
                    }
                }
            }
        }

        if let Some(next_audio_command) = adpcm_player_static.command_signal.try_take() {
            return Some(next_audio_command);
        }
    }

    if sample_buffer_len != 0 {
        sample_buffer[sample_buffer_len..].fill(stereo_sample(0));
        pio_i2s_out.write(sample_buffer).await;
        if let Some(next_audio_command) = adpcm_player_static.command_signal.try_take() {
            return Some(next_audio_command);
        }
    }

    None
}

fn read_i16_le(bytes: &[u8], byte_offset: usize) -> crate::Result<i16> {
    let Some(end_offset) = byte_offset.checked_add(2) else {
        return Err(crate::Error::FormatError);
    };
    if end_offset > bytes.len() {
        return Err(crate::Error::FormatError);
    }
    Ok(i16::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
    ]))
}

fn decode_adpcm_nibble(adpcm_nibble: u8, predictor_i32: &mut i32, step_index_i32: &mut i32) -> i16 {
    const ADPCM_INDEX_TABLE: [i32; 16] =
        [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
    const ADPCM_STEP_TABLE: [i32; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55,
        60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337,
        371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552,
        1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894,
        6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350,
        22385, 24623, 27086, 29794, 32767,
    ];

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

#[inline]
#[cfg(target_os = "none")]
const fn stereo_sample(sample_i16: i16) -> u32 {
    let sample_bits_u16 = sample_i16 as u16 as u32;
    (sample_bits_u16 << 16) | sample_bits_u16
}

// Must be `pub` so macro expansion works in downstream crates.
#[doc(hidden)]
pub use paste;

/// Macro to define an ADPCM clip namespace from a WAV file.
///
/// **Syntax:**
///
/// ```text
/// adpcm_clip! {
///     [<visibility>] <Name> {
///         sample_rate_hz: <sample_rate_expr>,
///         file: <path_expr>,
///     }
/// }
/// ```
///
/// **Inputs:**
///
/// - `$vis` - Optional generated module visibility.
/// - `$name` - Module name for the generated namespace.
///
/// **Required fields:**
///
/// - `sample_rate_hz` - Expected WAV sample rate in hertz.
/// - `file` - Path to an ADPCM WAV file.
///
/// **Generated items:**
///
/// - `<Name>::SAMPLE_RATE_HZ`
/// - `<Name>::SAMPLE_COUNT`
/// - `<Name>::DATA_LEN`
/// - `<Name>::AdpcmClip`
/// - `<Name>::adpcm_clip()`
///
/// See the [adpcm_player module documentation](mod@crate::adpcm_player) for usage examples.
pub use crate::adpcm_clip;

#[doc(hidden)]
#[macro_export]
macro_rules! adpcm_clip {
    (
        $vis:vis $name:ident {
            sample_rate_hz: $sample_rate_hz:expr,
            file: $file:expr $(,)?
        }
    ) => {
        $crate::adpcm_player::paste::paste! {
            #[allow(non_snake_case)]
            #[allow(missing_docs)]
            $vis mod $name {
                pub const SAMPLE_RATE_HZ: u32 = $sample_rate_hz;

                const PARSED_WAV: $crate::adpcm_player::ParsedAdpcmWavHeader =
                    $crate::adpcm_player::parse_adpcm_wav_header(include_bytes!($file));

                pub const SAMPLE_COUNT: usize = PARSED_WAV.sample_count;
                pub const DATA_LEN: usize = PARSED_WAV.data_chunk_len;

                pub type AdpcmClip = $crate::adpcm_player::AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN>;

                #[must_use]
                pub const fn adpcm_clip() -> AdpcmClip {
                    let wav_bytes = include_bytes!($file);
                    let parsed_wav = $crate::adpcm_player::parse_adpcm_wav_header(wav_bytes);
                    assert!(
                        parsed_wav.sample_rate_hz == SAMPLE_RATE_HZ,
                        "clip sample_rate_hz must match declared sample_rate_hz"
                    );
                    assert!(parsed_wav.block_align <= u16::MAX as usize, "block_align too large");
                    assert!(
                        parsed_wav.samples_per_block <= u16::MAX as usize,
                        "samples_per_block too large"
                    );

                    let mut adpcm_data = [0_u8; DATA_LEN];
                    let mut data_index = 0usize;
                    while data_index < DATA_LEN {
                        adpcm_data[data_index] = wav_bytes[parsed_wav.data_chunk_start + data_index];
                        data_index += 1;
                    }

                    AdpcmClip::new(
                        parsed_wav.block_align as u16,
                        parsed_wav.samples_per_block as u16,
                        adpcm_data,
                    )
                }
            }
        }
    };
}

/// Macro to generate an ADPCM player struct type.
///
/// This macro mirrors [`audio_player!`](macro@crate::audio_player::audio_player)
/// but plays ADPCM clips generated by [`adpcm_clip!`](macro@crate::adpcm_player::adpcm_clip).
///
/// See the [adpcm_player module documentation](mod@crate::adpcm_player) for usage examples.
pub use crate::adpcm_player;

#[doc(hidden)]
#[macro_export]
macro_rules! adpcm_player {
    (
        $vis:vis $name:ident {
            data_pin: $data_pin:ident,
            bit_clock_pin: $bit_clock_pin:ident,
            word_select_pin: $word_select_pin:ident,
            sample_rate_hz: $sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::adpcm_player! {
            $vis $name {
                data_pin: $data_pin,
                bit_clock_pin: $bit_clock_pin,
                word_select_pin: $word_select_pin,
                sample_rate_hz: $sample_rate_hz,
                pio: PIO0,
                dma: DMA_CH0,
                max_clips: 16,
                max_volume: $crate::adpcm_player::Volume::MAX,
                initial_volume: $crate::adpcm_player::Volume::MAX,
            }
        }
    };

    (
        $vis:vis $name:ident {
            data_pin: $data_pin:ident,
            bit_clock_pin: $bit_clock_pin:ident,
            word_select_pin: $word_select_pin:ident,
            sample_rate_hz: $sample_rate_hz:expr,
            pio: $pio:ident,
            dma: $dma:ident,
            max_clips: $max_clips:expr,
            max_volume: $max_volume:expr,
            initial_volume: $initial_volume:expr $(,)?
        }
    ) => {
        $crate::adpcm_player::paste::paste! {
            static [<$name:upper _ADPCM_PLAYER_STATIC>]:
                $crate::adpcm_player::AdpcmPlayerStatic<$max_clips, { $sample_rate_hz }> =
                $crate::adpcm_player::AdpcmPlayerStatic::new_static_with_max_volume_and_initial_volume(
                    $max_volume,
                    $initial_volume,
                );

            static [<$name:upper _ADPCM_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            #[allow(missing_docs)]
            $vis struct $name {
                adpcm_player: $crate::adpcm_player::AdpcmPlayer<$max_clips, { $sample_rate_hz }>,
            }

            #[allow(missing_docs)]
            $vis type [<$name AdpcmClip>] = $crate::adpcm_player::AdpcmClip<{ $sample_rate_hz }>;

            #[allow(missing_docs)]
            impl $name {
                pub const SAMPLE_RATE_HZ: u32 = $sample_rate_hz;
                pub const INITIAL_VOLUME: $crate::adpcm_player::Volume = $initial_volume;
                pub const MAX_VOLUME: $crate::adpcm_player::Volume = $max_volume;

                pub fn new(
                    data_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$data_pin>>,
                    bit_clock_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bit_clock_pin>>,
                    word_select_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$word_select_pin>>,
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let task_token = [<$name:snake _adpcm_player_task>](
                        &[<$name:upper _ADPCM_PLAYER_STATIC>],
                        pio.into(),
                        dma.into(),
                        data_pin.into(),
                        bit_clock_pin.into(),
                        word_select_pin.into(),
                    );
                    spawner.spawn(task_token)?;

                    let adpcm_player =
                        $crate::adpcm_player::AdpcmPlayer::new(&[<$name:upper _ADPCM_PLAYER_STATIC>]);
                    Ok([<$name:upper _ADPCM_PLAYER_CELL>].init(Self { adpcm_player }))
                }

                pub fn play<const CLIP_COUNT: usize>(
                    &self,
                    adpcm_clips: [&'static [<$name AdpcmClip>]; CLIP_COUNT],
                    at_end: $crate::adpcm_player::AtEnd,
                ) {
                    self.adpcm_player.play(adpcm_clips, at_end);
                }

                pub fn stop(&self) {
                    self.adpcm_player.stop();
                }

                pub fn set_volume(&self, volume: $crate::adpcm_player::Volume) {
                    self.adpcm_player.set_volume(volume);
                }

                #[must_use]
                pub fn volume(&self) -> $crate::adpcm_player::Volume {
                    self.adpcm_player.volume()
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _adpcm_player_task>](
                adpcm_player_static: &'static $crate::adpcm_player::AdpcmPlayerStatic<$max_clips, { $sample_rate_hz }>,
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                data_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$data_pin>,
                bit_clock_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bit_clock_pin>,
                word_select_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$word_select_pin>,
            ) -> ! {
                $crate::adpcm_player::device_loop(
                    adpcm_player_static,
                    pio,
                    dma,
                    data_pin,
                    bit_clock_pin,
                    word_select_pin,
                )
                .await
            }
        }
    };
}
