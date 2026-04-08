//! A device abstraction for playing audio clips over I²S hardware.
//!
//! Platform-independent types, macros, and helpers for the audio player.
//! For complete documentation and examples, see the platform-specific crate
//! (for example `device_envoy_rp::audio_player` or `device_envoy::audio_player`).

// TODO Add a realtime tone Playable (sine + ASR envelope) that uses parameter-only storage and matches ADPCM playback performance.

pub mod adpcm_clip_generated;
#[cfg(all(test, feature = "host"))]
mod host_tests;
pub mod pcm_clip_generated;

// Re-export `paste!` so platform crates can reference it as
// `__paste!` in their `audio_player!` macro.

use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use core::sync::atomic::{AtomicI32, Ordering};
use core::time::Duration;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

const I16_ABS_MAX_I64: i64 = -(i16::MIN as i64);
const ADPCM_ENCODE_BLOCK_ALIGN: usize = 256;

// Common audio sample-rate constants in hertz.

/// Narrowband telephony sample rate.
pub const NARROWBAND_8000_HZ: u32 = 8_000;
/// Wideband voice sample rate.
pub const VOICE_16000_HZ: u32 = 16_000;
/// Common low-memory voice/music sample rate.
///
/// Convenience constant: any sample rate supported by your hardware setup may
/// be used.
pub const VOICE_22050_HZ: u32 = 22_050;
/// Compact-disc sample rate.
pub const CD_44100_HZ: u32 = 44_100;
/// Pro-audio sample rate.
pub const PRO_48000_HZ: u32 = 48_000;

/// Absolute playback loudness setting for the whole player.
///
/// `Volume` is used by the player-level controls
/// [`max_volume`, `initial_volume`](mod@crate::audio_player), and
/// [`set_volume`](AudioPlayer::set_volume),
/// which set the absolute playback loudness behavior for the whole player.
///
/// This is different from [`Gain`] and [`PcmClipBuf::with_gain`], which
/// adjust the relative loudness of individual clips.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volume(i16);

impl Volume {
    /// Silence.
    pub const MUTE: Self = Self(0);

    /// Maximum playback volume.
    pub const MAX: Self = Self(i16::MAX);

    /// Creates a volume from a percentage of full scale.
    ///
    /// Values above `100` are clamped to `100`.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub const fn percent(percent: u8) -> Self {
        let percent = if percent > 100 { 100 } else { percent };
        let value_i32 = (percent as i32 * i16::MAX as i32) / 100;
        Self(value_i32 as i16)
    }

    /// Creates a humorous "goes to 11" demo volume scale.
    ///
    /// `0..=11` maps to `0..=100%` using a perceptual curve
    /// (roughly logarithmic, but not mathematically exact).
    ///
    /// Values above `11` clamp to `11`.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub const fn spinal_tap(spinal_tap: u8) -> Self {
        let spinal_tap = if spinal_tap > 11 { 11 } else { spinal_tap };
        let percent = match spinal_tap {
            0 => 0,
            1 => 1,
            2 => 3,
            3 => 6,
            4 => 13,
            5 => 25,
            6 => 35,
            7 => 50,
            8 => 71,
            9 => 89,
            10 => 100,
            11 => 100,
            _ => 100,
        };
        Self::percent(percent)
    }

    #[must_use]
    pub(crate) const fn to_i16(self) -> i16 {
        self.0
    }

    #[must_use]
    pub(crate) const fn from_i16(value_i16: i16) -> Self {
        Self(value_i16)
    }
}

/// Relative loudness adjustment for audio clips.
///
/// Use `Gain` with [`PcmClipBuf::with_gain`] to make a clip louder or quieter
/// before playback.
///
/// `with_gain` is intended for const clip definitions, so the adjusted samples
/// are precomputed at compile time with no extra runtime work.
///
/// You can set gain by percent or by dB:
/// - [`Gain::percent`] where `100` means unchanged and values above `100` are louder.
/// - [`Gain::db`] where positive dB is louder and negative dB is quieter.
///
/// This is different from [`Volume`] used by
/// [`max_volume`, `initial_volume`](mod@crate::audio_player), and
/// [`set_volume`](AudioPlayer::set_volume),
/// which set the absolute playback loudness behavior for the whole player.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gain(i32);

impl Gain {
    /// Silence.
    pub const MUTE: Self = Self(0);

    /// Creates a gain from percentage.
    ///
    /// `100` is unity gain. Values above `100` boost the signal.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub const fn percent(percent: u16) -> Self {
        let value_i32 = (percent as i32 * i16::MAX as i32) / 100;
        Self(value_i32)
    }

    /// Creates gain from dB with a bounded boost range.
    ///
    /// Values above `+12 dB` clamp to `+12 dB`.
    /// Values below `-96 dB` clamp to `-96 dB`.
    ///
    /// See [`PcmClipBuf::with_gain`] for usage.
    #[must_use]
    pub const fn db(db: i8) -> Self {
        const DB_UPPER_LIMIT: i8 = 12;
        const DB_LOWER_LIMIT: i8 = -96;
        let db = if db > DB_UPPER_LIMIT {
            DB_UPPER_LIMIT
        } else if db < DB_LOWER_LIMIT {
            DB_LOWER_LIMIT
        } else {
            db
        };

        if db == 0 {
            return Self::percent(100);
        }

        // Fixed-point multipliers for 10^(+/-1/20) (approximately +/-1 dB in amplitude).
        const DB_STEP_DOWN_Q15: i32 = 29_205;
        const DB_STEP_UP_Q15: i32 = 36_781;
        const ONE_Q15: i32 = 32_768;
        const ROUND_Q15: i32 = 16_384;
        let step_q15_i32 = if db > 0 {
            DB_STEP_UP_Q15
        } else {
            DB_STEP_DOWN_Q15
        };
        let db_steps_u8 = if db > 0 { db as u8 } else { (-db) as u8 };
        let mut scale_q15_i32 = ONE_Q15;
        let mut step_index = 0_u8;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while step_index < db_steps_u8 {
            scale_q15_i32 = (scale_q15_i32 * step_q15_i32 + ROUND_Q15) / ONE_Q15;
            step_index += 1;
        }

        let gain_i64 = (i16::MAX as i64 * scale_q15_i32 as i64 + ROUND_Q15 as i64) / ONE_Q15 as i64;
        let gain_i32 = if gain_i64 > i32::MAX as i64 {
            i32::MAX
        } else {
            gain_i64 as i32
        };
        Self(gain_i32)
    }

    #[must_use]
    const fn linear(self) -> i32 {
        self.0
    }
}

#[must_use]
#[doc(hidden)]
/// This uses [`core::time::Duration`] for clip timing.
pub const fn __samples_for_duration(duration: core::time::Duration, sample_rate_hz: u32) -> usize {
    assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");
    let sample_rate_hz_u64 = sample_rate_hz as u64;
    let samples_from_seconds_u64 = duration.as_secs() * sample_rate_hz_u64;
    let samples_from_subsec_nanos_u64 =
        (duration.subsec_nanos() as u64 * sample_rate_hz_u64) / 1_000_000_000_u64;
    let total_samples_u64 = samples_from_seconds_u64 + samples_from_subsec_nanos_u64;
    assert!(
        total_samples_u64 <= usize::MAX as u64,
        "duration/sample_rate result must fit usize"
    );
    total_samples_u64 as usize
}

const fn duration_for_sample_count(sample_count: usize, sample_rate_hz: u32) -> Duration {
    assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");
    let sample_rate_hz_usize = sample_rate_hz as usize;
    let whole_seconds = sample_count / sample_rate_hz_usize;
    let subsecond_sample_count = sample_count % sample_rate_hz_usize;
    let subsecond_nanos =
        ((subsecond_sample_count as u64) * 1_000_000_000_u64) / sample_rate_hz as u64;
    Duration::new(whole_seconds as u64, subsecond_nanos as u32)
}

// Must remain `pub` because exported macros (for example `pcm_clip!` and
// `adpcm_clip!`) expand in downstream crates and reference this helper via
// `$crate::...`.
#[doc(hidden)]
#[must_use]
pub const fn __resampled_sample_count(
    source_sample_count: usize,
    source_sample_rate_hz: u32,
    destination_sample_rate_hz: u32,
) -> usize {
    assert!(source_sample_count > 0, "source_sample_count must be > 0");
    assert!(
        source_sample_rate_hz > 0,
        "source_sample_rate_hz must be > 0"
    );
    assert!(
        destination_sample_rate_hz > 0,
        "destination_sample_rate_hz must be > 0"
    );
    let destination_sample_count = ((source_sample_count as u64
        * destination_sample_rate_hz as u64)
        + (source_sample_rate_hz as u64 / 2))
        / source_sample_rate_hz as u64;
    assert!(
        destination_sample_count > 0,
        "destination sample count must be > 0"
    );
    destination_sample_count as usize
}

#[inline]
const fn sine_sample_from_phase(phase_u32: u32) -> i16 {
    let half_cycle_u64 = 1_u64 << 31;
    let one_q31_u64 = 1_u64 << 31;
    let phase_u64 = phase_u32 as u64;
    let (half_phase_u64, sign_i64) = if phase_u64 < half_cycle_u64 {
        (phase_u64, 1_i64)
    } else {
        (phase_u64 - half_cycle_u64, -1_i64)
    };

    // Bhaskara approximation on a normalized half-cycle:
    // sin(pi * t) ~= 16 t (1 - t) / (5 - 4 t (1 - t)), for t in [0, 1].
    let product_q31_u64 = (half_phase_u64 * (one_q31_u64 - half_phase_u64)) >> 31;
    let denominator_q31_u64 = 5 * one_q31_u64 - 4 * product_q31_u64;
    let sine_q31_u64 = ((16 * product_q31_u64) << 31) / denominator_q31_u64;

    let sample_i64 = (sine_q31_u64 as i64 * sign_i64) >> 16;
    clamp_i64_to_i16(sample_i64)
}

