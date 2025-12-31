//! UDP network reception for RTP packets.
//!
//! Provides async UDP socket handling for receiving RTP packets
//! from the sender.

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

use crate::rtp::RtpPacket;

/// UDP receiver for RTP packet reception.
///
/// Wraps a tokio UDP socket for async reception of RTP packets.
/// Handles packet validation and provides statistics.
pub struct RtpReceiver {
    // ---
    socket: UdpSocket,
    packets_received: u64,
    bytes_received: u64,
    packets_dropped: u64,
}

impl RtpReceiver {
    // ---
    /// Creates a new RTP receiver bound to the specified port.
    ///
    /// Listens on all interfaces (0.0.0.0) for incoming packets.
    ///
    /// # Arguments
    ///
    /// * `port` - UDP port to listen on
    ///
    /// # Errors
    ///
    /// Returns error if socket binding fails.
    pub async fn new(port: u16) -> Result<Self> {
        // ---
        let addr = format!("0.0.0.0:{}", port);

        let socket = UdpSocket::bind(&addr)
            .await
            .with_context(|| format!("failed to bind UDP socket to {}", addr))?;

        info!("UDP socket bound to {}", socket.local_addr()?);

        Ok(Self {
            socket,
            packets_received: 0,
            bytes_received: 0,
            packets_dropped: 0,
        })
    }

    /// Receives the next RTP packet.
    ///
    /// Blocks until a packet arrives, then deserializes and validates it.
    /// Invalid packets are logged and counted as dropped.
    ///
    /// # Returns
    ///
    /// The next valid RTP packet, or None if packet was invalid.
    ///
    /// # Errors
    ///
    /// Returns error if network reception fails.
    pub async fn receive(&mut self) -> Result<Option<RtpPacket>> {
        // ---
        let mut buf = vec![0u8; 2048]; // Max UDP packet size for RTP

        let (len, src) = self
            .socket
            .recv_from(&mut buf)
            .await
            .context("failed to receive UDP packet")?;

        self.bytes_received += len as u64;

        // Parse RTP packet
        match RtpPacket::deserialize(&buf[..len]) {
            Ok(packet) => {
                self.packets_received += 1;

                if self.packets_received % 100 == 0 {
                    debug!(
                        "Received {} packets ({} bytes, {} dropped) from {} - seq={}",
                        self.packets_received,
                        self.bytes_received,
                        self.packets_dropped,
                        src,
                        packet.sequence
                    );
                }

                Ok(Some(packet))
            }
            Err(e) => {
                self.packets_dropped += 1;
                warn!("Dropped invalid packet from {}: {}", src, e);
                Ok(None)
            }
        }
    }

    /// Returns statistics about packets received.
    ///
    /// # Returns
    ///
    /// Tuple of (packets_received, bytes_received, packets_dropped)
    #[allow(dead_code)] // Will be used in Phase 3 for metrics
    pub fn stats(&self) -> (u64, u64, u64) {
        // ---
        (
            self.packets_received,
            self.bytes_received,
            self.packets_dropped,
        )
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[tokio::test]
    async fn test_receiver_creation() {
        // ---
        // Try binding to an ephemeral port
        let receiver = RtpReceiver::new(0).await;
        assert!(receiver.is_ok());
    }

    #[tokio::test]
    async fn test_receiver_stats() {
        // ---
        let receiver = RtpReceiver::new(0).await.expect("receiver creation failed");

        let (packets, bytes, dropped) = receiver.stats();
        assert_eq!(packets, 0);
        assert_eq!(bytes, 0);
        assert_eq!(dropped, 0);
    }
}
