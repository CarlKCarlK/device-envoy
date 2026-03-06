//! A device abstraction for playing audio clips over I²S hardware,
//! with runtime sequencing, volume control, and compression.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]
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
//! use device_envoy::{
//!     Result,
//!     audio_player::{AudioPlayer as _,AtEnd, SilenceClip, VOICE_22050_HZ, Volume, audio_player},
//!     tone,
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
//! use device_envoy::{
//!     Result,
//!     audio_player::{AudioPlayer as _,
//!         AtEnd, Gain, SilenceClip, Volume, pcm_clip, audio_player, VOICE_22050_HZ,
//!     },
//!     button::{ButtonEsp, PressedTo},
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
//!     let mut button = ButtonEsp::new(p.PIN_13, PressedTo::Ground);
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
//!         audio_player8.set_volume(<AudioPlayer8 as device_envoy::audio_player::AudioPlayer<{ AudioPlayer8::SAMPLE_RATE_HZ }>>::INITIAL_VOLUME);
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
//! use device_envoy::{
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
use embassy_futures::yield_now;
#[cfg(target_os = "none")]
use esp_hal::{
    dma::DmaChannelFor,
    gpio::interconnect::PeripheralOutput,
    i2s::{
        master::{Channels, Config, DataFormat, I2s},
        AnyI2s,
    },
    time::Rate,
};

#[cfg(target_os = "none")]
const SAMPLE_BUFFER_LEN: usize = 1024;
const DMA_TX_BYTES: usize = 16384;

#[cfg(target_os = "none")]
type AudioI2sTxTransfer = esp_hal::i2s::master::asynch::I2sWriteDmaTransferAsync<
    'static,
    &'static mut [u8; DMA_TX_BYTES],
>;

/// Internal runtime handle for macro-generated audio player types.
///
/// Must remain `pub` because `audio_player!` expands in downstream crates and
/// references this type directly. `#[doc(hidden)]` keeps it out of user-facing docs.
#[doc(hidden)]
pub struct AudioPlayerEsp<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> {
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
}

impl<const MAX_CLIPS: usize, const SAMPLE_RATE_HZ: u32> AudioPlayerEsp<MAX_CLIPS, SAMPLE_RATE_HZ> {
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
struct EspAudioOutputSink<'a> {
    i2s_tx_transfer: &'a mut AudioI2sTxTransfer,
}

#[cfg(target_os = "none")]
impl AudioOutputSink<SAMPLE_BUFFER_LEN> for EspAudioOutputSink<'_> {
    async fn write_stereo_words(
        &mut self,
        stereo_words: &[u32; SAMPLE_BUFFER_LEN],
        stereo_word_count: usize,
    ) -> core::result::Result<(), ()> {
        write_words_to_i2s_with_recovery(self.i2s_tx_transfer, stereo_words, stereo_word_count)
            .await
    }

    async fn after_write(&mut self) {
        yield_now().await;
    }
}

// Called by macro-generated code in downstream crates; must be public.
#[cfg(target_os = "none")]
#[doc(hidden)]
pub async fn device_loop<
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
    Dma: DmaChannelFor<AnyI2s<'static>>,
    DataPin: PeripheralOutput<'static>,
    BitBlockPin: PeripheralOutput<'static>,
    WordSelectPin: PeripheralOutput<'static>,
