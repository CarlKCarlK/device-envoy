#![allow(missing_docs)]
//! Audio cues demo with looping playback, runtime volume ramp-down, and button restart.
//!
//! Wiring:
//! - DIN  -> GP8
//! - BCLK -> GP9
//! - LRC  -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> the button to GND (restarts the loop)

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use device_envoy::{
    Result,
    audio_player::{AtEnd, Gain, VOICE_22050_HZ, Volume, audio_clip, audio_player, samples_ms},
    button::{Button, PressedTo},
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    AudioPlayer10 {
        din_pin: PIN_8,
        bclk_pin: PIN_9,
        lrc_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        pio: PIO1,
        dma: DMA_CH1,
        max_clips: 8,
        max_volume: Volume::spinal_tap(11),
        initial_volume: Volume::spinal_tap(5),
    }
}

audio_clip! {
    Nasa {
        sample_rate_hz: VOICE_22050_HZ,
        file: "data/audio/nasa_22k.s16",
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static NASA: Nasa::AudioClip = Nasa::audio_clip().with_gain(Gain::percent(25));
    static GAP: samples_ms! { AudioPlayer10, 80 } = AudioPlayer10::silence();

    let p = embassy_rp::init(Default::default());
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);
    let audio_player8 = AudioPlayer10::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO1, p.DMA_CH1, spawner)?;

    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];

    loop {
        info!("Audio cues ready. Press the button to start playback.");
        button.wait_for_press().await;

        audio_player8.play([&NASA, &GAP], AtEnd::Loop);
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