// Must be `pub` because platform-crate `device_loop` helpers call it at
// runtime (in a different crate from where it is defined).
#[doc(hidden)]
#[inline]
pub const fn scale_sample_with_linear(sample_i16: i16, linear_i32: i32) -> i16 {
    if linear_i32 == 0 {
        return 0;
    }
    // Use signed full-scale magnitude (32768) so i16::MIN is handled correctly.
    // Full-scale linear is 32767, so add one to map it to exact unity gain.
    let unity_scaled_linear_i64 = linear_i32 as i64 + 1;
    let scaled_i64 = (sample_i16 as i64 * unity_scaled_linear_i64) / I16_ABS_MAX_I64;
    clamp_i64_to_i16(scaled_i64)
}

// Must be `pub` because platform-crate `device_loop` helpers call it at
// runtime (in a different crate from where it is defined).
#[doc(hidden)]
#[inline]
pub const fn scale_sample_with_volume(sample_i16: i16, volume: Volume) -> i16 {
    scale_sample_with_linear(sample_i16, volume.to_i16() as i32)
}

#[inline]
const fn scale_linear(linear_i32: i32, volume: Volume) -> i32 {
    if volume.to_i16() == 0 || linear_i32 == 0 {
        return 0;
    }
    let unity_scaled_volume_i64 = volume.to_i16() as i64 + 1;
    ((linear_i32 as i64 * unity_scaled_volume_i64) / I16_ABS_MAX_I64) as i32
}

#[inline]
const fn clamp_i64_to_i16(value_i64: i64) -> i16 {
    if value_i64 > i16::MAX as i64 {
        i16::MAX
    } else if value_i64 < i16::MIN as i64 {
        i16::MIN
    } else {
        value_i64 as i16
    }
}

/// Platform sink used by shared audio playback routines.
#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait AudioOutputSink<const SAMPLE_BUFFER_LEN: usize> {
    /// Writes `stereo_word_count` samples from `stereo_words`.
    async fn write_stereo_words(
        &mut self,
        stereo_words: &[u32; SAMPLE_BUFFER_LEN],
        stereo_word_count: usize,
    ) -> Result<(), ()>;

    /// Optional hook for platform pacing after each successful write.
    async fn after_write(&mut self) {}
}

/// Packs a mono sample into a stereo I2S frame word.
#[doc(hidden)]
#[inline]
pub const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = sample as u16 as u32;
    (sample_bits << 16) | sample_bits
}

// Must be `pub` because platform `device_loop` implementations call this from
// another crate.
#[doc(hidden)]
pub async fn play_clip_sequence_once<
    Output: AudioOutputSink<SAMPLE_BUFFER_LEN>,
    const SAMPLE_BUFFER_LEN: usize,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    output: &mut Output,
    audio_clips: &[PlaybackClip<SAMPLE_RATE_HZ>],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> Option<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>> {
    for audio_clip in audio_clips {
        match audio_clip {
            PlaybackClip::Pcm(audio_clip) => {
                if let ControlFlow::Break(next_audio_command) =
                    play_full_pcm_clip_once(output, audio_clip, sample_buffer, audio_player_static)
                        .await
                {
                    return Some(next_audio_command);
                }
            }
            PlaybackClip::Adpcm(adpcm_clip) => {
                if let ControlFlow::Break(next_audio_command) = play_full_adpcm_clip_once(
                    output,
                    adpcm_clip,
                    sample_buffer,
                    audio_player_static,
                )
                .await
                {
                    return Some(next_audio_command);
                }
            }
            PlaybackClip::Silence(duration) => {
                if let ControlFlow::Break(next_audio_command) = play_silence_duration_once(
                    output,
                    *duration,
                    sample_buffer,
                    audio_player_static,
                )
                .await
                {
                    return Some(next_audio_command);
                }
            }
        }
    }
    None
}

async fn play_full_pcm_clip_once<
    Output: AudioOutputSink<SAMPLE_BUFFER_LEN>,
    const SAMPLE_BUFFER_LEN: usize,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    output: &mut Output,
    audio_clip: &PcmClip<SAMPLE_RATE_HZ>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> ControlFlow<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>, ()> {
    for audio_sample_chunk in audio_clip.samples().chunks(SAMPLE_BUFFER_LEN) {
        let runtime_volume = audio_player_static.effective_runtime_volume();
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            let scaled_sample_value = scale_sample_with_volume(sample_value, runtime_volume);
            *sample_buffer_slot = stereo_sample(scaled_sample_value);
        }
        sample_buffer[audio_sample_chunk.len()..].fill(stereo_sample(0));

        if output
            .write_stereo_words(sample_buffer, audio_sample_chunk.len())
            .await
            .is_err()
        {
            return ControlFlow::Continue(());
        }
        output.after_write().await;

        if let Some(next_audio_command) = audio_player_static.try_take_command() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    ControlFlow::Continue(())
}

