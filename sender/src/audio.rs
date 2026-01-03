//! Audio file reading and preprocessing.
//!
//! Handles WAV file parsing and conversion to the format required
//! for Opus encoding (16kHz mono PCM).

use anyhow::{Context, Result};
use hound::{WavReader, WavSpec};
use std::path::Path;
use tracing::info;

use crate::codec::{SAMPLES_PER_FRAME, SAMPLE_RATE};

/// Audio data container with PCM samples and metadata.
///
/// Contains preprocessed audio ready for encoding. Samples are always
/// 16kHz mono regardless of input file format.
#[derive(Debug)]
pub struct AudioData {
    // ---
    /// PCM samples as 16-bit signed integers
    pub samples: Vec<i16>,

    /// Original sample rate of the input file
    #[allow(dead_code)] // Metadata for debugging/logging
    pub original_sample_rate: u32,

    /// Number of channels in the original file
    #[allow(dead_code)] // Metadata for debugging/logging
    pub original_channels: u16,
}

impl AudioData {
    // ---
    /// Returns an iterator over 20ms audio frames.
    ///
    /// Each frame contains exactly SAMPLES_PER_FRAME (320) samples,
    /// suitable for Opus encoding. The last frame may be padded with
    /// zeros if the audio length is not an exact multiple of the frame size.
    pub fn frames(&self) -> impl Iterator<Item = &[i16]> {
        // ---
        self.samples.chunks(SAMPLES_PER_FRAME)
    }

    /// Returns the total duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        // ---
        self.samples.len() as f64 / SAMPLE_RATE as f64
    }

    /// Returns the number of complete frames.
    pub fn frame_count(&self) -> usize {
        // ---
        self.samples.len().div_ceil(SAMPLES_PER_FRAME)
    }
}

/// Reads and preprocesses a WAV file for streaming.
///
/// Automatically converts the audio to 16kHz mono format required for
/// Opus encoding. Supports various input sample rates and channel configurations.
///
/// # Arguments
///
/// * `path` - Path to the WAV file
///
/// # Returns
///
/// AudioData containing preprocessed samples ready for encoding.
///
/// # Errors
///
/// Returns error if:
/// - File cannot be opened
/// - WAV format is invalid
/// - Sample format is unsupported
///
/// # Example
///
/// ```no_run
/// use sender::audio::read_wav;
///
/// let audio = read_wav("voice.wav").unwrap();
/// println!("Duration: {:.2}s", audio.duration_secs());
/// ```
pub fn read_wav<P: AsRef<Path>>(path: P) -> Result<AudioData> {
    // ---
    let path = path.as_ref();
    info!("Reading WAV file: {}", path.display());

    let mut reader = WavReader::open(path)
        .with_context(|| format!("failed to open WAV file: {}", path.display()))?;

    let spec = reader.spec();
    info!(
        "WAV format: {}Hz, {} channels, {} bits",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );

    use hound::SampleFormat;

    let raw_samples: Vec<i16> = match (spec.sample_format, spec.bits_per_sample) {
        // --- Native path
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .context("failed to read 16-bit PCM WAV samples")?,

        // --- Float path
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .context("failed to read 32-bit float WAV samples")?
            .into_iter()
            .map(|s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect(),

        // --- Explicit rejection (GOOD error)
        (SampleFormat::Int, bits) => {
            anyhow::bail!(
                "unsupported integer PCM WAV format: {}-bit (only 16-bit PCM is supported)",
                bits
            );
        }

        (SampleFormat::Float, bits) => {
            anyhow::bail!(
                "unsupported float WAV format: {}-bit (only 32-bit float is supported)",
                bits
            );
        }
    };

    info!("Read {} samples from file", raw_samples.len());

    // Convert to target format (16kHz mono)
    let samples = convert_to_target_format(&raw_samples, &spec)?;

    Ok(AudioData {
        samples,
        original_sample_rate: spec.sample_rate,
        original_channels: spec.channels,
    })
}

/// Converts audio samples to target format (16kHz mono).
///
/// Handles resampling and channel conversion. Uses simple linear
/// interpolation for resampling - sufficient for voice quality
/// but not suitable for high-fidelity music.
fn convert_to_target_format(samples: &[i16], spec: &WavSpec) -> Result<Vec<i16>> {
    // ---
    let mut mono_samples = if spec.channels > 1 {
        info!("Converting {} channels to mono", spec.channels);
        convert_to_mono(samples, spec.channels as usize)
    } else {
        samples.to_vec()
    };

    // Resample if needed
    if spec.sample_rate != SAMPLE_RATE {
        info!(
            "Resampling from {}Hz to {}Hz",
            spec.sample_rate, SAMPLE_RATE
        );
        mono_samples = resample_linear(&mono_samples, spec.sample_rate, SAMPLE_RATE);
    }

    info!(
        "Converted to target format: {} samples ({} frames)",
        mono_samples.len(),
        mono_samples.len().div_ceil(SAMPLES_PER_FRAME)
    );

    Ok(mono_samples)
}

/// Converts multi-channel audio to mono by averaging channels.
fn convert_to_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    // ---
    let frame_count = samples.len() / channels;
    let mut mono = Vec::with_capacity(frame_count);

    for frame in samples.chunks(channels) {
        let sum: i32 = frame.iter().map(|&s| s as i32).sum();
        let avg = (sum / channels as i32) as i16;
        mono.push(avg);
    }

    mono
}

