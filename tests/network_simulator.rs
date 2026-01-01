//! Network simulator for testing resilience.
//!
//! Provides in-process network condition simulation including packet loss,
//! jitter, and reordering for integration testing.

use rand::Rng;
use rtp_opus_common::RtpPacket;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Network simulator configuration.
#[derive(Debug, Clone)]
pub struct NetworkSimulatorConfig {
    // ---
    /// Packet loss rate (0.0 to 1.0)
    pub loss_rate: f64,

    /// Jitter amount in milliseconds (±random delay)
    pub jitter_ms: u32,

    /// Packet reordering rate (0.0 to 1.0)
    pub reorder_rate: f64,

    /// Random seed for deterministic testing
    pub seed: Option<u64>,
}

impl Default for NetworkSimulatorConfig {
    fn default() -> Self {
        // ---
        Self {
            loss_rate: 0.0,
            jitter_ms: 0,
            reorder_rate: 0.0,
            seed: None,
        }
    }
}

/// Packet with delayed delivery.
#[derive(Debug, Clone)]
struct DelayedPacket {
    packet: RtpPacket,
    delivery_time: Instant,
}

/// Simulates network conditions for testing.
///
/// Applies configurable packet loss, jitter, and reordering to packets
/// passing through it. Useful for testing receiver resilience.
///
/// # Example
///
/// ```no_run
/// use tests::network_simulator::{NetworkSimulator, NetworkSimulatorConfig};
///
/// let config = NetworkSimulatorConfig {
///     loss_rate: 0.1,    // 10% loss
///     jitter_ms: 20,      // ±20ms jitter
///     reorder_rate: 0.05, // 5% reordering
///     seed: Some(42),     // Deterministic
/// };
///
/// let mut sim = NetworkSimulator::new(config);
///
/// // Process packet
/// sim.send(packet);
///
/// // Retrieve ready packets
/// while let Some(p) = sim.receive() {
///     // Handle packet
/// }
/// ```
pub struct NetworkSimulator {
    // ---
    config: NetworkSimulatorConfig,
    rng: rand::rngs::StdRng,
    delayed_queue: VecDeque<DelayedPacket>,
    packets_sent: u64,
    packets_lost: u64,
    packets_delayed: u64,
    packets_reordered: u64,
}

impl NetworkSimulator {
    // ---
    /// Creates a new network simulator with the given configuration.
    pub fn new(config: NetworkSimulatorConfig) -> Self {
        // ---
        use rand::SeedableRng;

        let rng = if let Some(seed) = config.seed {
            rand::rngs::StdRng::seed_from_u64(seed)
        } else {
            rand::rngs::StdRng::from_entropy()
        };

        Self {
            config,
            rng,
            delayed_queue: VecDeque::new(),
            packets_sent: 0,
            packets_lost: 0,
            packets_delayed: 0,
            packets_reordered: 0,
        }
    }

    /// Sends a packet through the simulator.
    ///
    /// Applies loss, jitter, and reordering based on configuration.
    /// Packet may be delayed or dropped.
    pub fn send(&mut self, packet: RtpPacket) {
        // ---
        self.packets_sent += 1;

        // Packet loss
        if self.should_drop() {
            self.packets_lost += 1;
            return;
        }

        // Calculate delivery time with jitter
        let delay = self.calculate_delay();
        let delivery_time = Instant::now() + delay;

        // Reordering: sometimes hold packet back
        if self.should_reorder() && !self.delayed_queue.is_empty() {
            // Insert earlier in queue to reorder
            self.packets_reordered += 1;
            let reorder_pos = self.rng.gen_range(0..self.delayed_queue.len());
            let delayed = DelayedPacket {
                packet,
                delivery_time,
            };
            self.delayed_queue.insert(reorder_pos, delayed);
        } else {
            // Normal insertion at end
            let delayed = DelayedPacket {
                packet,
                delivery_time,
            };
            self.delayed_queue.push_back(delayed);
        }

        if delay > Duration::from_millis(0) {
            self.packets_delayed += 1;
        }
    }

