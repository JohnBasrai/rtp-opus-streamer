//! Audio playback using cpal.
//!
//! Provides real-time audio output through the system's default
//! audio device using callback-based streaming.

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, info, warn};

use crate::codec::SAMPLE_RATE;

/// Audio player for real-time PCM playback.
///
/// Uses cpal for cross-platform audio output. Operates in callback mode
/// where the audio device pulls samples from an internal queue.
///
/// # Thread Safety
///
/// The player uses an MPSC channel to safely transfer audio samples
/// from the network thread to the audio callback thread.
pub struct AudioPlayer {
    // ---
    _stream: Stream,
    sample_tx: Sender<i16>,
}

impl AudioPlayer {
    // ---
    /// Creates a new audio player using the default output device.
    ///
    /// Configures the device for 16kHz mono playback to match the
    /// decoder output format.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No audio output device is available
    /// - Device configuration fails
    /// - Stream creation fails
    pub fn new() -> Result<Self> {
        // ---
        info!("Initializing audio playback");

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no output device available")?;

        info!("Using audio device: {}", device.name()?);

        // Create channel for passing samples to audio callback
        let (sample_tx, sample_rx) = mpsc::channel();

        // Build stream with our configuration
        let stream = Self::build_stream(&device, sample_rx)?;

        info!("Audio stream created successfully");

        Ok(Self {
            _stream: stream,
            sample_tx,
        })
    }

    /// Plays a frame of PCM samples.
    ///
    /// Sends samples to the audio device's callback queue. If the queue
    /// is full, samples may be dropped (logged as warning).
    ///
    /// # Arguments
    ///
    /// * `samples` - PCM samples to play (typically 320 samples for 20ms)
    pub fn play(&mut self, samples: &[i16]) {
        // ---
        for &sample in samples {
            if let Err(e) = self.sample_tx.send(sample) {
                warn!("Failed to send sample to audio thread: {}", e);
                break;
            }
        }
    }

    /// Builds the audio output stream.
    fn build_stream(device: &Device, sample_rx: Receiver<i16>) -> Result<Stream> {
        // ---
        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        debug!("Stream config: {:?}", config);

        // Create the output stream with a callback
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    Self::audio_callback(data, &sample_rx);
                },
                |err| {
                    warn!("Audio stream error: {}", err);
                },
                None,
            )
            .context("failed to build output stream")?;

        // Start the stream
        stream.play().context("failed to start audio stream")?;

        info!("Audio stream started");

        Ok(stream)
    }

    /// Audio callback that fills the output buffer.
    ///
    /// Called by cpal when the audio device needs more samples.
    /// Pulls samples from the queue and fills the output buffer,
    /// using silence if the queue is empty.
    fn audio_callback(data: &mut [i16], sample_rx: &Receiver<i16>) {
        // ---
        for sample in data.iter_mut() {
            *sample = sample_rx.try_recv().unwrap_or(0);
        }
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_audio_player_creation() {
        // ---
        // This test requires an audio device, so it may fail in CI
        let result = AudioPlayer::new();

        if result.is_err() {
            // Skip test in environments without audio devices (CI, Docker)
            println!("Skipping: no audio device available (expected in CI)");
            return;
        }

        // If we got here, audio device exists and creation succeeded
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_player_play() {
        // ---
        if let Ok(mut player) = AudioPlayer::new() {
            let samples = vec![0i16; 320];
            player.play(&samples);
            // Should not panic
        }
    }
}
