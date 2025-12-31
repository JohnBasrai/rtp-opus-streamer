//! RTP packet structure and serialization.
//!
//! Implements basic RTP packet format according to RFC 3550.
//! This implementation focuses on the minimum required fields for
//! audio streaming without optional extensions.

use anyhow::Result;

/// RTP packet version 2 (as per RFC 3550)
const RTP_VERSION: u8 = 2;

/// Payload type for dynamic Opus codec
const PAYLOAD_TYPE_OPUS: u8 = 96;

/// RTP packet structure for audio transmission.
///
/// Implements RFC 3550 RTP packet format with fixed header fields.
/// The packet contains timing information (sequence number and timestamp)
/// along with the encoded audio payload.
///
/// # Protocol Details
///
/// - Version: Always 2 (RFC 3550)
/// - Payload Type: 96 (dynamic assignment for Opus)
/// - Sequence: Increments by 1 for each packet
/// - Timestamp: Increments by 320 samples for 20ms @ 16kHz
/// - SSRC: Synchronization source identifier (random per session)
#[derive(Debug, Clone)]
pub struct RtpPacket {
    // ---
    /// Packet sequence number (wraps at 65535)
    pub sequence: u16,

    /// RTP timestamp in sample units
    pub timestamp: u32,

    /// Synchronization source identifier
    pub ssrc: u32,

    /// Encoded audio payload
    pub payload: Vec<u8>,
}

impl RtpPacket {
    // ---
    /// Creates a new RTP packet with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `sequence` - Packet sequence number
    /// * `timestamp` - RTP timestamp (in sample units)
    /// * `ssrc` - Synchronization source identifier
    /// * `payload` - Encoded audio data
    pub fn new(sequence: u16, timestamp: u32, ssrc: u32, payload: Vec<u8>) -> Self {
        // ---
        Self {
            sequence,
            timestamp,
            ssrc,
            payload,
        }
    }

    /// Serializes the RTP packet into wire format.
    ///
    /// Returns a byte vector ready for UDP transmission. The format follows
    /// RFC 3550 fixed header (12 bytes) followed by the payload.
    ///
    /// # Wire Format
    ///
    /// ```text
    ///  0                   1                   2                   3
    ///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |V=2|P|X|  CC   |M|     PT      |       sequence number         |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                           timestamp                           |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |           synchronization source (SSRC) identifier            |
    /// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
    /// |                           payload...                          |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails (currently infallible but returns
    /// Result for future extensibility).
    pub fn serialize(&self) -> Result<Vec<u8>> {
        // ---
        let mut buf = Vec::with_capacity(12 + self.payload.len());

        // Byte 0: V(2) | P(1) | X(1) | CC(4)
        // V=2, P=0 (no padding), X=0 (no extension), CC=0 (no CSRC)
        buf.push(RTP_VERSION << 6);

        // Byte 1: M(1) | PT(7)
        // M=0 (not marker), PT=96 (dynamic Opus)
        buf.push(PAYLOAD_TYPE_OPUS);

        // Bytes 2-3: Sequence number (big-endian)
        buf.extend_from_slice(&self.sequence.to_be_bytes());

        // Bytes 4-7: Timestamp (big-endian)
        buf.extend_from_slice(&self.timestamp.to_be_bytes());

        // Bytes 8-11: SSRC (big-endian)
        buf.extend_from_slice(&self.ssrc.to_be_bytes());

        // Payload
        buf.extend_from_slice(&self.payload);

        Ok(buf)
    }

    /// Deserializes an RTP packet from wire format.
    ///
    /// Parses the fixed 12-byte header and extracts the payload.
    /// Validates version field but does not validate payload type
    /// to allow for future codec flexibility.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes received from network
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Packet is smaller than minimum header size (12 bytes)
    /// - RTP version is not 2
    #[allow(dead_code)] // Only used by receiver; duplicated code in Phase 1
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // ---
        if data.len() < 12 {
            anyhow::bail!("packet too small: {} bytes", data.len());
        }

        // Validate version
        let version = (data[0] >> 6) & 0x03;
        if version != RTP_VERSION {
            anyhow::bail!("invalid RTP version: {}", version);
        }

        // Extract fields (big-endian)
        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        // Payload is everything after header
        let payload = data[12..].to_vec();

        Ok(Self {
            sequence,
            timestamp,
            ssrc,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_rtp_packet_serialization() {
        // ---
        let packet = RtpPacket::new(100, 32000, 0x12345678, vec![1, 2, 3, 4]);
        let serialized = packet.serialize().expect("serialization failed");

        // Check header fields
        assert_eq!(serialized[0] >> 6, 2); // Version
        assert_eq!(serialized[1] & 0x7F, 96); // Payload type
        assert_eq!(u16::from_be_bytes([serialized[2], serialized[3]]), 100); // Sequence

        // Check payload
        assert_eq!(&serialized[12..], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_rtp_packet_deserialization() {
        // ---
        let packet = RtpPacket::new(200, 64000, 0xAABBCCDD, vec![5, 6, 7, 8]);
        let serialized = packet.serialize().expect("serialization failed");

        let deserialized = RtpPacket::deserialize(&serialized).expect("deserialization failed");

        assert_eq!(deserialized.sequence, 200);
        assert_eq!(deserialized.timestamp, 64000);
        assert_eq!(deserialized.ssrc, 0xAABBCCDD);
        assert_eq!(deserialized.payload, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_rtp_packet_too_small() {
        // ---
        let data = vec![0, 1, 2]; // Only 3 bytes
        let result = RtpPacket::deserialize(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_rtp_invalid_version() {
        // ---
        let mut data = vec![0; 12];
        data[0] = 1 << 6; // Version 1 instead of 2

        let result = RtpPacket::deserialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_sequence_wraparound() {
        // ---
        let packet = RtpPacket::new(65535, 0, 0, vec![]);
        let serialized = packet.serialize().expect("serialization failed");
        let deserialized = RtpPacket::deserialize(&serialized).expect("deserialization failed");

        assert_eq!(deserialized.sequence, 65535);
    }
}
