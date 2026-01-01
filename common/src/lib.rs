//! Common RTP utilities shared between sender and receiver.
//!
//! This crate provides the core RTP packet structure and serialization
//! logic used by both the sender and receiver components.

pub mod rtp;

pub use rtp::RtpPacket;
