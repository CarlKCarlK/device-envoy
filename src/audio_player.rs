//! A device abstraction for preemptive PCM clip playback over PIO I2S.
//!
//! See [`AudioPlayer`] for the core API and [`audio_player!`] for the generated
//! device pattern that pins down `PIO`, `DMA`, and `max_clips`.
//TODO0 Review this code

use core::ops::ControlFlow;
use core::sync::atomic::{AtomicI32, Ordering};

use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::Pin;
use embassy_rp::pio::{Instance, Pio, PioPin};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

/// Audio sample rate used by `AudioPlayer` playback.
pub const SAMPLE_RATE_HZ: u32 = 22_050;
const BIT_DEPTH_BITS: u32 = 16;
const SAMPLE_BUFFER_LEN: usize = 256;
/// Maximum linear volume scale value.
pub const MAX_VOLUME: i16 = i16::MAX;
const SPINAL_TAP_11_LINEAR: i32 = 36_765;
const I16_ABS_MAX_I64: i64 = -(i16::MIN as i64);

/// Linear volume scale used by [`AudioClipN::with_volume`].
///
/// `Volume` is a value object, not device state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volume(i32);

impl Volume {
    /// Silence.
    pub const MUTE: Self = Self(0);

    /// Full-scale linear volume (no attenuation).
    pub const FULL_SCALE: Self = Self(MAX_VOLUME as i32);
    /// Alias for [`Self::FULL_SCALE`].
    pub const FULL: Self = Self::FULL_SCALE;

    /// Creates a volume from a percentage of full scale.
    ///
    /// Values above `100` are clamped to `100`.
    #[must_use]
    pub const fn percent(percent: u8) -> Self {
        let percent = if percent > 100 { 100 } else { percent };
        let value_i32 = (percent as i32 * MAX_VOLUME as i32) / 100;
        Self(value_i32)
    }

    /// Creates a volume from decibel attenuation.
    ///
    /// - `0 dB` is full-scale.
    /// - Positive values map to full-scale.
    /// - Negative values attenuate.
    #[must_use]
    pub const fn db(db: i8) -> Self {
        if db >= 0 {
            return Self::FULL_SCALE;
        }
        if db == i8::MIN {
            return Self::MUTE;
        }

        // Fixed-point multiplier for 10^(-1/20) (approximately -1 dB in amplitude).
        const DB_STEP_Q15: i32 = 29_205;
        const ONE_Q15: i32 = 32_768;
        const ROUND_Q15: i32 = 16_384;

        let attenuation_db_u8 = (-db) as u8;
        let mut scale_q15_i32 = ONE_Q15;
        let mut attenuation_index = 0_u8;
        while attenuation_index < attenuation_db_u8 {
            scale_q15_i32 = (scale_q15_i32 * DB_STEP_Q15 + ROUND_Q15) / ONE_Q15;
            attenuation_index += 1;
        }
        let value_i32 = (MAX_VOLUME as i32 * scale_q15_i32 + ROUND_Q15) / ONE_Q15;
        Self(value_i32)
    }

    /// Creates a humorous "goes to 11" demo volume scale.
    ///
    /// Range:
    /// - `0` -> mute
    /// - `10` -> full-scale
    /// - `11` -> slight overdrive (+1 dB)
    ///
    /// Values above `11` saturate to `11`.
    #[must_use]
    pub const fn spinal_tap(spinal_tap: u8) -> Self {
        let spinal_tap = if spinal_tap > 11 { 11 } else { spinal_tap };
        match spinal_tap {
            0 => Self::MUTE,
            1 => Self::db(-40),
            2 => Self::db(-30),
            3 => Self::db(-24),
            4 => Self::db(-18),
            5 => Self::db(-12),
            6 => Self::db(-9),
            7 => Self::db(-6),
            8 => Self::db(-3),
            9 => Self::db(-1),
            10 => Self::FULL_SCALE,
            11 => Self::from_linear(SPINAL_TAP_11_LINEAR),
            _ => Self::MUTE,
        }
    }

    #[must_use]
    const fn linear(self) -> i32 {
        self.0
    }

    #[must_use]
    const fn from_linear(linear: i32) -> Self {
        Self(linear)
    }
}

