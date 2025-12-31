use anyhow::Result;
use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input audio file (WAV format)
    #[arg(short, long)]
    input: String,

    /// Remote address (IP:port)
    #[arg(short, long, default_value = "127.0.0.1:5004")]
    remote: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    info!("Starting RTP Opus sender");
    info!("Input file: {}", args.input);
    info!("Remote address: {}", args.remote);

    // TODO: Phase 1 implementation
    // - Read WAV file
    // - Encode to Opus
    // - Packetize as RTP
    // - Send via UDP

    Ok(())
}
