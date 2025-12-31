//! RTP Opus audio sender.
//!
//! Reads a WAV file, encodes it to Opus, packetizes into RTP,
//! and transmits via UDP to a receiver.

mod audio;
mod codec;
mod network;
mod rtp;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

use audio::AudioData;
use codec::OpusEncoderWrapper;
use network::RtpSender;
use rtp::RtpPacket;

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
    #[arg(short, long, default_value = "20")]
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
    let audio = tokio::task::spawn_blocking(move || audio::read_wav(input_path))
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

/// Streams audio frames over RTP.
///
/// Encodes each frame with Opus and transmits as RTP packets with
/// proper timing and sequencing.
async fn stream_audio(
    audio: &AudioData,
    encoder: &mut OpusEncoderWrapper,
    sender: &mut RtpSender,
    ssrc: u32,
    interval_ms: u64,
) -> Result<()> {
    // ---
    let mut sequence: u16 = 0;
    let mut timestamp: u32 = 0;
    let mut frame_count = 0;

    for frame in audio.frames() {
        // Pad last frame if needed
        let mut frame_data = frame.to_vec();
        if frame_data.len() < codec::SAMPLES_PER_FRAME {
            warn!(
                "Padding last frame: {} samples -> {}",
                frame_data.len(),
                codec::SAMPLES_PER_FRAME
            );
            frame_data.resize(codec::SAMPLES_PER_FRAME, 0);
        }

        // Encode frame
        let payload = encoder
            .encode(&frame_data)
            .with_context(|| format!("failed to encode frame {}", frame_count))?;

        // Create and send RTP packet
        let packet = RtpPacket::new(sequence, timestamp, ssrc, payload);
        sender
            .send(&packet)
            .await
            .with_context(|| format!("failed to send packet {}", sequence))?;

        // Update sequence and timestamp
        sequence = sequence.wrapping_add(1);
        timestamp = timestamp.wrapping_add(codec::SAMPLES_PER_FRAME as u32);
        frame_count += 1;

        // Pace transmission (real-time simulation)
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
    }

    info!("Streamed {} frames", frame_count);
    Ok(())
}