/// Returns how many samples are needed for a duration in milliseconds.
///
/// Use this in const contexts to size static audio arrays.
#[must_use]
pub const fn samples_for_duration_ms(duration_ms: u32, sample_rate_hz: u32) -> usize {
    assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");
    ((duration_ms as u64 * sample_rate_hz as u64) / 1_000) as usize
}

/// Shorthand alias for [`samples_for_duration_ms`].
#[must_use]
pub const fn samples_ms(duration_ms: u32, sample_rate_hz: u32) -> usize {
    samples_for_duration_ms(duration_ms, sample_rate_hz)
}

#[must_use]
const fn silence_samples<const SAMPLE_COUNT: usize>() -> [i16; SAMPLE_COUNT] {
    [0; SAMPLE_COUNT]
}

#[must_use]
const fn tone_with_sample_rate<const SAMPLE_COUNT: usize>(
    frequency_hz: u32,
    sample_rate_hz: u32,
) -> [i16; SAMPLE_COUNT] {
    assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");

    let mut audio_sample_i16 = [0_i16; SAMPLE_COUNT];
    let phase_step_u64 = ((frequency_hz as u64) << 32) / sample_rate_hz as u64;
    let phase_step_u32 = phase_step_u64 as u32;
    let mut phase_u32 = 0_u32;

    let mut sample_index = 0_usize;
    while sample_index < SAMPLE_COUNT {
        audio_sample_i16[sample_index] = sine_sample_from_phase(phase_u32);
        phase_u32 = phase_u32.wrapping_add(phase_step_u32);
        sample_index += 1;
    }

    audio_sample_i16
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

#[must_use]
const fn with_volume<const SAMPLE_COUNT: usize>(
    audio_sample_i16: &[i16; SAMPLE_COUNT],
    volume: Volume,
) -> [i16; SAMPLE_COUNT] {
    let mut volume_adjusted_audio_sample_i16 = [0_i16; SAMPLE_COUNT];
    let mut sample_index = 0_usize;
    while sample_index < SAMPLE_COUNT {
        volume_adjusted_audio_sample_i16[sample_index] =
            scale_sample(audio_sample_i16[sample_index], volume);
        sample_index += 1;
    }
    volume_adjusted_audio_sample_i16
}

#[inline]
const fn scale_sample(sample_i16: i16, volume: Volume) -> i16 {
    if volume.linear() == 0 {
        return 0;
    }
    // Use signed full-scale magnitude (32768) so i16::MIN is handled correctly.
    // `Volume::FULL_SCALE` is 32767, so add one to map it to exact unity gain.
    let unity_scaled_volume_i64 = volume.linear() as i64 + 1;
    let scaled_i64 = (sample_i16 as i64 * unity_scaled_volume_i64) / I16_ABS_MAX_I64;
    clamp_i64_to_i16(scaled_i64)
}

#[inline]
const fn scale_linear(linear_i32: i32, volume: Volume) -> i32 {
    if volume.linear() == 0 || linear_i32 == 0 {
        return 0;
    }
    let unity_scaled_volume_i64 = volume.linear() as i64 + 1;
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

/// End-of-sequence behavior for playback.
///
/// `AudioPlayer` supports looping or stopping at the end of a clip sequence.
pub enum AtEnd {
    /// Repeat the full clip sequence forever.
    Loop,
    /// Stop after one full clip sequence pass.
    Stop,
}

/// Self-describing PCM clip view used by playback APIs.
///
/// `AudioClip` is unsized so clips with different lengths can share one
/// reference type (`&AudioClip`) in the same playback sequence.
#[repr(C)]
pub struct AudioClip<T: ?Sized = [i16]> {
    sample_rate_hz: u32,
    samples: T,
}

/// Unsized clip reference type used for playback.
pub type AudioClipRef = AudioClip<[i16]>;

impl AudioClipRef {
    /// Clip sample rate in hertz.
    #[must_use]
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Clip samples as an i16 PCM slice.
    #[must_use]
    pub(crate) const fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Number of PCM samples in this clip.
    #[must_use]
    pub const fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

/// Sized, const-friendly PCM clip storage.
pub type AudioClipN<const SAMPLE_COUNT: usize> = AudioClip<[i16; SAMPLE_COUNT]>;

impl<const SAMPLE_COUNT: usize> AudioClip<[i16; SAMPLE_COUNT]> {
    /// Creates a clip from sample rate and PCM samples.
    #[must_use]
    const fn new(sample_rate_hz: u32, samples: [i16; SAMPLE_COUNT]) -> Self {
        assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");
        Self {
            sample_rate_hz,
            samples,
        }
    }

    /// Clip sample rate in hertz.
    #[must_use]
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Number of PCM samples in this clip.
    #[must_use]
    pub const fn sample_count(&self) -> usize {
        SAMPLE_COUNT
    }

    /// Returns this clip as an unsized clip view.
    #[must_use]
    pub fn as_clip(&'static self) -> &'static AudioClipRef {
        self
    }

    /// Returns a new clip with linear volume scaling applied.
    #[must_use]
    pub const fn with_volume(self, volume: Volume) -> Self {
        Self::new(self.sample_rate_hz, with_volume(&self.samples, volume))
    }

    /// Creates a silent clip at `sample_rate_hz`.
    #[must_use]
    pub const fn silence(sample_rate_hz: u32) -> Self {
        Self::new(sample_rate_hz, silence_samples::<SAMPLE_COUNT>())
    }

    /// Creates a sine-wave clip at `sample_rate_hz`.
    #[must_use]
    pub const fn tone(sample_rate_hz: u32, frequency_hz: u32) -> Self {
        assert!(sample_rate_hz > 0, "sample_rate_hz must be > 0");
        Self::new(
            sample_rate_hz,
            tone_with_sample_rate::<SAMPLE_COUNT>(frequency_hz, sample_rate_hz),
        )
    }

    /// Creates a clip from little-endian s16 PCM bytes.
    ///
    /// `AUDIO_SAMPLE_BYTES_LEN` must be exactly `SAMPLE_COUNT * 2`.
    #[must_use]
    pub const fn from_s16le_bytes<const AUDIO_SAMPLE_BYTES_LEN: usize>(
        sample_rate_hz: u32,
        audio_sample_s16le: &[u8; AUDIO_SAMPLE_BYTES_LEN],
    ) -> Self {
        assert!(
            AUDIO_SAMPLE_BYTES_LEN == SAMPLE_COUNT * 2,
            "audio byte length must equal sample_count * 2"
        );

        let mut samples = [0_i16; SAMPLE_COUNT];
        let mut sample_index = 0_usize;
        while sample_index < SAMPLE_COUNT {
            let byte_index = sample_index * 2;
            samples[sample_index] =
                i16::from_le_bytes([audio_sample_s16le[byte_index], audio_sample_s16le[byte_index + 1]]);
            sample_index += 1;
        }

        Self::new(sample_rate_hz, samples)
    }
}

/// Supported clip input types for [`AudioPlayer::play_iter`].
pub trait IntoAudioClipRef {
    /// Converts this clip input into a static audio clip reference.
    fn into_audio_clip(self) -> &'static AudioClipRef;
}

impl IntoAudioClipRef for &'static AudioClipRef {
    fn into_audio_clip(self) -> &'static AudioClipRef {
        self
    }
}

impl<const SAMPLE_COUNT: usize> IntoAudioClipRef for &'static AudioClipN<SAMPLE_COUNT> {
    fn into_audio_clip(self) -> &'static AudioClipRef {
        self
    }
}

