//! FFT processing with windowing functions.
//!
//! Performs Fast Fourier Transform on audio samples to produce
//! magnitude spectra for spectral analysis.
//!
//! This module will be fully implemented in Phase B.

use anyhow::Result;

/// FFT processor for computing magnitude spectra.
///
/// Applies Hann windowing and computes FFT magnitude spectrum
/// suitable for spectral feature extraction.
///
/// # Performance
///
/// FFT computation typically takes 10-50 microseconds for 1024-point
/// transforms on modern hardware.
pub struct FftProcessor {
    // ---
    window_size: usize,
}

impl FftProcessor {
    // ---
    /// Creates a new FFT processor.
    ///
    /// # Arguments
    ///
    /// * `window_size` - FFT size in samples (must be power of 2)
    ///
    /// # Errors
    ///
    /// Returns error if window size is invalid.
    pub fn new(window_size: usize) -> Result<Self> {
        // ---
        // Verify power of 2
        if !window_size.is_power_of_two() {
            anyhow::bail!("FFT window size must be power of 2, got {}", window_size);
        }

        Ok(Self { window_size })
    }

    /// Process audio samples through FFT.
    ///
    /// Applies Hann window and returns magnitude spectrum.
    ///
    /// # Arguments
    ///
    /// * `samples` - Normalized f32 samples (length must equal window_size)
    ///
    /// # Returns
    ///
    /// Magnitude spectrum (half of FFT output due to symmetry).
    /// Length will be `window_size / 2 + 1`.
    ///
    /// # Panics
    ///
    /// Panics if samples.len() != window_size
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        // ---
        assert_eq!(
            samples.len(),
            self.window_size,
            "Sample buffer size mismatch"
        );

        // TODO Phase B: Implement Hann windowing and FFT
        // For now, return placeholder spectrum
        vec![0.0; self.window_size / 2 + 1]
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_fft_creation() {
        // ---
        let fft = FftProcessor::new(1024);
        assert!(fft.is_ok());
    }

    #[test]
    fn test_fft_invalid_size() {
        // ---
        let fft = FftProcessor::new(1000); // Not power of 2
        assert!(fft.is_err());
    }

    #[test]
    fn test_fft_process_placeholder() {
        // ---
        let mut fft = FftProcessor::new(512).unwrap();
        let samples = vec![0.0f32; 512];

        let spectrum = fft.process(&samples);
        assert_eq!(spectrum.len(), 257); // 512/2 + 1
    }
}
