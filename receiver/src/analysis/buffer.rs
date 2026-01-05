//! Sample buffering for FFT analysis.
//!
//! Accumulates incoming PCM samples and provides overlapped windows
//! for continuous spectral analysis.

use std::collections::VecDeque;

/// Sample buffer for accumulating PCM samples into FFT-ready windows.
///
/// Manages overlapped windowing to ensure continuous spectral analysis.
/// Converts i16 PCM samples to normalized f32 samples suitable for FFT.
///
/// # Design
///
/// - Accumulates samples in a ring buffer
/// - Returns windows when hop size reached (hop = window_size * (1.0 - overlap))
/// - Preserves overlap samples between windows for continuity
///
/// # Example
///
/// ```
/// use receiver::analysis::SampleBuffer;
///
/// let mut buffer = SampleBuffer::new(1024, 0.5); // 50% overlap
///
/// // Typically receive 320 samples per frame @ 16kHz
/// let frame: Vec<i16> = vec![0; 320];
///
/// // Accumulate frames until window ready
/// if let Some(window) = buffer.add_samples(&frame) {
///     // window.len() == 1024
///     assert_eq!(window.len(), 1024);
/// }
/// ```
pub struct SampleBuffer {
    // ---
    /// Ring buffer holding samples across windows
    buffer: VecDeque<f32>,

    /// FFT window size in samples
    window_size: usize,

    /// Hop size in samples (advance between windows)
    hop_size: usize,

    /// Number of samples accumulated since last window
    samples_since_last: usize,
}

impl SampleBuffer {
    // ---
    /// Creates a new sample buffer.
    ///
    /// # Arguments
    ///
    /// * `window_size` - FFT window size in samples (should be power of 2)
    /// * `overlap` - Overlap fraction (0.0 to 1.0), typically 0.5 for 50%
    ///
    /// # Example
    ///
    /// ```
    /// use receiver::analysis::SampleBuffer;
    ///
    /// let buffer = SampleBuffer::new(1024, 0.5);
    /// ```
    pub fn new(window_size: usize, overlap: f32) -> Self {
        // ---
        let hop_size = (window_size as f32 * (1.0 - overlap)) as usize;

        Self {
            buffer: VecDeque::with_capacity(window_size * 2),
            window_size,
            hop_size,
            samples_since_last: 0,
        }
    }

    /// Add PCM samples and optionally return a complete window.
    ///
    /// Converts i16 samples to normalized f32 and accumulates in buffer.
    /// Returns a window when enough samples have been accumulated (hop size reached).
    ///
    /// # Arguments
    ///
    /// * `samples` - i16 PCM samples (typically 320 samples @ 16kHz per 20ms frame)
    ///
    /// # Returns
    ///
    /// `Some(Vec<f32>)` containing `window_size` normalized samples when ready,
    /// `None` while accumulating.
    ///
    /// # Normalization
    ///
    /// i16 samples are normalized to [-1.0, 1.0] range:
    /// ```ignore
    /// f32_sample = i16_sample as f32 / 32768.0
    /// ```
    pub fn add_samples(&mut self, samples: &[i16]) -> Option<Vec<f32>> {
        // ---
        // Convert i16 → f32 and add to buffer
        for &sample in samples {
            let normalized = sample as f32 / 32768.0;
            self.buffer.push_back(normalized);
        }

        self.samples_since_last += samples.len();

        // Check if we have enough samples for next window
        if self.buffer.len() >= self.window_size && self.samples_since_last >= self.hop_size {
            // Extract window
            let window: Vec<f32> = self.buffer.iter().take(self.window_size).copied().collect();

            // Advance by hop size (maintain overlap)
            for _ in 0..self.hop_size {
                self.buffer.pop_front();
            }

            self.samples_since_last = 0;

            Some(window)
        } else {
            None
        }
    }

