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
//! - For ffmpeg conversion commands, see "Preparing audio files" at [`pcm_clip!`] and
//!   [`adpcm_clip!`](macro@crate::audio_player::adpcm_clip).
//!
//! **After reading the examples below, see also:**
//!
//! - [`audio_player!`] - Macro to generate an audio player struct type
//!   (includes syntax details). See
//!   [`AudioPlayerGenerated`](audio_player_generated::AudioPlayerGenerated)
//!   for sample generated methods and associated constants.
//! - [`AudioPlayerGenerated`](audio_player_generated::AudioPlayerGenerated) - Sample
//!   generated audio player struct type, showing constructor/associated constants
//!   (for example [`new`](audio_player_generated::AudioPlayerGenerated::new)).
//! - [`AudioPlayer`] - Trait providing playback operations
//!   (`play`, `stop`, `wait_until_stopped`, runtime volume controls) for generated types.
//! - [`pcm_clip!`] - Macro to "compile in" an uncompressed (PCM) clip from an external file
//!   (includes syntax details). See
//!   [`PcmClipGenerated`](pcm_clip_generated::PcmClipGenerated)
//!   for sample generated items.
//! - [`adpcm_clip!`](macro@crate::audio_player::adpcm_clip) - Macro to "compile in" a compressed (ADPCM) WAV clip from an external file
//!   (includes syntax details).
//!   See [`AdpcmClipGenerated`](adpcm_clip_generated::AdpcmClipGenerated) for
//!   sample generated items.
//! - [`tone!`](macro@crate::tone) - Macro to generate tone audio clips.
//! - [`SilenceClip`] - An audio clip of silence for a specific duration. Memory-efficient because it stores no audio sample data.
//! - [`PcmClip`] and [`PcmClipBuf`] - Unsized and sized const-friendly uncompressed (PCM) clip types.
//! - [`AdpcmClip`] and [`AdpcmClipBuf`] - Unsized and sized const-friendly compressed (ADPCM) clip types.
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
//! use device_envoy_rp::{
//!     Result,
//!     audio_player::{AudioPlayer as _,AtEnd, SilenceClip, VOICE_22050_HZ, Volume, audio_player},
//!     tone,
//! };
//! use core::time::Duration as StdDuration;
//!
//! // todo0000 why can't pins be generic and not given until later?
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
//!     // REST is 80 ms of silence. It stores only duration
//!     // (no PCM/ADPCM sample data in flash).
//!     const REST: &AudioPlayer8Playable = &SilenceClip::new(StdDuration::from_millis(80));
//!     // Define each note as a static clip of a sine wave at the appropriate frequency, 220 ms long.
//!     const SAMPLE_RATE_HZ: u32 = AudioPlayer8::SAMPLE_RATE_HZ;
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
//!             NOTE_E4, REST, NOTE_D4, REST, NOTE_C4, REST, NOTE_D4, REST, NOTE_E4, REST,
//!             NOTE_E4, REST, NOTE_E4,
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
//! use device_envoy_rp::{
//!     Result,
//!     audio_player::{AudioPlayer as _,
//!         AtEnd, Gain, SilenceClip, Volume, pcm_clip, audio_player, VOICE_22050_HZ,
//!     },
//!     button::{Button as _, ButtonRp, PressedTo},
//!     tone,
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
//! // Define a `const` function that returns audio from this PCM file.
//! // If unused, it adds nothing to the firmware image.
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
//!     const GAP: &AudioPlayer8Playable = &SilenceClip::new(ms(80));
//!     // 100ms of a pure 880Hz tone, at 20% loudness.
//!     const CHIME: &AudioPlayer8Playable =
//!         &tone!(880, SAMPLE_RATE_HZ, ms(100)).with_gain(Gain::percent(20));
//!
//!     let p = embassy_rp::init(Default::default());
//!     let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
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
//!         audio_player8.set_volume(<AudioPlayer8 as device_envoy_rp::audio_player::AudioPlayer<{ AudioPlayer8::SAMPLE_RATE_HZ }>>::INITIAL_VOLUME);
//!
//!     }
//! }
//! ```
//!
//! # Example: Resample and Play Countdown Once
//!
//! This example "compiles in" three 22.05 kHz clips (`2`, `1`, `0`) and NASA.
//! It changes them to 8 kHz at compile time (`resample` means changing how many
//! audio samples are stored per second), compresses them, and plays them once.
//!
//! Only the final 8 kHz compressed clips are stored in flash.
//!
//! `sample_rate_hz` means samples per second. The clip sample rate is part of the
//! clip type, so using the wrong rate is a compile-time error.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{
//!     Result,
//!     audio_player::{AudioPlayer as _,
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
//!     // We convert, at compile-time, to compressed (ADPCM) format.
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
// TODO Add a realtime tone Playable (sine + ASR envelope) that uses parameter-only storage and matches ADPCM playback performance.

pub mod adpcm_clip_generated;
pub mod audio_player_generated;
pub mod pcm_clip_generated;

pub use device_envoy_core::audio_player::*;

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

#[cfg(target_os = "none")]
const BIT_DEPTH_BITS: u32 = 16;
#[cfg(target_os = "none")]
const SAMPLE_BUFFER_LEN: usize = 256;

