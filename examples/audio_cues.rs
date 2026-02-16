#![allow(missing_docs)]
//! Audio cues demo with looping playback, runtime volume ramp-down, and button restart.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> the button to GND (restarts the loop)

#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration as StdDuration;

use defmt::info;
use device_envoy::{
    Result,
    audio_player::{AtEnd, Gain, VOICE_22050_HZ, Volume, audio_player, pcm_clip},
    button::{Button, PressedTo},
    silence,
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer10 {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        pio: PIO1,
        dma: DMA_CH1,
        max_clips: 8,
        max_volume: Volume::spinal_tap(11),
        initial_volume: Volume::spinal_tap(5),
    }
}

pcm_clip! {
    Nasa {
        source_sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/nasa_22k.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const NASA: &AudioPlayer10Playable = &Nasa::pcm_clip().with_gain(Gain::percent(25));
    const GAP: &AudioPlayer10Playable =
        &silence!(AudioPlayer10::SAMPLE_RATE_HZ, StdDuration::from_millis(80));

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);
    let audio_player8 = AudioPlayer10::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH1, spawner)?;

    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];

    loop {
        info!("Audio cues ready. Press the button to start playback.");
        button.wait_for_press().await;

        audio_player8.play([NASA, GAP], AtEnd::Loop);
        info!("Started looping NASA clip at initial volume (press the button to restart)");

        for volume_percent in VOLUME_STEPS_PERCENT {
            match select(
                button.wait_for_press(),
                Timer::after(Duration::from_secs(1)),
            )
            .await
            {
                Either::First(()) => {
                    info!("Button pressed: restarting");
                    break;
                }
                Either::Second(()) => {
                    audio_player8.set_volume(Volume::percent(volume_percent));
                    info!("Runtime volume set to {}%", volume_percent);
                }
            }
        }
        audio_player8.stop();
        audio_player8.set_volume(AudioPlayer10::INITIAL_VOLUME);
    }
}
