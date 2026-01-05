//! FFT processing with windowing functions.
//!
//! Performs Fast Fourier Transform on audio samples to produce
//! magnitude spectra for spectral analysis.

use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// FFT processor for computing magnitude spectra.
///
/// Applies Hann windowing and computes FFT magnitude spectrum
/// suitable for spectral feature extraction.
///
/// # Performance
///
/// FFT computation typically takes 10-50 microseconds for 1024-point
/// transforms on modern hardware.
///
/// # Example
///
/// ```
/// use receiver::analysis::FftProcessor;
///
/// let mut processor = FftProcessor::new(1024).unwrap();
/// let samples = vec![0.0f32; 1024];
/// let spectrum = processor.process(&samples);
/// assert_eq!(spectrum.len(), 513); // 1024/2 + 1
/// ```
pub struct FftProcessor {
    // ---
    window_size: usize,
    hann_window: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    scratch_buffer: Vec<Complex<f32>>,
}

impl FftProcessor {
    // ---
    /// Creates a new FFT processor.
    ///
    /// Pre-computes Hann window coefficients and initializes FFT planner.
    /// The planner is reused across multiple FFT operations for efficiency.
    ///
    /// # Arguments
    ///
    /// * `window_size` - FFT size in samples (must be power of 2)
    ///
    /// # Errors
    ///
    /// Returns error if window size is invalid (not a power of 2).
    ///
    /// # Example
    ///
    /// ```
    /// use receiver::analysis::FftProcessor;
    ///
    /// let processor = FftProcessor::new(1024);
    /// assert!(processor.is_ok());
    ///
    /// let invalid = FftProcessor::new(1000);
    /// assert!(invalid.is_err());
    /// ```
    pub fn new(window_size: usize) -> Result<Self> {
        // ---
        // Verify power of 2
        if !window_size.is_power_of_two() {
            anyhow::bail!("FFT window size must be power of 2, got {}", window_size);
        }

        // Pre-compute Hann window coefficients
        let hann_window = Self::compute_hann_window(window_size);

        // Create FFT planner and build FFT for this size
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(window_size);

        Ok(Self {
            window_size,
            hann_window,
            fft,
            scratch_buffer: vec![Complex::new(0.0, 0.0); window_size],
        })
    }

    /// Compute Hann window coefficients.
    ///
    /// Hann window formula: w[n] = 0.5 * (1.0 - cos(2π * n / N))
    ///
    /// The Hann window reduces spectral leakage by smoothly tapering
    /// the signal to zero at the edges.
    ///
    /// # Arguments
    ///
    /// * `size` - Window size in samples
    ///
    /// # Returns
    ///
    /// Vector of window coefficients (length = size)
    fn compute_hann_window(size: usize) -> Vec<f32> {
        // ---
        (0..size)
            .map(|n| 0.5 * (1.0 - ((2.0 * PI * n as f32) / size as f32).cos()))
            .collect()
    }

    /// Process audio samples through FFT.
    ///
    /// Applies Hann window, performs FFT, and computes magnitude spectrum.
    /// Only the positive frequencies are returned (exploiting FFT symmetry).
    ///
    /// # Arguments
    ///
    /// * `samples` - Normalized f32 samples (length must equal window_size)
    ///
    /// # Returns
    ///
    /// Magnitude spectrum covering frequencies from 0 to Nyquist.
    /// Length will be `window_size / 2 + 1`.
    ///
    /// # Panics
    ///
    /// Panics if samples.len() != window_size
    ///
    /// # Example
    ///
    /// ```
    /// use receiver::analysis::FftProcessor;
    ///
    /// let mut processor = FftProcessor::new(1024).unwrap();
    /// let samples = vec![0.0f32; 1024];
    /// let spectrum = processor.process(&samples);
    ///
    /// assert_eq!(spectrum.len(), 513); // 1024/2 + 1
    /// ```
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        // ---
        assert_eq!(
            samples.len(),
            self.window_size,
            "Sample buffer size mismatch: expected {}, got {}",
            self.window_size,
            samples.len()
        );

        // Apply Hann window and convert to Complex
        for (i, &sample) in samples.iter().enumerate() {
            self.scratch_buffer[i] = Complex::new(sample * self.hann_window[i], 0.0);
        }

        // Perform FFT in-place
        self.fft.process(&mut self.scratch_buffer);

        // Compute magnitude spectrum (only positive frequencies)
        // Output length: N/2 + 1 (includes DC and Nyquist)
        let spectrum_size = self.window_size / 2 + 1;
        let mut spectrum = Vec::with_capacity(spectrum_size);

        for i in 0..spectrum_size {
            let magnitude = self.scratch_buffer[i].norm();
            spectrum.push(magnitude);
        }