async fn play_full_adpcm_clip_once<
    Output: AudioOutputSink<SAMPLE_BUFFER_LEN>,
    const SAMPLE_BUFFER_LEN: usize,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    output: &mut Output,
    adpcm_clip: &AdpcmClip<SAMPLE_RATE_HZ>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> ControlFlow<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>, ()> {
    let mut sample_buffer_len = 0usize;
    let mut remaining_pcm_sample_count = adpcm_clip.pcm_sample_count();
    if remaining_pcm_sample_count == 0 {
        return ControlFlow::Continue(());
    }

    let block_align = adpcm_clip.block_align() as usize;
    for adpcm_block in adpcm_clip.data().chunks_exact(block_align) {
        if remaining_pcm_sample_count == 0 {
            break;
        }
        if adpcm_block.len() < 4 {
            return ControlFlow::Continue(());
        }

        let runtime_volume = audio_player_static.effective_runtime_volume();
        let mut predictor_i32 = match read_i16_le(adpcm_block, 0) {
            Some(value) => value as i32,
            None => return ControlFlow::Continue(()),
        };
        let mut step_index_i32 = adpcm_block[2] as i32;
        if !(0..=88).contains(&step_index_i32) {
            return ControlFlow::Continue(());
        }

        if remaining_pcm_sample_count > 0 {
            sample_buffer[sample_buffer_len] = stereo_sample(scale_sample_with_volume(
                predictor_i32 as i16,
                runtime_volume,
            ));
            sample_buffer_len += 1;
            remaining_pcm_sample_count -= 1;
            if sample_buffer_len == SAMPLE_BUFFER_LEN {
                if output
                    .write_stereo_words(sample_buffer, sample_buffer_len)
                    .await
                    .is_err()
                {
                    return ControlFlow::Continue(());
                }
                output.after_write().await;
                sample_buffer_len = 0;
                if let Some(next_audio_command) = audio_player_static.try_take_command() {
                    return ControlFlow::Break(next_audio_command);
                }
            }
        }

        let mut samples_decoded_in_block = 1usize;
        let samples_per_block = adpcm_clip.samples_per_block() as usize;

        for adpcm_byte in &adpcm_block[4..] {
            for adpcm_nibble in [adpcm_byte & 0x0F, adpcm_byte >> 4] {
                if samples_decoded_in_block >= samples_per_block || remaining_pcm_sample_count == 0
                {
                    break;
                }

                let decoded_sample_i16 = decode_adpcm_nibble_const(
                    adpcm_nibble,
                    &mut predictor_i32,
                    &mut step_index_i32,
                );
                sample_buffer[sample_buffer_len] =
                    stereo_sample(scale_sample_with_volume(decoded_sample_i16, runtime_volume));
                sample_buffer_len += 1;
                remaining_pcm_sample_count -= 1;
                samples_decoded_in_block += 1;

                if sample_buffer_len == SAMPLE_BUFFER_LEN {
                    if output
                        .write_stereo_words(sample_buffer, sample_buffer_len)
                        .await
                        .is_err()
                    {
                        return ControlFlow::Continue(());
                    }
                    output.after_write().await;
                    sample_buffer_len = 0;
                    if let Some(next_audio_command) = audio_player_static.try_take_command() {
                        return ControlFlow::Break(next_audio_command);
                    }
                }
            }
            if remaining_pcm_sample_count == 0 {
                break;
            }
        }

        if let Some(next_audio_command) = audio_player_static.try_take_command() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    if sample_buffer_len != 0 {
        sample_buffer[sample_buffer_len..].fill(stereo_sample(0));
        if output
            .write_stereo_words(sample_buffer, sample_buffer_len)
            .await
            .is_err()
        {
            return ControlFlow::Continue(());
        }
        output.after_write().await;
        if let Some(next_audio_command) = audio_player_static.try_take_command() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    ControlFlow::Continue(())
}

async fn play_silence_duration_once<
    Output: AudioOutputSink<SAMPLE_BUFFER_LEN>,
    const SAMPLE_BUFFER_LEN: usize,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    output: &mut Output,
    duration: Duration,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> ControlFlow<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>, ()> {
    let silence_sample_count = __samples_for_duration(duration, SAMPLE_RATE_HZ);
    let mut remaining_sample_count = silence_sample_count;
    sample_buffer.fill(stereo_sample(0));

    while remaining_sample_count > 0 {
        let chunk_sample_count = remaining_sample_count.min(SAMPLE_BUFFER_LEN);
        if output
            .write_stereo_words(sample_buffer, chunk_sample_count)
            .await
            .is_err()
        {
            return ControlFlow::Continue(());
        }
        output.after_write().await;
        remaining_sample_count -= chunk_sample_count;
        if let Some(next_audio_command) = audio_player_static.try_take_command() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    ControlFlow::Continue(())
}

#[inline]
fn read_i16_le(bytes: &[u8], byte_offset: usize) -> Option<i16> {
    let end_offset = byte_offset.checked_add(2)?;
    if end_offset > bytes.len() {
        return None;
    }
    Some(i16::from_le_bytes([
        bytes[byte_offset],
        bytes[byte_offset + 1],
    ]))
}

/// End-of-sequence behavior for playback.
///
/// Generated audio player types support looping or stopping at the end of a clip sequence.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
pub enum AtEnd {
    /// Repeat the full clip sequence forever.
    Loop,
    /// Stop after one full clip sequence pass.
    Stop,
}

/// Unsized view of static compressed (ADPCM) clip data.
///
/// For fixed-size, const-friendly storage, see [`AdpcmClipBuf`].
pub struct AdpcmClip<const SAMPLE_RATE_HZ: u32, T: ?Sized = [u8]> {
    block_align: u16,
    samples_per_block: u16,
    pcm_sample_count: u32,
    data: T,
}

/// Sized, const-friendly storage for compressed (ADPCM) clip data.
pub type AdpcmClipBuf<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> =
    AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]>;

impl<const SAMPLE_RATE_HZ: u32, T: ?Sized> AdpcmClip<SAMPLE_RATE_HZ, T> {
    /// Returns the ADPCM block size in bytes.
    #[must_use]
    pub fn block_align(&self) -> u16 {
        self.block_align
    }

    /// Returns the number of decoded samples per ADPCM block.
    #[must_use]
    pub fn samples_per_block(&self) -> u16 {
        self.samples_per_block
    }

    /// Returns the decoded PCM sample count represented by this ADPCM clip.
    #[must_use]
    pub fn pcm_sample_count(&self) -> usize {
        self.pcm_sample_count as usize
    }

    /// Returns a reference to the raw ADPCM byte data.
    #[must_use]
    pub fn data(&self) -> &T {
        &self.data
    }
}

/// **Implementation for fixed-size clips (`AdpcmClipBuf`).**
///
/// This impl applies to [`AdpcmClip`] with array-backed storage:
/// `AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]>`
/// (which is what [`AdpcmClipBuf`] aliases).
impl<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]> {
    /// Creates a fixed-size ADPCM clip.
    #[must_use]
    pub(crate) const fn new(
        block_align: u16,
        samples_per_block: u16,
        pcm_sample_count: usize,
        data: [u8; DATA_LEN],
    ) -> Self {
        assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
        assert!(block_align >= 5, "block_align must be >= 5");
        assert!(samples_per_block > 0, "samples_per_block must be > 0");
        assert!(
            DATA_LEN % block_align as usize == 0,
            "adpcm data length must be block aligned"
        );
        let max_decoded_sample_count =
            (DATA_LEN / block_align as usize) * samples_per_block as usize;
        assert!(
            pcm_sample_count <= max_decoded_sample_count,
            "pcm_sample_count must not exceed ADPCM block capacity"
        );
        assert!(
            pcm_sample_count <= u32::MAX as usize,
            "pcm_sample_count must fit in u32"
        );
        Self {
            block_align,
            samples_per_block,
            pcm_sample_count: pcm_sample_count as u32,
            data,
        }
    }

    /// Returns the uncompressed (PCM) version of this clip.
    ///
    /// `SAMPLE_COUNT` is the number of samples in the resulting PCM clip.
    /// Typically, use the generated clip-module constant:
    /// [`AdpcmClipGenerated::PCM_SAMPLE_COUNT`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::PCM_SAMPLE_COUNT).
    #[must_use]
    pub const fn with_pcm<const SAMPLE_COUNT: usize>(
        &self,
    ) -> PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT> {
        let block_align = self.block_align as usize;
        assert!(block_align >= 5, "block_align must be >= 5");
        assert!(
            DATA_LEN % block_align == 0,
            "adpcm data length must be block aligned"
        );

        let samples_per_block = self.samples_per_block as usize;
        assert!(samples_per_block > 0, "samples_per_block must be > 0");
        let expected_sample_count = self.pcm_sample_count as usize;
        assert!(
            SAMPLE_COUNT == expected_sample_count,
            "sample count must match decoded ADPCM length"
        );

        let mut samples = [0_i16; SAMPLE_COUNT];
        if SAMPLE_COUNT == 0 {
            assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
            return PcmClip { samples };
        }

        let mut sample_index = 0usize;
        let mut remaining_sample_count = SAMPLE_COUNT;
        let mut block_start = 0usize;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while block_start < DATA_LEN && remaining_sample_count > 0 {
            let mut predictor_i32 = read_i16_le_const(&self.data, block_start) as i32;
            let mut step_index_i32 = self.data[block_start + 2] as i32;
            assert!(step_index_i32 >= 0, "ADPCM step_index must be >= 0");
            assert!(step_index_i32 <= 88, "ADPCM step_index must be <= 88");

            samples[sample_index] = predictor_i32 as i16;
            sample_index += 1;
            remaining_sample_count -= 1;
            let mut decoded_in_block = 1usize;

            let mut adpcm_byte_offset = block_start + 4;
            let adpcm_block_end = block_start + block_align;
            // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
            while adpcm_byte_offset < adpcm_block_end {
                let adpcm_byte = self.data[adpcm_byte_offset];
                let adpcm_nibble_low = adpcm_byte & 0x0F;
                let adpcm_nibble_high = adpcm_byte >> 4;

                if decoded_in_block < samples_per_block && remaining_sample_count > 0 {
                    samples[sample_index] = decode_adpcm_nibble_const(
                        adpcm_nibble_low,
                        &mut predictor_i32,
                        &mut step_index_i32,
                    );
                    sample_index += 1;
                    remaining_sample_count -= 1;
                    decoded_in_block += 1;
                }
                if decoded_in_block < samples_per_block && remaining_sample_count > 0 {
                    samples[sample_index] = decode_adpcm_nibble_const(
                        adpcm_nibble_high,
                        &mut predictor_i32,
                        &mut step_index_i32,
                    );
                    sample_index += 1;
                    remaining_sample_count -= 1;
                    decoded_in_block += 1;
                }
                if remaining_sample_count == 0 {
                    break;
                }

                adpcm_byte_offset += 1;
            }

            block_start += block_align;
        }

        assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
        PcmClip { samples }
    }

    /// Returns this fixed-size ADPCM clip with linear sample gain applied.
    ///
    /// This operation decodes ADPCM to PCM, applies gain, then re-encodes ADPCM.
    /// The extra ADPCM encode pass can be more lossy than applying gain once on
    /// PCM before a single ADPCM encode.
    #[must_use]
    pub const fn with_gain(self, gain: Gain) -> Self {
        let block_align = self.block_align as usize;
        assert!(block_align >= 5, "block_align must be >= 5");
        assert!(
            DATA_LEN % block_align == 0,
            "adpcm data length must be block aligned"
        );

        let samples_per_block = self.samples_per_block as usize;
        assert!(samples_per_block > 0, "samples_per_block must be > 0");
        let max_samples_per_block = __adpcm_samples_per_block(block_align);
        assert!(
            samples_per_block <= max_samples_per_block,
            "samples_per_block exceeds block_align capacity"
        );

        let mut gained_data = [0_u8; DATA_LEN];
        let mut block_start = 0usize;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while block_start < DATA_LEN {
            let mut source_predictor_i32 = read_i16_le_const(&self.data, block_start) as i32;
            let mut source_step_index_i32 = self.data[block_start + 2] as i32;
            assert!(
                source_step_index_i32 >= 0 && source_step_index_i32 <= 88,
                "ADPCM step_index must be in 0..=88"
            );

            let scaled_first_sample_i16 =
                scale_sample_with_linear(source_predictor_i32 as i16, gain.linear());
            let mut destination_predictor_i32 = scaled_first_sample_i16 as i32;
            let mut destination_step_index_i32 = source_step_index_i32;

            let scaled_first_sample_bytes = scaled_first_sample_i16.to_le_bytes();
            gained_data[block_start] = scaled_first_sample_bytes[0];
            gained_data[block_start + 1] = scaled_first_sample_bytes[1];
            gained_data[block_start + 2] = destination_step_index_i32 as u8;
            gained_data[block_start + 3] = 0;

            let mut decoded_in_block = 1usize;
            let mut source_byte_offset = block_start + 4;
            let mut destination_byte_offset = block_start + 4;
            let block_end = block_start + block_align;

            // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
            while source_byte_offset < block_end {
                let source_byte = self.data[source_byte_offset];
                let mut destination_byte = 0_u8;

                let mut nibble_index = 0usize;
                // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
                while nibble_index < 2 {
                    if decoded_in_block < samples_per_block {
                        let source_nibble = if nibble_index == 0 {
                            source_byte & 0x0F
                        } else {
                            source_byte >> 4
                        };
                        let decoded_sample_i16 = decode_adpcm_nibble_const(
                            source_nibble,
                            &mut source_predictor_i32,
                            &mut source_step_index_i32,
                        );
                        let scaled_sample_i32 =
                            scale_sample_with_linear(decoded_sample_i16, gain.linear()) as i32;
                        let destination_nibble = encode_adpcm_nibble(
                            scaled_sample_i32,
                            &mut destination_predictor_i32,
                            &mut destination_step_index_i32,
                        );
                        destination_byte |= destination_nibble << (nibble_index * 4);
                        decoded_in_block += 1;
                    }
                    nibble_index += 1;
                }

                gained_data[destination_byte_offset] = destination_byte;
                source_byte_offset += 1;
                destination_byte_offset += 1;
            }

            block_start += block_align;
        }

        Self::new(
            self.block_align,
            self.samples_per_block,
            self.pcm_sample_count as usize,
            gained_data,
        )
    }
}

/// Parsed ADPCM WAV metadata used by [`adpcm_clip!`](macro@crate::audio_player::adpcm_clip).
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
pub const fn __parse_adpcm_wav_header(wav_bytes: &[u8]) -> ParsedAdpcmWavHeader {
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

    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
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

const ADPCM_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
const ADPCM_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// Returns decoded samples per IMA ADPCM block for mono 4-bit data.
// Must remain `pub` because exported macros/constants can reference this via
// `$crate::audio_player::...` in downstream crates.
#[doc(hidden)]
#[must_use]
pub const fn __adpcm_samples_per_block(block_align: usize) -> usize {
    if block_align < 5 {
        panic!("block_align must be >= 5 for ADPCM");
    }
    derive_samples_per_block_const(block_align)
}

/// Returns ADPCM byte length needed to encode `sample_count` mono PCM samples.
#[doc(hidden)]
#[must_use]
pub const fn __adpcm_data_len_for_pcm_samples(sample_count: usize) -> usize {
    __adpcm_data_len_for_pcm_samples_with_block_align(sample_count, ADPCM_ENCODE_BLOCK_ALIGN)
}

/// Returns ADPCM byte length needed to encode `sample_count` mono PCM samples
/// with a specific ADPCM `block_align`.
#[doc(hidden)]
#[must_use]
pub const fn __adpcm_data_len_for_pcm_samples_with_block_align(
    sample_count: usize,
    block_align: usize,
) -> usize {
    let samples_per_block = __adpcm_samples_per_block(block_align);
    let block_count = if sample_count == 0 {
        0
    } else {
        ((sample_count - 1) / samples_per_block) + 1
    };
    block_count * block_align
}

const fn read_u16_le_const(bytes: &[u8], byte_offset: usize) -> u16 {
    if byte_offset > bytes.len().saturating_sub(2) {
        panic!("read_u16_le_const out of bounds");
    }
    u16::from_le_bytes([bytes[byte_offset], bytes[byte_offset + 1]])
}

const fn read_i16_le_const(bytes: &[u8], byte_offset: usize) -> i16 {
    if byte_offset > bytes.len().saturating_sub(2) {
        panic!("read_i16_le_const out of bounds");
    }
    i16::from_le_bytes([bytes[byte_offset], bytes[byte_offset + 1]])
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

// Must be `pub` so platform-crate `device_loop` and play functions can match
// on variants across the crate boundary.
#[doc(hidden)]
pub enum PlaybackClip<const SAMPLE_RATE_HZ: u32> {
    Pcm(&'static PcmClip<SAMPLE_RATE_HZ>),
    Adpcm(&'static AdpcmClip<SAMPLE_RATE_HZ>),
    Silence(Duration),
}

/// An audio clip of silence for a specific duration. Memory-efficient because it stores no audio sample data.
///
/// This clip type is sample-rate agnostic. It can be used with any generated
/// player sample rate because silence is rendered at playback time.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SilenceClip {
    duration: Duration,
}

impl SilenceClip {
    /// Creates a silence clip for a specific duration.
    /// This uses [`core::time::Duration`].
    #[must_use]
    pub const fn new(duration: core::time::Duration) -> Self {
        Self { duration }
    }

    /// Returns the silence duration.
    /// This returns [`core::time::Duration`].
    #[must_use]
    pub const fn duration(self) -> core::time::Duration {
        self.duration
    }
}

/// A clip source trait for generated audio player `play(...)` methods.
///
/// This trait let's us pass audio clips of different types (PCM, ADPCM, or silence) in a single heterogeneous sequence to `play`.
///
/// This trait is object-safe, so mixed clips are passed as:
/// `&'static dyn Playable<SAMPLE_RATE_HZ>`.
#[allow(private_bounds)]
pub trait Playable<const SAMPLE_RATE_HZ: u32>: sealed::PlayableSealed<SAMPLE_RATE_HZ> {}

impl<const SAMPLE_RATE_HZ: u32, T: ?Sized> Playable<SAMPLE_RATE_HZ> for T where
    T: sealed::PlayableSealed<SAMPLE_RATE_HZ>
{
}

/// Platform-agnostic audio player device contract.
///
/// Platform crates implement this trait for generated audio player types so
/// playback operations resolve through trait methods instead of inherent methods.
///
/// # Example: Play "Mary Had a Little Lamb" (Phrase) Once
///
/// This example plays the opening phrase (`E D C D E E E`) and then stops.
///
/// ```rust,no_run
/// use device_envoy_core::audio_player::{
///     AtEnd, AudioPlayer, Playable, SilenceClip, VOICE_22050_HZ, Volume, tone,
/// };
/// use core::time::Duration as StdDuration;
///
/// const SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;
///
/// fn play_mary_phrase(audio_player: &impl AudioPlayer<SAMPLE_RATE_HZ>) {
///     type PlayableRef = &'static dyn Playable<SAMPLE_RATE_HZ>;
///
///     const REST: PlayableRef = &SilenceClip::new(StdDuration::from_millis(80));
///     const NOTE_DURATION: StdDuration = StdDuration::from_millis(220);
///     const NOTE_E4: PlayableRef = &tone!(330, SAMPLE_RATE_HZ, NOTE_DURATION);
///     const NOTE_D4: PlayableRef = &tone!(294, SAMPLE_RATE_HZ, NOTE_DURATION);
///     const NOTE_C4: PlayableRef = &tone!(262, SAMPLE_RATE_HZ, NOTE_DURATION);
///
///     audio_player.play(
///         [
///             NOTE_E4, REST, NOTE_D4, REST, NOTE_C4, REST, NOTE_D4, REST, NOTE_E4, REST,
///             NOTE_E4, REST, NOTE_E4,
///         ],
///         AtEnd::Stop,
///     );
/// }
///
/// # struct DemoAudioPlayer;
/// # impl AudioPlayer<SAMPLE_RATE_HZ> for DemoAudioPlayer {
/// #     const SAMPLE_RATE_HZ: u32 = SAMPLE_RATE_HZ;
/// #     const MAX_CLIPS: usize = 16;
/// #     const INITIAL_VOLUME: Volume = Volume::MAX;
/// #     const MAX_VOLUME: Volume = Volume::MAX;
/// #     fn play<I>(&self, _audio_clips: I, _at_end: AtEnd)
/// #     where
/// #         I: IntoIterator<Item = &'static dyn Playable<SAMPLE_RATE_HZ>>,
/// #     {
/// #     }
/// #     fn stop(&self) {}
/// #     async fn wait_until_stopped(&self) {}
/// #     fn set_volume(&self, _volume: Volume) {}
/// #     fn volume(&self) -> Volume {
/// #         Self::INITIAL_VOLUME
/// #     }
/// # }
/// # let audio_player = DemoAudioPlayer;
/// # play_mary_phrase(&audio_player);
/// ```
///
/// # Example: Compiling in an External Audio Clip and Runtime Volume Changes
///
/// This example shows how to compile in an external clip, play it in a loop,
/// change volume at runtime, and then stop/reset playback settings.
///
/// ```rust,no_run,standalone_crate
/// use device_envoy_core::audio_player::{
///     AtEnd, AudioPlayer, Gain, Playable, SilenceClip, VOICE_22050_HZ, Volume, pcm_clip, tone,
/// };
/// use device_envoy_core::button::Button;
/// use core::time::Duration as StdDuration;
/// use embassy_futures::select::{Either, select};
/// use embassy_time::{Duration, Timer};
///
/// pcm_clip! {
///     Nasa {
///         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
///         source_sample_rate_hz: VOICE_22050_HZ,
///     }
/// }
///
/// async fn play_nasa_with_runtime_volume(
///     audio_player: &impl AudioPlayer<VOICE_22050_HZ>,
///     button: &mut impl Button,
/// ) -> ! {
///     type PlayableRef = &'static dyn Playable<VOICE_22050_HZ>;
///
///     const fn ms(milliseconds: u64) -> StdDuration {
///         StdDuration::from_millis(milliseconds)
///     }
///
///     const NASA: PlayableRef = &Nasa::adpcm_clip();
///     const GAP: PlayableRef = &SilenceClip::new(ms(80));
///     const CHIME: PlayableRef = &tone!(880, VOICE_22050_HZ, ms(100)).with_gain(Gain::percent(20));
///     const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];
///     let initial_volume = audio_player.volume();
///
///     loop {
///         audio_player.play([CHIME, NASA, GAP], AtEnd::Loop);
///
///         for volume_percent in VOLUME_STEPS_PERCENT {
///             match select(button.wait_for_press(), Timer::after(Duration::from_secs(1))).await {
///                 Either::First(()) => break,
///                 Either::Second(()) => audio_player.set_volume(Volume::percent(volume_percent)),
///             }
///         }
///
///         audio_player.stop();
///         audio_player.set_volume(initial_volume);
///         button.wait_for_press().await;
///     }
/// }
///
/// # struct DemoAudioPlayer;
/// # impl AudioPlayer<VOICE_22050_HZ> for DemoAudioPlayer {
/// #     const SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;
/// #     const MAX_CLIPS: usize = 8;
/// #     const INITIAL_VOLUME: Volume = Volume::spinal_tap(5);
/// #     const MAX_VOLUME: Volume = Volume::spinal_tap(11);
/// #     fn play<I>(&self, _audio_clips: I, _at_end: AtEnd)
/// #     where
/// #         I: IntoIterator<Item = &'static dyn Playable<VOICE_22050_HZ>>,
/// #     {
/// #     }
/// #     fn stop(&self) {}
/// #     async fn wait_until_stopped(&self) {}
/// #     fn set_volume(&self, _volume: Volume) {}
/// #     fn volume(&self) -> Volume {
/// #         Self::INITIAL_VOLUME
/// #     }
/// # }
/// # struct ButtonMock;
/// # impl device_envoy_core::button::__ButtonMonitor for ButtonMock {
/// #     fn is_pressed_raw(&self) -> bool { false }
/// #     async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
/// # }
/// # impl Button for ButtonMock {}
/// fn main() {
///     let mut button = ButtonMock;
///     let audio_player = DemoAudioPlayer;
///     let _future = play_nasa_with_runtime_volume(&audio_player, &mut button);
/// }
/// ```
///
/// # Example: Resample and Play Countdown Once
///
/// This example compiles in `2`, `1`, `0`, and NASA at `22.05 kHz`, resamples
/// them to narrowband (`8 kHz`) at compile time, and then plays the sequence.
///
/// ```rust,no_run,standalone_crate
/// use device_envoy_core::audio_player::{
///     AtEnd, AudioPlayer, Gain, NARROWBAND_8000_HZ, Playable, VOICE_22050_HZ, Volume, pcm_clip,
/// };
/// use device_envoy_core::button::Button;
///
/// pcm_clip! {
///     Digit0 {
///         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/0_22050.s16"),
///         source_sample_rate_hz: VOICE_22050_HZ,
///         target_sample_rate_hz: NARROWBAND_8000_HZ,
///     }
/// }
///
/// pcm_clip! {
///     Digit1 {
///         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/1_22050.s16"),
///         source_sample_rate_hz: VOICE_22050_HZ,
///         target_sample_rate_hz: NARROWBAND_8000_HZ,
///     }
/// }
///
/// pcm_clip! {
///     Digit2 {
///         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/2_22050.s16"),
///         source_sample_rate_hz: VOICE_22050_HZ,
///         target_sample_rate_hz: NARROWBAND_8000_HZ,
///     }
/// }
///
/// pcm_clip! {
///     Nasa {
///         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
///         source_sample_rate_hz: VOICE_22050_HZ,
///         target_sample_rate_hz: NARROWBAND_8000_HZ,
///     }
/// }
///
/// fn play_resampled_countdown(audio_player: &impl AudioPlayer<NARROWBAND_8000_HZ>) {
///     type PlayableRef = &'static dyn Playable<NARROWBAND_8000_HZ>;
///
///     const DIGITS: [PlayableRef; 3] =
///         [&Digit0::adpcm_clip(), &Digit1::adpcm_clip(), &Digit2::adpcm_clip()];
///     const NASA: PlayableRef = &Nasa::pcm_clip()
///         .with_gain(Gain::percent(25))
///         .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();
///
///     audio_player.play([DIGITS[2], DIGITS[1], DIGITS[0], NASA], AtEnd::Stop);
/// }
///
/// # struct DemoAudioPlayer;
/// # impl AudioPlayer<NARROWBAND_8000_HZ> for DemoAudioPlayer {
/// #     const SAMPLE_RATE_HZ: u32 = NARROWBAND_8000_HZ;
/// #     const MAX_CLIPS: usize = 16;
/// #     const INITIAL_VOLUME: Volume = Volume::MAX;
/// #     const MAX_VOLUME: Volume = Volume::percent(50);
/// #     fn play<I>(&self, _audio_clips: I, _at_end: AtEnd)
/// #     where
/// #         I: IntoIterator<Item = &'static dyn Playable<NARROWBAND_8000_HZ>>,
/// #     {
/// #     }
/// #     fn stop(&self) {}
/// #     async fn wait_until_stopped(&self) {}
/// #     fn set_volume(&self, _volume: Volume) {}
/// #     fn volume(&self) -> Volume {
/// #         Self::INITIAL_VOLUME
/// #     }
/// # }
/// # struct ButtonMock;
/// # impl device_envoy_core::button::__ButtonMonitor for ButtonMock {
/// #     fn is_pressed_raw(&self) -> bool { false }
/// #     async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
/// # }
/// # impl Button for ButtonMock {}
/// fn main() {
///     let mut button = ButtonMock;
///     let audio_player = DemoAudioPlayer;
///     play_resampled_countdown(&audio_player);
///     let _future = button.wait_for_press();
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait AudioPlayer<const SAMPLE_RATE_HZ: u32> {
    /// Sample rate in hertz for this generated player type.
    const SAMPLE_RATE_HZ: u32;
    /// Maximum number of clips accepted by `play(...)` for this generated type.
    const MAX_CLIPS: usize;
    /// Initial runtime volume relative to [`Self::MAX_VOLUME`].
    const INITIAL_VOLUME: Volume;
    /// Runtime volume ceiling for this generated player type.
    const MAX_VOLUME: Volume;

    /// Starts playback of one or more static audio clips.
    ///
    /// Accepts any array-like or iterator input. The maximum number of clips
    /// is determined by the generated type configuration.
    ///
    /// See the [AudioPlayer trait documentation](Self) for usage examples.
    fn play<I>(&self, audio_clips: I, at_end: AtEnd)
    where
        I: IntoIterator<Item = &'static dyn Playable<SAMPLE_RATE_HZ>>;

    /// Stops current playback as soon as possible.
    ///
    /// See the [AudioPlayer trait documentation](Self) for usage examples.
    fn stop(&self);

    /// Waits until playback is stopped.
    ///
    /// See the [AudioPlayer trait documentation](Self) for usage examples.
    async fn wait_until_stopped(&self);

    /// Sets runtime playback volume relative to the generated player's max volume.
    ///
    /// See the [AudioPlayer trait documentation](Self) for usage examples.
    fn set_volume(&self, volume: Volume);

    /// Returns the current runtime playback volume relative to max volume.
    ///
    /// See the [AudioPlayer trait documentation](Self) for usage examples.
    fn volume(&self) -> Volume;
}

mod sealed {
    use super::{AdpcmClip, PcmClip, PlaybackClip, SilenceClip};

    pub(crate) trait PlayableSealed<const SAMPLE_RATE_HZ: u32> {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ>;
    }

    impl<const SAMPLE_RATE_HZ: u32> PlayableSealed<SAMPLE_RATE_HZ> for PcmClip<SAMPLE_RATE_HZ> {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ> {
            PlaybackClip::Pcm(self)
        }
    }

    impl<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize> PlayableSealed<SAMPLE_RATE_HZ>
        for PcmClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>
    {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ> {
            PlaybackClip::Pcm(self)
        }
    }

    impl<const SAMPLE_RATE_HZ: u32> PlayableSealed<SAMPLE_RATE_HZ> for AdpcmClip<SAMPLE_RATE_HZ> {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ> {
            PlaybackClip::Adpcm(self)
        }
    }

    impl<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> PlayableSealed<SAMPLE_RATE_HZ>
        for AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]>
    {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ> {
            PlaybackClip::Adpcm(self)
        }
    }

    impl<const SAMPLE_RATE_HZ: u32> PlayableSealed<SAMPLE_RATE_HZ> for SilenceClip {
        fn playback_clip(&'static self) -> PlaybackClip<SAMPLE_RATE_HZ> {
            PlaybackClip::Silence(self.duration())
        }
    }
}

/// Unsized view of static uncompressed (PCM) audio clip data.
///
/// For fixed-size, const-friendly storage, see [`PcmClipBuf`].
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
pub struct PcmClip<const SAMPLE_RATE_HZ: u32, T: ?Sized = [i16]> {
    samples: T,
}

impl<const SAMPLE_RATE_HZ: u32, T: ?Sized> PcmClip<SAMPLE_RATE_HZ, T> {
    /// Returns a reference to the raw PCM sample data.
    #[must_use]
    pub fn samples(&self) -> &T {
        &self.samples
    }
}

/// Sized, const-friendly storage for uncompressed (PCM) audio clip data.
///
/// For unsized clip references, see [`PcmClip`].
///
/// Sample rate is part of the type, so clips with different sample rates are
/// not assignment-compatible.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
pub type PcmClipBuf<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize> =
    PcmClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>;

/// **Implementation for fixed-size clips (`PcmClipBuf`).**
///
/// This impl applies to [`PcmClip`] with array-backed storage:
/// `PcmClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>`
/// (which is what [`PcmClipBuf`] aliases).
impl<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>
    PcmClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>
{
    /// Returns a new clip with linear sample gain applied.
    ///
    /// This is intended to be used in const clip definitions so the adjusted
    /// samples are computed ahead of time.
    ///
    /// Gain multiplication uses i32 math and saturates to i16 sample bounds.
    /// Large boosts can hard-clip peaks and introduce distortion.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub const fn with_gain(self, gain: Gain) -> Self {
        assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
        let mut scaled_samples = [0_i16; SAMPLE_COUNT];
        let mut sample_index = 0_usize;
        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
        while sample_index < SAMPLE_COUNT {
            scaled_samples[sample_index] =
                scale_sample_with_linear(self.samples[sample_index], gain.linear());
            sample_index += 1;
        }
        Self {
            samples: scaled_samples,
        }
    }

    /// Returns this clip with a linear attack and release envelope.
    ///
    /// This is useful when you want explicit control over click reduction at
    /// the start/end of generated tones.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub(crate) const fn with_attack_release(self, attack: Duration, release: Duration) -> Self {
        assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
        let attack_sample_count = __samples_for_duration(attack, SAMPLE_RATE_HZ);
        let release_sample_count = __samples_for_duration(release, SAMPLE_RATE_HZ);
        self.with_attack_release_sample_count(attack_sample_count, release_sample_count)
    }

    #[must_use]
    const fn with_attack_release_sample_count(
        self,
        attack_sample_count: usize,
        release_sample_count: usize,
    ) -> Self {
        assert!(
            attack_sample_count <= SAMPLE_COUNT,
            "attack duration must fit within clip duration"
        );
        assert!(
            release_sample_count <= SAMPLE_COUNT,
            "release duration must fit within clip duration"
        );
        assert!(
            attack_sample_count + release_sample_count <= SAMPLE_COUNT,
            "attack + release must fit within clip duration"
        );

        let mut shaped_samples = self.samples;

        if attack_sample_count > 0 {
            let attack_sample_count_i32 = attack_sample_count as i32;
            let mut sample_index = 0usize;
            // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
            while sample_index < attack_sample_count {
                let envelope_numerator_i32 = sample_index as i32;
                shaped_samples[sample_index] = scale_sample_with_linear(
                    shaped_samples[sample_index],
                    (envelope_numerator_i32 * i16::MAX as i32) / attack_sample_count_i32,
                );
                sample_index += 1;
            }
        }

        if release_sample_count > 0 {
            let release_sample_count_i32 = release_sample_count as i32;
            let release_start_index = SAMPLE_COUNT - release_sample_count;
            let mut release_index = 0usize;
            // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
            while release_index < release_sample_count {
                let sample_index = release_start_index + release_index;
                let envelope_numerator_i32 = (release_sample_count - release_index) as i32;
                shaped_samples[sample_index] = scale_sample_with_linear(
                    shaped_samples[sample_index],
                    (envelope_numerator_i32 * i16::MAX as i32) / release_sample_count_i32,
                );
                release_index += 1;
            }
        }

        Self {
            samples: shaped_samples,
        }
    }

    /// Returns the compressed (ADPCM) encoding for this clip.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub const fn with_adpcm<const DATA_LEN: usize>(
        &self,
    ) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
        self.with_adpcm_block_align::<DATA_LEN>(ADPCM_ENCODE_BLOCK_ALIGN)
    }

    #[must_use]
    pub(crate) const fn with_adpcm_block_align<const DATA_LEN: usize>(
        &self,
        block_align: usize,
    ) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
        assert!(block_align >= 5, "block_align must be >= 5");
        assert!(
            block_align <= u16::MAX as usize,
            "block_align must fit in u16"
        );
        let samples_per_block = __adpcm_samples_per_block(block_align);
        assert!(
            samples_per_block <= u16::MAX as usize,
            "samples_per_block must fit in u16"
        );
        assert!(
            DATA_LEN
                == __adpcm_data_len_for_pcm_samples_with_block_align(SAMPLE_COUNT, block_align),
            "adpcm data length must match sample count and block_align"
        );
        if SAMPLE_COUNT == 0 {
            return AdpcmClip::new(
                block_align as u16,
                samples_per_block as u16,
                SAMPLE_COUNT,
                [0; DATA_LEN],
            );
        }

        let mut adpcm_data = [0_u8; DATA_LEN];
        let mut sample_index = 0usize;
        let mut data_index = 0usize;
        let payload_len_per_block = block_align - 4;

        // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
        while sample_index < SAMPLE_COUNT {
            let mut predictor_i32 = self.samples[sample_index] as i32;
            let mut step_index_i32 = 0_i32;

            let predictor_i16 = predictor_i32 as i16;
            let predictor_bytes = predictor_i16.to_le_bytes();
            adpcm_data[data_index] = predictor_bytes[0];
            adpcm_data[data_index + 1] = predictor_bytes[1];
            adpcm_data[data_index + 2] = step_index_i32 as u8;
            adpcm_data[data_index + 3] = 0;
            data_index += 4;
            sample_index += 1;

            let mut payload_byte_index = 0usize;
            // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
            while payload_byte_index < payload_len_per_block {
                let mut adpcm_byte = 0_u8;

                let mut nibble_index = 0usize;
                // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
                while nibble_index < 2 {
                    let target_sample_i32 = if sample_index < SAMPLE_COUNT {
                        self.samples[sample_index] as i32
                    } else {
                        predictor_i32
                    };
                    let adpcm_nibble = encode_adpcm_nibble(
                        target_sample_i32,
                        &mut predictor_i32,
                        &mut step_index_i32,
                    );
                    adpcm_byte |= adpcm_nibble << (nibble_index * 4);
                    sample_index += 1;
                    nibble_index += 1;
                }

                adpcm_data[data_index] = adpcm_byte;
                data_index += 1;
                payload_byte_index += 1;
            }
        }

        AdpcmClip::new(
            block_align as u16,
            samples_per_block as u16,
            SAMPLE_COUNT,
            adpcm_data,
        )
    }
}

