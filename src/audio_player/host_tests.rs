#![allow(missing_docs)]

use super::{AudioClip, AudioClipBuf, Gain, VOICE_22050_HZ};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TONE_SAMPLE_COUNT: usize = 32;
const TONE_FREQUENCY_HZ: u32 = 440;
type AudioClipTone = AudioClipBuf<VOICE_22050_HZ, TONE_SAMPLE_COUNT>;

#[test]
fn silence_s16le_matches_expected() -> Result<(), Box<dyn Error>> {
    let silence_audio_clip: AudioClipTone = AudioClipTone::silence();
    assert!(
        silence_audio_clip
            .samples()
            .iter()
            .all(|sample_value_ref| *sample_value_ref == 0),
        "silence clip must contain only zero samples"
    );
    assert_clip_file_matches_expected("silence_32.s16", &silence_audio_clip)
}

#[test]
fn tone_s16le_matches_expected() -> Result<(), Box<dyn Error>> {
    let tone_audio_clip: AudioClipTone = AudioClipTone::tone(TONE_FREQUENCY_HZ);
    assert!(
        tone_audio_clip
            .samples()
            .iter()
            .any(|sample_value_ref| *sample_value_ref != 0),
        "tone clip must contain non-zero samples"
    );
    assert_clip_file_matches_expected("tone_440hz_32.s16", &tone_audio_clip)
}

#[test]
fn with_gain_on_tone_changes_s16le_files_as_expected() -> Result<(), Box<dyn Error>> {
    let tone_audio_clip: AudioClipTone = AudioClipTone::tone(TONE_FREQUENCY_HZ);
    let tone_gain50_audio_clip: AudioClipTone =
        AudioClipTone::tone(TONE_FREQUENCY_HZ).with_gain(Gain::percent(50));
    let tone_gain200_audio_clip: AudioClipTone =
        AudioClipTone::tone(TONE_FREQUENCY_HZ).with_gain(Gain::percent(200));

    assert_ne!(
        tone_audio_clip.samples(),
        tone_gain50_audio_clip.samples(),
        "50% gain must change sample data"
    );
    assert_ne!(
        tone_audio_clip.samples(),
        tone_gain200_audio_clip.samples(),
        "200% gain must change sample data"
    );

    assert_clip_file_matches_expected("tone_440hz_32.s16", &tone_audio_clip)?;
    assert_clip_file_matches_expected("tone_440hz_32_gain_50.s16", &tone_gain50_audio_clip)?;
    assert_clip_file_matches_expected("tone_440hz_32_gain_200.s16", &tone_gain200_audio_clip)?;
    Ok(())
}

#[test]
fn with_resampled_same_rate_same_count_is_identity() {
    let tone_audio_clip: AudioClipTone = AudioClipTone::tone(TONE_FREQUENCY_HZ);
    let tone_resampled_audio_clip: AudioClipTone =
        AudioClipTone::tone(TONE_FREQUENCY_HZ).with_resampled();
    assert_eq!(
        tone_audio_clip.samples(),
        tone_resampled_audio_clip.samples(),
        "resampling to same rate and sample count must be identity"
    );
}

#[test]
fn with_resampled_changes_timeline_as_expected() -> Result<(), Box<dyn Error>> {
    type Tone22k = AudioClipBuf<VOICE_22050_HZ, 32>;
    type Tone16k = AudioClipBuf<16_000, 23>;
    type Tone22kRetimed = AudioClipBuf<VOICE_22050_HZ, 16>;

    let tone16k_audio_clip: Tone16k = Tone22k::tone(TONE_FREQUENCY_HZ).with_resampled();
    let tone22k_retimed_audio_clip: Tone22kRetimed =
        Tone22k::tone(TONE_FREQUENCY_HZ).with_resampled();

    assert_clip_file_matches_expected("tone_440hz_32_resampled_16000hz_23.s16", &tone16k_audio_clip)?;
    assert_clip_file_matches_expected(
        "tone_440hz_32_resampled_22050hz_16.s16",
        &tone22k_retimed_audio_clip,
    )?;
    Ok(())
}

fn assert_clip_file_matches_expected<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    filename: &str,
    audio_clip: &AudioClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>,
) -> Result<(), Box<dyn Error>> {
    let expected_path = audio_with_gain_path(filename);
    let actual_bytes = clip_to_s16le_bytes(audio_clip);

    if std::env::var_os("DEVICE_KIT_UPDATE_AUDIO").is_some() {
        fs::write(&expected_path, &actual_bytes)?;
        println!("updated audio at {}", expected_path.display());
        return Ok(());
    }

    if !expected_path.exists() {
        return Err(format!("expected audio is missing at {}", expected_path.display()).into());
    }

    let output_path = temp_output_path(filename);
    fs::write(&output_path, &actual_bytes)?;

    let expected_bytes = fs::read(&expected_path)?;
    let actual_file_bytes = fs::read(&output_path)?;
    assert_eq!(
        expected_bytes, actual_file_bytes,
        "audio bytes must match for {filename}"
    );

    fs::remove_file(&output_path)?;
    Ok(())
}

fn clip_to_s16le_bytes<const SAMPLE_RATE_HZ: u32, const SAMPLE_COUNT: usize>(
    audio_clip: &AudioClip<SAMPLE_RATE_HZ, [i16; SAMPLE_COUNT]>,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(SAMPLE_COUNT * 2);
    for sample in audio_clip.samples() {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn temp_output_path(filename: &str) -> PathBuf {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be valid")
        .as_nanos();
    let process_id = std::process::id();
    let mut path = std::env::temp_dir();
    path.push(format!("{filename}-{process_id}-{unix_time}"));
    path
}

fn audio_with_gain_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from("tests");
    path.push("data");
    path.push("audio_with_gain");
    path.push(filename);
    path
}
