//TODO000 should I add __ to all doc hidden items.
//! A device abstraction for playing audio clips over I²S hardware,
//! with runtime sequencing, volume control, and compression.
//!
//! This page provides the primary documentation for generated audio player
//! types and clip utilities.
//!
//! Audio clip sample data is defined at compile time as static values.
//! At runtime, you select which clips to play and in what order.
//! Playback runs in the background while the application does other work.
//! Volume can be adjusted on the fly, and playback can be stopped or
//! interrupted mid-clip.
//! Audio samples can be compressed and are stored in flash. Only a small DMA buffer is used at
//! runtime.
//!
//! **Supported audio formats**
//!
//! - Any sample rate supported by your hardware
//! - Either:
//!   - Uncompressed: 16-bit PCM (s16le)
//!   - Compressed: IMA ADPCM in WAV (mono; ~25% the size of PCM; ideal for speech)
//! - Mono input audio (duplicated to left/right on I²S output)
//! todo000 add link to compression directions
//!
//! **After reading the examples below, see also:**
//!
//! - [`audio_player!`] - Macro to generate an audio player struct type
//!   (includes syntax details). See
//!   [`AudioPlayerGenerated`](audio_player_generated::AudioPlayerGenerated)
//!   for sample generated methods and associated constants.
//! - [`pcm_clip!`] - Macro to "compile in" a PCM clip from an external file
//!   (includes syntax details). See
//!   [`PcmClipGenerated`](pcm_clip_generated::PcmClipGenerated)
//!   for sample generated items.
//! - [`adpcm_clip!`](macro@crate::audio_player::adpcm_clip) - Macro to "compile in" an ADPCM WAV clip from an external file
//!   (includes syntax details).
//!   See [`AdpcmClipGenerated`](adpcm_clip_generated::AdpcmClipGenerated) for
//!   sample generated items.
//! - [`tone!`](macro@crate::tone) and [`silence!`](macro@crate::silence) - Macros to generate
//!   tone and silence audio clips.
//! - [`PcmClip`] and [`PcmClipBuf`] - Unsized and sized const-friendly PCM clip types.
//! - [`AdpcmClip`] and [`AdpcmClipBuf`] - Unsized and sized const-friendly ADPCM clip types.
//!
//! # Example: Play "Mary Had a Little Lamb" (Phrase) Once
//!
//! This example plays the opening phrase (`E D C D E E E`) and then stops.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::result::Result::Ok;
//! use device_envoy::{
//!     Result,
//!     audio_player::{AtEnd, VOICE_22050_HZ, Volume, audio_player},
//!     silence, tone,
//! };
//! use core::time::Duration as StdDuration;
//!
//! // Generate `AudioPlayer8`, a struct type with the specified configuration.
//! audio_player! {
//!     AudioPlayer8 {
//!         data_pin: PIN_8,
//!         bit_clock_pin: PIN_9,
//!         word_select_pin: PIN_10,
//!         sample_rate_hz: VOICE_22050_HZ, // Convenience constant for this example; any hardware-supported sample rate can be used.
//!         max_volume: Volume::percent(50),
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     // Define REST_MS as a static clip of silence, 80 milliseconds long.
//!     const SAMPLE_RATE_HZ: u32 = AudioPlayer8::SAMPLE_RATE_HZ;
//!     const REST_MS: &AudioPlayer8Playable = &silence!((SAMPLE_RATE_HZ), StdDuration::from_millis(80));
//!     // Define each note as a static clip of a sine wave at the appropriate frequency, 220 ms long.
//!     const NOTE_DURATION: StdDuration = StdDuration::from_millis(220);
//!     const NOTE_E4: &AudioPlayer8Playable = &tone!(330, SAMPLE_RATE_HZ, NOTE_DURATION);
//!     const NOTE_D4: &AudioPlayer8Playable = &tone!(294, SAMPLE_RATE_HZ, NOTE_DURATION);
//!     const NOTE_C4: &AudioPlayer8Playable = &tone!(262, SAMPLE_RATE_HZ, NOTE_DURATION);
//!
//!     let p = embassy_rp::init(Default::default());
//!     // Create an `AudioPlayer8` instance with the specified pins and resources.
//!     let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     audio_player8.play(
//!         [
//!             NOTE_E4, REST_MS,
//!             NOTE_D4, REST_MS,
//!             NOTE_C4, REST_MS,
//!             NOTE_D4, REST_MS,
//!             NOTE_E4, REST_MS,
//!             NOTE_E4, REST_MS,
//!             NOTE_E4,
//!         ],
//!         AtEnd::Stop,
//!     );
//!
//!     // Audio plays in the background while we can do other things here, like blink an LED or read a button.
//!
//!     core::future::pending().await // run forever
//!
//! }
//! ```
//!
//! # Example: Compiling in an External Audio Clip and Runtime Volume Changes
//!
//! This example shows how to "compile in" an audio clip from an external file,
//! compress it at compile time, and then play it in a loop while changing the volume
//! while it plays. This also demonstrates how to stop playback and reset the volume.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::result::Result::Ok;
//! use device_envoy::{
//!     Result,
//!     audio_player::{
//!         AtEnd, Gain, Volume, pcm_clip, audio_player, VOICE_22050_HZ,
//!     },
//!     button::{Button, PressedTo},
//!     silence, tone,
//! };
//! use core::time::Duration as StdDuration;
//! use embassy_futures::select::{Either, select};
//! use embassy_time::{Duration, Timer};
//!
//! audio_player! {
//!     AudioPlayer8 {
//!         data_pin: PIN_8,
//!         bit_clock_pin: PIN_9,
//!         word_select_pin: PIN_10,
//!         sample_rate_hz: VOICE_22050_HZ,
//!         pio: PIO0,                             // optional, defaults to PIO0
//!         dma: DMA_CH1,                          // optional, defaults to DMA_CH0
//!         max_clips: 8,                          // optional, defaults to 16
//!         max_volume: Volume::spinal_tap(11),    // optional, defaults to Volume::MAX
//!         initial_volume: Volume::spinal_tap(5), // optional, defaults to Volume::MAX
//!     }
//! }
//!
//! // Define a `const` function that returns audio from this PCM file; if unused, it adds nothing to the firmware image.
//! pcm_clip! {
//!     Nasa {
//!         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
//!         source_sample_rate_hz: VOICE_22050_HZ,
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     const fn ms(milliseconds: u64) -> StdDuration {
//!         StdDuration::from_millis(milliseconds)
//!     }
//!     const SAMPLE_RATE_HZ: u32 = AudioPlayer8::SAMPLE_RATE_HZ;
//!
//!     // Only the final transformed clips are stored in flash.
//!     // Intermediate compile-time temporaries (such as compression and gain steps) are not stored.
//!
//!     // Read the uncompressed (PCM) NASA clip in compressed (ADPCM) format.
//!     const NASA: &AudioPlayer8Playable = &Nasa::adpcm_clip();
//!     // 80ms of silence
//!     const GAP: &AudioPlayer8Playable = &silence!(SAMPLE_RATE_HZ, ms(80));
//!     // 100ms of a pure 880Hz tone, at 20% loudness.
//!     const CHIME: &AudioPlayer8Playable =
//!         &tone!(880, SAMPLE_RATE_HZ, ms(100)).with_gain(Gain::percent(20));
//!
//!     let p = embassy_rp::init(Default::default());
//!     let mut button = Button::new(p.PIN_13, PressedTo::Ground);
//!     let audio_player8 =
//!         AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH1, spawner)?;
//!
//!     const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];
//!
//!     loop {
//!         // Wait for user input before starting.
//!         button.wait_for_press().await;
//!
//!         // Start playing the NASA clip, over and over.
//!         audio_player8.play([CHIME, NASA, GAP], AtEnd::Loop);
//!
//!         // Lower runtime volume over time, unless the button is pressed.
//!         for volume_percent in VOLUME_STEPS_PERCENT {
//!             match select(
//!                 button.wait_for_press(),
//!                 Timer::after(Duration::from_secs(1)),
//!             )
//!             .await
//!             {
//!                 Either::First(()) => {
//!                     // Button pressed: leave inner loop.
//!                     break;
//!                 }
//!                 Either::Second(()) => {
//!                     // Timer elapsed: lower volume and keep looping.
//!                     audio_player8.set_volume(Volume::percent(volume_percent));
//!                 }
//!             }
//!         }
//!         audio_player8.stop();
//!         audio_player8.set_volume(AudioPlayer8::INITIAL_VOLUME);
//!
//!     }
//! }
//! ```
//!
//! # Example: Resample and Play Countdown Once
//!
//! This example compiles in three 22.05 kHz clips (`2`, `1`, `0`) and NASA,
//! resamples them to narrowband 8 kHz at compile time, compresses them,
//! and plays the sequence once.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::result::Result::Ok;
//! use device_envoy::{
//!     Result,
//!     audio_player::{
//!         AtEnd, Gain, NARROWBAND_8000_HZ, VOICE_22050_HZ, Volume, pcm_clip,
//!         audio_player,
//!     },
//! };
//!
//! // To save memory, we use a lower sample rate.
//! audio_player! {
//!     AudioPlayer8 {
//!         data_pin: PIN_8,
//!         bit_clock_pin: PIN_9,
//!         word_select_pin: PIN_10,
//!         sample_rate_hz: NARROWBAND_8000_HZ,
//!         max_volume: Volume::percent(50),
//!     }
//! }
//!
//! // We resample each clip from the original 22KHz to the 8KHz sample rate of our audio player.
//! pcm_clip! {
//!     Digit0 {
//!         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/0_22050.s16"),
//!         source_sample_rate_hz: VOICE_22050_HZ,
//!         target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
//!     }
//! }
//!
//! pcm_clip! {
//!     Digit1 {
//!         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/1_22050.s16"),
//!         source_sample_rate_hz: VOICE_22050_HZ,
//!         target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
//!     }
//! }
//!
//! pcm_clip! {
//!     Digit2 {
//!         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/2_22050.s16"),
//!         source_sample_rate_hz: VOICE_22050_HZ,
//!         target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
//!     }
//! }
//!
//! pcm_clip! {
//!     Nasa {
//!         file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
//!         source_sample_rate_hz: VOICE_22050_HZ,
//!         target_sample_rate_hz: AudioPlayer8::SAMPLE_RATE_HZ,
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     // We convert, at compile-time, to compressed format.
//!     const DIGITS: [&AudioPlayer8Playable; 3] = [
//!         &Digit0::adpcm_clip(),
//!         &Digit1::adpcm_clip(),
//!         &Digit2::adpcm_clip(),
//!     ];
//!
//!     // We read the uncompressed (PCM) NASA clip, change its loudness, and then convert it to compressed (ADPCM) format.
//!     const NASA: &AudioPlayer8Playable = &Nasa::pcm_clip()
//!         .with_gain(Gain::percent(25))
//!         .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();
//!
//!     let p = embassy_rp::init(Default::default());
//!     let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     audio_player8.play([DIGITS[2], DIGITS[1], DIGITS[0], NASA], AtEnd::Stop);
//!     core::future::pending().await // run forever
//! }
//! ```
#![cfg_attr(all(test, feature = "host"), allow(dead_code))]

pub mod adpcm_clip_generated;
pub mod audio_player_generated;
#[cfg(all(test, feature = "host"))]
mod host_tests;
pub mod pcm_clip_generated;

#[cfg(target_os = "none")]
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use core::sync::atomic::{AtomicI32, Ordering};
pub use core::time::Duration as StdDuration;

#[cfg(target_os = "none")]
use crate::pio_irqs::PioIrqMap;
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
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

#[cfg(target_os = "none")]
const BIT_DEPTH_BITS: u32 = 16;
#[cfg(target_os = "none")]
const SAMPLE_BUFFER_LEN: usize = 256;
const I16_ABS_MAX_I64: i64 = -(i16::MIN as i64);

/// Common audio sample-rate constants in hertz.
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
/// [`max_volume`, `initial_volume`](macro@crate::audio_player), and
/// [`set_volume`](audio_player_generated::AudioPlayerGenerated::set_volume),
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
    const fn to_i16(self) -> i16 {
        self.0
    }

    #[must_use]
    const fn from_i16(value_i16: i16) -> Self {
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
/// [`max_volume`, `initial_volume`](macro@crate::audio_player), and
/// [`set_volume`](audio_player_generated::AudioPlayerGenerated::set_volume),
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
pub const fn __samples_for_duration(duration: StdDuration, sample_rate_hz: u32) -> usize {
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

// Must remain `pub` because exported macros (for example `pcm_clip!` and
// `adpcm_clip!`) expand in downstream crates and reference this helper via
// `$crate::...`.
#[doc(hidden)]
#[must_use]
pub const fn resampled_sample_count(
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

#[inline]
const fn scale_sample_with_linear(sample_i16: i16, linear_i32: i32) -> i16 {
    if linear_i32 == 0 {
        return 0;
    }
    // Use signed full-scale magnitude (32768) so i16::MIN is handled correctly.
    // Full-scale linear is 32767, so add one to map it to exact unity gain.
    let unity_scaled_linear_i64 = linear_i32 as i64 + 1;
    let scaled_i64 = (sample_i16 as i64 * unity_scaled_linear_i64) / I16_ABS_MAX_I64;
    clamp_i64_to_i16(scaled_i64)
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

/// End-of-sequence behavior for playback.
///
/// `AudioPlayer` supports looping or stopping at the end of a clip sequence.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
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

/// Sized, const-friendly storage for ADPCM clip data.
pub type AdpcmClipBuf<const SAMPLE_RATE_HZ: u32, const DATA_LEN: usize> =
    AdpcmClip<SAMPLE_RATE_HZ, [u8; DATA_LEN]>;

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
        data: [u8; DATA_LEN],
    ) -> Self {
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

    /// Returns this ADPCM clip decoded to PCM samples.
    ///
    /// `SAMPLE_COUNT` must match the decoded sample count implied by this
    /// clip's block structure.
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
        let expected_sample_count = (DATA_LEN / block_align) * samples_per_block;
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
        let mut block_start = 0usize;
        while block_start < DATA_LEN {
            let mut predictor_i32 = read_i16_le_const(&self.data, block_start) as i32;
            let mut step_index_i32 = self.data[block_start + 2] as i32;
            assert!(step_index_i32 >= 0, "ADPCM step_index must be >= 0");
            assert!(step_index_i32 <= 88, "ADPCM step_index must be <= 88");

            samples[sample_index] = predictor_i32 as i16;
            sample_index += 1;
            let mut decoded_in_block = 1usize;

            let mut adpcm_byte_offset = block_start + 4;
            let adpcm_block_end = block_start + block_align;
            while adpcm_byte_offset < adpcm_block_end {
                let adpcm_byte = self.data[adpcm_byte_offset];
                let adpcm_nibble_low = adpcm_byte & 0x0F;
                let adpcm_nibble_high = adpcm_byte >> 4;

                if decoded_in_block < samples_per_block {
                    samples[sample_index] = decode_adpcm_nibble_const(
                        adpcm_nibble_low,
                        &mut predictor_i32,
                        &mut step_index_i32,
                    );
                    sample_index += 1;
                    decoded_in_block += 1;
                }
                if decoded_in_block < samples_per_block {
                    samples[sample_index] = decode_adpcm_nibble_const(
                        adpcm_nibble_high,
                        &mut predictor_i32,
                        &mut step_index_i32,
                    );
                    sample_index += 1;
                    decoded_in_block += 1;
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
        let max_samples_per_block = adpcm_samples_per_block(block_align);
        assert!(
            samples_per_block <= max_samples_per_block,
            "samples_per_block exceeds block_align capacity"
        );

        let mut gained_data = [0_u8; DATA_LEN];
        let mut block_start = 0usize;
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

            while source_byte_offset < block_end {
                let source_byte = self.data[source_byte_offset];
                let mut destination_byte = 0_u8;

                let mut nibble_index = 0usize;
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

        Self::new(self.block_align, self.samples_per_block, gained_data)
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
pub const fn adpcm_samples_per_block(block_align: usize) -> usize {
    if block_align < 5 {
        panic!("block_align must be >= 5 for ADPCM");
    }
    derive_samples_per_block_const(block_align)
}

/// Returns ADPCM byte length needed to encode `sample_count` mono PCM samples.
#[doc(hidden)]
#[must_use]
pub const fn adpcm_data_len_for_pcm_samples(sample_count: usize) -> usize {
    adpcm_data_len_for_pcm_samples_with_block_align(sample_count, ADPCM_ENCODE_BLOCK_ALIGN)
}

/// Returns ADPCM byte length needed to encode `sample_count` mono PCM samples
/// with a specific ADPCM `block_align`.
#[doc(hidden)]
#[must_use]
pub const fn adpcm_data_len_for_pcm_samples_with_block_align(
    sample_count: usize,
    block_align: usize,
) -> usize {
    let samples_per_block = adpcm_samples_per_block(block_align);
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

pub(crate) enum PlaybackClip<const SAMPLE_RATE_HZ: u32> {
    Pcm(&'static PcmClip<SAMPLE_RATE_HZ>),
    Adpcm(&'static AdpcmClip<SAMPLE_RATE_HZ>),
}

/// A statically stored clip source used for mixed playback without enum wrappers at call sites.
///
/// This trait is object-safe, so you can pass heterogeneous static clips as:
/// `&'static dyn Playable<SAMPLE_RATE_HZ>`.
#[allow(private_bounds)]
pub trait Playable<const SAMPLE_RATE_HZ: u32>: sealed::PlayableSealed<SAMPLE_RATE_HZ> {}

impl<const SAMPLE_RATE_HZ: u32, T: ?Sized> Playable<SAMPLE_RATE_HZ> for T where
    T: sealed::PlayableSealed<SAMPLE_RATE_HZ>
{
}

mod sealed {
    use super::{AdpcmClip, PcmClip, PlaybackClip};

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
}

/// Unsized view of static PCM clip data. `&PcmClip` values of different lengths can be sequenced together.
///
/// For fixed-size, const-friendly storage, see [`PcmClipBuf`].
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
pub struct PcmClip<const SAMPLE_RATE_HZ: u32, T: ?Sized = [i16]> {
    samples: T,
}

/// Sized, const-friendly storage for static audio clip data.
///
/// For unsized clip references (for sequencing different clip lengths), see
/// [`PcmClip`].
///
/// todo000 say this somewhere more conspicuous.
/// Sample rate is part of the type, so clips with different sample rates are
/// not assignment-compatible:
///
/// ```rust,compile_fail
/// use device_envoy::audio_player::PcmClipBuf;
///
/// let clip22050: PcmClipBuf<22_050, 4> =
///     device_envoy::audio_player::__pcm_clip_from_samples([0; 4]);
/// let _clip16000: PcmClipBuf<16_000, 4> = clip22050;
/// ```
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
pub type PcmClipBuf<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize> =
    PcmClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>;

const ADPCM_ENCODE_BLOCK_ALIGN: usize = 256;

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
        while sample_index < SAMPLE_COUNT {
            scaled_samples[sample_index] =
                scale_sample_with_linear(self.samples[sample_index], gain.linear());
            sample_index += 1;
        }
        Self {
            samples: scaled_samples,
        }
    }

    /// Returns this clip encoded as mono 4-bit IMA ADPCM.
    ///
    /// Uses a fixed ADPCM block size of 256 bytes. `DATA_LEN` must match
    /// [`adpcm_data_len_for_pcm_samples`]
    /// for this clip's sample count.
    #[must_use]
    pub const fn with_adpcm<const DATA_LEN: usize>(
        &self,
    ) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
        self.with_adpcm_block_align::<DATA_LEN>(ADPCM_ENCODE_BLOCK_ALIGN)
    }

    /// Returns this clip encoded as mono 4-bit IMA ADPCM with a requested block size.
    ///
    /// `DATA_LEN` must match the encoded length implied by this clip's sample
    /// count and `block_align`.
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
        let samples_per_block = adpcm_samples_per_block(block_align);
        assert!(
            samples_per_block <= u16::MAX as usize,
            "samples_per_block must fit in u16"
        );
        assert!(
            DATA_LEN == adpcm_data_len_for_pcm_samples_with_block_align(SAMPLE_COUNT, block_align),
            "adpcm data length must match sample count and block_align"
        );
        if SAMPLE_COUNT == 0 {
            return AdpcmClip::new(block_align as u16, samples_per_block as u16, [0; DATA_LEN]);
        }

        let mut adpcm_data = [0_u8; DATA_LEN];
        let mut sample_index = 0usize;
        let mut data_index = 0usize;
        let payload_len_per_block = block_align - 4;

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
            while payload_byte_index < payload_len_per_block {
                let mut adpcm_byte = 0_u8;

                let mut nibble_index = 0usize;
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

        AdpcmClip::new(block_align as u16, samples_per_block as u16, adpcm_data)
    }
}

/// Const backend helper that creates a PCM sine-wave clip.
///
/// This is intentionally `#[doc(hidden)]` because user-facing construction
/// should prefer [`tone!`](macro@crate::tone).
#[must_use]
#[doc(hidden)]
pub const fn tone_pcm_clip<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    frequency_hz: u32,
) -> PcmClipBuf<SAMPLE_RATE_HZ, SAMPLE_COUNT> {
    assert!(SAMPLE_RATE_HZ > 0, "sample_rate_hz must be > 0");
    let mut samples = [0_i16; SAMPLE_COUNT];
    let phase_step_u64 = ((frequency_hz as u64) << 32) / SAMPLE_RATE_HZ as u64;
    let phase_step_u32 = phase_step_u64 as u32;
    let mut phase_u32 = 0_u32;

    let mut sample_index = 0usize;
    while sample_index < SAMPLE_COUNT {
        samples[sample_index] = sine_sample_from_phase(phase_u32);
        phase_u32 = phase_u32.wrapping_add(phase_step_u32);
        sample_index += 1;
    }

    // Apply a short attack/release envelope to avoid clicks from discontinuities
    // at note boundaries (especially obvious on pure sine tones).
    let mut fade_samples = (SAMPLE_RATE_HZ as usize * 4) / 1000;
    if fade_samples * 2 > SAMPLE_COUNT {
        fade_samples = SAMPLE_COUNT / 2;
    }
    if fade_samples > 0 {
        let fade_samples_i32 = fade_samples as i32;
        let mut fade_index = 0usize;
        while fade_index < fade_samples {
            let fade_numerator = fade_index as i32;
            let leading_scaled = (samples[fade_index] as i32 * fade_numerator) / fade_samples_i32;
            samples[fade_index] = leading_scaled as i16;

            let trailing_index = SAMPLE_COUNT - 1 - fade_index;
            let trailing_scaled =
                (samples[trailing_index] as i32 * fade_numerator) / fade_samples_i32;
            samples[trailing_index] = trailing_scaled as i16;
            fade_index += 1;
        }
    }

    PcmClip { samples }
}

/// Builds a fixed-size PCM clip from samples.
///
/// This is intentionally `#[doc(hidden)]` because user-facing clip
/// construction should prefer `pcm_clip!`, `adpcm_clip!`, `tone!`, and
/// `silence!`.
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
    data: [u8; DATA_LEN],
) -> AdpcmClipBuf<SAMPLE_RATE_HZ, DATA_LEN> {
    AdpcmClip::new(block_align, samples_per_block, data)
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
pub const fn resample_pcm_clip<
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
        resampled_sample_count(SOURCE_COUNT, SOURCE_HZ, TARGET_HZ);
    assert!(
        TARGET_COUNT == expected_destination_sample_count,
        "destination sample count must preserve duration"
    );

    let source_samples = source_pcm_clip.samples;
    let mut resampled_samples = [0_i16; TARGET_COUNT];
    let mut sample_index = 0_usize;

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

enum AudioCommand<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    Play {
        audio_clips: Vec<PlaybackClip<SAMPLE_RATE_HZ>, MAX_CLIPS>,
        at_end: AtEnd,
    },
    Stop,
}

/// Static resources for [`AudioPlayer`].
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

    async fn wait(&self) -> AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ> {
        self.command_signal.wait().await
    }

    fn mark_playing(&self) {
        self.has_pending_play.store(false, AtomicOrdering::Relaxed);
        self.is_playing.store(true, AtomicOrdering::Relaxed);
    }

    fn mark_stopped(&self) {
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

    fn effective_runtime_volume(&self) -> Volume {
        let runtime_volume_relative = self.runtime_volume();
        Volume::from_i16(scale_linear(self.max_volume_linear, runtime_volume_relative) as i16)
    }
}

/// Plays static audio clips with preemptive command handling in the background device task.
///
/// See the [`audio_player!`] macro for the normal construction pattern.
// Must be `pub` so `audio_player!` expansions in downstream crates can reference this type.
#[doc(hidden)]
pub struct AudioPlayer<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> AudioPlayer<MAX_CLIPS, SAMPLE_RATE_HZ> {
    /// Creates static resources for a player.
    #[must_use]
    pub const fn new_static() -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static()
    }

    /// Creates static resources for a player with a runtime volume ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume(
        max_volume: Volume,
    ) -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static_with_max_volume(max_volume)
    }

    /// Creates static resources for a player with a runtime volume ceiling
    /// and an initial runtime volume relative to that ceiling.
    #[must_use]
    pub const fn new_static_with_max_volume_and_initial_volume(
        max_volume: Volume,
        initial_volume: Volume,
    ) -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static_with_max_volume_and_initial_volume(max_volume, initial_volume)
    }

    /// Creates a player handle. The device task must already be running.
    #[must_use]
    pub const fn new(
        audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    ) -> Self {
        Self {
            audio_player_static,
        }
    }

    /// Starts playback of one or more statically defined audio clips.
    ///
    /// This supports mixed PCM + ADPCM literals like
    /// `[&adpcm_clip, &silence_100ms, &tone_a4]`.
    ///
    /// Clip samples are predeclared static data, but sequence order is chosen
    /// at runtime and copied into a fixed-capacity clip list defined by `MAX_CLIPS`.
    /// A newer call to [`Self::play`] interrupts current playback as soon as possible
    /// (at the next DMA chunk boundary).
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    pub fn play<const CLIP_COUNT: usize>(
        &self,
        audio_clips: [&'static dyn Playable<SAMPLE_RATE_HZ>; CLIP_COUNT],
        at_end: AtEnd,
    ) {
        self.play_iter(audio_clips, at_end);
    }

    /// Starts playback from a generic iterator of static clip sources.
    ///
    /// This allows runtime-selected sequencing while still requiring static
    /// clip sample storage.
    pub fn play_iter<I>(&self, audio_clips: I, at_end: AtEnd)
    where
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

        self.audio_player_static.mark_pending_play();
        self.audio_player_static.signal(AudioCommand::Play {
            audio_clips: audio_clip_sequence,
            at_end,
        });
    }

    /// Stops current playback as soon as possible.
    ///
    /// If playback is active, it is interrupted at the next DMA chunk boundary.
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    pub fn stop(&self) {
        self.audio_player_static.signal(AudioCommand::Stop);
    }

    /// Waits until playback is stopped.
    ///
    /// If playback is currently stopped, this returns immediately.
    /// If playback is active, this waits until the player reaches the stopped
    /// state (natural end with [`AtEnd::Stop`] or a processed [`Self::stop`]).
    pub async fn wait_until_stopped(&self) {
        self.audio_player_static.wait_until_stopped().await;
    }

    /// Sets runtime playback volume relative to [`Self::MAX_VOLUME`].
    ///
    /// - `Volume::percent(100)` plays at exactly `max_volume`.
    /// - `Volume::percent(50)` plays at half of `max_volume`.
    ///
    /// This relative scale composes multiplicatively with any per-clip gain
    /// pre-applied via [`PcmClipBuf::with_gain`].
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    pub fn set_volume(&self, volume: Volume) {
        self.audio_player_static.set_runtime_volume(volume);
    }

    /// Returns the current runtime playback volume relative to [`Self::MAX_VOLUME`].
    ///
    /// See the [audio_player module documentation](mod@crate::audio_player) for
    /// usage examples.
    #[must_use]
    pub fn volume(&self) -> Volume {
        self.audio_player_static.runtime_volume()
    }
}

// Called by macro-generated code in downstream crates; must be public.
#[cfg(target_os = "none")]
#[doc(hidden)]
pub async fn device_loop<
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
    PIO: PioIrqMap,
    DMA: Channel,
    DinPin: Pin + PioPin,
    BclkPin: Pin + PioPin,
    LrcPin: Pin + PioPin,
>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, DMA>,
    data_pin: Peri<'static, DinPin>,
    bit_clock_pin: Peri<'static, BclkPin>,
    word_select_pin: Peri<'static, LrcPin>,
) -> ! {
    let mut pio = Pio::new(pio, <PIO as PioIrqMap>::irqs());
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
        let mut audio_command = audio_player_static.wait().await;

        loop {
            match audio_command {
                AudioCommand::Play {
                    audio_clips,
                    at_end,
                } => {
                    audio_player_static.mark_playing();
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

                    audio_player_static.mark_stopped();
                }
                AudioCommand::Stop => audio_player_static.mark_stopped(),
            }

            break;
        }
    }
}

#[cfg(target_os = "none")]
async fn play_clip_sequence_once<
    PIO: Instance,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_clips: &[PlaybackClip<SAMPLE_RATE_HZ>],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> Option<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>> {
    for audio_clip in audio_clips {
        match audio_clip {
            PlaybackClip::Pcm(audio_clip) => {
                if let ControlFlow::Break(next_audio_command) = play_full_pcm_clip_once(
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
            PlaybackClip::Adpcm(adpcm_clip) => {
                if let ControlFlow::Break(next_audio_command) = play_full_adpcm_clip_once(
                    pio_i2s_out,
                    adpcm_clip,
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

#[cfg(target_os = "none")]
async fn play_full_pcm_clip_once<
    PIO: Instance,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_clip: &PcmClip<SAMPLE_RATE_HZ>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> ControlFlow<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>, ()> {
    for audio_sample_chunk in audio_clip.samples.chunks(SAMPLE_BUFFER_LEN) {
        let runtime_volume = audio_player_static.effective_runtime_volume();
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            let scaled_sample_value =
                scale_sample_with_linear(sample_value, runtime_volume.to_i16() as i32);
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

#[cfg(target_os = "none")]
async fn play_full_adpcm_clip_once<
    PIO: Instance,
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    adpcm_clip: &AdpcmClip<SAMPLE_RATE_HZ>,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> ControlFlow<AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ>, ()> {
    let mut sample_buffer_len = 0usize;

    let block_align = adpcm_clip.block_align as usize;
    for adpcm_block in adpcm_clip.data.chunks_exact(block_align) {
        if adpcm_block.len() < 4 {
            return ControlFlow::Continue(());
        }

        let runtime_volume = audio_player_static.effective_runtime_volume();
        let mut predictor_i32 = match read_i16_le(adpcm_block, 0) {
            Ok(value) => value as i32,
            Err(_) => return ControlFlow::Continue(()),
        };
        let mut step_index_i32 = adpcm_block[2] as i32;
        if !(0..=88).contains(&step_index_i32) {
            return ControlFlow::Continue(());
        }

        sample_buffer[sample_buffer_len] = stereo_sample(scale_sample_with_linear(
            predictor_i32 as i16,
            runtime_volume.to_i16() as i32,
        ));
        sample_buffer_len += 1;
        if sample_buffer_len == SAMPLE_BUFFER_LEN {
            pio_i2s_out.write(sample_buffer).await;
            sample_buffer_len = 0;
            if let Some(next_audio_command) = audio_player_static.command_signal.try_take() {
                return ControlFlow::Break(next_audio_command);
            }
        }

        let mut samples_decoded_in_block = 1usize;
        let samples_per_block = adpcm_clip.samples_per_block as usize;

        for adpcm_byte in &adpcm_block[4..] {
            for adpcm_nibble in [adpcm_byte & 0x0F, adpcm_byte >> 4] {
                if samples_decoded_in_block >= samples_per_block {
                    break;
                }

                let decoded_sample_i16 =
                    decode_adpcm_nibble(adpcm_nibble, &mut predictor_i32, &mut step_index_i32);
                sample_buffer[sample_buffer_len] = stereo_sample(scale_sample_with_linear(
                    decoded_sample_i16,
                    runtime_volume.to_i16() as i32,
                ));
                sample_buffer_len += 1;
                samples_decoded_in_block += 1;

                if sample_buffer_len == SAMPLE_BUFFER_LEN {
                    pio_i2s_out.write(sample_buffer).await;
                    sample_buffer_len = 0;
                    if let Some(next_audio_command) = audio_player_static.command_signal.try_take()
                    {
                        return ControlFlow::Break(next_audio_command);
                    }
                }
            }
        }

        if let Some(next_audio_command) = audio_player_static.command_signal.try_take() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    if sample_buffer_len != 0 {
        sample_buffer[sample_buffer_len..].fill(stereo_sample(0));
        pio_i2s_out.write(sample_buffer).await;
        if let Some(next_audio_command) = audio_player_static.command_signal.try_take() {
            return ControlFlow::Break(next_audio_command);
        }
    }

    ControlFlow::Continue(())
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
fn decode_adpcm_nibble(adpcm_nibble: u8, predictor_i32: &mut i32, step_index_i32: &mut i32) -> i16 {
    decode_adpcm_nibble_const(adpcm_nibble, predictor_i32, step_index_i32)
}

const fn decode_adpcm_nibble_const(
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

#[inline]
#[cfg(target_os = "none")]
const fn stereo_sample(sample: i16) -> u32 {
    let sample_bits = sample as u16 as u32;
    (sample_bits << 16) | sample_bits
}

// Must be `pub` so macro expansion works in downstream crates.
#[doc(hidden)]
pub use paste;

#[doc = include_str!("audio_player/pcm_clip_docs.md")]
#[doc = include_str!("audio_player/audio_prep_steps_1_2.md")]
#[doc = include_str!("audio_player/pcm_clip_step_3.md")]
#[doc(hidden)]
#[macro_export]
macro_rules! pcm_clip {
    ($($tt:tt)*) => { $crate::__audio_clip_parse! { $($tt)* } };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __audio_clip_parse {
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
            source_sample_rate_hz: $source_sample_rate_hz:expr,
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
            source_sample_rate_hz: $source_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $source_sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            source_sample_rate_hz: $source_sample_rate_hz:expr,
            $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $source_sample_rate_hz,
            target_sample_rate_hz: $source_sample_rate_hz,
        }
    };

    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $sample_rate_hz:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
            target_sample_rate_hz: $target_sample_rate_hz,
        }
    };
    (
        $vis:vis $name:ident {
            file: $file:expr,
            sample_rate_hz: $sample_rate_hz:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr,
            $(,)?
        }
    ) => {
        $crate::__audio_clip_dispatch! {
            vis: $vis,
            name: $name,
            file: $file,
            source_sample_rate_hz: $sample_rate_hz,
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
        $crate::audio_player::paste::paste! {
            const [<$name:upper _SOURCE_SAMPLE_RATE_HZ>]: u32 = $source_sample_rate_hz;
            const [<$name:upper _TARGET_SAMPLE_RATE_HZ>]: u32 = $target_sample_rate_hz;

            #[allow(non_snake_case)]
            #[doc = concat!(
                "Audio clip namespace generated by [`pcm_clip!`](macro@crate::audio_player::pcm_clip).\n\n",
                "[`SAMPLE_RATE_HZ`](Self::SAMPLE_RATE_HZ), ",
                "[`PCM_SAMPLE_COUNT`](Self::PCM_SAMPLE_COUNT), ",
                "[`ADPCM_DATA_LEN`](Self::ADPCM_DATA_LEN), ",
                "[`pcm_clip`](Self::pcm_clip), ",
                "and [`adpcm_clip`](Self::adpcm_clip)."
            )]
            $vis mod $name {
                const SOURCE_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _SOURCE_SAMPLE_RATE_HZ>];
                const TARGET_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _TARGET_SAMPLE_RATE_HZ>];
                #[doc = "Sample rate in hertz for this generated clip output."]
                pub const SAMPLE_RATE_HZ: u32 = TARGET_SAMPLE_RATE_HZ;
                const AUDIO_SAMPLE_BYTES_LEN: usize = include_bytes!($file).len();
                const SOURCE_SAMPLE_COUNT: usize = AUDIO_SAMPLE_BYTES_LEN / 2;
                #[doc = "Number of i16 PCM samples in this generated clip."]
                #[doc = "See the [audio_player module documentation](mod@crate::audio_player) for usage examples."]
                pub const PCM_SAMPLE_COUNT: usize = $crate::audio_player::resampled_sample_count(
                    SOURCE_SAMPLE_COUNT,
                    SOURCE_SAMPLE_RATE_HZ,
                    TARGET_SAMPLE_RATE_HZ,
                );
                #[doc = "Byte length of the ADPCM data when encoding this clip."]
                pub const ADPCM_DATA_LEN: usize =
                    $crate::audio_player::adpcm_data_len_for_pcm_samples(PCM_SAMPLE_COUNT);

                #[allow(dead_code)]
                type SourcePcmClip = $crate::audio_player::PcmClipBuf<
                    { SOURCE_SAMPLE_RATE_HZ },
                    { SOURCE_SAMPLE_COUNT },
                >;

                #[doc = "Const constructor generated by [`pcm_clip!`](macro@crate::audio_player::pcm_clip)."]
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
                    $crate::audio_player::resample_pcm_clip::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                        TARGET_SAMPLE_RATE_HZ,
                        PCM_SAMPLE_COUNT,
                    >($crate::audio_player::__pcm_clip_from_samples::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                    >(samples))
                }

                #[doc = "Const ADPCM (256-byte block) constructor generated by [`pcm_clip!`](macro@crate::audio_player::pcm_clip)."]
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

#[doc = include_str!("audio_player/adpcm_clip_docs.md")]
#[doc = include_str!("audio_player/audio_prep_steps_1_2.md")]
#[doc = include_str!("audio_player/adpcm_clip_step_3.md")]
#[doc(inline)]
pub use crate::adpcm_clip;

#[doc(hidden)]
#[macro_export]
macro_rules! adpcm_clip {
    (
        $vis:vis $name:ident {
            file: $file:expr,
            target_sample_rate_hz: $target_sample_rate_hz:expr $(,)?
        }
    ) => {
        $crate::audio_player::paste::paste! {
            const [<$name:upper _TARGET_SAMPLE_RATE_HZ>]: u32 = $target_sample_rate_hz;

            #[allow(non_snake_case)]
            #[allow(missing_docs)]
            $vis mod $name {
                const PARSED_WAV: $crate::audio_player::ParsedAdpcmWavHeader =
                    $crate::audio_player::parse_adpcm_wav_header(include_bytes!($file));
                const SOURCE_SAMPLE_RATE_HZ: u32 = PARSED_WAV.sample_rate_hz;
                const TARGET_SAMPLE_RATE_HZ: u32 = super::[<$name:upper _TARGET_SAMPLE_RATE_HZ>];
                pub const SAMPLE_RATE_HZ: u32 = TARGET_SAMPLE_RATE_HZ;

                const SOURCE_SAMPLE_COUNT: usize = PARSED_WAV.sample_count;
                pub const PCM_SAMPLE_COUNT: usize = $crate::audio_player::resampled_sample_count(
                    SOURCE_SAMPLE_COUNT,
                    SOURCE_SAMPLE_RATE_HZ,
                    TARGET_SAMPLE_RATE_HZ,
                );
                const BLOCK_ALIGN: usize = PARSED_WAV.block_align;
                const SOURCE_DATA_LEN: usize = PARSED_WAV.data_chunk_len;
                pub const ADPCM_DATA_LEN: usize = if TARGET_SAMPLE_RATE_HZ == SOURCE_SAMPLE_RATE_HZ {
                    SOURCE_DATA_LEN
                } else {
                    $crate::audio_player::adpcm_data_len_for_pcm_samples_with_block_align(
                        PCM_SAMPLE_COUNT,
                        BLOCK_ALIGN,
                    )
                };
                type SourceAdpcmClip = $crate::audio_player::AdpcmClipBuf<SOURCE_SAMPLE_RATE_HZ, SOURCE_DATA_LEN>;

                #[must_use]
                const fn source_adpcm_clip() -> SourceAdpcmClip {
                    let wav_bytes = include_bytes!($file);
                    let parsed_wav = $crate::audio_player::parse_adpcm_wav_header(wav_bytes);
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
                        adpcm_data,
                    )
                }

                #[must_use]
                pub const fn pcm_clip() -> $crate::audio_player::PcmClipBuf<SAMPLE_RATE_HZ, PCM_SAMPLE_COUNT> {
                    $crate::audio_player::resample_pcm_clip::<
                        SOURCE_SAMPLE_RATE_HZ,
                        SOURCE_SAMPLE_COUNT,
                        TARGET_SAMPLE_RATE_HZ,
                        PCM_SAMPLE_COUNT,
                    >(source_adpcm_clip().with_pcm::<SOURCE_SAMPLE_COUNT>())
                }

                #[must_use]
                pub const fn adpcm_clip() -> $crate::audio_player::AdpcmClipBuf<SAMPLE_RATE_HZ, ADPCM_DATA_LEN> {
                    if TARGET_SAMPLE_RATE_HZ == SOURCE_SAMPLE_RATE_HZ {
                        let wav_bytes = include_bytes!($file);
                        let parsed_wav = $crate::audio_player::parse_adpcm_wav_header(wav_bytes);
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
        $crate::adpcm_clip! {
            $vis $name {
                file: $file,
                target_sample_rate_hz: $crate::audio_player::parse_adpcm_wav_header(include_bytes!($file)).sample_rate_hz,
            }
        }
    };
}

/// Macro that expands to a PCM silence clip expression for a sample rate and duration.
///
/// Examples:
/// - `silence!(VOICE_22050_HZ, Duration::from_millis(100))`
/// - `silence!(AudioPlayer8::SAMPLE_RATE_HZ, Duration::from_millis(100))`
///
/// The result is a `PcmClipBuf` silence clip.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[doc(hidden)]
#[macro_export]
macro_rules! silence {
    ($sample_rate_hz:expr, $duration:expr) => {
        $crate::audio_player::__pcm_clip_from_samples::<
            { $sample_rate_hz },
            { $crate::audio_player::__samples_for_duration($duration, $sample_rate_hz) },
        >([0; { $crate::audio_player::__samples_for_duration($duration, $sample_rate_hz) }])
    };
}

/// Macro that expands to a PCM tone clip expression for frequency,
/// sample rate, and duration.
///
/// TODO000 why not adpcm
///
/// Examples:
/// - `tone!(440, VOICE_22050_HZ, Duration::from_millis(500))`
/// - `tone!(440, AudioPlayer8::SAMPLE_RATE_HZ, Duration::from_millis(500))`
///
/// The result is a `PcmClipBuf` sine-wave clip.
///
/// See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.
#[doc(hidden)]
#[macro_export]
macro_rules! tone {
    ($frequency_hz:expr, $sample_rate_hz:expr, $duration:expr) => {
        $crate::audio_player::tone_pcm_clip::<
            { $sample_rate_hz },
            { $crate::audio_player::__samples_for_duration($duration, $sample_rate_hz) },
        >($frequency_hz)
    };
}

/// Macro to generate an audio player struct type (includes syntax details). See
/// [`AudioPlayerGenerated`](crate::audio_player::audio_player_generated::AudioPlayerGenerated)
/// for a sample of a generated type.
///
/// **See the [audio_player module documentation](mod@crate::audio_player) for
/// usage examples.**
///
/// **Syntax:**
///
/// ```text
/// audio_player! {
///     [<visibility>] <Name> {
///         data_pin: <pin_ident>,
///         bit_clock_pin: <pin_ident>,
///         word_select_pin: <pin_ident>,
///         sample_rate_hz: <sample_rate_expr>,
///         pio: <pio_ident>,                 // optional
///         dma: <dma_ident>,                 // optional
///         max_clips: <usize_expr>,          // optional
///         max_volume: <Volume_expr>,        // optional
///         initial_volume: <Volume_expr>,    // optional
///     }
/// }
/// ```
///
/// **Inputs:**
///
/// - `$vis` - Optional generated type visibility (for example: `pub`,
///   `pub(crate)`, `pub(self)`). Defaults to private visibility when omitted.
/// - `$name` - Generated type name (for example: `AudioPlayer10`)
///
/// **Required fields:**
///
/// - `data_pin` - GPIO pin carrying I²S data (`DIN`)
/// - `bit_clock_pin` - GPIO pin carrying I²S bit clock (`BCLK`)
/// - `word_select_pin` - GPIO pin carrying I²S word-select / LR clock (`LRC` / `LRCLK`)
/// - `sample_rate_hz` - Playback sample rate in hertz (for example:
///   [`VOICE_22050_HZ`](crate::audio_player::VOICE_22050_HZ))
///
/// **Optional fields:**
///
/// - `pio` - PIO resource (default: `PIO0`)
/// - `dma` - DMA channel (default: `DMA_CH0`)
/// - `max_clips` - Maximum clips per queued play request (default: `16`)
/// - `max_volume` - Runtime volume ceiling (default: [`Volume::MAX`])
/// - `initial_volume` - Initial runtime volume relative to `max_volume`
///   (default: [`Volume::MAX`])
///
/// **Generated items:**
///
/// - `<Name>` - generated player struct type
/// - `<Name>Playable` - trait-object clip source alias at this player's sample rate
/// - associated constants and methods on `<Name>` (for example:
///   `SAMPLE_RATE_HZ`, `samples(...)`,
///   `new(...)`, `play(...)`, and runtime volume controls)
///
/// The generated type contains static resources and spawns its background device
/// task from `new(...)`.
#[doc(hidden)]
#[macro_export]
macro_rules! audio_player {
    // TODO_NIGHTLY When nightly feature `decl_macro` becomes stable, change this
    // code by replacing `#[macro_export] macro_rules!` with module-scoped `pub macro`
    // so macro visibility and helper exposure can be controlled more precisely.
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
            vis: pub(self),
            name: $name,
            data_pin: _UNSET_,
            bit_clock_pin: _UNSET_,
            word_select_pin: _UNSET_,
            sample_rate_hz: _UNSET_,
            pio: PIO0,
            dma: DMA_CH0,
            max_clips: 16,
            max_volume: $crate::audio_player::Volume::MAX,
            initial_volume: $crate::audio_player::Volume::MAX,
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
            data_pin: _UNSET_,
            bit_clock_pin: _UNSET_,
            word_select_pin: _UNSET_,
            sample_rate_hz: _UNSET_,
            pio: PIO0,
            dma: DMA_CH0,
            max_clips: 16,
            max_volume: $crate::audio_player::Volume::MAX,
            initial_volume: $crate::audio_player::Volume::MAX,
            fields: [ $($fields)* ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ data_pin: $din_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            data_pin: $din_pin_value,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ sample_rate_hz: $sample_rate_hz_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz_value,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ bit_clock_pin: $bclk_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            data_pin: $data_pin,
            bit_clock_pin: $bclk_pin_value,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ word_select_pin: $lrc_pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__audio_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $lrc_pin_value,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
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
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
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
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
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
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
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
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: $data_pin:tt,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
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
            data_pin: $data_pin,
            bit_clock_pin: $bit_clock_pin,
            word_select_pin: $word_select_pin,
            sample_rate_hz: $sample_rate_hz,
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
        data_pin: _UNSET_,
        bit_clock_pin: $bit_clock_pin:tt,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires data_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        data_pin: $data_pin:ident,
        bit_clock_pin: _UNSET_,
        word_select_pin: $word_select_pin:tt,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires bit_clock_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        data_pin: $data_pin:ident,
        bit_clock_pin: $bit_clock_pin:ident,
        word_select_pin: _UNSET_,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires word_select_pin");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        data_pin: $data_pin:ident,
        bit_clock_pin: $bit_clock_pin:ident,
        word_select_pin: $word_select_pin:ident,
        sample_rate_hz: _UNSET_,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        compile_error!("audio_player! requires sample_rate_hz");
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        data_pin: $data_pin:ident,
        bit_clock_pin: $bit_clock_pin:ident,
        word_select_pin: $word_select_pin:ident,
        sample_rate_hz: $sample_rate_hz:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_clips: $max_clips:expr,
        max_volume: $max_volume:expr,
        initial_volume: $initial_volume:expr,
        fields: [ ]
    ) => {
        $crate::audio_player::paste::paste! {
            static [<$name:upper _AUDIO_PLAYER_STATIC>]:
                $crate::audio_player::AudioPlayerStatic<$max_clips, { $sample_rate_hz }> =
                $crate::audio_player::AudioPlayer::<$max_clips, { $sample_rate_hz }>::new_static_with_max_volume_and_initial_volume(
                    $max_volume,
                    $initial_volume,
                );
            static [<$name:upper _AUDIO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            #[doc = concat!(
                "Audio player generated by [`audio_player!`](macro@crate::audio_player).\n\n",
                "See the [audio_player module documentation](mod@crate::audio_player) for usage and examples."
            )]
            $vis struct $name {
                player: $crate::audio_player::AudioPlayer<$max_clips, { $sample_rate_hz }>,
            }

            #[doc = concat!(
                "Trait-object clip source type at [`",
                stringify!($name),
                "::SAMPLE_RATE_HZ`](struct@",
                stringify!($name),
                ").\n\n",
                "Use this in signatures like `&'static ",
                stringify!([<$name Playable>]),
                "` instead of repeating `dyn Playable<{ ",
                stringify!($name),
                "::SAMPLE_RATE_HZ }>`."
            )]
            $vis type [<$name Playable>] =
                dyn $crate::audio_player::Playable<{ $sample_rate_hz }>;

            impl $name {
                /// Sample rate used for audio playback by this generated player type.
                pub const SAMPLE_RATE_HZ: u32 = $sample_rate_hz;
                /// Initial runtime volume relative to [`Self::MAX_VOLUME`].
                pub const INITIAL_VOLUME: $crate::audio_player::Volume = $initial_volume;
                /// Runtime volume ceiling for this generated player type.
                pub const MAX_VOLUME: $crate::audio_player::Volume = $max_volume;

                /// Returns how many samples are needed for a duration
                /// at this player's sample rate.
                #[must_use]
                pub const fn samples(duration: $crate::audio_player::StdDuration) -> usize {
                    $crate::audio_player::__samples_for_duration(duration, Self::SAMPLE_RATE_HZ)
                }

                /// Creates and spawns the generated audio player instance.
                ///
                /// See the [audio_player module documentation](mod@crate::audio_player)
                /// for example usage.
                pub fn new(
                    data_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$data_pin>>,
                    bit_clock_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bit_clock_pin>>,
                    word_select_pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$word_select_pin>>,
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let token = [<$name:snake _audio_player_task>](
                        &[<$name:upper _AUDIO_PLAYER_STATIC>],
                        pio.into(),
                        dma.into(),
                        data_pin.into(),
                        bit_clock_pin.into(),
                        word_select_pin.into(),
                    );
                    spawner.spawn(token)?;
                    let player =
                        $crate::audio_player::AudioPlayer::new(&[<$name:upper _AUDIO_PLAYER_STATIC>]);
                    Ok([<$name:upper _AUDIO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::audio_player::AudioPlayer<$max_clips, { $sample_rate_hz }>;

                fn deref(&self) -> &Self::Target {
                    &self.player
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _audio_player_task>](
                audio_player_static: &'static $crate::audio_player::AudioPlayerStatic<$max_clips, { $sample_rate_hz }>,
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                data_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$data_pin>,
                bit_clock_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$bit_clock_pin>,
                word_select_pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$word_select_pin>,
            ) -> ! {
                $crate::audio_player::device_loop::<
                    $max_clips,
                    { $sample_rate_hz },
                    ::embassy_rp::peripherals::$pio,
                    ::embassy_rp::peripherals::$dma,
                    ::embassy_rp::peripherals::$data_pin,
                    ::embassy_rp::peripherals::$bit_clock_pin,
                    ::embassy_rp::peripherals::$word_select_pin,
                >(audio_player_static, pio, dma, data_pin, bit_clock_pin, word_select_pin).await
            }
        }
    };
}

#[doc(inline)]
pub use audio_player;
#[doc(inline)]
pub use pcm_clip;
#[doc(inline)]
pub use silence;
#[doc(inline)]
pub use tone;