// Must be `pub` so platform-crate `device_loop` and play functions can access
// these across the crate boundary.
#[doc(hidden)]
pub enum AudioCommand<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    Play {
        audio_clips: Vec<PlaybackClip<SAMPLE_RATE_HZ>, MAX_CLIPS>,
        at_end: AtEnd,
    },
    Stop,
}

/// Static resources for audio player runtime handles.
// Must be `pub` so `audio_player!` expansions in downstream crates can reference this type.
#[doc(hidden)]
pub struct AudioPlayerStatic<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    command_signal: Signal<CriticalSectionRawMutex, AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>>,
    stopped_signal: Signal<CriticalSectionRawMutex, ()>,
    is_playing: AtomicBool,
    has_pending_play: AtomicBool,
    max_volume_linear: i32,
    runtime_volume_relative_linear: AtomicI32,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>
    AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>
{
    /// Creates static resources for a player.
    #[must_use]
    pub const fn new_static() -> Self {
        Self::new_static_with_max_volume_and_initial_volume(Volume::MAX, Volume::MAX)
    }

    /// Creates static resources for a player with a runtime volume ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume(max_volume: Volume) -> Self {
        Self::new_static_with_max_volume_and_initial_volume(max_volume, Volume::MAX)
    }

    /// Creates static resources for a player with a runtime volume ceiling
    /// and an initial runtime volume relative to that ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume_and_initial_volume(
        max_volume: Volume,
        initial_volume: Volume,
    ) -> Self {
        Self {
            command_signal: Signal::new(),
            stopped_signal: Signal::new(),
            is_playing: AtomicBool::new(false),
            has_pending_play: AtomicBool::new(false),
            max_volume_linear: max_volume.to_i16() as i32,
            runtime_volume_relative_linear: AtomicI32::new(initial_volume.to_i16() as i32),
        }
    }

    fn signal(&self, audio_command: AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>) {
        self.command_signal.signal(audio_command);
    }

    fn mark_pending_play(&self) {
        self.has_pending_play.store(true, AtomicOrdering::Relaxed);
    }

    /// Marks the player as currently playing. Called by platform-crate `device_loop`.
    #[doc(hidden)]
    pub fn mark_playing(&self) {
        self.has_pending_play.store(false, AtomicOrdering::Relaxed);
        self.is_playing.store(true, AtomicOrdering::Relaxed);
    }

    /// Marks the player as stopped. Called by platform-crate `device_loop`.
    #[doc(hidden)]
    pub fn mark_stopped(&self) {
        self.has_pending_play.store(false, AtomicOrdering::Relaxed);
        self.is_playing.store(false, AtomicOrdering::Relaxed);
        self.stopped_signal.signal(());
    }

    fn is_idle(&self) -> bool {
        !self.has_pending_play.load(AtomicOrdering::Relaxed)
            && !self.is_playing.load(AtomicOrdering::Relaxed)
    }

    async fn wait_until_stopped(&self) {
        while !self.is_idle() {
            self.stopped_signal.wait().await;
        }
    }

    fn set_runtime_volume(&self, volume: Volume) {
        self.runtime_volume_relative_linear
            .store(volume.to_i16() as i32, Ordering::Relaxed);
    }

    fn runtime_volume(&self) -> Volume {
        Volume::from_i16(self.runtime_volume_relative_linear.load(Ordering::Relaxed) as i16)
    }

    /// Returns the effective runtime volume after applying max-volume scaling.
    /// Called by platform-crate play functions.
    #[doc(hidden)]
    pub fn effective_runtime_volume(&self) -> Volume {
        let runtime_volume_relative = self.runtime_volume();
        Volume::from_i16(scale_linear(self.max_volume_linear, runtime_volume_relative) as i16)
    }

    /// Awaits the next audio command. Used by RP-style `device_loop`.
    #[doc(hidden)]
    pub async fn wait(&self) -> AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ> {
        self.command_signal.wait().await
    }

    /// Tries to take a pending audio command without waiting. Used by
    /// ESP-style play functions and both platforms' `device_loop`.
    #[doc(hidden)]
    pub fn try_take_command(&self) -> Option<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>> {
        self.command_signal.try_take()
    }
}

