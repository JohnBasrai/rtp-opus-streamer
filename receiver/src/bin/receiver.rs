//! RTP Opus audio receiver - CLI binary.
//!
//! Receives RTP packets via UDP, decodes Opus audio,
//! and plays it through the system audio device.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use receiver::{receive_loop, AudioPlayer, JitterBufferConfig, OpusDecoderWrapper, RtpReceiver};

/// RTP Opus Receiver - Receive and play audio streams
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // ---
    /// Port to listen on
    #[arg(short, long, default_value = "5004")]
    port: u16,

    /// Audio output device (not yet implemented - uses default)
    #[arg(short, long, default_value = "default")]
    device: String,

    /// Jitter buffer depth in milliseconds
    #[arg(short = 'b', long, default_value = "60")]
    buffer_depth_ms: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // ---
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting RTP Opus receiver");
    info!("Listening on port: {}", args.port);
    info!("Output device: {}", args.device);
    info!("Jitter buffer depth: {}ms", args.buffer_depth_ms);

    // Create decoder and network receiver
    let mut decoder = OpusDecoderWrapper::new().context("failed to create decoder")?;
    let mut receiver = RtpReceiver::new(args.port)
        .await
        .context("failed to create receiver")?;

    // Create audio player
    let mut player = AudioPlayer::new().context("failed to create audio player")?;

    // Configure jitter buffer
    let jitter_config = JitterBufferConfig {
        depth_ms: args.buffer_depth_ms,
        max_packets: 100,
    };

    info!("Ready to receive audio...");

    // Run receiver loop
    receive_loop(&mut receiver, &mut decoder, &mut player, jitter_config).await?;

    Ok(())
}