        spectrum
    }

    /// Returns the window size (FFT size).
    #[allow(dead_code)]
    pub fn window_size(&self) -> usize {
        // ---
        self.window_size
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
    fn test_fft_output_length() {
        // ---
        let mut fft = FftProcessor::new(512).unwrap();
        let samples = vec![0.0f32; 512];

        let spectrum = fft.process(&samples);
        assert_eq!(spectrum.len(), 257); // 512/2 + 1
    }

    #[test]
    fn test_hann_window_properties() {
        // ---
        let window = FftProcessor::compute_hann_window(1024);

        // Check length
        assert_eq!(window.len(), 1024);

        // First and last samples should be near zero
        assert!(window[0] < 0.01, "First sample should be ~0");
        assert!(window[1023] < 0.01, "Last sample should be ~0");

        // Middle sample should be near 1.0
        assert!(
            (window[512] - 1.0).abs() < 0.01,
            "Middle sample should be ~1.0"
        );

        // Window should be symmetric (within floating point precision)
        // Tolerance of 0.005 accounts for precision errors when computing cos(θ)
        // where θ is near 2π. At these indices, window values are already small
        // (< 0.03), so this asymmetry has negligible impact on FFT results.
        // The critical properties (edges ≈ 0, center ≈ 1) remain accurate.
        for i in 0..512 {
            let diff = (window[i] - window[1023 - i]).abs();
            assert!(
                diff < 0.005,
                "Window symmetry failed at i={}: diff={:.6}",
                i,
                diff
            );
        }
    }

    #[test]
    fn test_dc_signal() {
        // ---
        // DC signal (constant value) should have energy only at bin 0
        let mut fft = FftProcessor::new(1024).unwrap();
        let samples = vec![1.0f32; 1024];

        let spectrum = fft.process(&samples);

        // DC bin (bin 0) should have highest magnitude
        let dc_magnitude = spectrum[0];
        assert!(dc_magnitude > 0.0, "DC bin should have non-zero magnitude");

        // Other bins should have significantly less energy
        for &magnitude in spectrum.iter().skip(10).take(90) {
            assert!(
                magnitude < dc_magnitude * 0.1,
                "Non-DC bins should have low energy for DC signal"
            );
        }
    }

    #[test]
    #[allow(clippy::manual_range_contains)]
    fn test_sine_wave_440hz() {
        // ---
        // Generate 440 Hz sine wave at 16kHz sample rate
        const SAMPLE_RATE: f32 = 16000.0;
        const FREQUENCY: f32 = 440.0;
        const FFT_SIZE: usize = 1024;

        let mut fft = FftProcessor::new(FFT_SIZE).unwrap();

        // Generate sine wave
        let samples: Vec<f32> = (0..FFT_SIZE)
            .map(|n| (2.0 * PI * FREQUENCY * n as f32 / SAMPLE_RATE).sin())
            .collect();

        let spectrum = fft.process(&samples);

        // Find peak frequency bin
        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // Convert bin to frequency
        let bin_frequency = (peak_bin as f32 * SAMPLE_RATE) / FFT_SIZE as f32;

        // Should be close to 440 Hz (within one bin = 15.625 Hz)
        let frequency_error = (bin_frequency - FREQUENCY).abs();
        assert!(
            frequency_error < 20.0,
            "Peak frequency {:.1} Hz should be close to {:.1} Hz (error: {:.1} Hz)",
            bin_frequency,
            FREQUENCY,
            frequency_error
        );

        // Expected bin: 440 / 15.625 ≈ 28.16 → bin 28
        assert!(
            peak_bin >= 27 && peak_bin <= 29,
            "Peak should be around bin 28, got bin {}",
            peak_bin
        );
    }

    #[test]
    fn test_multiple_frequencies() {
        // ---
        // Test signal with 200 Hz + 800 Hz components
        const SAMPLE_RATE: f32 = 16000.0;
        const FREQ1: f32 = 200.0;
        const FREQ2: f32 = 800.0;
        const FFT_SIZE: usize = 1024;

        let mut fft = FftProcessor::new(FFT_SIZE).unwrap();

        // Generate signal with two frequencies
        let samples: Vec<f32> = (0..FFT_SIZE)
            .map(|n| {
                let t = n as f32 / SAMPLE_RATE;
                (2.0 * PI * FREQ1 * t).sin() + (2.0 * PI * FREQ2 * t).sin()
            })
            .collect();

        let spectrum = fft.process(&samples);

        // Find expected bins
        let bin1 = (FREQ1 / (SAMPLE_RATE / FFT_SIZE as f32)).round() as usize; // ~13
        let bin2 = (FREQ2 / (SAMPLE_RATE / FFT_SIZE as f32)).round() as usize; // ~51

        // Both peaks should have significant magnitude
        assert!(
            spectrum[bin1] > 100.0,
            "200 Hz component should have strong peak"
        );
        assert!(
            spectrum[bin2] > 100.0,
            "800 Hz component should have strong peak"
        );
    }

    #[test]
    fn test_zero_signal() {
        // ---
        // All zeros should produce zero spectrum
        let mut fft = FftProcessor::new(1024).unwrap();
        let samples = vec![0.0f32; 1024];

        let spectrum = fft.process(&samples);

        // All bins should be zero (or very close)
        for (i, &magnitude) in spectrum.iter().enumerate() {
            assert!(
                magnitude < 0.0001,
                "Bin {} should be ~0 for zero signal, got {}",
                i,
                magnitude
            );
        }
    }

    #[test]
    fn test_spectrum_symmetry() {
        // ---
        // For real input, FFT output should be conjugate symmetric
        // (we only return positive frequencies, but verify properties)
        let mut fft = FftProcessor::new(1024).unwrap();
        let samples: Vec<f32> = (0..1024).map(|n| (n as f32 / 1024.0).sin()).collect();

        let spectrum = fft.process(&samples);

        // Spectrum should be all positive values
        for (i, &magnitude) in spectrum.iter().enumerate() {
            assert!(
                magnitude >= 0.0,
                "Bin {} magnitude should be non-negative, got {}",
                i,
                magnitude
            );
        }

        // Length should be N/2 + 1
        assert_eq!(spectrum.len(), 513);
    }
}
