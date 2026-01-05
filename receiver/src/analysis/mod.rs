//! Real-time audio analysis module.
//!
//! Provides FFT-based spectral analysis for audio streams, including:
//! - Sample buffering with overlap for continuous analysis
//! - FFT processing with windowing functions
//! - Spectral feature extraction (dominant frequency, energy, centroid)
//!
//! # Example
//!
//! ```no_run
//! use receiver::analysis::{AudioAnalyzer, AnalyzerConfig};
//!
//! let mut analyzer = AudioAnalyzer::new(AnalyzerConfig::default()).unwrap();
//!
//! // Process each decoded audio frame (320 samples @ 16kHz)
//! let samples: Vec<i16> = vec![0; 320];
//! if let Some(features) = analyzer.process_frame(&samples) {
//!     println!("Dominant frequency: {:.1} Hz", features.dominant_frequency);
//!     println!("Spectral energy: {:.1} dB", features.spectral_energy);
//! }
//! ```

mod buffer;
mod fft;
mod spectral;

pub use buffer::SampleBuffer;
pub use fft::FftProcessor;
pub use spectral::{extract_features, SpectralFeatures};

use anyhow::Result;

/// Configuration for audio analyzer.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    // ---
    /// FFT window size in samples (must be power of 2)
    pub window_size: usize,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Overlap between windows (0.0 to 1.0)
    pub overlap: f32,
}

impl Default for AnalyzerConfig {
    // ---
    fn default() -> Self {
        // ---
        Self {
            window_size: 1024,
            sample_rate: 16000,
            overlap: 0.5,
        }
    }
}

/// Main audio analyzer combining buffering, FFT, and feature extraction.
///
/// Processes incoming PCM audio frames and produces spectral features
/// when sufficient samples have been accumulated.
///
/// # Configuration
///
/// - Window size: 1024 samples (64ms @ 16kHz)
/// - Overlap: 50% (512 samples)
/// - Frequency resolution: ~15.6 Hz per bin
///
/// # Performance
///
/// FFT processing typically takes 10-50 microseconds on modern hardware.
/// Features are extracted approximately every 1.6 audio frames (32ms).
pub struct AudioAnalyzer {
    // ---
    buffer: SampleBuffer,
    fft: FftProcessor,
    sample_rate: u32,
}

impl AudioAnalyzer {
    // ---
    /// Creates a new audio analyzer with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns error if FFT processor initialization fails.
    pub fn new(config: AnalyzerConfig) -> Result<Self> {
        // ---
        Ok(Self {
            buffer: SampleBuffer::new(config.window_size, config.overlap),
            fft: FftProcessor::new(config.window_size)?,
            sample_rate: config.sample_rate,
        })
    }

    /// Process an audio frame and optionally return spectral features.
    ///
    /// Accumulates samples in an overlapped buffer. When enough samples
    /// are available, performs FFT and feature extraction.
    ///
    /// # Arguments
    ///
    /// * `samples` - PCM audio samples (i16 format)
    ///
    /// # Returns
    ///
    /// `Some(SpectralFeatures)` when a complete window is ready for analysis,
    /// `None` while accumulating samples.
    pub fn process_frame(&mut self, samples: &[i16]) -> Option<SpectralFeatures> {
        // ---
        // Add samples to buffer
        if let Some(window) = self.buffer.add_samples(samples) {
            // Window ready - perform FFT
            let spectrum = self.fft.process(&window);

            // Extract features from magnitude spectrum
            Some(extract_features(&spectrum, self.sample_rate))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        // ---
        let analyzer = AudioAnalyzer::new(AnalyzerConfig::default());
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_analyzer_custom_config() {
        // ---
        let config = AnalyzerConfig {
            window_size: 512,
            sample_rate: 16000,
            overlap: 0.5,
        };

        let analyzer = AudioAnalyzer::new(config);
        assert!(analyzer.is_ok());
    }
}
