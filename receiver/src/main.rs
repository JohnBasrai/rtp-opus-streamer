//! RTP Opus audio receiver.
//!
//! Receives RTP packets via UDP, decodes Opus audio,
//! and plays it through the system audio device.

mod audio;
mod codec;
mod network;
mod rtp;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

use audio::AudioPlayer;
use codec::OpusDecoderWrapper;
use network::RtpReceiver;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    // ---
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting RTP Opus receiver");
    info!("Listening on port: {}", args.port);
    info!("Output device: {}", args.device);

    // Create decoder and network receiver
    let mut decoder = OpusDecoderWrapper::new().context("failed to create decoder")?;
    let mut receiver = RtpReceiver::new(args.port)
        .await
        .context("failed to create receiver")?;

    // Create audio player (in main thread for now)
    let mut player = AudioPlayer::new().context("failed to create audio player")?;

    info!("Ready to receive audio...");

    // Receive and play loop
    let mut last_sequence: Option<u16> = None;
    let mut frames_played = 0;

    loop {
        match receiver.receive().await? {
            Some(packet) => {
                // Check for packet loss
                if let Some(last_seq) = last_sequence {
                    let expected = last_seq.wrapping_add(1);
                    if packet.sequence != expected {
                        let lost = packet.sequence.wrapping_sub(expected);
                        warn!(
                            "Packet loss detected: expected seq={}, got seq={} ({} packets lost)",
                            expected, packet.sequence, lost
                        );

                        // Conceal lost packets with PLC
                        for _ in 0..lost.min(10) {
                            // Limit concealment to 10 packets
                            if let Ok(concealed) = decoder.conceal_loss() {
                                player.play(&concealed);
                            }
                        }
                    }
                }

                // Decode and play
                match decoder.decode(&packet.payload) {
                    Ok(samples) => {
                        player.play(&samples);
                        frames_played += 1;

                        if frames_played % 100 == 0 {
                            info!("Played {} frames", frames_played);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to decode packet seq={}: {}", packet.sequence, e);
                        // Use PLC for decode errors
                        if let Ok(concealed) = decoder.conceal_loss() {
                            player.play(&concealed);
                        }
                    }
                }

                last_sequence = Some(packet.sequence);
            }
            None => {
                // Invalid packet, already logged by receiver
                continue;
            }
        }
    }
}