// Must be `pub` so platform runtime handles can call this from another crate.
#[doc(hidden)]
pub fn __audio_player_play<I, const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    audio_clips: I,
    at_end: AtEnd,
) where
    I: IntoIterator<Item = &'static dyn Playable<SAMPLE_RATE_HZ>>,
{
    assert!(MAX_CLIPS > 0, "play disabled: max_clips is 0");
    let mut audio_clip_sequence: Vec<PlaybackClip<SAMPLE_RATE_HZ>, MAX_CLIPS> = Vec::new();
    for audio_clip in audio_clips {
        assert!(
            audio_clip_sequence
                .push(sealed::PlayableSealed::playback_clip(audio_clip))
                .is_ok(),
            "play sequence fits within max_clips"
        );
    }
    assert!(
        !audio_clip_sequence.is_empty(),
        "play requires at least one clip"
    );

    audio_player_static.mark_pending_play();
    audio_player_static.signal(AudioCommand::Play {
        audio_clips: audio_clip_sequence,
        at_end,
    });
}

// Must be `pub` so platform runtime handles can call this from another crate.
#[doc(hidden)]
pub fn __audio_player_stop<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) {
    audio_player_static.signal(AudioCommand::Stop);
}

// Must be `pub` so platform runtime handles can call this from another crate.
#[doc(hidden)]
pub async fn __audio_player_wait_until_stopped<
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) {
    audio_player_static.wait_until_stopped().await;
}

