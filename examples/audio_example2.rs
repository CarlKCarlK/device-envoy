#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use device_envoy::{
    Result,
    audio_player::{AtEnd, Gain, SilenceClip, VOICE_22050_HZ, Volume, audio_player, pcm_clip},
    button::{Button, PressedTo},
    tone,
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer8 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        pio: PIO0,
        dma: DMA_CH1,
        max_clips: 8,
        max_volume: Volume::spinal_tap(11),
        initial_volume: Volume::spinal_tap(5),
    }
}

// Define a `const` function that returns audio from this PCM file.
// If unused, it adds nothing to the firmware image.
pcm_clip! {
    Nasa {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = example(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn example(spawner: Spawner) -> Result<Infallible> {
    const fn ms(milliseconds: u64) -> StdDuration {
        StdDuration::from_millis(milliseconds)
    }
    const SAMPLE_RATE_HZ: u32 = AudioPlayer8::SAMPLE_RATE_HZ;

    // Only the final transformed clips are stored in flash.
    // Intermediate compile-time temporaries (such as compression and gain steps) are not stored.

    // Read the uncompressed (PCM) NASA clip in compressed (ADPCM) format.
    const NASA: &AudioPlayer8Playable = &Nasa::adpcm_clip();
    // 80ms of silence
    const GAP: &AudioPlayer8Playable = &SilenceClip::new(ms(80));
    // 100ms of a pure 880Hz tone, at 20% loudness.
    const CHIME: &AudioPlayer8Playable =
        &tone!(880, SAMPLE_RATE_HZ, ms(100)).with_gain(Gain::percent(20));

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);
    let audio_player8 = AudioPlayer8::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH1, spawner)?;

    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];

    loop {
        // Wait for user input before starting.
        button.wait_for_press().await;
        // Start playing the NASA clip, over and over.
        audio_player8.play([CHIME, NASA, GAP], AtEnd::Loop);

        // Lower runtime volume over time, unless the button is pressed.
        for volume_percent in VOLUME_STEPS_PERCENT {
            match select(
                button.wait_for_press(),
                Timer::after(Duration::from_secs(1)),
            )
            .await
            {
                Either::First(()) => {
                    // Button pressed: leave inner loop.
                    break;
                }
                Either::Second(()) => {
                    // Timer elapsed: lower volume and keep looping.
                    audio_player8.set_volume(Volume::percent(volume_percent));
                }
            }
        }
        audio_player8.stop();
        audio_player8.set_volume(AudioPlayer8::INITIAL_VOLUME);
    }
}
