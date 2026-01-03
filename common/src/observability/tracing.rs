//! Tracing initialization.
//!
//! Centralizes tracing config so both binaries behave the same.

use crate::ColorWhen;
use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing subscriber.
///
/// - Respects `RUST_LOG` via `EnvFilter`.
/// - ANSI color controlled by `ColorWhen`.
pub fn init_tracing(color: ColorWhen) -> Result<()> {
    // ---
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_ansi(color.should_color_stderr())
        .init();

    Ok(())
}
