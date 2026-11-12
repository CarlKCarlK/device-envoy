//! Compile-time clip construction and golden data checks for the file macros.

use device_envoy_core::audio_player::{AdpcmClip, PcmClip, adpcm_clip, pcm_clip};

const SOURCE_SAMPLE_RATE_HZ: u32 = 22_050;
const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;

adpcm_clip! {
    #[cfg(any())]
    Disabled {
        file: "data/audio_with_gain/constant_1000_adpcm.wav",
    }
}

pcm_clip! {
    Tone {
        file: "data/audio_with_gain/tone_440hz_32.s16",
        source_sample_rate_hz: SOURCE_SAMPLE_RATE_HZ,
    }
}

pcm_clip! {
    ResampledTone {
        file: "data/audio_with_gain/tone_440hz_32.s16",
        source_sample_rate_hz: SOURCE_SAMPLE_RATE_HZ,
        target_sample_rate_hz: TARGET_SAMPLE_RATE_HZ,
    }
}

adpcm_clip! {
    // Fixture: mono IMA ADPCM WAV at 8 kHz, one 8-byte block with predictor
    // 1000, step index 0, and zero nibbles, decoding to nine samples of 1000.
    Constant {
        file: "data/audio_with_gain/constant_1000_adpcm.wav",
    }
}

adpcm_clip! {
    SameRateConstant {
        file: "data/audio_with_gain/constant_1000_adpcm.wav",
        target_sample_rate_hz: 8_000,
    }
}

adpcm_clip! {
    ResampledConstant {
        file: "data/audio_with_gain/constant_1000_adpcm.wav",
        target_sample_rate_hz: 4_000,
    }
}

#[test]
fn pcm_macros_construct_both_encodings_at_compile_time() {
    const PCM: PcmClip<22_050, 32> = Tone::pcm_clip();
    const ADPCM: AdpcmClip<22_050, { Tone::ADPCM_DATA_LEN }> = Tone::adpcm_clip();
    const RESAMPLED_PCM: PcmClip<16_000, 23> = ResampledTone::pcm_clip();
    const RESAMPLED_ADPCM: AdpcmClip<16_000, { ResampledTone::ADPCM_DATA_LEN }> =
        ResampledTone::adpcm_clip();

    for (samples, expected) in [
        (
            PCM.samples().as_slice(),
            include_bytes!("data/audio_with_gain/tone_440hz_32.s16").as_slice(),
        ),
        (
            RESAMPLED_PCM.samples().as_slice(),
            include_bytes!("data/audio_with_gain/tone_440hz_32_resampled_16000hz_23.s16")
                .as_slice(),
        ),
    ] {
        let actual: Vec<u8> = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        assert_eq!(actual, expected);
    }
    assert_eq!(ADPCM.pcm_sample_count(), 32);
    assert_eq!(RESAMPLED_ADPCM.pcm_sample_count(), 23);
}

#[test]
fn adpcm_macros_construct_both_encodings_at_compile_time() {
    const PCM: PcmClip<8_000, 9> = Constant::pcm_clip();
    const ADPCM: AdpcmClip<8_000, 8> = Constant::adpcm_clip();
    const SAME_RATE_PCM: PcmClip<8_000, 9> = SameRateConstant::pcm_clip();
    const SAME_RATE_ADPCM: AdpcmClip<8_000, 8> = SameRateConstant::adpcm_clip();
    const RESAMPLED_PCM: PcmClip<4_000, 5> = ResampledConstant::pcm_clip();
    const RESAMPLED_ADPCM: AdpcmClip<4_000, 8> = ResampledConstant::adpcm_clip();

    assert_eq!(PCM.samples(), &[1_000; 9]);
    assert_eq!(SAME_RATE_PCM.samples(), PCM.samples());
    assert_eq!(RESAMPLED_PCM.samples(), &[1_000; 5]);
    assert_eq!(ADPCM.data(), &[0xe8, 0x03, 0, 0, 0, 0, 0, 0]);
    assert_eq!(SAME_RATE_ADPCM.data(), ADPCM.data());
    assert_eq!(RESAMPLED_ADPCM.data(), ADPCM.data());
    assert_eq!(ADPCM.block_align(), 8);
    assert_eq!(ADPCM.samples_per_block(), 9);
    assert_eq!(ADPCM.pcm_sample_count(), 9);
    assert_eq!(RESAMPLED_ADPCM.pcm_sample_count(), 5);
}
