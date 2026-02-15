#![allow(missing_docs)]
//! Compile-only negative test: with_resampled must reject a wrong destination sample count.
//!
//! This file is expected to fail compilation and is validated by `cargo check-all`.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy::audio_player::AudioClipBuf;
use embassy_executor::Spawner;

static SOURCE_CLIP: AudioClipBuf<4, 4> = AudioClipBuf::new([100, 200, 300, 400]);
static BAD_RESAMPLED_CLIP: AudioClipBuf<8, 7> = SOURCE_CLIP.with_resampled();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _ = BAD_RESAMPLED_CLIP.sample_count();
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
