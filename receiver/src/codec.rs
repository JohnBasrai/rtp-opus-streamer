//! Opus audio codec decoding.
//!
//! Provides a wrapper around the Opus decoder for decompressing
//! Opus-encoded audio back to PCM samples.

use anyhow::{Context, Result};
use opus::{Channels, Decoder};

/// Sample rate for audio decoding (16kHz wideband)
pub const SAMPLE_RATE: u32 = 16000;

/// Number of audio channels (mono)
#[allow(dead_code)]
pub const CHANNELS: usize = 1;

/// Frame duration in milliseconds
pub const FRAME_DURATION_MS: usize = 20;

/// Samples per frame (20ms at 16kHz)
pub const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize * FRAME_DURATION_MS) / 1000;

/// Opus decoder wrapper for audio decompression.
///
/// Decodes Opus-compressed audio frames back to PCM samples (16-bit signed integers).
/// Configured to match the sender's encoding parameters.
///
/// # Configuration
///
/// - Sample Rate: 16kHz (wideband)
/// - Channels: Mono
/// - Frame Size: 20ms (320 samples)
///
/// # Example
///
/// ```no_run
/// use receiver::codec::OpusDecoderWrapper;
///
/// let mut decoder = OpusDecoderWrapper::new().unwrap();
/// let compressed = vec![0u8; 60]; // Opus frame
/// let pcm = decoder.decode(&compressed).unwrap();
/// ```
pub struct OpusDecoderWrapper {
    // ---
    decoder: Decoder,
}

impl OpusDecoderWrapper {
    // ---
    /// Creates a new Opus decoder.
    ///
    /// # Errors
    ///
    /// Returns error if Opus decoder initialization fails.
    pub fn new() -> Result<Self> {
        // ---
        let decoder =
            Decoder::new(SAMPLE_RATE, Channels::Mono).context("failed to create Opus decoder")?;

        Ok(Self { decoder })
    }

    /// Decodes an Opus frame to PCM samples.
    ///
    /// Outputs exactly SAMPLES_PER_FRAME (320) samples regardless of
    /// input size, as Opus is a constant frame size codec.
    ///
    /// # Arguments
    ///
    /// * `data` - Compressed Opus frame data
    ///
    /// # Returns
    ///
    /// Vector of 320 PCM samples (16-bit signed integers).
    ///
    /// # Errors
    ///
    /// Returns error if Opus decoding fails (corrupted data, invalid format).
    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<i16>> {
        // ---
        let mut output = vec![0i16; SAMPLES_PER_FRAME];

        let decoded = self
            .decoder
            .decode(data, &mut output, false)
            .context("Opus decoding failed")?;

        if decoded != SAMPLES_PER_FRAME {
            anyhow::bail!(
                "unexpected decoded frame size: expected {}, got {}",
                SAMPLES_PER_FRAME,
                decoded
            );
        }

        Ok(output)
    }

    /// Generates packet loss concealment (PLC) when a frame is lost.
    ///
    /// Uses Opus's built-in PLC algorithm to synthesize plausible audio
    /// when network packets are lost. The output maintains temporal continuity
    /// with previous frames.
    ///
    /// # Returns
    ///
    /// Vector of 320 concealed PCM samples.
    ///
    /// # Errors
    ///
    /// Returns error if PLC generation fails.
    pub fn conceal_loss(&mut self) -> Result<Vec<i16>> {
        // ---
        let mut output = vec![0i16; SAMPLES_PER_FRAME];

        let decoded = self
            .decoder
            .decode(&[], &mut output, true) // fec=true triggers PLC
            .context("Opus PLC failed")?;

        if decoded != SAMPLES_PER_FRAME {
            anyhow::bail!(
                "unexpected PLC frame size: expected {}, got {}",
                SAMPLES_PER_FRAME,
                decoded
            );
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_decoder_creation() {
        // ---
        let decoder = OpusDecoderWrapper::new();
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_decode_opus_frame() {
        // ---
        // First encode a frame so we have valid Opus data
        use opus::{Application, Encoder};

        let mut encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Voip)
            .expect("encoder creation failed");

        let silence = vec![0i16; SAMPLES_PER_FRAME];
        let mut encoded = vec![0u8; 4000];
        let len = encoder
            .encode(&silence, &mut encoded)
            .expect("encoding failed");
        encoded.truncate(len);

        // Now decode it
        let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");
        let result = decoder.decode(&encoded);

        assert!(result.is_ok());
        let decoded = result.unwrap();
        assert_eq!(decoded.len(), SAMPLES_PER_FRAME);
    }

    #[test]
    fn test_packet_loss_concealment() {
        // ---
        let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");

        let result = decoder.conceal_loss();
        assert!(result.is_ok());

        let concealed = result.unwrap();
        assert_eq!(concealed.len(), SAMPLES_PER_FRAME);
    }

    #[test]
    fn test_decode_invalid_data() {
        // ---
        let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");

        let invalid = vec![0xFF; 10]; // Invalid Opus data
        let result = decoder.decode(&invalid);

        // Should fail gracefully
        assert!(result.is_err());
    }
}
