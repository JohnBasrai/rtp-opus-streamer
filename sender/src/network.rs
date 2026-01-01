//! UDP network transmission for RTP packets.
//!
//! Provides async UDP socket handling for sending RTP packets
//! to the receiver.

use anyhow::{Context, Result};
use rtp_opus_common::RtpPacket;
use tokio::net::UdpSocket;
use tracing::{debug, error, warn};

/// UDP sender for RTP packet transmission.
///
/// Wraps a tokio UDP socket for async transmission of RTP packets.
/// Handles network errors gracefully by logging but continuing operation.
///
/// # Example
///
/// ```ignore
/// use sender::network::RtpSender;
///
/// // Async context required
/// let sender = RtpSender::new("127.0.0.1:5004").await.unwrap();
/// // Use sender.send() to transmit packets
/// ```
pub struct RtpSender {
    // ---
    socket: UdpSocket,
    remote_addr: String,
    packets_sent: u64,
    bytes_sent: u64,
}

impl RtpSender {
    // ---
    /// Creates a new RTP sender bound to any available port.
    ///
    /// The socket will send packets to the specified remote address.
    ///
    /// # Arguments
    ///
    /// * `remote_addr` - Destination address in "IP:port" format
    ///
    /// # Errors
    ///
    /// Returns error if socket binding fails.
    pub async fn new(remote_addr: impl Into<String>) -> Result<Self> {
        // ---
        let remote_addr = remote_addr.into();

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("failed to bind UDP socket")?;

        debug!("UDP socket bound to {}", socket.local_addr()?);

        Ok(Self {
            socket,
            remote_addr,
            packets_sent: 0,
            bytes_sent: 0,
        })
    }

    /// Sends an RTP packet to the remote endpoint.
    ///
    /// Serializes the packet and transmits it via UDP. Network errors
    /// are logged but do not stop operation (resilient behavior).
    ///
    /// # Arguments
    ///
    /// * `packet` - RTP packet to transmit
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Packet serialization fails
    /// - Network transmission fails persistently
    pub async fn send(&mut self, packet: &RtpPacket) -> Result<()> {
        // ---
        let data = packet
            .serialize()
            .context("failed to serialize RTP packet")?;

        match self.socket.send_to(&data, &self.remote_addr).await {
            Ok(bytes) => {
                self.packets_sent += 1;
                self.bytes_sent += bytes as u64;

                if self.packets_sent.is_multiple_of(100) {
                    debug!(
                        "Sent {} packets ({} bytes) - seq={}",
                        self.packets_sent, self.bytes_sent, packet.sequence
                    );
                }
            }
            Err(e) => {
                error!("Failed to send packet seq={}: {}", packet.sequence, e);
                // Don't bail - continue sending to demonstrate resilience
                warn!("Continuing despite network error");
            }
        }

        Ok(())
    }

    /// Returns statistics about packets sent.
    pub fn stats(&self) -> (u64, u64) {
        // ---
        (self.packets_sent, self.bytes_sent)
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[tokio::test]
    async fn test_sender_creation() {
        // ---
        let sender = RtpSender::new("127.0.0.1:5004").await;
        assert!(sender.is_ok());
    }

    #[tokio::test]
    async fn test_sender_send_packet() {
        // ---
        let mut sender = RtpSender::new("127.0.0.1:5004")
            .await
            .expect("sender creation failed");

        let packet = RtpPacket::new(1, 320, 0x12345678, vec![1, 2, 3]);
        let result = sender.send(&packet).await;

        // Should succeed even if no receiver (UDP is fire-and-forget)
        assert!(result.is_ok());

        let (packets, bytes) = sender.stats();
        assert_eq!(packets, 1);
        assert!(bytes > 0);
    }
}