>(
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
    i2s: esp_hal::peripherals::I2S0<'static>,
    dma: Dma,
    data_pin: DataPin,
    bit_clock_pin: BitBlockPin,
    word_select_pin: WordSelectPin,
) -> ! {
    let i2s = I2s::new(
        i2s,
        dma,
        Config::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(SAMPLE_RATE_HZ))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    )
    .expect("I2S init failed")
    .into_async();
    let (_, _, tx_buffer, tx_descriptors) = esp_hal::dma_circular_buffers!(0, DMA_TX_BYTES);
    let mut i2s_tx_transfer = i2s
        .i2s_tx
        .with_bclk(bit_clock_pin)
        .with_ws(word_select_pin)
        .with_dout(data_pin)
        .build(tx_descriptors)
        .write_dma_circular_async(tx_buffer)
        .expect("I2S circular DMA setup failed");
    let _ = fill_dma_ring_with_silence(&mut i2s_tx_transfer).await;
    let mut sample_buffer = [0_u32; SAMPLE_BUFFER_LEN];

    loop {
        let mut audio_command = wait_for_audio_command_while_feeding_silence(
            &mut i2s_tx_transfer,
            &mut sample_buffer,
            audio_player_static,
        )
        .await;

        loop {
            match audio_command {
                AudioCommand::Play {
                    audio_clips,
                    at_end,
                } => {
                    audio_player_static.mark_playing();
                    let next_audio_command = match at_end {
                        AtEnd::Loop => loop {
                            let mut esp_audio_output_sink = EspAudioOutputSink {
                                i2s_tx_transfer: &mut i2s_tx_transfer,
                            };
                            if let Some(next_audio_command) = play_clip_sequence_once(
                                &mut esp_audio_output_sink,
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
                            let mut esp_audio_output_sink = EspAudioOutputSink {
                                i2s_tx_transfer: &mut i2s_tx_transfer,
                            };
                            play_clip_sequence_once(
                                &mut esp_audio_output_sink,
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
                AudioCommand::Stop => {
                    audio_player_static.mark_stopped();
                }
            }

            break;
        }
    }
}

#[cfg(target_os = "none")]
async fn wait_for_audio_command_while_feeding_silence<
    const MAX_CLIPS: usize,
    const SAMPLE_RATE_HZ: u32,
>(
    i2s_tx_transfer: &mut AudioI2sTxTransfer,
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
    audio_player_static: &'static AudioPlayerStatic<MAX_CLIPS, SAMPLE_RATE_HZ>,
) -> AudioCommand<MAX_CLIPS, SAMPLE_RATE_HZ> {
    const IDLE_SILENCE_CHUNK_WORDS: usize = 256;
    sample_buffer.fill(stereo_sample(0));

    loop {
        if let Some(audio_command) = audio_player_static.try_take_command() {
            return audio_command;
        }

        if write_words_to_i2s_with_recovery(
            i2s_tx_transfer,
            sample_buffer,
            IDLE_SILENCE_CHUNK_WORDS.min(SAMPLE_BUFFER_LEN),
        )
        .await
        .is_err()
        {
            let _ = fill_dma_ring_with_silence(i2s_tx_transfer).await;
            yield_now().await;
            continue;
        }

        yield_now().await;
    }
}

#[cfg(target_os = "none")]
async fn write_words_to_i2s(
    i2s_tx_transfer: &mut AudioI2sTxTransfer,
    sample_words: &[u32; SAMPLE_BUFFER_LEN],
    sample_word_count: usize,
) -> Result<(), ()> {
    let byte_count = sample_word_count * 4;
    let mut sample_bytes = [0_u8; SAMPLE_BUFFER_LEN * 4];
    for (write_index, sample_word_ref) in sample_words[..sample_word_count].iter().enumerate() {
        let sample_bytes_le = sample_word_ref.to_le_bytes();
        let byte_offset = write_index * 4;
        sample_bytes[byte_offset..byte_offset + 4].copy_from_slice(&sample_bytes_le);
    }

    let mut write_index = 0usize;
    while write_index < byte_count {
        let pushed_byte_count = match i2s_tx_transfer
            .push(&sample_bytes[write_index..byte_count])
            .await
        {
            Ok(pushed_byte_count) => pushed_byte_count,
            Err(_) => return Err(()),
        };
        if pushed_byte_count == 0 {
            continue;
        }
        write_index += pushed_byte_count;
    }

    Ok(())
}

#[cfg(target_os = "none")]
async fn write_words_to_i2s_with_recovery(
    i2s_tx_transfer: &mut AudioI2sTxTransfer,
    sample_words: &[u32; SAMPLE_BUFFER_LEN],
    sample_word_count: usize,
) -> Result<(), ()> {
    const MAX_RECOVERY_ATTEMPTS: usize = 3;
    for _recovery_attempt in 0..MAX_RECOVERY_ATTEMPTS {
        if write_words_to_i2s(i2s_tx_transfer, sample_words, sample_word_count)
            .await
            .is_ok()
        {
            return Ok(());
        }

        let _ = fill_dma_ring_with_silence(i2s_tx_transfer).await;
        yield_now().await;
    }

    Err(())
}

#[cfg(target_os = "none")]
async fn fill_dma_ring_with_silence(i2s_tx_transfer: &mut AudioI2sTxTransfer) -> Result<(), ()> {
    let silence_bytes = [0_u8; DMA_TX_BYTES];
    let mut write_index = 0usize;
    while write_index < DMA_TX_BYTES {
        let pushed_byte_count = match i2s_tx_transfer.push(&silence_bytes[write_index..]).await {
            Ok(pushed_byte_count) => pushed_byte_count,
            Err(_) => return Err(()),
        };
        if pushed_byte_count == 0 {
            continue;
        }
        write_index += pushed_byte_count;
    }
    Ok(())
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
            pio: I2S0,
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
            pio: I2S0,
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
                $crate::audio_player::AudioPlayerEsp::<$max_clips, { $sample_rate_hz }>::new_static_with_max_volume_and_initial_volume(
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
                player: $crate::audio_player::AudioPlayerEsp<$max_clips, { $sample_rate_hz }>,
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

                /// Creates and spawns the generated audio player instance.
                ///
                /// See the [audio_player module documentation](mod@crate::audio_player)
                /// for example usage.
                pub fn new(
                    data_pin: $crate::esp_hal::peripherals::$data_pin<'static>,
                    bit_clock_pin: $crate::esp_hal::peripherals::$bit_clock_pin<'static>,
                    word_select_pin: $crate::esp_hal::peripherals::$word_select_pin<'static>,
                    pio: $crate::esp_hal::peripherals::$pio<'static>,
                    dma: $crate::esp_hal::peripherals::$dma<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let token = [<$name:snake _audio_player_task>](
                        &[<$name:upper _AUDIO_PLAYER_STATIC>],
                        pio,
                        dma,
                        data_pin,
                        bit_clock_pin,
                        word_select_pin,
                    );
                    spawner.spawn(token)?;
                    let player =
                        $crate::audio_player::AudioPlayerEsp::new(&[<$name:upper _AUDIO_PLAYER_STATIC>]);
                    Ok([<$name:upper _AUDIO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl $crate::audio_player::AudioPlayer<{ $sample_rate_hz }> for $name {
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
                pio: $crate::esp_hal::peripherals::$pio<'static>,
                dma: $crate::esp_hal::peripherals::$dma<'static>,
                data_pin: $crate::esp_hal::peripherals::$data_pin<'static>,
                bit_clock_pin: $crate::esp_hal::peripherals::$bit_clock_pin<'static>,
                word_select_pin: $crate::esp_hal::peripherals::$word_select_pin<'static>,
            ) -> ! {
                $crate::audio_player::device_loop::<
                    $max_clips,
                    { $sample_rate_hz },
                    $crate::esp_hal::peripherals::$dma<'static>,
                    $crate::esp_hal::peripherals::$data_pin<'static>,
                    $crate::esp_hal::peripherals::$bit_clock_pin<'static>,
                    $crate::esp_hal::peripherals::$word_select_pin<'static>,
                >(audio_player_static, pio, dma, data_pin, bit_clock_pin, word_select_pin).await
            }
        }
    };
}

#[doc(inline)]
pub use audio_player;
#[doc(inline)]
pub use tone;