enum AudioCommand<const MAX_CLIPS: usize> {
    Play {
        audio_clips: Vec<&'static AudioClipRef, MAX_CLIPS>,
        at_end: AtEnd,
    },
    Stop,
}

/// Static resources for [`AudioPlayer`].
pub struct AudioPlayerStatic<const MAX_CLIPS: usize> {
    command_signal: Signal<CriticalSectionRawMutex, AudioCommand<MAX_CLIPS>>,
    max_volume_linear: i32,
    runtime_volume_relative_linear: AtomicI32,
}

impl<const MAX_CLIPS: usize> AudioPlayerStatic<MAX_CLIPS> {
    /// Creates static resources for a player.
    #[must_use]
    pub const fn new_static() -> Self {
        Self::new_static_with_max_volume_and_initial_volume(
            Volume::FULL_SCALE,
            Volume::FULL_SCALE,
        )
    }

    /// Creates static resources for a player with a runtime volume ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume(max_volume: Volume) -> Self {
        Self::new_static_with_max_volume_and_initial_volume(max_volume, Volume::FULL_SCALE)
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
            max_volume_linear: max_volume.linear(),
            runtime_volume_relative_linear: AtomicI32::new(initial_volume.linear()),
        }
    }

    fn signal(&self, audio_command: AudioCommand<MAX_CLIPS>) {
        self.command_signal.signal(audio_command);
    }

    async fn wait(&self) -> AudioCommand<MAX_CLIPS> {
        self.command_signal.wait().await
    }

    fn set_runtime_volume(&self, volume: Volume) {
        self.runtime_volume_relative_linear
            .store(volume.linear(), Ordering::Relaxed);
    }

    fn runtime_volume(&self) -> Volume {
        Volume::from_linear(self.runtime_volume_relative_linear.load(Ordering::Relaxed))
    }

    fn effective_runtime_volume(&self) -> Volume {
        let runtime_volume_relative = self.runtime_volume();
        Volume::from_linear(scale_linear(self.max_volume_linear, runtime_volume_relative))
    }

    fn max_volume(&self) -> Volume {
        Volume::from_linear(self.max_volume_linear)
    }
}