// Must be `pub` so platform runtime handles can call this from another crate.
#[doc(hidden)]
pub fn __audio_player_set_volume<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    volume: Volume,
) {
    audio_player_static.set_runtime_volume(volume);
}

// Must be `pub` so platform runtime handles can call this from another crate.
#[doc(hidden)]
#[must_use]
pub fn __audio_player_volume<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> Volume {
    audio_player_static.runtime_volume()
}

/// Const backend helper that creates a PCM sine-wave clip.
///
/// This is intentionally `#[doc(hidden)]` because user-facing construction
/// should prefer [`tone!`](macro@crate::tone).
#[must_use]
#[doc(hidden)]
pub const fn __tone_pcm_clip<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    frequency_hz: u32,
) -> PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT> {
    __tone_pcm_clip_with_duration::<SAMPLE_RATE_HZ, SAMPLE_COUNT>(
        frequency_hz,
        duration_for_sample_count(SAMPLE_COUNT, SAMPLE_RATE_HZ),
    )
}

/// Const backend helper that creates a PCM sine-wave clip with explicit
/// duration metadata used for built-in shaping.
/// This uses [`core::time::Duration`] for tone duration.
#[must_use]
#[doc(hidden)]
pub const fn __tone_pcm_clip_with_duration<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    frequency_hz: u32,
    duration: core::time::Duration,
) -> PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT> {
    assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
    let mut samples = [0_i16; SAMPLE_COUNT];
    let phase_step_u64 = ((frequency_hz as u64) << 32) / SAMPLE_RATE_HZ as u64;
    let phase_step_u32 = phase_step_u64 as u32;
    let mut phase_u32 = 0_u32;

    let mut sample_index = 0usize;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
    while sample_index < SAMPLE_COUNT {
        samples[sample_index] = sine_sample_from_phase(phase_u32);
        phase_u32 = phase_u32.wrapping_add(phase_step_u32);
        sample_index += 1;
    }

    // Add an attack and release duration to reduce clicks. It is min(50ms, 1/4 of total duration)
    let max_duration = Duration::from_millis(50);
    assert!(max_duration.as_secs() == 0, "50ms cap must be sub-second");
    let attack_release_duration = match (duration.as_secs(), duration.subsec_nanos()) {
        (0, nanos) if nanos / 4 < max_duration.subsec_nanos() => Duration::new(0, nanos / 4),
        (_, _) => max_duration,
    };
    PcmClip { samples }.with_attack_release(attack_release_duration, attack_release_duration)
}

