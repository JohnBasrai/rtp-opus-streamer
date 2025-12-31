use anyhow::Result;
use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "5004")]
    port: u16,

    /// Audio output device
    #[arg(short, long, default_value = "default")]
    device: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting RTP Opus receiver");
    info!("Listening on port: {}", args.port);
    info!("Output device: {}", args.device);

    // TODO: Phase 1 implementation
    // - Receive RTP packets via UDP
    // - Decode Opus to PCM
    // - Play via audio device

    Ok(())
}