/// Internal runtime handle for macro-generated audio player types.
///
/// Must remain `pub` because `audio_player!` expands in downstream crates and
/// references this type directly. `#[doc(hidden)]` keeps it out of user-facing docs.
#[doc(hidden)]
pub struct AudioPlayerRp<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> AudioPlayerRp<MAX_CLIPS, SAMPLE_RATE_HZ> {
    #[doc(hidden)]
    pub const fn new_static() -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static()
    }

    #[doc(hidden)]
    pub const fn new_static_with_max_volume(
        max_volume: Volume,
    ) -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static_with_max_volume(max_volume)
    }

    #[doc(hidden)]
    pub const fn new_static_with_max_volume_and_initial_volume(
        max_volume: Volume,
        initial_volume: Volume,
    ) -> AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        AudioPlayerStatic::new_static_with_max_volume_and_initial_volume(max_volume, initial_volume)
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    ) -> Self {
        Self {
            audio_player_static,
        }
    }

    // Must be `pub` for macro expansion at foreign call sites — not user-facing.
    #[doc(hidden)]
    pub fn __audio_player_static(&self) -> &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ> {
        self.audio_player_static
    }
}

#[cfg(target_os = "none")]
struct RpAudioOutputSink<'a, PIO: Instance + 'static> {
    pio_i2s_out: &'a mut PioI2sOut<'static, PIO, 0>,
}

#[cfg(target_os = "none")]
impl<PIO: Instance + 'static> AudioOutputSink<SAMPLE_BUFFER_LEN> for RpAudioOutputSink<'_, PIO> {
    async fn write_stereo_words(
        &mut self,
        stereo_words: &[u32; SAMPLE_BUFFER_LEN],
        stereo_word_count: usize,
    ) -> core::result::Result<(), ()> {
        self.pio_i2s_out
            .write(&stereo_words[..stereo_word_count])
            .await;
        Ok(())
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
    DataPin: Pin + PioPin,
    BitClockPin: Pin + PioPin,
    WordSelectPin: Pin + PioPin,
>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    pio: Peri<'static, PIO>,
    dma: Peri<'static, DMA>,
    data_pin: Peri<'static, DataPin>,
    bit_clock_pin: Peri<'static, BitClockPin>,
    word_select_pin: Peri<'static, WordSelectPin>,
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
                            let mut rp_audio_output_sink = RpAudioOutputSink {
                                pio_i2s_out: &mut pio_i2s_out,
                            };
                            if let Some(next_audio_command) = play_clip_sequence_once(
                                &mut rp_audio_output_sink,
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
                            let mut rp_audio_output_sink = RpAudioOutputSink {
                                pio_i2s_out: &mut pio_i2s_out,
                            };
                            play_clip_sequence_once(
                                &mut rp_audio_output_sink,
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

/// Macro to generate an audio player struct type (includes syntax details).
///
/// See [`AudioPlayerGenerated`](crate::audio_player::audio_player_generated::AudioPlayerGenerated)
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
/// - associated constants and constructor on `<Name>` (for example:
///   `SAMPLE_RATE_HZ`, `new(...)`)
/// - playback operations via [`AudioPlayer`](crate::audio_player::AudioPlayer) trait
///   (`play(...)`, `stop()`, `wait_until_stopped(...)`, volume controls)
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
                $crate::audio_player::AudioPlayerRp::<$max_clips, { $sample_rate_hz }>::new_static_with_max_volume_and_initial_volume(
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
                player: $crate::audio_player::AudioPlayerRp<$max_clips, { $sample_rate_hz }>,
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
                /// Maximum number of clips accepted by `play(...)`.
                pub const MAX_CLIPS: usize = $max_clips;
                /// Initial runtime volume relative to [`Self::MAX_VOLUME`].
                pub const INITIAL_VOLUME: $crate::audio_player::Volume = $initial_volume;
                /// Runtime volume ceiling for this generated player type.
                pub const MAX_VOLUME: $crate::audio_player::Volume = $max_volume;

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
                        $crate::audio_player::AudioPlayerRp::new(&[<$name:upper _AUDIO_PLAYER_STATIC>]);
                    Ok([<$name:upper _AUDIO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl $crate::audio_player::AudioPlayer<{ $sample_rate_hz }> for $name {
                const SAMPLE_RATE_HZ: u32 = $sample_rate_hz;
                const MAX_CLIPS: usize = $max_clips;
                const INITIAL_VOLUME: $crate::audio_player::Volume = $initial_volume;
                const MAX_VOLUME: $crate::audio_player::Volume = $max_volume;

                fn play<I>(&self, audio_clips: I, at_end: $crate::audio_player::AtEnd)
                where
                    I: IntoIterator<Item = &'static dyn $crate::audio_player::Playable<{ $sample_rate_hz }>>,
                {
                    $crate::audio_player::__audio_player_play(
                        self.player.__audio_player_static(),
                        audio_clips,
                        at_end,
                    );
                }

                fn stop(&self) {
                    $crate::audio_player::__audio_player_stop(self.player.__audio_player_static());
                }

                async fn wait_until_stopped(&self) {
                    $crate::audio_player::__audio_player_wait_until_stopped(
                        self.player.__audio_player_static(),
                    )
                    .await;
                }

                fn set_volume(&self, volume: $crate::audio_player::Volume) {
                    $crate::audio_player::__audio_player_set_volume(
                        self.player.__audio_player_static(),
                        volume,
                    );
                }

                fn volume(&self) -> $crate::audio_player::Volume {
                    $crate::audio_player::__audio_player_volume(self.player.__audio_player_static())
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
pub use tone;
