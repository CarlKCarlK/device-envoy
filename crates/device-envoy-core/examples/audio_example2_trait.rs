#![allow(missing_docs)]

use core::time::Duration as StdDuration;

use device_envoy_core::{
    audio_player::{
        AtEnd, AudioPlayer, Gain, Playable, SilenceClip, VOICE_22050_HZ, Volume, pcm_clip, tone,
    },
    button::Button,
};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};

pcm_clip! {
    Nasa {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
    }
}

async fn play_nasa_with_runtime_volume(
    audio_player: &impl AudioPlayer<VOICE_22050_HZ>,
    button: &mut impl Button,
) -> ! {
    type PlayableRef = &'static dyn Playable<VOICE_22050_HZ>;

    const fn ms(milliseconds: u64) -> StdDuration {
        StdDuration::from_millis(milliseconds)
    }

    const NASA: PlayableRef = &Nasa::adpcm_clip();
    const GAP: PlayableRef = &SilenceClip::new(ms(80));
    const CHIME: PlayableRef = &tone!(880, VOICE_22050_HZ, ms(100)).with_gain(Gain::percent(20));
    const VOLUME_STEPS_PERCENT: [u8; 7] = [50, 25, 12, 6, 3, 1, 0];
    let initial_volume = audio_player.volume();

    loop {
        audio_player.play([CHIME, NASA, GAP], AtEnd::Loop);

        for volume_percent in VOLUME_STEPS_PERCENT {
            match select(
                button.wait_for_press(),
                Timer::after(Duration::from_secs(1)),
            )
            .await
            {
                Either::First(()) => break,
                Either::Second(()) => audio_player.set_volume(Volume::percent(volume_percent)),
            }
        }

        audio_player.stop();
        audio_player.set_volume(initial_volume);
        button.wait_for_press().await;
    }
}

struct DemoAudioPlayer;

impl AudioPlayer<VOICE_22050_HZ> for DemoAudioPlayer {
    const SAMPLE_RATE_HZ: u32 = VOICE_22050_HZ;
    const MAX_CLIPS: usize = 8;
    const INITIAL_VOLUME: Volume = Volume::spinal_tap(5);
    const MAX_VOLUME: Volume = Volume::spinal_tap(11);

    fn play<I>(&self, _audio_clips: I, _at_end: AtEnd)
    where
        I: IntoIterator<Item = &'static dyn Playable<VOICE_22050_HZ>>,
    {
    }

    fn stop(&self) {}

    async fn wait_until_stopped(&self) {}

    fn set_volume(&self, _volume: Volume) {}

    fn volume(&self) -> Volume {
        Self::INITIAL_VOLUME
    }
}

struct DemoButton;

impl Button for DemoButton {
    fn is_pressed(&self) -> bool {
        false
    }

    async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
}

fn main() {
    let mut button = DemoButton;
    let audio_player = DemoAudioPlayer;
    let _future = play_nasa_with_runtime_volume(&audio_player, &mut button);
}
// todo00 understand every use of DemoButton.