/// Plays static PCM clips with preemptive command handling in the background device task.
///
/// See the [`audio_player!`] macro for the normal construction pattern.
pub struct AudioPlayer<const MAX_CLIPS: usize> {
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
}

impl<const MAX_CLIPS: usize> AudioPlayer<MAX_CLIPS> {
    /// Creates static resources for a player.
    #[must_use]
    pub const fn new_static() -> AudioPlayerStatic<MAX_CLIPS> {
        AudioPlayerStatic::new_static()
    }

    /// Creates static resources for a player with a runtime volume ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume(max_volume: Volume) -> AudioPlayerStatic<MAX_CLIPS> {
        AudioPlayerStatic::new_static_with_max_volume(max_volume)
    }

    /// Creates static resources for a player with a runtime volume ceiling
    /// and an initial runtime volume relative to that ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume_and_initial_volume(
        max_volume: Volume,
        initial_volume: Volume,
    ) -> AudioPlayerStatic<MAX_CLIPS> {
        AudioPlayerStatic::new_static_with_max_volume_and_initial_volume(max_volume, initial_volume)
    }

    /// Creates a player handle. The device task must already be running.
    #[must_use]
    pub const fn new(audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>) -> Self {
        Self {
            audio_player_static,
        }
    }

    /// Starts playback of one or more static PCM clips.
    ///
    /// This array-based API supports concise mixed-length clip literals like
    /// `[&tone_a4, &silence_100ms, &tone_a4]`.
    ///
    /// The clips are copied into a fixed-capacity clip list defined by `MAX_CLIPS`.
    /// A newer call to [`Self::play`] interrupts current playback as soon as possible
    /// (at the next DMA chunk boundary).
    pub fn play<const CLIP_COUNT: usize>(
        &self,
        audio_clips: [&'static AudioClipRef; CLIP_COUNT],
        at_end: AtEnd,
    ) {
        self.play_iter(audio_clips, at_end);
    }

    /// Starts playback from a generic iterator of static clip-like values.
    pub fn play_iter<I>(&self, audio_clips: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: IntoAudioClipRef,
    {
        assert!(MAX_CLIPS > 0, "play disabled: max_clips is 0");
        let mut audio_clip_sequence: Vec<&'static AudioClipRef, MAX_CLIPS> = Vec::new();
        for audio_clip in audio_clips {
            let audio_clip = audio_clip.into_audio_clip();
            assert!(
                audio_clip.sample_rate_hz() == SAMPLE_RATE_HZ,
                "clip sample rate ({}) must match player sample rate ({})",
                audio_clip.sample_rate_hz(),
                SAMPLE_RATE_HZ
            );
            assert!(
                audio_clip_sequence.push(audio_clip).is_ok(),
                "play sequence fits within max_clips"
            );
        }
        assert!(
            !audio_clip_sequence.is_empty(),
            "play requires at least one clip"
        );

        self.audio_player_static.signal(AudioCommand::Play {
            audio_clips: audio_clip_sequence,
            at_end,
        });
    }

    /// Stops current playback as soon as possible.
    ///
    /// If playback is active, it is interrupted at the next DMA chunk boundary.
    pub fn stop(&self) {
        self.audio_player_static.signal(AudioCommand::Stop);
    }

    /// Sets runtime playback volume relative to [`Self::max_volume`].
    ///
    /// - `Volume::percent(100)` plays at exactly `max_volume`.
    /// - `Volume::percent(50)` plays at half of `max_volume`.
    ///
    /// This relative scale composes multiplicatively with any per-clip gain
    /// pre-applied via [`AudioClipN::with_volume`].
    pub fn set_volume(&self, volume: Volume) {
        self.audio_player_static.set_runtime_volume(volume);
    }

    /// Returns the current runtime playback volume relative to [`Self::max_volume`].
    #[must_use]
    pub fn volume(&self) -> Volume {
        self.audio_player_static.runtime_volume()
    }

    /// Returns the configured runtime volume ceiling.
    #[must_use]
    pub fn max_volume(&self) -> Volume {
        self.audio_player_static.max_volume()
    }
}

/// Trait mapping a PIO peripheral to its interrupt binding.
pub trait AudioPlayerPio: Instance {
    /// Interrupt binding type for this PIO resource.
    type Irqs: embassy_rp::interrupt::typelevel::Binding<
            <Self as Instance>::Interrupt,
            embassy_rp::pio::InterruptHandler<Self>,
        >;

    /// Returns interrupt bindings for this PIO resource.
    fn irqs() -> Self::Irqs;
}

impl AudioPlayerPio for embassy_rp::peripherals::PIO0 {
    type Irqs = crate::pio_irqs::Pio0Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio0Irqs
    }
}

