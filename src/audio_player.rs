//! A device abstraction for preemptive PCM clip playback over PIO I2S.
//!
//! See [`AudioPlayer`] for the core API and [`audio_player!`] for the generated
//! device pattern that pins down `PIO`, `DMA`, and `max_clips`.
//TODO0 Review this code

use core::ops::ControlFlow;

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
const I16_ABS_MAX_I64: i64 = -(i16::MIN as i64);

/// Linear volume scale used by [`with_volume`].
///
/// `Volume` is a value object, not device state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volume(i16);

impl Volume {
    /// Silence.
    pub const MUTE: Self = Self(0);

    /// Full-scale linear volume (no attenuation).
    pub const FULL_SCALE: Self = Self(MAX_VOLUME);

    /// Creates a volume from a percentage of full scale.
    ///
    /// Values above `100` are clamped to `100`.
    #[must_use]
    pub const fn percent(percent: u8) -> Self {
        let percent = if percent > 100 { 100 } else { percent };
        let value_i32 = (percent as i32 * MAX_VOLUME as i32) / 100;
        Self(value_i32 as i16)
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
        Self(value_i32 as i16)
    }

    #[must_use]
    const fn linear(self) -> i16 {
        self.0
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

/// Generates a silent i16 PCM clip with `SAMPLE_COUNT` samples.
#[must_use]
pub const fn silence<const SAMPLE_COUNT: usize>() -> [i16; SAMPLE_COUNT] {
    [0; SAMPLE_COUNT]
}

/// Generates an i16 PCM sine-wave clip with `SAMPLE_COUNT` samples.
///
/// - `frequency_hz`: Tone frequency in Hz.
/// - `duration` is represented by `SAMPLE_COUNT`.
#[must_use]
pub const fn tone<const SAMPLE_COUNT: usize>(frequency_hz: u32) -> [i16; SAMPLE_COUNT] {
    tone_with_sample_rate::<SAMPLE_COUNT>(frequency_hz, SAMPLE_RATE_HZ)
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

/// Returns a new clip with linear volume scaling applied.
///
/// This transform is pure and const-evaluable.
#[must_use]
pub const fn with_volume<const SAMPLE_COUNT: usize>(
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

/// Supported clip input types for [`AudioPlayer::play_iter`].
pub trait AudioClip {
    /// Converts this clip input into a static i16 PCM slice.
    fn into_audio_clip(self) -> &'static [i16];
}

impl AudioClip for &'static [i16] {
    fn into_audio_clip(self) -> &'static [i16] {
        self
    }
}

impl<const SAMPLE_COUNT: usize> AudioClip for &'static [i16; SAMPLE_COUNT] {
    fn into_audio_clip(self) -> &'static [i16] {
        self
    }
}

enum AudioCommand<const MAX_CLIPS: usize> {
    Play {
        audio_clips: Vec<&'static [i16], MAX_CLIPS>,
        at_end: AtEnd,
    },
    Stop,
}

/// Static resources for [`AudioPlayer`].
pub struct AudioPlayerStatic<const MAX_CLIPS: usize> {
    command_signal: Signal<CriticalSectionRawMutex, AudioCommand<MAX_CLIPS>>,
}

impl<const MAX_CLIPS: usize> AudioPlayerStatic<MAX_CLIPS> {
    /// Creates static resources for a player.
    #[must_use]
    pub const fn new_static() -> Self {
        Self {
            command_signal: Signal::new(),
        }
    }

    fn signal(&self, audio_command: AudioCommand<MAX_CLIPS>) {
        self.command_signal.signal(audio_command);
    }

    async fn wait(&self) -> AudioCommand<MAX_CLIPS> {
        self.command_signal.wait().await
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
        audio_clips: [&'static [i16]; CLIP_COUNT],
        at_end: AtEnd,
    ) {
        self.play_iter(audio_clips, at_end);
    }

    /// Starts playback from a generic iterator of static clip-like values.
    ///
    /// This accepts iterators of `&'static [i16]` and `&'static [i16; N]`.
    pub fn play_iter<I>(&self, audio_clips: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: AudioClip,
    {
        assert!(MAX_CLIPS > 0, "play disabled: max_clips is 0");
        let mut audio_clip_sequence: Vec<&'static [i16], MAX_CLIPS> = Vec::new();
        for audio_clip in audio_clips {
            let audio_clip = audio_clip.into_audio_clip();
            audio_clip_sequence
                .push(audio_clip)
                .expect("play sequence fits within max_clips");
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
    audio_clips: &[&'static [i16]],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
) -> Option<AudioCommand<MAX_CLIPS>> {
    for audio_sample_i16 in audio_clips {
        if let ControlFlow::Break(next_audio_command) = play_full_clip_once(
            pio_i2s_out,
            audio_sample_i16,
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
    audio_sample_i16: &[i16],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS>,
) -> ControlFlow<AudioCommand<MAX_CLIPS>, ()> {
    for audio_sample_chunk in audio_sample_i16.chunks(SAMPLE_BUFFER_LEN) {
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            // TODO0 should we preprocess all this? (moved from examples/audio.rs) (may no longer apply)
            *sample_buffer_slot = stereo_sample(sample_value);
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
            fields: [ $($($rest)*)? ]
        }
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
        fields: [ ]
    ) => {
        $crate::audio_player::paste::paste! {
            static [<$name:upper _AUDIO_PLAYER_STATIC>]: $crate::audio_player::AudioPlayerStatic<$max_clips> =
                $crate::audio_player::AudioPlayer::<$max_clips>::new_static();
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

                /// Generates a silent i16 PCM clip with `SAMPLE_COUNT` samples.
                #[must_use]
                pub const fn silence<const SAMPLE_COUNT: usize>() -> [i16; SAMPLE_COUNT] {
                    $crate::audio_player::silence::<SAMPLE_COUNT>()
                }

                /// Generates an i16 PCM sine-wave clip with `SAMPLE_COUNT` samples
                /// at this player's sample rate.
                #[must_use]
                pub const fn tone<const SAMPLE_COUNT: usize>(
                    frequency_hz: u32,
                ) -> [i16; SAMPLE_COUNT] {
                    $crate::audio_player::tone::<SAMPLE_COUNT>(frequency_hz)
                }

                /// Returns a new clip with linear volume scaling applied.
                #[must_use]
                pub const fn with_volume<const SAMPLE_COUNT: usize>(
                    audio_sample_i16: &[i16; SAMPLE_COUNT],
                    volume: $crate::audio_player::Volume,
                ) -> [i16; SAMPLE_COUNT] {
                    $crate::audio_player::with_volume(audio_sample_i16, volume)
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
