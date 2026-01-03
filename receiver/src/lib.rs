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
    metrics: &rtp_opus_common::MetricsContext,
) -> Result<()> {
    // ---
    let mut jitter_buffer = JitterBuffer::new(jitter_config);
    let mut stats = ReceiverStats::new(Duration::from_secs(5));

    // Used for estimating network transit time using RTP timestamp deltas.
    let mut first_ts: Option<u32> = None;
    let mut first_arrival: Option<std::time::Instant> = None;

    loop {
        // Receive packet from network
        match receiver.receive().await? {
            Some(packet) => {
                let arrival = std::time::Instant::now();
                let sequence = packet.sequence;
                let was_reordered = jitter_buffer.was_reordered(sequence);

                metrics.packets_received_total.inc();
                metrics
                    .bytes_received_total
                    .inc_by(packet.payload.len() as u64);

                // Baseline for RTP timestamp -> media time.
                if first_ts.is_none() {
                    first_ts = Some(packet.timestamp);
                    first_arrival = Some(arrival);
                }

                // Estimate network transit variation (no wall-clock sync required).
                if let (Some(t0), Some(a0)) = (first_ts, first_arrival) {
                    let dt_samples = packet.timestamp.wrapping_sub(t0) as u64;
                    let media_secs = dt_samples as f64 / codec::SAMPLE_RATE as f64;
                    let expected_arrival = a0 + std::time::Duration::from_secs_f64(media_secs);
                    if arrival >= expected_arrival {
                        metrics
                            .network_transit_seconds
                            .observe(arrival.duration_since(expected_arrival).as_secs_f64());
                    }
                }

                // Insert into jitter buffer
                if !jitter_buffer.insert_with_arrival(packet, arrival) {
                    // Packet was late or duplicate
                    stats.record_late_packet();
                    metrics.packets_late_total.inc();
                    continue;
                }

                metrics
                    .jitter_buffer_occupancy_packets
                    .set(jitter_buffer.status().buffered_packets as i64);

                // Record in stats
                let lost_gap = stats.record_packet_and_get_loss(sequence, was_reordered);
                if lost_gap > 0 {
                    metrics.packets_lost_total.inc_by(lost_gap);
                }
                if was_reordered {
                    metrics.packets_reordered_total.inc();
                }
            }
            None => {
                // Invalid packet, already logged by receiver
                continue;
            }
        }

        // Try to get packets ready for playout
        while let Some((packet, buffer_delay)) = jitter_buffer.get_next_with_delay() {
            metrics
                .jitter_buffer_delay_seconds
                .observe(buffer_delay.as_secs_f64());
            metrics
                .jitter_buffer_occupancy_packets
                .set(jitter_buffer.status().buffered_packets as i64);

            let pipeline_start = std::time::Instant::now();
            let decode_start = std::time::Instant::now();

            match decoder.decode(&packet.payload) {
                Ok(samples) => {
                    metrics
                        .decode_seconds
                        .observe(decode_start.elapsed().as_secs_f64());
                    player.play(&samples);
                    metrics
                        .receiver_pipeline_seconds
                        .observe(pipeline_start.elapsed().as_secs_f64());
                }
                Err(e) => {
                    warn!("Failed to decode packet seq={}: {}", packet.sequence, e);
                    // Use PLC for decode errors
                    if let Ok(concealed) = decoder.conceal_loss() {
                        metrics
                            .decode_seconds
                            .observe(decode_start.elapsed().as_secs_f64());
                        player.play(&concealed);
                        metrics
                            .receiver_pipeline_seconds
                            .observe(pipeline_start.elapsed().as_secs_f64());
                    }
                }
            }
        }
    }
}