/// Resamples audio using linear interpolation.
///
/// This is a simple resampling algorithm suitable for voice.
/// For high-quality music, consider using a proper resampling library.
fn resample_linear(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    // ---
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut resampled = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos as usize;

        if src_idx >= samples.len() - 1 {
            // Near end, just copy last sample
            resampled.push(samples[samples.len() - 1]);
        } else {
            // Linear interpolation between adjacent samples
            let frac = src_pos - src_idx as f64;
            let s0 = samples[src_idx] as f64;
            let s1 = samples[src_idx + 1] as f64;
            let interpolated = s0 + (s1 - s0) * frac;
            resampled.push(interpolated as i16);
        }
    }

    resampled
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_convert_to_mono_stereo() {
        // ---
        // Stereo: [L1, R1, L2, R2, L3, R3]
        let stereo = vec![100, 200, 300, 400, 500, 600];
        let mono = convert_to_mono(&stereo, 2);

        // Should average each pair
        assert_eq!(mono, vec![150, 350, 550]);
    }

    #[test]
    fn test_convert_to_mono_quad() {
        // ---
        let quad = vec![100, 200, 300, 400]; // One quad frame
        let mono = convert_to_mono(&quad, 4);

        assert_eq!(mono.len(), 1);
        assert_eq!(mono[0], 250); // Average of 4 channels
    }

    #[test]
    fn test_resample_linear_upsample() {
        // ---
        let samples = vec![0, 1000, 2000];
        let resampled = resample_linear(&samples, 8000, 16000);

        // Should approximately double the sample count
        assert!(resampled.len() >= 5 && resampled.len() <= 7);
    }

    #[test]
    fn test_resample_linear_downsample() {
        // ---
        let samples = vec![0, 500, 1000, 1500, 2000];
        let resampled = resample_linear(&samples, 16000, 8000);

        // Should approximately halve the sample count
        assert!(resampled.len() >= 2 && resampled.len() <= 3);
    }

    #[test]
    fn test_resample_linear_same_rate() {
        // ---
        let samples = vec![100, 200, 300];
        let resampled = resample_linear(&samples, 16000, 16000);

        assert_eq!(resampled, samples);
    }

    #[test]
    fn test_audio_data_frames() {
        // ---
        let samples = vec![0i16; 640]; // 2 complete frames
        let audio = AudioData {
            samples,
            original_sample_rate: 16000,
            original_channels: 1,
        };

        let frames: Vec<_> = audio.frames().collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].len(), SAMPLES_PER_FRAME);
        assert_eq!(frames[1].len(), SAMPLES_PER_FRAME);
    }

    #[test]
    fn test_audio_data_duration() {
        // ---
        let samples = vec![0i16; 16000]; // 1 second at 16kHz
        let audio = AudioData {
            samples,
            original_sample_rate: 16000,
            original_channels: 1,
        };

        assert!((audio.duration_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_audio_data_frame_count() {
        // ---
        let samples = vec![0i16; 500]; // 1.56 frames
        let audio = AudioData {
            samples,
            original_sample_rate: 16000,
            original_channels: 1,
        };

        assert_eq!(audio.frame_count(), 2); // Rounds up
    }
}
