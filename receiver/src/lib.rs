//! RTP Opus Receiver Library
//!
//! Provides audio reception and decoding functionality from RTP streams.
//! This library can be used to build custom receivers or for integration testing.

pub mod audio;
pub mod codec;
pub mod jitter_buffer;
pub mod network;
pub mod stats;

pub use audio::AudioPlayer;
pub use codec::OpusDecoderWrapper;
pub use jitter_buffer::{JitterBuffer, JitterBufferConfig};
pub use network::RtpReceiver;
pub use rtp_opus_common::RtpPacket;
pub use stats::ReceiverStats;

use anyhow::Result;
use std::time::Duration;
use tracing::warn;

/// Runs the receiver loop with jitter buffer and stats tracking.
///
/// This is the main reception function that integrates all receiver components:
/// network reception, jitter buffering, packet loss concealment, decoding, and playback.
///
/// # Arguments
///
/// * `receiver` - Network receiver for incoming RTP packets
/// * `decoder` - Opus decoder instance
/// * `player` - Audio playback device
/// * `jitter_config` - Jitter buffer configuration
///
/// # Errors
///
/// Returns error if network or audio system fails critically.
pub async fn receive_loop(
    receiver: &mut RtpReceiver,
    decoder: &mut OpusDecoderWrapper,
    player: &mut AudioPlayer,
    jitter_config: JitterBufferConfig,
) -> Result<()> {
    // ---
    let mut jitter_buffer = JitterBuffer::new(jitter_config);
    let mut stats = ReceiverStats::new(Duration::from_secs(5));

    loop {
        // Receive packet from network
        match receiver.receive().await? {
            Some(packet) => {
                let sequence = packet.sequence;
                let was_reordered = jitter_buffer.was_reordered(sequence);

                // Insert into jitter buffer
                if !jitter_buffer.insert(packet) {
                    // Packet was late or duplicate
                    stats.record_late_packet();
                    continue;
                }

                // Record in stats
                stats.record_packet(sequence, was_reordered);
            }
            None => {
                // Invalid packet, already logged by receiver
                continue;
            }
        }

        // Try to get packets ready for playout
        while let Some(packet) = jitter_buffer.get_next() {
            match decoder.decode(&packet.payload) {
                Ok(samples) => {
                    player.play(&samples);
                }
                Err(e) => {
                    warn!("Failed to decode packet seq={}: {}", packet.sequence, e);
                    // Use PLC for decode errors
                    if let Ok(concealed) = decoder.conceal_loss() {
                        player.play(&concealed);
                    }
                }
            }
        }
    }
}
