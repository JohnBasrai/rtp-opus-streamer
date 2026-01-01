//! RTP Opus audio sender - CLI binary.
//!
//! Reads a WAV file, encodes it to Opus, packetizes into RTP,
//! and transmits via UDP to a receiver.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use sender::{stream_audio, OpusEncoderWrapper, RtpSender};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    // ---
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting RTP Opus sender");
    info!("Input file: {}", args.input);
    info!("Remote address: {}", args.remote);
    info!("Transmission interval: {}ms", args.interval_ms);

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
    stream_audio(&audio, &mut encoder, &mut sender, ssrc, args.interval_ms).await?;

    let (packets, bytes) = sender.stats();
    info!(
        "Transmission complete: {} packets, {} bytes",
        packets, bytes
    );

    Ok(())
}
