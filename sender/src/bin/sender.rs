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
    #[arg(
        short,
        long,
        help = "Input audio file (WAV format)",
        long_help = "Path to an input WAV file to be streamed over RTP.\n\n\
                     The file is decoded, packetized, and transmitted in real time."
    )]
    input: String,

    /// Remote address (IP:port) to send to
    #[arg(
        short,
        long,
        default_value = "127.0.0.1:5004",
        help = "Remote address (IP:port) to send to",
        long_help = "Remote address of the RTP receiver.\n\n\
                     The sender transmits RTP packets to this address."
    )]
    remote: String,

    /// Packet transmission interval in milliseconds
    ///
    /// Controls pacing of packet transmission. Default 20ms matches
    /// the frame duration for real-time streaming.
    #[arg(
        short = 't',
        long,
        default_value_t = 20,
        help = "Packet transmission interval in milliseconds",
        long_help = "Packet transmission interval in milliseconds.\n\n\
                     Controls the pacing of RTP packet transmission.\n\
                     The default of 20ms matches typical Opus frame duration."
    )]
    interval_ms: u64,

    #[arg(
        long = "no-loop",
        help = "Play input audio once and exit",
        long_help = "Disable looping of the input audio file.\n\n\
                     By default, the sender replays the input file continuously.\n\
                     When this flag is set, the file is played once and the sender exits."
    )]
    no_loop: bool,

    /// Prometheus metrics bind address (serves `GET /metrics`).
    #[arg(
        long,
        default_value = "127.0.0.1:9100",
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

    info!("Starting RTP Opus sender v{VERSION}");
    info!("Input file: {}", args.input);
    info!("Remote address: {}", args.remote);
    info!("Transmission interval: {}ms", args.interval_ms);
    info!("Loop audio: {}", !args.no_loop);
    info!("Metrics bind: {}", args.metrics_bind);

    let metrics = MetricsContext::new("sender")?;
    let metrics_bind = args.metrics_bind.parse().context("invalid metrics bind")?;
    let _metrics_task = metrics.spawn_metrics_server(MetricsServerConfig::new(metrics_bind));

    // Read and preprocess audio in blocking task
    info!("Reading audio file...");
    let input_path = args.input.clone();
    let audio = match tokio::task::spawn_blocking(move || sender::read_wav(input_path))
        .await
        .context("audio reading task failed")?
    {
        Ok(audio) => audio,
        Err(err) => {
            tracing::error!("Failed to read audio file: {err}");
            std::process::exit(1);
        }
    };

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
        args.no_loop,
    )
    .await?;

    let (packets, bytes) = sender.stats();
    info!(
        "Transmission complete: {} packets, {} bytes",
        packets, bytes
    );

    Ok(())
}