    /// Retrieves the next packet ready for delivery.
    ///
    /// Returns `None` if no packets are ready yet.
    pub fn receive(&mut self) -> Option<RtpPacket> {
        // ---
        let now = Instant::now();

        // Check if front packet is ready
        if let Some(delayed) = self.delayed_queue.front() {
            if delayed.delivery_time <= now {
                return Some(self.delayed_queue.pop_front().unwrap().packet);
            }
        }

        None
    }

    /// Returns number of packets currently in flight.
    pub fn in_flight(&self) -> usize {
        // ---
        self.delayed_queue.len()
    }

    /// Returns simulator statistics.
    pub fn stats(&self) -> NetworkSimulatorStats {
        // ---
        NetworkSimulatorStats {
            packets_sent: self.packets_sent,
            packets_lost: self.packets_lost,
            packets_delayed: self.packets_delayed,
            packets_reordered: self.packets_reordered,
            loss_rate: if self.packets_sent > 0 {
                self.packets_lost as f64 / self.packets_sent as f64
            } else {
                0.0
            },
        }
    }

    /// Determines if packet should be dropped.
    fn should_drop(&mut self) -> bool {
        // ---
        self.rng.gen_bool(self.config.loss_rate)
    }

    /// Determines if packet should be reordered.
    fn should_reorder(&mut self) -> bool {
        // ---
        self.rng.gen_bool(self.config.reorder_rate)
    }

    /// Calculates random delay for jitter.
    fn calculate_delay(&mut self) -> Duration {
        // ---
        if self.config.jitter_ms == 0 {
            return Duration::from_millis(0);
        }

        // Random delay: ±jitter_ms
        let jitter = self.rng.gen_range(0..=(2 * self.config.jitter_ms));
        Duration::from_millis(jitter as u64)
    }
}

/// Network simulator statistics.
#[derive(Debug, Clone)]
pub struct NetworkSimulatorStats {
    pub packets_sent: u64,
    pub packets_lost: u64,
    pub packets_delayed: u64,
    pub packets_reordered: u64,
    pub loss_rate: f64,
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    fn make_packet(seq: u16) -> RtpPacket {
        RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3])
    }

    #[test]
    fn test_no_loss_no_delay() {
        // ---
        let config = NetworkSimulatorConfig::default();
        let mut sim = NetworkSimulator::new(config);

        sim.send(make_packet(0));
        sim.send(make_packet(1));

        assert_eq!(sim.receive().unwrap().sequence, 0);
        assert_eq!(sim.receive().unwrap().sequence, 1);

        let stats = sim.stats();
        assert_eq!(stats.packets_lost, 0);
        assert_eq!(stats.loss_rate, 0.0);
    }

    #[test]
    fn test_packet_loss() {
        // ---
        let config = NetworkSimulatorConfig {
            loss_rate: 1.0, // 100% loss
            seed: Some(42),
            ..Default::default()
        };
        let mut sim = NetworkSimulator::new(config);

        for i in 0..10 {
            sim.send(make_packet(i));
        }

        assert!(sim.receive().is_none());

        let stats = sim.stats();
        assert_eq!(stats.packets_lost, 10);
        assert_eq!(stats.loss_rate, 1.0);
    }

    #[test]
    fn test_jitter() {
        // ---
        let config = NetworkSimulatorConfig {
            jitter_ms: 50,
            seed: Some(42),
            ..Default::default()
        };
        let mut sim = NetworkSimulator::new(config);

        sim.send(make_packet(0));

        // Packet might not be immediately available
        let immediate = sim.receive();

        // But should arrive eventually
        std::thread::sleep(Duration::from_millis(150));
        let delayed = sim.receive();

        assert!(immediate.is_none() || delayed.is_some());
    }

    #[test]
    fn test_deterministic_with_seed() {
        // ---
        let config = NetworkSimulatorConfig {
            loss_rate: 0.5,
            seed: Some(42), // Fixed seed
            ..Default::default()
        };

        let mut sim1 = NetworkSimulator::new(config.clone());
        let mut sim2 = NetworkSimulator::new(config);

        for i in 0..100 {
            sim1.send(make_packet(i));
            sim2.send(make_packet(i));
        }

        let stats1 = sim1.stats();
        let stats2 = sim2.stats();

        // Same seed should give same results
        assert_eq!(stats1.packets_lost, stats2.packets_lost);
    }
}
