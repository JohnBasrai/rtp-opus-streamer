//! Shared library used by both binaries.
//!
//! This crate is the **public gateway** for all shared functionality. Per EMBP,
//! downstream crates should import through `common::*` exports and should not
//! drill into internal module structure.

mod cli;
mod observability;
mod rtp;

pub use cli::ColorWhen;
pub use observability::{init_tracing, MetricsContext, MetricsServerConfig};
pub use rtp::RtpPacket;
