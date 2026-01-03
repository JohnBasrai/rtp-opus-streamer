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
    metrics: &rtp_opus_common::MetricsContext,
    ssrc: u32,
    interval_ms: u64,
    loop_audio: bool,
) -> Result<()> {
    // ---
    let mut sequence: u16 = 0;
    let mut timestamp: u32 = 0;
    let mut frame_count = 0;

    // Only stream complete frames. Any tail shorter than a full Opus frame
    // is discarded to avoid partial-packet semantics at EOF.
    let full_frames = audio.samples.chunks_exact(codec::SAMPLES_PER_FRAME);
    let remainder = full_frames.remainder().len();
    if remainder != 0 {
        warn!(
            "Discarding {} trailing samples at EOF (not enough for a full frame)",
            remainder
        );
    }

    loop {
        // ---
        for frame in audio.samples.chunks_exact(codec::SAMPLES_PER_FRAME) {
            // Encode frame (measure cold-ish but still small)
            let start = std::time::Instant::now();
            let payload = encoder
                .encode(frame)
                .with_context(|| format!("failed to encode frame {}", frame_count))?;
            metrics
                .encode_seconds
                .observe(start.elapsed().as_secs_f64());

            // Create and send RTP packet
            let packet = RtpPacket::new(sequence, timestamp, ssrc, payload);
            sender
                .send(&packet)
                .await
                .with_context(|| format!("failed to send packet {}", sequence))?;

            metrics.packets_sent_total.inc();
            metrics.bytes_sent_total.inc_by(packet.payload.len() as u64);

            // Update sequence and timestamp
            sequence = sequence.wrapping_add(1);
            timestamp = timestamp.wrapping_add(codec::SAMPLES_PER_FRAME as u32);
            frame_count += 1;

            // Pace transmission (real-time simulation)
            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
        }

        if !loop_audio {
            break;
        }
    }

    tracing::info!("Streamed {} frames", frame_count);
    Ok(())
}
