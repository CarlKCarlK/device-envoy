// @generated examples/templates/audio20k_one.rs.j2 by cargo xtask generate-board-examples.
//!
//! Wiring:
//! - Audio data pin (`DIN`) -> GPIO21
//! - Audio bit clock pin (`BCLK`) -> GPIO4
//! - Audio word select pin (`LRC` / `LRCLK`) -> GPIO5
#![allow(missing_docs)]
//! audio20k_one: one short 22.05 kHz clip, then stop.
//!
//! Purpose:
//! - Tail-repeat verification for `AtEnd::Stop`
//!
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::audio_player::{
    AtEnd, AudioPlayer as _, VOICE_22050_HZ, Volume, audio_player, pcm_clip,
};
use device_envoy_esp::{Result, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

audio_player! {
    AudioPlayerBoard {
        data_pin: GPIO21,
        bit_clock_pin: GPIO4,
        word_select_pin: GPIO5,
        sample_rate_hz: VOICE_22050_HZ,
        dma: DMA_I2S0,
        max_volume: Volume::percent(40),
    }
}

pcm_clip! {
    Digit0 {
        file: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../device-envoy-core/examples/data/audio/0_22050.s16"
        ),
        source_sample_rate_hz: VOICE_22050_HZ,
    }
}

const CLIP: &AudioPlayerBoardPlayable = &Digit0::pcm_clip();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let audio_player_board =
        AudioPlayerBoard::new(p.GPIO21, p.GPIO4, p.GPIO5, p.I2S0, p.DMA_I2S0, spawner)?;

    info!(
        "phase: before_play sample_rate_hz={}",
        AudioPlayerBoard::SAMPLE_RATE_HZ
    );
    audio_player_board.play([CLIP], AtEnd::Stop);
    info!("phase: after_play_call");
    audio_player_board.wait_until_stopped().await;
    info!("phase: wait_until_stopped_returned");
    info!("If audio stopped cleanly with no repeated tail, this board is good.");

    pending().await
}