/// Builds a fixed-size PCM clip from samples.
///
/// This is intentionally `#[doc(hidden)]` because user-facing clip
/// construction should prefer `pcm_clip!`, `adpcm_clip!`, and `tone!`.
#[must_use]
#[doc(hidden)]
pub const fn __pcm_clip_from_samples<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    samples: [i16; SAMPLE_COUNT],
) -> PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT> {
    assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
    PcmClip { samples }
}

/// Const backend helper that builds a fixed-size ADPCM clip from parts.
///
/// This is intentionally `#[doc(hidden)]` because user-facing clip
/// construction should prefer `adpcm_clip!` and conversion helpers.
#[must_use]
#[doc(hidden)]
pub const fn __adpcm_clip_from_parts<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize>(
    block_align: u16,
    samples_per_block: u16,
    pcm_sample_count: usize,
    data: [u8; DATA_LEN],
) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
    AdpcmClip::new(block_align, samples_per_block, pcm_sample_count, data)
}

/// Const backend helper that encodes PCM into ADPCM with an explicit block size.
///
/// This helper must be `pub` because macro expansions in downstream crates call
/// it at the call site, but it is not a user-facing API.
#[must_use]
#[doc(hidden)]
pub const fn __pcm_with_adpcm_block_align<
    const SAMPLE_RATE_HZ: u32,
    const SAMPLE_COUNT: usize,
    const DATA_LEN: usize,
>(
    source_pcm_clip: &PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT>,
    block_align: usize,
) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
    source_pcm_clip.with_adpcm_block_align::<DATA_LEN>(block_align)
}

/// Const backend helper that resamples a PCM clip to a destination timeline.
///
/// This is intentionally `#[doc(hidden)]` because resampling is configured by
/// `pcm_clip!`/`adpcm_clip!` inputs (`target_sample_rate_hz`) rather than by a
/// direct clip method.
#[must_use]
#[doc(hidden)]
pub const fn __resample_pcm_clip<
    const SOURCE_HZ: u32,
    const SOURCE_COUNT: usize,
    const TARGET_HZ: u32,
    const TARGET_COUNT: usize,
>(
    source_pcm_clip: PcmClipBuf<SOURCE_HZ, SOURCE_COUNT>,
) -> PcmClipBuf<TARGET_HZ, TARGET_COUNT> {
    assert!(SOURCE_COUNT > 0, "source sample count must be > 0");
    assert!(TARGET_HZ > 0, "destination sample_rate_hz must be > 0");
    let expected_destination_sample_count =
        __resampled_sample_count(SOURCE_COUNT, SOURCE_HZ, TARGET_HZ);
    assert!(
        TARGET_COUNT == expected_destination_sample_count,
        "destination sample count must preserve duration"
    );

    let source_samples = source_pcm_clip.samples;
    let mut resampled_samples = [0_i16; TARGET_COUNT];
    let mut sample_index = 0_usize;

    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
    while sample_index < TARGET_COUNT {
        let source_position_numerator_u128 = sample_index as u128 * SOURCE_HZ as u128;
        let source_index_u128 = source_position_numerator_u128 / TARGET_HZ as u128;
        let source_fraction_numerator_u128 = source_position_numerator_u128 % TARGET_HZ as u128;
        let source_index = source_index_u128 as usize;

        resampled_samples[sample_index] = if source_index + 1 >= SOURCE_COUNT {
            source_samples[SOURCE_COUNT - 1]
        } else if source_fraction_numerator_u128 == 0 {
            source_samples[source_index]
        } else {
            let left_sample_i128 = source_samples[source_index] as i128;
            let right_sample_i128 = source_samples[source_index + 1] as i128;
            let sample_delta_i128 = right_sample_i128 - left_sample_i128;
            let denom_i128 = TARGET_HZ as i128;
            let numerator_i128 = sample_delta_i128 * source_fraction_numerator_u128 as i128;
            let rounded_i128 = if numerator_i128 >= 0 {
                (numerator_i128 + (denom_i128 / 2)) / denom_i128
            } else {
                (numerator_i128 - (denom_i128 / 2)) / denom_i128
            };
            clamp_i64_to_i16((left_sample_i128 + rounded_i128) as i64)
        };

        sample_index += 1;
    }

    PcmClip {
        samples: resampled_samples,
    }
}

// Must be `pub` because platform-crate `device_loop` and play functions call
// this directly.
#[doc(hidden)]
pub const fn decode_adpcm_nibble_const(
    adpcm_nibble: u8,
    predictor_i32: &mut i32,
    step_index_i32: &mut i32,
) -> i16 {
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

    if *predictor_i32 < i16::MIN as i32 {
        *predictor_i32 = i16::MIN as i32;
    } else if *predictor_i32 > i16::MAX as i32 {
        *predictor_i32 = i16::MAX as i32;
    }
    *step_index_i32 += ADPCM_INDEX_TABLE[adpcm_nibble as usize];
    if *step_index_i32 < 0 {
        *step_index_i32 = 0;
    } else if *step_index_i32 > 88 {
        *step_index_i32 = 88;
    }

    *predictor_i32 as i16
}

const fn encode_adpcm_nibble(
    target_sample_i32: i32,
    predictor_i32: &mut i32,
    step_index_i32: &mut i32,
) -> u8 {
    let step = ADPCM_STEP_TABLE[*step_index_i32 as usize];
    let mut diff = target_sample_i32 - *predictor_i32;
    let mut adpcm_nibble = 0_u8;
    if diff < 0 {
        adpcm_nibble |= 0x08;
        diff = -diff;
    }

    let mut delta = step >> 3;
    if diff >= step {
        adpcm_nibble |= 0x04;
        diff -= step;
        delta += step;
    }
    if diff >= (step >> 1) {
        adpcm_nibble |= 0x02;
        diff -= step >> 1;
        delta += step >> 1;
    }
    if diff >= (step >> 2) {
        adpcm_nibble |= 0x01;
        delta += step >> 2;
    }

    if (adpcm_nibble & 0x08) != 0 {
        *predictor_i32 -= delta;
    } else {
        *predictor_i32 += delta;
    }

    if *predictor_i32 < i16::MIN as i32 {
        *predictor_i32 = i16::MIN as i32;
    } else if *predictor_i32 > i16::MAX as i32 {
        *predictor_i32 = i16::MAX as i32;
    }
    *step_index_i32 += ADPCM_INDEX_TABLE[adpcm_nibble as usize];
    if *step_index_i32 < 0 {
        *step_index_i32 = 0;
    } else if *step_index_i32 > 88 {
        *step_index_i32 = 88;
    }

    adpcm_nibble
}

// ============================================================================
// Macros
// ============================================================================

#[doc(hidden)]
#[macro_export]
macro_rules! pcm_clip {
    // TODO_NIGHTLY When nightly feature `decl_macro` becomes stable, change this
    // code by replacing `#[macro_export] macro_rules!` with module-scoped `pub macro`
    // so macro visibility and helper exposure can be controlled more precisely.
    ($($tt:tt)*) => { $crate::__audio_clip_parse! { $($tt)* } };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __audio_clip_parse {
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $source_sample_rate_hz:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $target_sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $source_sample_rate_hz:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr,
            $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $target_sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
            target_sample_rate_hz: $sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $sample_rate_hz:expr,
            $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
            target_sample_rate_hz: $sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $sample_rate_hz:expr,
            $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
            target_sample_rate_hz: $sample_rate_hz,
        }
    };
    // Alias: `source_sample_rate_hz:` is accepted as a synonym for `sample_rate_hz:`.
    (
        $vis:vis $name:ident {
            file: $file:expr,
            source_sample_rate_hz: $source_sample_rate_hz:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $target_sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            source_sample_rate_hz: $sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
            target_sample_rate_hz: $sample_rate_hz,
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __audio_clip_dispatch {
    (
        vis: $vis:vis,
        name: $name:ident,
        file: $file:expr,
        source_sample_rate_hz: $source_sample_rate_hz:expr,
        target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
    ) => {
        $crate::__audio_clip_impl! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $target_sample_rate_hz,
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __audio_clip_impl {
    (
        vis: $vis:vis,
        name: $name:ident,
        file: $file:expr,
        source_sample_rate_hz: $source_sample_rate_hz:expr,
        target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
    ) => {
        $crate::__paste! {
            const [<$name:upper _SOURCE_SAMPLE_RATE_HZ>]: u32 = $source_sample_rate_hz;
            const [<$name:upper _TARGET_SAMPLE_RATE_HZ>]: u32 = $target_sample_rate_hz;

            #[allow(non_snake_case)]
            #[doc = concat!(
                "Audio clip module generated by [`pcm_clip!`](macro@crate::audio_player::pcm_clip).\n\n",
                "[`SAMPLE_RATE_HZ`](Self::SAMPLE_RATE_HZ), ",
                "[`PCM_SAMPLE_COUNT`](Self::PCM_SAMPLE_COUNT), ",
                "[`ADPCM_DATA_LEN`](Self::ADPCM_DATA_LEN), ",
                "[`pcm_clip`](Self::pcm_clip), ",
                "and [`adpcm_clip`](Self::adpcm_clip)."
            )]
            $vis mod $name {
                // TODO_NIGHTLY When nightly feature inherent_associated_types becomes stable,
                // change generated clip items from a module to inherent associated items on a struct.
                const SOURCE_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _SOURCE_SAMPLE_RATE_HZ>];
                const TARGET_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _TARGET_SAMPLE_RATE_HZ>];
                #[doc = "Sample rate in hertz for this generated clip output."]
                pub const SAMPLE_RATE_HZ: u32 = TARGET_SAMPLE_RATE_HZ;
                const AUDIO_SAMPLE_BYTES_LEN: usize = include_bytes!($file).len();
                const SOURCE_SAMPLE_COUNT: usize = AUDIO_SAMPLE_BYTES_LEN / 2;
                #[doc = "Number of samples for uncompressed (PCM) version of this clip."]
                pub const PCM_SAMPLE_COUNT: usize = $crate::audio_player::__resampled_sample_count(
                    SOURCE_SAMPLE_COUNT,
                    SOURCE_SAMPLE_RATE_HZ,
                    TARGET_SAMPLE_RATE_HZ,
                );
                #[doc = "Byte length for compressed (ADPCM) encoding this clip."]
                pub const ADPCM_DATA_LEN: usize =
                    $crate::audio_player::__adpcm_data_len_for_pcm_samples(PCM_SAMPLE_COUNT);

                #[allow(dead_code)]
                type SourcePcmClip = $crate::audio_player::PcmClipBuf<
                    { SOURCE_SAMPLE_RATE_HZ },
                    { SOURCE_SAMPLE_COUNT },
                >;

                #[doc = "`const` function that returns the uncompressed (PCM) version of this clip."]
                #[must_use]
                pub const fn pcm_clip() -> $crate::audio_player::PcmClipBuf<
                    { SAMPLE_RATE_HZ },
                    { PCM_SAMPLE_COUNT },
                > {
                    assert!(
                        AUDIO_SAMPLE_BYTES_LEN % 2 == 0,
                        "audio byte length must be even for s16le"
                    );

                    let audio_sample_s16le: &[u8; AUDIO_SAMPLE_BYTES_LEN] = include_bytes!($file);
                    let mut samples = [0_i16; SOURCE_SAMPLE_COUNT];
                    let mut sample_index = 0_usize;
                    while sample_index < SOURCE_SAMPLE_COUNT {
                        let byte_index = sample_index * 2;
                        samples[sample_index] = i16::from_le_bytes([
                            audio_sample_s16le[byte_index],
                            audio_sample_s16le[byte_index + 1],
                        ]);
                        sample_index += 1;
                    }
                    $crate::audio_player::__resample_pcm_clip::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                        TARGET_SAMPLE_RATE_HZ,
                        PCM_SAMPLE_COUNT,
                    >($crate::audio_player::__pcm_clip_from_samples::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                    >(samples))
                }

                #[doc = "`const` function that returns the compressed (ADPCM) encoding for this clip."]
                #[must_use]
                pub const fn adpcm_clip() -> $crate::audio_player::AdpcmClipBuf<
                    { SAMPLE_RATE_HZ },
                    { ADPCM_DATA_LEN },
                > {
                    pcm_clip().with_adpcm::<ADPCM_DATA_LEN>()
                }

            }
        }
    };
}

