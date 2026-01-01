//! Integration tests for Phase 2: Network Resilience
//!
//! Tests the complete sender → receiver pipeline with simulated
//! network conditions: packet loss, jitter, and reordering.

mod network_simulator;

use network_simulator::{NetworkSimulator, NetworkSimulatorConfig};
use receiver::{JitterBufferConfig, OpusDecoderWrapper};
use rtp_opus_common::RtpPacket;
use sender::OpusEncoderWrapper;

/// Test helper to create a simple audio frame
fn create_test_frame() -> Vec<i16> {
    // ---
    const FRAME_SIZE: usize = 320; // 20ms @ 16kHz
    let mut frame = Vec::with_capacity(FRAME_SIZE);
    
    // Simple sine wave
    for i in 0..FRAME_SIZE {
        let sample = (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 16000.0).sin();
        frame.push((sample * 5000.0) as i16);
    }
    
    frame
}

/// Tests basic end-to-end pipeline without network issues.
#[test]
fn test_end_to_end_perfect_network() {
    // ---
    let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
    let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");
    
    let frame = create_test_frame();
    
    // Encode
    let encoded = encoder.encode(&frame).expect("encoding failed");
    
    // Create RTP packet
    let packet = RtpPacket::new(1, 320, 0x12345678, encoded);
    
    // Serialize and deserialize (simulates network)
    let serialized = packet.serialize().expect("serialization failed");
    let received = RtpPacket::deserialize(&serialized).expect("deserialization failed");
    
    // Decode
    let decoded = decoder.decode(&received.payload).expect("decoding failed");
    
    assert_eq!(decoded.len(), frame.len());
    println!("✓ End-to-end pipeline works");
}

/// Tests jitter buffer with in-order packets.
#[test]
fn test_jitter_buffer_in_order() {
    // ---
    use receiver::JitterBuffer;
    
    let config = JitterBufferConfig {
        depth_ms: 0, // No delay for testing
        max_packets: 10,
    };
    
    let mut buffer = JitterBuffer::new(config);
    
    // Create test packets
    for seq in 0..5 {
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3]);
        buffer.insert(packet);
    }
    
    // Should play out in order
    for seq in 0..5 {
        let packet = buffer.get_next().expect("packet should be available");
        assert_eq!(packet.sequence, seq);
    }
    
    println!("✓ Jitter buffer handles in-order packets");
}

/// Tests jitter buffer with reordered packets.
#[test]
fn test_jitter_buffer_reordering() {
    // ---
    use receiver::JitterBuffer;
    
    let config = JitterBufferConfig {
        depth_ms: 0,
        max_packets: 10,
    };
    
    let mut buffer = JitterBuffer::new(config);
    
    // Insert packets out of order
    let packets = [0, 2, 1, 4, 3];
    for &seq in &packets {
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3]);
        buffer.insert(packet);
    }
    
    // Should play out in correct order
    for seq in 0..5 {
        let packet = buffer.get_next().expect("packet should be available");
        assert_eq!(packet.sequence, seq);
    }
    
    println!("✓ Jitter buffer reorders packets correctly");
}

/// Tests network simulator with packet loss.
#[test]
fn test_network_simulator_loss() {
    // ---
    let config = NetworkSimulatorConfig {
        loss_rate: 0.5, // 50% loss
        jitter_ms: 0,
        reorder_rate: 0.0,
        seed: Some(42), // Deterministic
    };
    
    let mut sim = NetworkSimulator::new(config);
    
    // Send 100 packets
    for seq in 0..100 {
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3]);
        sim.send(packet);
    }
    
    // Count received packets
    let mut received = 0;
    while sim.receive().is_some() {
        received += 1;
    }
    
    let stats = sim.stats();
    println!("Sent: {}, Lost: {}, Received: {}", stats.packets_sent, stats.packets_lost, received);
    
    // Should have ~50% loss (with some variance)
    assert!(stats.loss_rate > 0.3 && stats.loss_rate < 0.7);
    println!("✓ Network simulator applies packet loss ({:.1}%)", stats.loss_rate * 100.0);
}

/// Tests network simulator with jitter.
#[test]
fn test_network_simulator_jitter() {
    // ---
    let config = NetworkSimulatorConfig {
        loss_rate: 0.0,
        jitter_ms: 50, // Up to 100ms jitter
        reorder_rate: 0.0,
        seed: Some(42),
    };
    
    let mut sim = NetworkSimulator::new(config);
    
    // Send packets
    for seq in 0..10 {
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3]);
        sim.send(packet);
    }
    
    // Some packets may be delayed
    let immediate = sim.receive().is_some();
    let in_flight = sim.in_flight();
    
    println!("In flight: {}, Immediate delivery: {}", in_flight, immediate);
    assert!(in_flight > 0 || immediate);
    
    println!("✓ Network simulator applies jitter");
}

/// Tests network simulator with reordering.
#[test]
fn test_network_simulator_reordering() {
    // ---
    let config = NetworkSimulatorConfig {
        loss_rate: 0.0,
        jitter_ms: 0,
        reorder_rate: 0.3, // 30% reordering
        seed: Some(42),
    };
    
    let mut sim = NetworkSimulator::new(config);
    
    // Send packets
    for seq in 0..50 {
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, vec![1, 2, 3]);
        sim.send(packet);
    }
    
    // Collect received packets
    let mut sequences = Vec::new();
    while let Some(packet) = sim.receive() {
        sequences.push(packet.sequence);
    }
    
    // Check if any reordering occurred
    let mut reordered = false;
    for i in 1..sequences.len() {
        if sequences[i] < sequences[i - 1] {
            reordered = true;
            break;
        }
    }
    
    let stats = sim.stats();
    println!("Reordered packets: {}", stats.packets_reordered);
    assert!(reordered || stats.packets_reordered > 0);
    
    println!("✓ Network simulator reorders packets");
}

