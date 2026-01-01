//! Jitter buffer for RTP packet reordering and delay compensation.
//!
//! Implements a fixed-depth jitter buffer that compensates for network
//! variance by buffering packets and playing them out in sequence order.

use rtp_opus_common::RtpPacket;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Jitter buffer configuration.
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    // ---
    /// Buffer depth in milliseconds
    pub depth_ms: u32,

    /// Maximum packets to buffer
    pub max_packets: usize,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        // ---
        Self {
            depth_ms: 60,     // 60ms default (3 frames @ 20ms)
            max_packets: 100, // Safety limit
        }
    }
}

/// Buffered packet with arrival timestamp.
#[derive(Debug, Clone)]
struct BufferedPacket {
    packet: RtpPacket,
}

/// Jitter buffer for packet reordering and playout smoothing.
///
/// Buffers incoming packets and releases them in sequence order
/// after a fixed delay to compensate for network jitter.
///
/// # Design
///
/// - **Fixed depth**: Simpler than adaptive, good enough for Phase 2
/// - **Sequence-based ordering**: Packets sorted by sequence number
/// - **Late packet handling**: Discard packets arriving after playout deadline
///
/// # Example
///
/// ```
/// use receiver::jitter_buffer::{JitterBuffer, JitterBufferConfig};
/// use rtp_opus_common::RtpPacket;
///
/// let mut buffer = JitterBuffer::new(JitterBufferConfig::default());
///
/// // Insert packets as they arrive (possibly out of order)
/// let packet1 = RtpPacket::new(0, 0, 0x12345678, vec![1, 2, 3]);
/// let packet2 = RtpPacket::new(1, 320, 0x12345678, vec![4, 5, 6]);
/// buffer.insert(packet1);
/// buffer.insert(packet2);
///
/// // Retrieve packets in sequence order when ready
/// if let Some(packet) = buffer.get_next() {
///     // Play packet
///     assert_eq!(packet.sequence, 0);
/// }
/// ```
pub struct JitterBuffer {
    // ---
    /// Buffer configuration
    config: JitterBufferConfig,

    /// Buffered packets sorted by sequence number
    buffer: VecDeque<BufferedPacket>,

    /// Next expected sequence number for playout
    next_sequence: Option<u16>,

    /// Time when buffer started (for playout timing)
    start_time: Option<Instant>,

    /// Whether buffer has been primed (filled to depth)
    is_primed: bool,
}

impl JitterBuffer {
    // ---
    /// Creates a new jitter buffer with the given configuration.
    pub fn new(config: JitterBufferConfig) -> Self {
        // ---
        Self {
            config,
            buffer: VecDeque::new(),
            next_sequence: None,
            start_time: None,
            is_primed: false,
        }
    }

    /// Inserts a packet into the buffer.
    ///
    /// Packets are stored in sequence order. Late packets (arriving after
    /// their playout deadline) are discarded.
    ///
    /// Returns `true` if packet was inserted, `false` if discarded (late or duplicate).
    pub fn insert(&mut self, packet: RtpPacket) -> bool {
        // ---
        // Initialize on first packet
        if self.next_sequence.is_none() {
            self.next_sequence = Some(packet.sequence);
            self.start_time = Some(Instant::now());
        }

        let packet_sequence = packet.sequence;

        // Check if packet is too late
        if self.is_late(&packet) {
            warn!(
                "Discarding late packet: seq={} (expected={})",
                packet_sequence,
                self.next_sequence.unwrap_or(0)
            );
            return false;
        }

        // Check for duplicates
        if self
            .buffer
            .iter()
            .any(|bp| bp.packet.sequence == packet_sequence)
        {
            debug!("Discarding duplicate packet: seq={}", packet_sequence);
            return false;
        }

        // Insert in sequence order
        let buffered = BufferedPacket { packet };

        let insert_pos = self
            .buffer
            .iter()
            .position(|bp| sequence_compare(packet_sequence, bp.packet.sequence))
            .unwrap_or(self.buffer.len());

        self.buffer.insert(insert_pos, buffered);

        // Enforce max buffer size
        if self.buffer.len() > self.config.max_packets {
            warn!("Buffer overflow, dropping oldest packet");
            self.buffer.pop_front();
        }

        true
    }

    /// Retrieves the next packet ready for playout.
    ///
    /// Returns `None` if:
    /// - Buffer is still priming (waiting for initial fill)
    /// - Next expected packet hasn't arrived yet
    ///
    /// Returns `Some(packet)` when ready to play.
    pub fn get_next(&mut self) -> Option<RtpPacket> {
        // ---
        // Wait for buffer to prime (fill to target depth)
        if !self.is_primed {
            if self.should_start_playout() {
                self.is_primed = true;
                debug!("Jitter buffer primed, starting playout");
            } else {
                return None;
            }
        }

        // Get next packet if available
        let next_seq = self.next_sequence?;

        if let Some(pos) = self
            .buffer
            .iter()
            .position(|bp| bp.packet.sequence == next_seq)
        {
            let buffered = self.buffer.remove(pos).unwrap();
            self.next_sequence = Some(next_seq.wrapping_add(1));
            return Some(buffered.packet);
        }

        None
    }