#[doc = "Macro to \"compile in\" a compressed (ADPCM) WAV clip from an external file (includes syntax details)."]
#[doc = include_str!("audio_player/adpcm_clip_docs.md")]
#[doc = include_str!("audio_player/audio_prep_steps_1_2.md")]
#[doc = include_str!("audio_player/adpcm_clip_step_3.md")]
#[doc(inline)]
pub use crate::adpcm_clip;

#[doc(hidden)]
#[macro_export]
macro_rules! adpcm_clip {
    ($($tt:tt)*) => { $crate::__adpcm_clip_parse! { $($tt)* } };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __adpcm_clip_parse {
    (
        $vis:vis $name:ident {
            file: $file:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__paste! {
            const [<$name:upper _TARGET_SAMPLE_RATE_HZ>]: u32 = $target_sample_rate_hz;

            #[allow(non_snake_case)]
            #[allow(missing_docs)]
            $vis mod $name {
                const PARSED_WAV: $crate::audio_player::ParsedAdpcmWavHeader =
                    $crate::audio_player::__parse_adpcm_wav_header(include_bytes!($file));
                const SOURCE_SAMPLE_RATE_HZ: u32 = PARSED_WAV.sample_rate_hz;
                const TARGET_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _TARGET_SAMPLE_RATE_HZ>];
                pub const SAMPLE_RATE_HZ: u32 = TARGET_SAMPLE_RATE_HZ;

                const SOURCE_SAMPLE_COUNT: usize = PARSED_WAV.sample_count;
                #[doc = "Number of samples for uncompressed (PCM) version of this clip."]
                pub const PCM_SAMPLE_COUNT: usize = $crate::audio_player::__resampled_sample_count(
                    SOURCE_SAMPLE_COUNT,
                    SOURCE_SAMPLE_RATE_HZ,
                    TARGET_SAMPLE_RATE_HZ,
                );
                const BLOCK_ALIGN: usize = PARSED_WAV.block_align;
                const SOURCE_DATA_LEN: usize = PARSED_WAV.data_chunk_len;
                #[doc = "Byte length for compressed (ADPCM) encoding this clip."]
                pub const ADPCM_DATA_LEN: usize = if TARGET_SAMPLE_RATE_HZ == SOURCE_SAMPLE_RATE_HZ {
                    SOURCE_DATA_LEN
                } else {
                    $crate::audio_player::__adpcm_data_len_for_pcm_samples_with_block_align(
                        PCM_SAMPLE_COUNT,
                        BLOCK_ALIGN,
                    )
                };
                type SourceAdpcmClip = $crate::audio_player::AdpcmClipBuf<SOURCE_SAMPLE_RATE_HZ, SOURCE_DATA_LEN>;

                #[must_use]
                const fn source_adpcm_clip() -> SourceAdpcmClip {
                    let wav_bytes = include_bytes!($file);
                    let parsed_wav = $crate::audio_player::__parse_adpcm_wav_header(wav_bytes);
                    assert!(parsed_wav.block_align <= u16::MAX as usize, "block_align too large");
                    assert!(
                        parsed_wav.samples_per_block <= u16::MAX as usize,
                        "samples_per_block too large"
                    );

                    let mut adpcm_data = [0_u8; SOURCE_DATA_LEN];
                    let mut data_index = 0usize;
                    while data_index < SOURCE_DATA_LEN {
                        adpcm_data[data_index] = wav_bytes[parsed_wav.data_chunk_start + data_index];
                        data_index += 1;
                    }

                    $crate::audio_player::__adpcm_clip_from_parts(
                        parsed_wav.block_align as u16,
                        parsed_wav.samples_per_block as u16,
                        parsed_wav.sample_count,
                        adpcm_data,
                    )
                }

                #[doc = "`const` function that returns the uncompressed (PCM) version of this clip."]
                #[must_use]
                pub const fn pcm_clip() -> $crate::audio_player::PcmClipBuf<SAMPLE_RATE_HZ, PCM_SAMPLE_COUNT> {
                    $crate::audio_player::__resample_pcm_clip::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                        TARGET_SAMPLE_RATE_HZ,
                        PCM_SAMPLE_COUNT,
                    >(source_adpcm_clip().with_pcm::<SOURCE_SAMPLE_COUNT>())
                }

                #[doc = "`const` function that returns the compressed (ADPCM) encoding for this clip."]
                #[must_use]
                pub const fn adpcm_clip() -> $crate::audio_player::AdpcmClipBuf<SAMPLE_RATE_HZ, ADPCM_DATA_LEN> {
                    if TARGET_SAMPLE_RATE_HZ == SOURCE_SAMPLE_RATE_HZ {
                        let wav_bytes = include_bytes!($file);
                        let parsed_wav = $crate::audio_player::__parse_adpcm_wav_header(wav_bytes);
                        assert!(parsed_wav.block_align <= u16::MAX as usize, "block_align too large");
                        assert!(
                            parsed_wav.samples_per_block <= u16::MAX as usize,
                            "samples_per_block too large"
                        );
                        let mut adpcm_data = [0_u8; ADPCM_DATA_LEN];
                        let mut data_index = 0usize;
                        while data_index < ADPCM_DATA_LEN {
                            adpcm_data[data_index] =
                                wav_bytes[parsed_wav.data_chunk_start + data_index];
                            data_index += 1;
                        }
                        $crate::audio_player::__adpcm_clip_from_parts(
                            parsed_wav.block_align as u16,
                            parsed_wav.samples_per_block as u16,
                            parsed_wav.sample_count,
                            adpcm_data,
                        )
                    } else {
                        $crate::audio_player::__pcm_with_adpcm_block_align::<
                            SAMPLE_RATE_HZ,
                            PCM_SAMPLE_COUNT,
                            ADPCM_DATA_LEN,
                        >(&pcm_clip(), BLOCK_ALIGN)
                    }
                }

            }
        }
    };

    (
        $vis:vis $name:ident {
            file: $file:expr $(,)?
        }
    ) => {
        $crate::__adpcm_clip_parse! {
            $vis $name {
                file: $file,
                target_sample_rate_hz: $crate::audio_player::__parse_adpcm_wav_header(include_bytes!($file)).sample_rate_hz,
            }
        }
    };
}

/// Macro to create an audio clip of a musical tone.
///
/// Examples:
/// - `tone!(440, VOICE_22050_HZ, Duration::from_millis(500))`
/// - `tone!(440, AudioPlayer8::SAMPLE_RATE_HZ, Duration::from_millis(500))`
///
/// The result is an uncompressed (PCM) clip.
/// (It does not use compressed because ADPCM sounds poor for pure sine tones.)
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[doc(hidden)]
#[macro_export]
macro_rules! tone {
    ($frequency_hz:expr, $sample_rate_hz:expr, $duration:expr) => {
        $crate::audio_player::__tone_pcm_clip_with_duration::<
            { $sample_rate_hz },
            { $crate::audio_player::__samples_for_duration($duration, $sample_rate_hz) },
        >($frequency_hz, $duration)
    };
}

#[doc = "Macro to \"compile in\" an uncompressed (PCM) clip from an external file (includes syntax details)."]
#[doc = include_str!("audio_player/pcm_clip_docs.md")]
#[doc = include_str!("audio_player/audio_prep_steps_1_2.md")]
#[doc = include_str!("audio_player/pcm_clip_step_3.md")]
#[doc(inline)]
pub use crate::pcm_clip;
#[doc(inline)]
pub use crate::tone;