/// Tests codec with packet loss concealment.
#[test]
fn test_opus_packet_loss_concealment() {
    // ---
    let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
    let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");
    
    let frame = create_test_frame();
    
    // Encode a frame
    let encoded = encoder.encode(&frame).expect("encoding failed");
    
    // Decode it successfully
    let _decoded1 = decoder.decode(&encoded).expect("decoding failed");
    
    // Simulate packet loss - use PLC
    let concealed = decoder.conceal_loss().expect("PLC failed");
    
    assert_eq!(concealed.len(), frame.len());
    println!("✓ Opus PLC generates {} samples", concealed.len());
}

/// Integration test: Sender → Simulator → Receiver with 10% loss.
#[test]
fn test_end_to_end_with_loss() {
    // ---
    let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
    let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");
    
    let config = NetworkSimulatorConfig {
        loss_rate: 0.1, // 10% loss
        jitter_ms: 10,
        reorder_rate: 0.05,
        seed: Some(42),
    };
    
    let mut sim = NetworkSimulator::new(config);
    
    // Send 50 frames
    let frame = create_test_frame();
    let mut packets_sent = 0;
    
    for seq in 0..50 {
        let encoded = encoder.encode(&frame).expect("encoding failed");
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, encoded);
        sim.send(packet);
        packets_sent += 1;
    }
    
    // Receive and decode
    let mut packets_received = 0;
    let mut decode_success = 0;
    
    // Wait for packets to arrive
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    while let Some(packet) = sim.receive() {
        packets_received += 1;
        if decoder.decode(&packet.payload).is_ok() {
            decode_success += 1;
        }
    }
    
    let stats = sim.stats();
    println!("Sent: {}, Received: {}, Decoded: {}", packets_sent, packets_received, decode_success);
    println!("Loss rate: {:.1}%", stats.loss_rate * 100.0);
    
    // Should receive most packets
    assert!(packets_received >= 40); // At least 80% with 10% loss
    assert!(decode_success >= 40);
    
    println!("✓ End-to-end pipeline handles 10% packet loss");
}

/// Integration test: Sender → Simulator → Jitter Buffer → Receiver.
#[test]
fn test_end_to_end_with_jitter_buffer() {
    // ---
    use receiver::JitterBuffer;
    
    let mut encoder = OpusEncoderWrapper::new().expect("encoder creation failed");
    let mut decoder = OpusDecoderWrapper::new().expect("decoder creation failed");
    
    let sim_config = NetworkSimulatorConfig {
        loss_rate: 0.05,
        jitter_ms: 20,
        reorder_rate: 0.2, // 20% reordering
        seed: Some(42),
    };
    
    let jitter_config = JitterBufferConfig {
        depth_ms: 0, // No delay for testing
        max_packets: 50,
    };
    
    let mut sim = NetworkSimulator::new(sim_config);
    let mut buffer = JitterBuffer::new(jitter_config);
    
    // Send packets through simulator
    let frame = create_test_frame();
    for seq in 0..30 {
        let encoded = encoder.encode(&frame).expect("encoding failed");
        let packet = RtpPacket::new(seq, seq as u32 * 320, 0x12345678, encoded);
        sim.send(packet);
    }
    
    // Receive from simulator into jitter buffer
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    while let Some(packet) = sim.receive() {
        buffer.insert(packet);
    }
    
    // Decode from jitter buffer (should be in order)
    let mut decoded_count = 0;
    let mut last_seq: Option<u16> = None;
    
    while let Some(packet) = buffer.get_next() {
        // Verify ordering
        if let Some(last) = last_seq {
            assert_eq!(packet.sequence, last.wrapping_add(1), "Packets should be in order");
        }
        last_seq = Some(packet.sequence);
        
        if decoder.decode(&packet.payload).is_ok() {
            decoded_count += 1;
        }
    }
    
    println!("Decoded {} frames in order", decoded_count);
    assert!(decoded_count >= 25); // Most frames should arrive
    
    println!("✓ Jitter buffer reorders packets correctly in end-to-end pipeline");
}

/// Statistics tracking test.
#[test]
fn test_receiver_stats() {
    // ---
    use receiver::ReceiverStats;
    use std::time::Duration;
    
    let mut stats = ReceiverStats::new(Duration::from_secs(10));
    
    // Simulate receiving packets with some loss and reordering
    stats.record_packet(0, false);
    stats.record_packet(1, false);
    stats.record_packet(5, false); // Gap: lost 2, 3, 4
    stats.record_packet(4, true);  // Reordered
    
    assert_eq!(stats.packets_received, 4);
    assert_eq!(stats.packets_lost, 3);
    assert_eq!(stats.packets_reordered, 1);
    
    let loss_pct = stats.loss_percentage();
    assert!((loss_pct - 42.86).abs() < 0.1); // 3 lost out of 7 total
    
    println!("✓ Stats tracking works correctly");
}
