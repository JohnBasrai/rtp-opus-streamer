//! RTP Opus audio receiver - CLI binary.
//!
//! Receives RTP packets via UDP, decodes Opus audio,
//! and plays it through the system audio device.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use receiver::{receive_loop, AudioPlayer, JitterBufferConfig, OpusDecoderWrapper, RtpReceiver};
use rtp_opus_common::{init_tracing, ColorWhen, MetricsContext, MetricsServerConfig};

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

impl From<ColorArg> for ColorWhen {
    fn from(v: ColorArg) -> Self {
        match v {
            ColorArg::Auto => ColorWhen::Auto,
            ColorArg::Always => ColorWhen::Always,
            ColorArg::Never => ColorWhen::Never,
        }
    }
}

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

    /// Prometheus metrics bind address (serves `GET /metrics`).
    #[arg(long, default_value = "127.0.0.1:9200")]
    metrics_bind: String,

    /// Coloring
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorArg,
}

/// Capture version number from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // ---
    let args = Args::parse();
    init_tracing(args.color.into())?;
    info!("Starting RTP Opus receiver v{VERSION}");
    info!("Listening on port: {}", args.port);
    info!("Output device: {}", args.device);
    info!("Jitter buffer depth: {}ms", args.buffer_depth_ms);
    info!("Metrics bind: {}", args.metrics_bind);

    let metrics = MetricsContext::new("receiver")?;
    let metrics_bind = args.metrics_bind.parse().context("invalid metrics bind")?;
    let _metrics_task = metrics.spawn_metrics_server(MetricsServerConfig::new(metrics_bind));

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
    receive_loop(
        &mut receiver,
        &mut decoder,
        &mut player,
        jitter_config,
        &metrics,
    )
    .await?;

    Ok(())
}
