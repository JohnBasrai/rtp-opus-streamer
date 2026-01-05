//! Spectral feature extraction from magnitude spectra.
//!
//! Extracts meaningful audio features from FFT magnitude spectra,
//! including dominant frequency, spectral energy, and spectral centroid.

/// Spectral features extracted from magnitude spectrum.
///
/// Provides key audio characteristics useful for analysis and classification.
/// All fields include explicit units in their names for clarity.
///
/// # Example
///
/// ```
/// use receiver::analysis::{extract_features, SpectralFeatures};
///
/// let spectrum = vec![0.0f32; 513]; // 1024-point FFT @ 16kHz
/// let features = extract_features(&spectrum, 16000);
///
/// println!("Dominant frequency: {:.1} Hz", features.dominant_frequency_hz);
/// println!("Spectral energy: {:.1} dB", features.spectral_energy_db);
/// println!("Spectral centroid: {:.1} Hz", features.spectral_centroid_hz);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralFeatures {
    // ---
    /// Dominant frequency in Hz (frequency of peak magnitude in spectrum)
    pub dominant_frequency_hz: f32,

    /// Spectral energy in dB (overall signal magnitude)
    pub spectral_energy_db: f32,

    /// Spectral centroid in Hz (weighted average frequency, indicates "brightness")
    pub spectral_centroid_hz: f32,
}

/// Extract spectral features from magnitude spectrum.
///
/// Analyzes FFT magnitude spectrum to compute audio characteristics.
/// The spectrum should be the output of an FFT (positive frequencies only).
///
/// # Arguments
///
/// * `spectrum` - Magnitude spectrum from FFT (length = FFT_size/2 + 1)
/// * `sample_rate` - Audio sample rate in Hz
///
/// # Returns
///
/// Spectral features including dominant frequency, energy, and centroid.
///
/// # Example
///
/// ```
/// use receiver::analysis::{extract_features, SpectralFeatures};
///
/// let spectrum = vec![0.0f32; 513]; // 1024-point FFT
/// let features = extract_features(&spectrum, 16000);
/// ```
pub fn extract_features(spectrum: &[f32], sample_rate: u32) -> SpectralFeatures {
    // ---
    // Compute frequency resolution (Hz per bin)
    let fft_size = (spectrum.len() - 1) * 2; // Reconstruct FFT size from spectrum length
    let freq_resolution = sample_rate as f32 / fft_size as f32;

    // Find dominant frequency (peak bin)
    let dominant_frequency_hz = find_dominant_frequency(spectrum, freq_resolution);

    // Compute spectral energy in dB
    let spectral_energy_db = compute_spectral_energy_db(spectrum);

    // Compute spectral centroid
    let spectral_centroid_hz = compute_spectral_centroid(spectrum, freq_resolution);

    SpectralFeatures {
        dominant_frequency_hz,
        spectral_energy_db,
        spectral_centroid_hz,
    }
}

/// Find the dominant frequency (frequency of peak magnitude).
///
/// Locates the bin with maximum magnitude and converts to frequency.
///
/// # Arguments
///
/// * `spectrum` - Magnitude spectrum
/// * `freq_resolution` - Frequency resolution in Hz per bin
///
/// # Returns
///
/// Dominant frequency in Hz
fn find_dominant_frequency(spectrum: &[f32], freq_resolution: f32) -> f32 {
    // ---
    let peak_bin = spectrum
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    peak_bin as f32 * freq_resolution
}

/// Compute spectral energy in dB.
///
/// Calculates the mean magnitude across all bins and converts to dB scale.
/// Uses 20 * log10(magnitude) conversion, with floor to handle zero/near-zero values.
///
/// # Arguments
///
/// * `spectrum` - Magnitude spectrum
///
/// # Returns
///
/// Spectral energy in dB (typically negative values, 0 dB = reference magnitude of 1.0)
fn compute_spectral_energy_db(spectrum: &[f32]) -> f32 {
    // ---
    if spectrum.is_empty() {
        return -100.0; // Silence floor
    }

    // Compute mean magnitude
    let mean_magnitude: f32 = spectrum.iter().sum::<f32>() / spectrum.len() as f32;

    // Convert to dB with floor to handle silence
    if mean_magnitude > 1e-10 {
        20.0 * mean_magnitude.log10()
    } else {
        -100.0 // Silence floor
    }
}