    /// Returns the current number of samples in the buffer.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        // ---
        self.buffer.len()
    }

    /// Returns true if the buffer is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        // ---
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_buffer_creation() {
        // ---
        let buffer = SampleBuffer::new(1024, 0.5);
        assert_eq!(buffer.window_size, 1024);
        assert_eq!(buffer.hop_size, 512);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_buffer_accumulation() {
        // ---
        let mut buffer = SampleBuffer::new(1024, 0.5);

        // Simulate receiving 320-sample frames (typical @ 16kHz)
        let frame = vec![0i16; 320];

        // First three frames should not produce window
        assert!(buffer.add_samples(&frame).is_none());
        assert!(buffer.add_samples(&frame).is_none());
        assert!(buffer.add_samples(&frame).is_none());

        // Fourth frame should complete first window (3*320 + 320 = 1280 > 1024)
        let window = buffer.add_samples(&frame);
        assert!(window.is_some());
        assert_eq!(window.unwrap().len(), 1024);
    }

    #[test]
    fn test_i16_to_f32_normalization() {
        // ---
        let mut buffer = SampleBuffer::new(512, 0.5);

        // Test normalization with known values
        let samples = vec![
            0i16,   // 0.0
            16384,  // 0.5
            32767,  // ~1.0 (max positive)
            -16384, // -0.5
            -32768, // -1.0 (max negative)
        ];

        // Add enough samples to fill window
        let mut all_samples = samples.clone();
        all_samples.extend(vec![0i16; 507]); // Fill to 512

        let window = buffer.add_samples(&all_samples);
        assert!(window.is_some());

        let window = window.unwrap();
        assert_eq!(window.len(), 512);

        // Check normalization of first few samples
        assert!((window[0] - 0.0).abs() < 0.01);
        assert!((window[1] - 0.5).abs() < 0.01);
        assert!((window[2] - 1.0).abs() < 0.01);
        assert!((window[3] - (-0.5)).abs() < 0.01);
        assert!((window[4] - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn test_overlap_behavior() {
        // ---
        let mut buffer = SampleBuffer::new(512, 0.5); // 50% overlap = 256 hop

        // Fill initial window
        let frame1 = vec![100i16; 512];
        let window1 = buffer.add_samples(&frame1);
        assert!(window1.is_some());

        // After hop (256 samples), we should still have 256 samples in buffer
        assert_eq!(buffer.len(), 256);

        // Add 256 more samples to trigger next window
        let frame2 = vec![200i16; 256];
        let window2 = buffer.add_samples(&frame2);
        assert!(window2.is_some());

        // Window should contain both old (256) and new (256) samples
        let window2 = window2.unwrap();
        assert_eq!(window2.len(), 512);
    }

    #[test]
    fn test_no_overlap() {
        // ---
        let mut buffer = SampleBuffer::new(512, 0.0); // No overlap

        // First window
        let frame = vec![0i16; 512];
        assert!(buffer.add_samples(&frame).is_some());
        assert_eq!(buffer.len(), 0); // All samples consumed

        // Second window
        assert!(buffer.add_samples(&frame).is_some());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_high_overlap() {
        // ---
        let mut buffer = SampleBuffer::new(512, 0.75); // 75% overlap = 128 hop

        let frame = vec![0i16; 512];
        let window1 = buffer.add_samples(&frame);
        assert!(window1.is_some());

        // Should have 384 samples remaining (512 - 128 hop)
        assert_eq!(buffer.len(), 384);
    }

    #[test]
    fn test_multiple_windows() {
        // ---
        let mut buffer = SampleBuffer::new(1024, 0.5);
        let mut windows_produced = 0;

        // Simulate continuous stream of 320-sample frames
        for _ in 0..20 {
            let frame = vec![0i16; 320];
            if buffer.add_samples(&frame).is_some() {
                windows_produced += 1;
            }
        }

        // With 20 frames = 6400 samples, hop=512:
        // First window at frame 4 (1280 samples accumulated)
        // Subsequent windows every 2 frames (640 samples / 512 hop)
        // Windows at frames: 4, 6, 8, 10, 12, 14, 16, 18, 20 = 9 windows
        assert_eq!(windows_produced, 9);
    }
}
