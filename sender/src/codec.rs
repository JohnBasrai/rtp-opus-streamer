//! Opus audio codec encoding.
//!
//! Provides a wrapper around the Opus encoder for consistent encoding
//! of PCM audio samples to compressed Opus format.

use anyhow::{Context, Result};
use opus::{Application, Channels, Encoder};

/// Sample rate for audio encoding (16kHz wideband)
pub const SAMPLE_RATE: u32 = 16000;

/// Number of audio channels (mono)
#[allow(dead_code)] // Kept for consistency with receiver
pub const CHANNELS: usize = 1;

/// Frame duration in milliseconds
pub const FRAME_DURATION_MS: usize = 20;

/// Samples per frame (20ms at 16kHz)
pub const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize * FRAME_DURATION_MS) / 1000;

/// Target bitrate in bits per second
pub const BITRATE: i32 = 24000;

/// Opus encoder wrapper for audio compression.
///
/// Encodes PCM audio samples (16-bit signed integers) into Opus-compressed
/// frames. Configured for voice-optimized encoding at 16kHz sample rate.
///
/// # Configuration
///
/// - Sample Rate: 16kHz (wideband)
/// - Channels: Mono
/// - Bitrate: 24 kbps
/// - Frame Size: 20ms (320 samples)
/// - Application: VOIP (optimized for speech)
///
/// # Example
///
/// ```no_run
/// use sender::codec::OpusEncoderWrapper;
///
/// let mut encoder = OpusEncoderWrapper::new().unwrap();
/// let pcm_samples = vec![0i16; 320]; // 20ms of silence
/// let compressed = encoder.encode(&pcm_samples).unwrap();
/// ```
pub struct OpusEncoderWrapper {
    // ---
    encoder: Encoder,
}

impl OpusEncoderWrapper {
    // ---
    /// Creates a new Opus encoder with voice-optimized settings.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Opus encoder initialization fails
    /// - Bitrate setting fails
    pub fn new() -> Result<Self> {
        // ---
        let mut encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Voip)
            .context("failed to create Opus encoder")?;

        encoder
            .set_bitrate(opus::Bitrate::Bits(BITRATE))
            .context("failed to set bitrate")?;

        Ok(Self { encoder })
    }

    /// Encodes PCM audio samples into Opus format.
    ///
    /// Expects exactly 320 samples (20ms at 16kHz). The output size varies
    /// depending on audio complexity but is typically 60-120 bytes at 24 kbps.
    ///
    /// # Arguments
    ///
    /// * `pcm` - Slice of 16-bit PCM samples (must be exactly 320 samples)
    ///
    /// # Returns
    ///
    /// Compressed audio data ready for RTP packetization.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Input size is not exactly SAMPLES_PER_FRAME (320)
    /// - Opus encoding fails
    pub fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>> {
        // ---
        if pcm.len() != SAMPLES_PER_FRAME {
            anyhow::bail!(
                "invalid frame size: expected {}, got {}",
                SAMPLES_PER_FRAME,
                pcm.len()
            );
        }

        let mut output = vec![0u8; 4000]; // Max Opus frame size
        let len = self
            .encoder
            .encode(pcm, &mut output)
            .context("Opus encoding failed")?;

        output.truncate(len);
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_encoder_creation() {
        // ---
        let encoder = OpusEncoderWrapper::new();
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_encode_silence() {
        // ---
        let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
        let silence = vec![0i16; SAMPLES_PER_FRAME];

        let result = encoder.encode(&silence);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert!(!encoded.is_empty());
        // Opus should compress silence very efficiently
        assert!(encoded.len() < 100);
    }

    #[test]
    fn test_encode_invalid_frame_size() {
        // ---
        let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
        let wrong_size = vec![0i16; 160]; // Wrong size

        let result = encoder.encode(&wrong_size);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_tone() {
        // ---
        let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");

        // Generate a simple sine wave tone
        let mut tone = Vec::with_capacity(SAMPLES_PER_FRAME);
        for i in 0..SAMPLES_PER_FRAME {
            let sample = (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / SAMPLE_RATE as f32).sin();
            tone.push((sample * 16000.0) as i16);
        }

        let result = encoder.encode(&tone);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert!(!encoded.is_empty());
        // Tone should be less compressible than silence
        assert!(encoded.len() > 20);
    }
}
