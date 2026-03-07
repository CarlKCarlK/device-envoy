#![allow(missing_docs)]

use device_envoy_core::{
    audio_player::{
        AtEnd, AudioPlayer, Gain, NARROWBAND_8000_HZ, Playable, VOICE_22050_HZ, Volume, pcm_clip,
    },
    button::Button,
};

pcm_clip! {
    Digit0 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/0_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Digit1 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/1_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Digit2 {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/2_22050.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

pcm_clip! {
    Nasa {
        file: concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/audio/nasa_22k.s16"),
        source_sample_rate_hz: VOICE_22050_HZ,
        target_sample_rate_hz: NARROWBAND_8000_HZ,
    }
}

fn play_resampled_countdown(audio_player: &impl AudioPlayer<NARROWBAND_8000_HZ>) {
    type PlayableRef = &'static dyn Playable<NARROWBAND_8000_HZ>;

    const DIGITS: [PlayableRef; 3] =
        [&Digit0::adpcm_clip(), &Digit1::adpcm_clip(), &Digit2::adpcm_clip()];
    const NASA: PlayableRef = &Nasa::pcm_clip()
        .with_gain(Gain::percent(25))
        .with_adpcm::<{ Nasa::ADPCM_DATA_LEN }>();

    audio_player.play([DIGITS[2], DIGITS[1], DIGITS[0], NASA], AtEnd::Stop);
}

struct DemoAudioPlayer;

impl AudioPlayer<NARROWBAND_8000_HZ> for DemoAudioPlayer {
    const SAMPLE_RATE_HZ: u32 = NARROWBAND_8000_HZ;
    const MAX_CLIPS: usize = 16;
    const INITIAL_VOLUME: Volume = Volume::MAX;
    const MAX_VOLUME: Volume = Volume::percent(50);

    fn play<I>(&self, _audio_clips: I, _at_end: AtEnd)
    where
        I: IntoIterator<Item = &'static dyn Playable<NARROWBAND_8000_HZ>>,
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
    play_resampled_countdown(&audio_player);
    let _future = button.wait_for_press();
}