impl AudioPlayerPio for embassy_rp::peripherals::PIO1 {
    type Irqs = crate::pio_irqs::Pio1Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio1Irqs
    }
}

#[cfg(feature = "pico2")]
impl AudioPlayerPio for embassy_rp::peripherals::PIO2 {
    type Irqs = crate::pio_irqs::Pio2Irqs;

    fn irqs() -> Self::Irqs {
        crate::pio_irqs::Pio2Irqs
    }
}

// Called by macro-generated code in downstream crates; must be public.
#[doc(hidden)]
pub async fn device_loop<
    const MAX_CLIPS: usize,
    PIO: AudioPlayerPio,
    DMA: Channel,
    DinPin: Pin + PioPin,
    BclkPin: Pin + PioPin,
    LrcPin: Pin + PioPin,
>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, DMA>,
    din_pin: Peri<'static, DinPin>,
    bclk_pin: Peri<'static, BclkPin>,
    lrc_pin: Peri<'static, LrcPin>,
) -> ! {
    let mut pio = Pio::new(pio, PIO::irqs());
    let pio_i2s_out_program = PioI2sOutProgram::new(&mut pio.common);
    let mut pio_i2s_out = PioI2sOut::new(
        &mut pio.common,
        pio.sm0,
        dma,
        din_pin,
        bclk_pin,
        lrc_pin,
        SAMPLE_RATE_HZ,
        BIT_DEPTH_BITS,
        &pio_i2s_out_program,
    );

    let _pio_i2s_out_program = pio_i2s_out_program;
    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    loop {
        let mut audio_command = audio_player_static.wait().await;

        loop {
            match audio_command {
                AudioCommand::Play {
                    audio_clips,
                    at_end,
                } => {
                    let next_audio_command = match at_end {
                        AtEnd::Loop => loop {
                            if let Some(next_audio_command) = play_clip_sequence_once(
                                &mut pio_i2s_out,
                                &audio_clips,
                                &mut sample_buffer,
                                audio_player_static,
                            )
                            .await
                            {
                                break Some(next_audio_command);
                            }
                        },
                        AtEnd::Stop => {
                            play_clip_sequence_once(
                                &mut pio_i2s_out,
                                &audio_clips,
                                &mut sample_buffer,
                                audio_player_static,
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

async fn play_clip_sequence_once<PIO: Instance, const MAX_CLIPS: usize>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_clips: &[&'static AudioClipRef],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
) -> Option<AudioCommand<MAX_CLIPS>> {
    for audio_clip in audio_clips {
        if let ControlFlow::Break(next_audio_command) = play_full_clip_once(
            pio_i2s_out,
            audio_clip,
            sample_buffer,
            audio_player_static,
        )
        .await
        {
            return Some(next_audio_command);
        }
    }
    None
}

async fn play_full_clip_once<PIO: Instance, const MAX_CLIPS: usize>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_clip: &AudioClipRef,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
) -> ControlFlow<AudioCommand<MAX_CLIPS>, ()> {
    for audio_sample_chunk in audio_clip.samples().chunks(SAMPLE_BUFFER_LEN) {
        let runtime_volume = audio_player_static.effective_runtime_volume();
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            // TODO0 should we preprocess all this? (moved from examples/audio.rs) (may no longer apply)
            let scaled_sample_value = scale_sample(sample_value, runtime_volume);
            *sample_buffer_slot = stereo_sample(scaled_sample_value);
        }

        sample_buffer[audio_sample_chunk.len()..].fill(stereo_sample(0));
        pio_i2s_out.write(sample_buffer).await;

        if let Some(next_audio_command) = audio_player_static.command_signal.try_take() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    ControlFlow::Continue(())
}

#[inline]
const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = sample as u16 as u32;
    (sample_bits << 16) | sample_bits
}

pub use paste;

/// Generates a named audio player type with fixed PIO/DMA/pin resources.
// TODO00 do pio and dma in macro (may no longer apply)
#[macro_export]
macro_rules! audio_player {
    ($($tt:tt)*) => { $crate::__audio_player_impl! { $($tt)* } };
}

/// Internal implementation macro for [`audio_player!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __audio_player_impl {
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: pub,
            name: $name,
            din_pin: _UNSET_,
            bclk_pin: _UNSET_,
            lrc_pin: _UNSET_,
            pio: PIO1,
            dma: DMA_CH0,
            max_clips: 16,
            max_volume: $crate::audio_player::Volume::FULL_SCALE,
            initial_volume: $crate::audio_player::Volume::FULL_SCALE,
            fields: [ $($fields)* ]
        }
    };

    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: _UNSET_,
            bclk_pin: _UNSET_,
            lrc_pin: _UNSET_,
            pio: PIO1,
            dma: DMA_CH0,
            max_clips: 16,
            max_volume: $crate::audio_player::Volume::FULL_SCALE,
            initial_volume: $crate::audio_player::Volume::FULL_SCALE,
            fields: [ $($fields)* ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ din_pin: $din_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin_value,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ bclk_pin: $bclk_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin_value,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ lrc_pin: $lrc_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin_value,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ pio: $pio_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio_value,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ dma: $dma_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma_value,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ max_clips: $max_clips_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips_value,
            max_volume: $max_volume,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ max_volume: $max_volume_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume_value,
            initial_volume: $initial_volume,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ initial_volume: $initial_volume_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            din_pin: $din_pin,
            bclk_pin: $bclk_pin,
            lrc_pin: $lrc_pin,
            pio: $pio,
            dma: $dma,
            max_clips: $max_clips,
            max_volume: $max_volume,
            initial_volume: $initial_volume_value,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:tt,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ volume: $volume_value:expr $(, $($rest:tt)* )? ]
    ) => {
        compile_error!("audio_player! field `volume` was renamed to `max_volume`");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: _UNSET_,
        bclk_pin: $bclk_pin:tt,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires din_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:ident,
        bclk_pin: _UNSET_,
        lrc_pin: $lrc_pin:tt,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires bclk_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:ident,
        bclk_pin: $bclk_pin:ident,
        lrc_pin: _UNSET_,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires lrc_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        din_pin: $din_pin:ident,
        bclk_pin: $bclk_pin:ident,
        lrc_pin: $lrc_pin:ident,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        $crate::audio_player::paste::paste! {
            static [<$name:upper _AUDIO_PLAYER_STATIC>]: $crate::audio_player::AudioPlayerStatic<$max_clips> =
                $crate::audio_player::AudioPlayer::<$max_clips>::new_static_with_max_volume_and_initial_volume(
                    $max_volume,
                    $initial_volume,
                );
            static [<$name:upper _AUDIO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            $vis struct $name {
                player: $crate::audio_player::AudioPlayer<$max_clips>,
            }

            impl $name {
                /// Sample rate used for PCM playback by this generated player type.
                pub const SAMPLE_RATE_HZ: u32 = $crate::audio_player::SAMPLE_RATE_HZ;

                /// Returns how many samples are needed for a duration in milliseconds
                /// at this player's sample rate.
                #[must_use]
                pub const fn samples_for_duration_ms(duration_ms: u32) -> usize {
                    $crate::audio_player::samples_for_duration_ms(duration_ms, Self::SAMPLE_RATE_HZ)
                }

                /// Shorthand alias for [`Self::samples_for_duration_ms`].
                #[must_use]
                pub const fn samples_ms(duration_ms: u32) -> usize {
                    Self::samples_for_duration_ms(duration_ms)
                }

                /// Creates a silent clip at this player's sample rate.
                #[must_use]
                pub const fn silence<const SAMPLE_COUNT: usize>(
                ) -> $crate::audio_player::AudioClipN<SAMPLE_COUNT> {
                    $crate::audio_player::AudioClipN::silence(Self::SAMPLE_RATE_HZ)
                }

                /// Creates a sine-wave clip at this player's sample rate.
                #[must_use]
                pub const fn tone<const SAMPLE_COUNT: usize>(
                    frequency_hz: u32,
                ) -> $crate::audio_player::AudioClipN<SAMPLE_COUNT> {
                    $crate::audio_player::AudioClipN::tone(Self::SAMPLE_RATE_HZ, frequency_hz)
                }

                /// Creates a clip from little-endian s16 PCM bytes at this player's sample rate.
                #[must_use]
                pub const fn clip_from_s16le_bytes<const SAMPLE_COUNT: usize, const AUDIO_SAMPLE_BYTES_LEN: usize>(
                    audio_sample_s16le: &[u8; AUDIO_SAMPLE_BYTES_LEN],
                ) -> $crate::audio_player::AudioClipN<SAMPLE_COUNT> {
                    $crate::audio_player::AudioClipN::from_s16le_bytes(
                        Self::SAMPLE_RATE_HZ,
                        audio_sample_s16le,
                    )
                }

                pub fn new(
                    din_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$din_pin>>,
                    bclk_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bclk_pin>>,
                    lrc_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$lrc_pin>>,
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let token = [<$name:snake _audio_player_task>](
                        &[<$name:upper _AUDIO_PLAYER_STATIC>],
                        pio.into(),
                        dma.into(),
                        din_pin.into(),
                        bclk_pin.into(),
                        lrc_pin.into(),
                    );
                    spawner.spawn(token)?;
                    let player = $crate::audio_player::AudioPlayer::new(&[<$name:upper _AUDIO_PLAYER_STATIC>]);
                    Ok([<$name:upper _AUDIO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::audio_player::AudioPlayer<$max_clips>;

                fn deref(&self) -> &Self::Target {
                    &self.player
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _audio_player_task>](
                audio_player_static: &'static $crate::audio_player::AudioPlayerStatic<$max_clips>,
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                din_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$din_pin>,
                bclk_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bclk_pin>,
                lrc_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$lrc_pin>,
            ) -> ! {
                $crate::audio_player::device_loop::<
                    $max_clips,
                    ::embassy_rp::peripherals::$pio,
                    ::embassy_rp::peripherals::$dma,
                    ::embassy_rp::peripherals::$din_pin,
                    ::embassy_rp::peripherals::$bclk_pin,
                    ::embassy_rp::peripherals::$lrc_pin,
                >(audio_player_static, pio, dma, din_pin, bclk_pin, lrc_pin).await
            }
        }
    };
}

pub use audio_player;
