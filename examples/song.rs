#![allow(missing_docs)]
//! Play the opening phrase of "Mary Had a Little Lamb" on MAX98357A over I²S.
//!
//! Wiring:
//! - Data pin (`DIN`) -> GP8
//! - Bit clock pin (`BCLK`) -> GP9
//! - Word select pin (`LRC` / `LRCLK`) -> GP10
//! - SD   -> 3V3 (enabled; commonly selects left channel depending on breakout)
//! - Button -> GP13 to GND (starts playback)

#![no_std]
#![no_main]

use core::convert::Infallible;
use core::time::Duration;

use defmt::info;
use device_envoy::{
    Result,
    audio_player::{AtEnd, PRO_48000_HZ, Volume, audio_player},
    silence, tone,
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

audio_player! {
    SongPlayer {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: PRO_48000_HZ,
        max_volume: Volume::percent(50),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    const NOTE_E4: &SongPlayerPlayable =
        &tone!(330, SongPlayer::SAMPLE_RATE_HZ, Duration::from_millis(220));
    const NOTE_D4: &SongPlayerPlayable =
        &tone!(294, SongPlayer::SAMPLE_RATE_HZ, Duration::from_millis(220));
    const NOTE_C4: &SongPlayerPlayable =
        &tone!(262, SongPlayer::SAMPLE_RATE_HZ, Duration::from_millis(220));
    const REST_80MS: &SongPlayerPlayable =
        &silence!(SongPlayer::SAMPLE_RATE_HZ, Duration::from_millis(80));

    let p = embassy_rp::init(Default::default());
    let song_player = SongPlayer::new(p.PIN_8, p.PIN_9, p.PIN_10, p.PIO0, p.DMA_CH0, spawner)?;

    info!(
        "I²S ready: GP8 data pin (DIN), GP9 bit clock pin (BCLK), GP10 word select pin (LRC/LRCLK)"
    );
    info!("Playing the Mary phrase once");

    // Mary had a little lamb (opening phrase): E D C D E E E
    song_player.play(
        [
            NOTE_E4, REST_80MS, NOTE_D4, REST_80MS, NOTE_C4, REST_80MS, NOTE_D4, REST_80MS,
            NOTE_E4, REST_80MS, NOTE_E4, REST_80MS, NOTE_E4,
        ],
        AtEnd::Stop,
    );

    core::future::pending().await
}
