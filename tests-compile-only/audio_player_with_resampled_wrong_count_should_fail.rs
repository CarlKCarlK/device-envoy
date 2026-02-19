#![allow(missing_docs)]
//! Compile-only negative test: resample helper must reject a wrong destination sample count.
//!
//! This file is expected to fail compilation and is validated by `cargo check-all`.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy::audio_player::{__pcm_clip_from_samples, __resample_pcm_clip, PcmClipBuf};
use embassy_executor::Spawner;

static BAD_RESAMPLED_CLIP: PcmClipBuf<8, 7> =
    __resample_pcm_clip::<4, 4, 8, 7>(__pcm_clip_from_samples([100, 200, 300, 400]));

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _ = &BAD_RESAMPLED_CLIP;
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
