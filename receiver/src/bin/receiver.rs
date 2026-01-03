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
    #[arg(
        short,
        long,
        default_value_t = 5004,
        help = "Port to listen on",
        long_help = "UDP port to listen on for incoming RTP packets."
    )]
    port: u16,

    /// Jitter buffer depth in milliseconds
    #[arg(
        short = 'b',
        long,
        default_value_t = 60,
        help = "Jitter buffer depth in milliseconds",
        long_help = "Jitter buffer depth in milliseconds.\n\n\
                     Controls how much packet reordering and jitter the receiver can tolerate.\n\
                     Higher values improve robustness at the cost of additional latency."
    )]
    buffer_depth_ms: u32,

    /// Prometheus metrics bind address (serves `GET /metrics`).
    #[arg(
        long,
        default_value = "127.0.0.1:9200",
        help = "Prometheus metrics bind address",
        long_help = "Bind address for the Prometheus metrics endpoint.\n\n\
                     Metrics are exposed via HTTP at GET /metrics."
    )]
    metrics_bind: String,

    /// Coloring
    #[arg(
        long,
        value_enum,
        default_value_t = ColorArg::Auto,
        help = "Coloring",
        long_help = "Controls colored output.\n\n\
                     auto: Enable colors when stdout is a TTY and EMACS is not set.\n\
                     always: Always enable colors.\n\
                     never: Disable colors."
    )]
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
    info!("Output device: {}", "default");
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
