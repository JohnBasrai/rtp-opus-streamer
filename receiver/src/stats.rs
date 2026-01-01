//! Statistics tracking for RTP receiver.
//!
//! Tracks packet reception metrics including loss rate, jitter,
//! and reordering events for observability and quality monitoring.

use std::time::{Duration, Instant};
use tracing::info;

/// Network and reception statistics.
///
/// Tracks key metrics for monitoring receiver health and network conditions.
/// Statistics are designed to be logged periodically for observability.
#[derive(Debug, Clone)]
pub struct ReceiverStats {
    // ---
    /// Total packets received successfully
    pub packets_received: u64,

    /// Total packets lost (detected via sequence gaps)
    pub packets_lost: u64,

    /// Total packets that arrived out of order
    pub packets_reordered: u64,

    /// Total packets that arrived too late (after playout deadline)
    pub packets_late: u64,

    /// Last sequence number seen
    last_sequence: Option<u16>,

    /// Start time for rate calculations
    start_time: Instant,

    /// Last time stats were logged
    last_log_time: Instant,

    /// Interval between periodic logs
    log_interval: Duration,
}

impl ReceiverStats {
    // ---
    /// Creates a new stats tracker.
    ///
    /// # Arguments
    ///
    /// * `log_interval` - How often to automatically log stats
    pub fn new(log_interval: Duration) -> Self {
        // ---
        let now = Instant::now();
        Self {
            packets_received: 0,
            packets_lost: 0,
            packets_reordered: 0,
            packets_late: 0,
            last_sequence: None,
            start_time: now,
            last_log_time: now,
            log_interval,
        }
    }

    /// Records a received packet.
    ///
    /// Detects loss based on sequence number gaps and tracks reordering.
    ///
    /// # Arguments
    ///
    /// * `sequence` - Sequence number of received packet
    /// * `was_reordered` - Whether packet arrived out of sequence
    pub fn record_packet(&mut self, sequence: u16, was_reordered: bool) {
        // ---
        self.packets_received += 1;

        if was_reordered {
            self.packets_reordered += 1;
        }

        // Detect packet loss via sequence gaps
        if let Some(last_seq) = self.last_sequence {
            let expected = last_seq.wrapping_add(1);
            if sequence != expected && !was_reordered {
                // Gap detected - calculate lost packets
                let gap = sequence.wrapping_sub(expected);
                self.packets_lost += gap as u64;
            }
        }

        // Update last sequence only if not reordered (to maintain monotonic progression)
        if !was_reordered {
            self.last_sequence = Some(sequence);
        }

        // Periodic logging
        self.maybe_log();
    }

    /// Records a packet that arrived too late to be played.
    pub fn record_late_packet(&mut self) {
        // ---
        self.packets_late += 1;
    }

    /// Calculates current packet loss percentage.
    pub fn loss_percentage(&self) -> f64 {
        // ---
        let total = self.packets_received + self.packets_lost;
        if total == 0 {
            0.0
        } else {
            (self.packets_lost as f64 / total as f64) * 100.0
        }
    }

    /// Calculates reorder percentage.
    pub fn reorder_percentage(&self) -> f64 {
        // ---
        if self.packets_received == 0 {
            0.0
        } else {
            (self.packets_reordered as f64 / self.packets_received as f64) * 100.0
        }
    }

    /// Calculates packets per second reception rate.
    pub fn packets_per_second(&self) -> f64 {
        // ---
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            0.0
        } else {
            self.packets_received as f64 / elapsed
        }
    }

    /// Returns runtime duration.
    pub fn runtime(&self) -> Duration {
        // ---
        self.start_time.elapsed()
    }

    /// Logs statistics if interval has elapsed.
    fn maybe_log(&mut self) {
        // ---
        if self.last_log_time.elapsed() >= self.log_interval {
            self.log();
            self.last_log_time = Instant::now();
        }
    }

    /// Force log current statistics.
    pub fn log(&self) {
        // ---
        info!(
            "RX Stats: {} pkts ({:.2} pkt/s), {:.2}% loss, {:.2}% reordered, {} late",
            self.packets_received,
            self.packets_per_second(),
            self.loss_percentage(),
            self.reorder_percentage(),
            self.packets_late
        );
    }
}

impl Default for ReceiverStats {
    fn default() -> Self {
        // ---
        Self::new(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_stats_no_loss() {
        // ---
        let mut stats = ReceiverStats::default();

        stats.record_packet(0, false);
        stats.record_packet(1, false);
        stats.record_packet(2, false);

        assert_eq!(stats.packets_received, 3);
        assert_eq!(stats.packets_lost, 0);
        assert_eq!(stats.loss_percentage(), 0.0);
    }

    #[test]
    fn test_stats_with_loss() {
        // ---
        let mut stats = ReceiverStats::default();

        stats.record_packet(0, false);
        stats.record_packet(1, false);
        stats.record_packet(5, false); // Gap: lost 2, 3, 4

        assert_eq!(stats.packets_received, 3);
        assert_eq!(stats.packets_lost, 3); // Packets 2, 3, 4
        assert_eq!(stats.loss_percentage(), 50.0); // 3 lost out of 6 total
    }

    #[test]
    fn test_stats_with_reordering() {
        // ---
        let mut stats = ReceiverStats::default();

        stats.record_packet(0, false);
        stats.record_packet(2, false);
        stats.record_packet(1, true); // Out of order

        assert_eq!(stats.packets_received, 3);
        assert_eq!(stats.packets_reordered, 1);

        // Use approximate equality for floating point
        let expected = 100.0 / 3.0;
        let actual = stats.reorder_percentage();
        assert!(
            (actual - expected).abs() < 0.001,
            "Expected ~{}, got {}",
            expected,
            actual
        );
    }

    #[test]
    fn test_sequence_wraparound() {
        // ---
        let mut stats = ReceiverStats::default();

        stats.record_packet(65534, false);
        stats.record_packet(65535, false);
        stats.record_packet(0, false); // Wraparound

        assert_eq!(stats.packets_received, 3);
        assert_eq!(stats.packets_lost, 0);
    }

    #[test]
    fn test_late_packets() {
        // ---
        let mut stats = ReceiverStats::default();

        stats.record_packet(0, false);
        stats.record_late_packet();
        stats.record_late_packet();

        assert_eq!(stats.packets_late, 2);
    }
}
