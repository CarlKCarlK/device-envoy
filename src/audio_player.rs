//! A device abstraction for queued PCM clip playback over PIO I2S.
//!
//! See [`AudioPlayer`] for the core API and [`audio_player!`] for the generated
//! device pattern that pins down `PIO`, `DMA`, and `max_clips`.

use core::borrow::Borrow;

use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::Pin;
use embassy_rp::pio::{Instance, Pio, PioPin};
use embassy_rp::pio_programs::i2s::{PioI2sOut, PioI2sOutProgram};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

const SAMPLE_RATE_HZ: u32 = 22_050;
const BIT_DEPTH_BITS: u32 = 16;
const AMPLITUDE: i16 = 8_000;
const SAMPLE_BUFFER_LEN: usize = 256;

/// End-of-sequence behavior for playback.
pub enum AtEnd {
    /// Replay the full clip sequence forever.
    Loop,
    /// Stop after one full clip sequence pass.
    AtEnd,
}

enum AudioCommand<const MAX_CLIPS: usize> {
    Play {
        audio_clips: Vec<&'static [i16], MAX_CLIPS>,
        at_end: AtEnd,
    },
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

/// Queues static PCM clips for playback by the background device task.
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
        Self { audio_player_static }
    }

    /// Queues one or more static PCM clips for playback.
    ///
    /// The iterator is copied into a fixed-capacity queue defined by `MAX_CLIPS`.
    pub fn play<I>(&self, audio_clips: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<&'static [i16]>,
    {
        assert!(MAX_CLIPS > 0, "play disabled: max_clips is 0");
        let mut audio_clip_sequence: Vec<&'static [i16], MAX_CLIPS> = Vec::new();
        for audio_clip in audio_clips {
            let audio_clip = *audio_clip.borrow();
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
        let audio_command = audio_player_static.wait().await;
        audio_player_static.command_signal.reset();

        match audio_command {
            AudioCommand::Play { audio_clips, at_end } => match at_end {
                AtEnd::AtEnd => {
                    play_clip_sequence_once(&mut pio_i2s_out, &audio_clips, &mut sample_buffer).await;
                }
                AtEnd::Loop => loop {
                    play_clip_sequence_once(&mut pio_i2s_out, &audio_clips, &mut sample_buffer).await;
                },
            },
        }
    }
}

async fn play_clip_sequence_once<PIO: Instance>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_clips: &[&'static [i16]],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    for audio_sample_i16 in audio_clips {
        play_full_clip_once(pio_i2s_out, audio_sample_i16, sample_buffer).await;
    }
}

async fn play_full_clip_once<PIO: Instance>(
    pio_i2s_out: &mut PioI2sOut<'static, PIO, 0>,
    audio_sample_i16: &[i16],
    sample_buffer: &mut [u32; SAMPLE_BUFFER_LEN],
) {
    for audio_sample_chunk in audio_sample_i16.chunks(SAMPLE_BUFFER_LEN) {
        for (sample_buffer_slot, sample_value_ref) in
            sample_buffer.iter_mut().zip(audio_sample_chunk.iter())
        {
            let sample_value = *sample_value_ref;
            // TODO0 should we preprocess all this? (moved from examples/audio.rs)
            let scaled_sample = ((i32::from(sample_value) * i32::from(AMPLITUDE)) / 32_768) as i16;
            *sample_buffer_slot = stereo_sample(scaled_sample);
        }

        sample_buffer[audio_sample_chunk.len()..].fill(stereo_sample(0));
        pio_i2s_out.write(sample_buffer).await;
    }
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
    (
        $vis:vis $name:ident {
            din_pin: $din_pin:ident,
            bclk_pin: $bclk_pin:ident,
            lrc_pin: $lrc_pin:ident,
            pio: $pio:ident,
            dma: $dma:ident,
            max_clips: $max_clips:expr $(,)?
        }
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