/// Compute spectral centroid (weighted average frequency).
///
/// The spectral centroid indicates the "center of mass" of the spectrum
/// and is correlated with perceived brightness of the sound.
///
/// Formula: Σ(f[i] * magnitude[i]) / Σ(magnitude[i])
///
/// # Arguments
///
/// * `spectrum` - Magnitude spectrum
/// * `freq_resolution` - Frequency resolution in Hz per bin
///
/// # Returns
///
/// Spectral centroid in Hz
fn compute_spectral_centroid(spectrum: &[f32], freq_resolution: f32) -> f32 {
    // ---
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (bin, &magnitude) in spectrum.iter().enumerate() {
        let frequency = bin as f32 * freq_resolution;
        weighted_sum += frequency * magnitude;
        magnitude_sum += magnitude;
    }

    if magnitude_sum > 1e-10 {
        weighted_sum / magnitude_sum
    } else {
        0.0 // Silence or near-silence
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_dominant_frequency_single_peak() {
        // ---
        // Create spectrum with peak at bin 28 (440 Hz @ 16kHz, 1024 FFT)
        // Freq resolution: 16000 / 1024 = 15.625 Hz/bin
        let mut spectrum = vec![0.0f32; 513]; // 1024-point FFT
        spectrum[28] = 100.0; // Peak at bin 28

        let features = extract_features(&spectrum, 16000);

        // Expected: 28 * 15.625 = 437.5 Hz
        assert!(
            (features.dominant_frequency_hz - 437.5).abs() < 1.0,
            "Expected ~437.5 Hz, got {}",
            features.dominant_frequency_hz
        );
    }

    #[test]
    fn test_dominant_frequency_dc() {
        // ---
        // DC signal (peak at bin 0)
        let mut spectrum = vec![0.0f32; 513];
        spectrum[0] = 100.0;

        let features = extract_features(&spectrum, 16000);

        assert_eq!(
            features.dominant_frequency_hz, 0.0,
            "DC signal should have 0 Hz dominant frequency"
        );
    }

    #[test]
    fn test_spectral_energy_calculation() {
        // ---
        // Uniform spectrum (all bins = 1.0)
        let spectrum = vec![1.0f32; 513];

        let features = extract_features(&spectrum, 16000);

        // Mean magnitude = 1.0 → 20*log10(1.0) = 0 dB
        assert!(
            (features.spectral_energy_db - 0.0).abs() < 0.1,
            "Expected ~0 dB for magnitude 1.0, got {}",
            features.spectral_energy_db
        );
    }

    #[test]
    fn test_spectral_energy_silence() {
        // ---
        // All zeros (silence)
        let spectrum = vec![0.0f32; 513];

        let features = extract_features(&spectrum, 16000);

        // Should return silence floor
        assert!(
            features.spectral_energy_db <= -99.0,
            "Silence should produce very low dB value"
        );
    }

    #[test]
    fn test_spectral_energy_scale() {
        // ---
        // Test that higher magnitude = higher dB
        let spectrum1 = vec![1.0f32; 513];
        let spectrum2 = vec![10.0f32; 513];

        let features1 = extract_features(&spectrum1, 16000);
        let features2 = extract_features(&spectrum2, 16000);

        // 10x magnitude = +20 dB
        let db_diff = features2.spectral_energy_db - features1.spectral_energy_db;
        assert!(
            (db_diff - 20.0).abs() < 0.1,
            "10x magnitude should be +20 dB, got {} dB difference",
            db_diff
        );
    }

    #[test]
    fn test_spectral_centroid_single_peak() {
        // ---
        // Peak at bin 50 only
        let mut spectrum = vec![0.0f32; 513];
        spectrum[50] = 100.0;

        let features = extract_features(&spectrum, 16000);

        // Centroid should be at the peak frequency
        // 50 * 15.625 = 781.25 Hz
        let expected = 50.0 * 15.625;
        assert!(
            (features.spectral_centroid_hz - expected).abs() < 1.0,
            "Expected centroid at {} Hz, got {}",
            expected,
            features.spectral_centroid_hz
        );
    }

    #[test]
    fn test_spectral_centroid_two_peaks() {
        // ---
        // Two equal peaks at bins 20 and 60
        let mut spectrum = vec![0.0f32; 513];
        spectrum[20] = 50.0;
        spectrum[60] = 50.0;

        let features = extract_features(&spectrum, 16000);

        // Centroid should be midway: (20 + 60) / 2 = 40
        // 40 * 15.625 = 625 Hz
        let expected = 40.0 * 15.625;
        assert!(
            (features.spectral_centroid_hz - expected).abs() < 1.0,
            "Expected centroid at {} Hz, got {}",
            expected,
            features.spectral_centroid_hz
        );
    }

    #[test]
    fn test_spectral_centroid_weighted() {
        // ---
        // Asymmetric peaks: bin 20 (mag 100), bin 60 (mag 50)
        let mut spectrum = vec![0.0f32; 513];
        spectrum[20] = 100.0;
        spectrum[60] = 50.0;

        let features = extract_features(&spectrum, 16000);

        // Weighted average: (20*100 + 60*50) / (100 + 50) = 5000/150 = 33.33
        // 33.33 * 15.625 ≈ 520.8 Hz
        let expected = (20.0 * 100.0 + 60.0 * 50.0) / (100.0 + 50.0) * 15.625;
        assert!(
            (features.spectral_centroid_hz - expected).abs() < 1.0,
            "Expected weighted centroid at {} Hz, got {}",
            expected,
            features.spectral_centroid_hz
        );
    }

    #[test]
    fn test_spectral_centroid_silence() {
        // ---
        // All zeros
        let spectrum = vec![0.0f32; 513];

        let features = extract_features(&spectrum, 16000);

        assert_eq!(
            features.spectral_centroid_hz, 0.0,
            "Silence should have 0 Hz centroid"
        );
    }

    #[test]
    fn test_field_names_have_units() {
        // ---
        // Compile-time check that fields have unit suffixes
        let features = SpectralFeatures {
            dominant_frequency_hz: 440.0,
            spectral_energy_db: -20.0,
            spectral_centroid_hz: 1000.0,
        };

        // This test ensures the field names are correct
        assert!(features.dominant_frequency_hz > 0.0);
        assert!(features.spectral_energy_db < 0.0);
        assert!(features.spectral_centroid_hz > 0.0);
    }
}