    /// Checks if we should start playout (buffer priming complete).
    fn should_start_playout(&self) -> bool {
        // ---
        if self.buffer.is_empty() {
            return false;
        }

        let start = match self.start_time {
            Some(s) => s,
            None => return false,
        };

        let elapsed = start.elapsed();
        let target_depth = Duration::from_millis(self.config.depth_ms as u64);

        // Start playout after target depth or if buffer has enough packets
        elapsed >= target_depth || self.buffer.len() >= 3
    }

    /// Checks if a packet is too late for playout.
    fn is_late(&self, packet: &RtpPacket) -> bool {
        // ---
        let next_seq = match self.next_sequence {
            Some(seq) => seq,
            None => return false, // First packet can't be late
        };

        // Packet is late if its sequence is behind next expected
        // (accounting for wraparound)
        let distance = packet.sequence.wrapping_sub(next_seq);
        distance > 32768 // More than half the sequence space behind
    }

    /// Returns current buffer status for debugging.
    pub fn status(&self) -> JitterBufferStatus {
        // ---
        JitterBufferStatus {
            buffered_packets: self.buffer.len(),
            is_primed: self.is_primed,
            next_sequence: self.next_sequence,
        }
    }

    /// Returns whether the given sequence was reordered.
    ///
    /// A packet is reordered if it arrived out of sequence but was still buffered.
    pub fn was_reordered(&self, sequence: u16) -> bool {
        // ---
        if let Some(next_seq) = self.next_sequence {
            sequence != next_seq
        } else {
            false
        }
    }
}

/// Jitter buffer status for observability.
#[derive(Debug, Clone)]
pub struct JitterBufferStatus {
    pub buffered_packets: usize,
    pub is_primed: bool,
    pub next_sequence: Option<u16>,
}

/// Compares two sequence numbers accounting for wraparound.
///
/// Returns `true` if `a` comes before `b` in sequence space.
fn sequence_compare(a: u16, b: u16) -> bool {
    // ---
    let diff = a.wrapping_sub(b);
    diff < 32768
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    fn make_packet(seq: u16) -> RtpPacket {
        RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3])
    }

    #[test]
    fn test_jitter_buffer_in_order() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 0, // No delay for testing
            max_packets: 10,
        });

        buffer.insert(make_packet(0));
        buffer.insert(make_packet(1));
        buffer.insert(make_packet(2));

        assert_eq!(buffer.get_next().unwrap().sequence, 0);
        assert_eq!(buffer.get_next().unwrap().sequence, 1);
        assert_eq!(buffer.get_next().unwrap().sequence, 2);
    }

    #[test]
    fn test_jitter_buffer_reordering() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 0,
            max_packets: 10,
        });

        // Insert out of order
        buffer.insert(make_packet(0));
        buffer.insert(make_packet(2));
        buffer.insert(make_packet(1)); // Out of sequence

        // Should play in order
        assert_eq!(buffer.get_next().unwrap().sequence, 0);
        assert_eq!(buffer.get_next().unwrap().sequence, 1);
        assert_eq!(buffer.get_next().unwrap().sequence, 2);
    }

    #[test]
    fn test_jitter_buffer_late_packet() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 0,
            max_packets: 10,
        });

        buffer.insert(make_packet(0));
        buffer.insert(make_packet(1));
        buffer.get_next(); // Play packet 0, next expected is 1
        buffer.get_next(); // Play packet 1, next expected is 2

        // Packet 0 arrives again - should be discarded as late
        let inserted = buffer.insert(make_packet(0));
        assert!(!inserted);
    }

    #[test]
    fn test_sequence_wraparound() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 0,
            max_packets: 10,
        });

        buffer.insert(make_packet(65534));
        buffer.insert(make_packet(65535));
        buffer.insert(make_packet(0)); // Wraparound

        assert_eq!(buffer.get_next().unwrap().sequence, 65534);
        assert_eq!(buffer.get_next().unwrap().sequence, 65535);
        assert_eq!(buffer.get_next().unwrap().sequence, 0);
    }

    #[test]
    fn test_duplicate_packets() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 0,
            max_packets: 10,
        });

        buffer.insert(make_packet(0));
        let inserted = buffer.insert(make_packet(0)); // Duplicate

        assert!(!inserted);
        assert_eq!(buffer.buffer.len(), 1);
    }

    #[test]
    fn test_buffer_priming() {
        // ---
        let mut buffer = JitterBuffer::new(JitterBufferConfig {
            depth_ms: 100, // 100ms depth
            max_packets: 10,
        });

        buffer.insert(make_packet(0));

        // Buffer not primed yet, should not release packet
        assert!(buffer.get_next().is_none());
        assert!(!buffer.is_primed);

        // After enough time or packets, should prime
        std::thread::sleep(Duration::from_millis(110));
        assert!(buffer.get_next().is_some());
    }
}
