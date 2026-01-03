//! RTP Opus audio sender - CLI binary.
//!
//! Reads a WAV file, encodes it to Opus, packetizes into RTP,
//! and transmits via UDP to a receiver.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use rtp_opus_common::{init_tracing, ColorWhen, MetricsContext, MetricsServerConfig};
use sender::{stream_audio, OpusEncoderWrapper, RtpSender};

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

/// RTP Opus Sender - Stream audio files over RTP
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // ---
    /// Input audio file (WAV format)
    #[arg(short, long)]
    input: String,

    /// Remote address (IP:port) to send to
    #[arg(short, long, default_value = "127.0.0.1:5004")]
    remote: String,

    /// Packet transmission interval in milliseconds
    ///
    /// Controls pacing of packet transmission. Default 20ms matches
    /// the frame duration for real-time streaming.
    #[arg(short = 't', long, default_value = "20")]
    interval_ms: u64,

    /// Replay input audio continuously (default). Use `--no-loop` to play once and exit.
    #[arg(long = "no-loop", default_value_t = true, action = clap::ArgAction::SetFalse)]
    loop_audio: bool,

    /// Prometheus metrics bind address (serves `GET /metrics`).
    #[arg(long, default_value = "127.0.0.1:9100")]
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

    info!("Starting RTP Opus sender v{VERSION}");
    info!("Input file: {}", args.input);
    info!("Remote address: {}", args.remote);
    info!("Transmission interval: {}ms", args.interval_ms);
    info!("Loop audio: {}", args.loop_audio);
    info!("Metrics bind: {}", args.metrics_bind);

    let metrics = MetricsContext::new("sender")?;
    let metrics_bind = args.metrics_bind.parse().context("invalid metrics bind")?;
    let _metrics_task = metrics.spawn_metrics_server(MetricsServerConfig::new(metrics_bind));

    // Read and preprocess audio in blocking task
    info!("Reading audio file...");
    let input_path = args.input.clone();
    let audio = tokio::task::spawn_blocking(move || sender::read_wav(input_path))
        .await
        .context("audio reading task failed")??;

    info!(
        "Loaded {:.2}s of audio ({} frames)",
        audio.duration_secs(),
        audio.frame_count()
    );

    // Create encoder and network sender
    let mut encoder = OpusEncoderWrapper::new().context("failed to create encoder")?;
    let mut sender = RtpSender::new(&args.remote)
        .await
        .context("failed to create sender")?;

    // Generate random SSRC for this session
    let ssrc = rand::random::<u32>();
    info!("Session SSRC: 0x{:08X}", ssrc);

    // Stream audio frames
    info!("Starting transmission...");
    stream_audio(
        &audio,
        &mut encoder,
        &mut sender,
        &metrics,
        ssrc,
        args.interval_ms,
        args.loop_audio,
    )
    .await?;

    let (packets, bytes) = sender.stats();
    info!(
        "Transmission complete: {} packets, {} bytes",
        packets, bytes
    );

    Ok(())
}
