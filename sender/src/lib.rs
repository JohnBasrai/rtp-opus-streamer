//! RTP Opus Sender Library
//!
//! Provides audio streaming functionality over RTP with Opus encoding.
//! This library can be used to build custom senders or for integration testing.

pub mod audio;
pub mod codec;
pub mod network;

pub use audio::{read_wav, AudioData};
pub use codec::OpusEncoderWrapper;
pub use network::RtpSender;
pub use rtp_opus_common::RtpPacket;

use anyhow::{Context, Result};
use tracing::warn;

/// Streams audio frames over RTP.
///
/// Encodes each frame with Opus and transmits as RTP packets with
/// proper timing and sequencing.
///
/// # Arguments
///
/// * `audio` - Audio data to stream
/// * `encoder` - Opus encoder instance
/// * `sender` - RTP network sender
/// * `ssrc` - Synchronization source identifier for this session
/// * `interval_ms` - Milliseconds between packet transmissions
///
/// # Errors
///
/// Returns error if encoding or network transmission fails.
pub async fn stream_audio(
    audio: &AudioData,
    encoder: &mut OpusEncoderWrapper,
    sender: &mut RtpSender,
    ssrc: u32,
    interval_ms: u64,
) -> Result<()> {
    // ---
    let mut sequence: u16 = 0;
    let mut timestamp: u32 = 0;
    let mut frame_count = 0;

    for frame in audio.frames() {
        // Pad last frame if needed
        let mut frame_data = frame.to_vec();
        if frame_data.len() < codec::SAMPLES_PER_FRAME {
            warn!(
                "Padding last frame: {} samples -> {}",
                frame_data.len(),
                codec::SAMPLES_PER_FRAME
            );
            frame_data.resize(codec::SAMPLES_PER_FRAME, 0);
        }

        // Encode frame
        let payload = encoder
            .encode(&frame_data)
            .with_context(|| format!("failed to encode frame {}", frame_count))?;

        // Create and send RTP packet
        let packet = RtpPacket::new(sequence, timestamp, ssrc, payload);
        sender
            .send(&packet)
            .await
            .with_context(|| format!("failed to send packet {}", sequence))?;

        // Update sequence and timestamp
        sequence = sequence.wrapping_add(1);
        timestamp = timestamp.wrapping_add(codec::SAMPLES_PER_FRAME as u32);
        frame_count += 1;

        // Pace transmission (real-time simulation)
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
    }

    tracing::info!("Streamed {} frames", frame_count);
    Ok(())
}
