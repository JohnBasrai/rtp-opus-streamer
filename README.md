# RTP Opus Streamer

Real-time audio streaming system demonstrating production-grade network protocols, audio processing, and observability. Built with Rust for performance and safety.

## Overview

This project implements a complete RTP/Opus streaming pipeline with:
- **Network Resilience:** Jitter buffering, packet reordering, loss concealment
- **Audio Analysis:** Real-time FFT-based spectral analysis (Phase 4)
- **Observability:** Prometheus metrics, structured logging, performance profiling
- **Production Quality:** Comprehensive testing, CI/CD, RFC compliance

**Current Configuration:** Voice-optimized (16kHz, 24 kbps). Music content will sound degraded. See `samples/README.md` for details.

**Target Use Cases:** VoIP systems, live streaming, real-time communication platforms

**Documentation:**
- 📋 [Design Document](docs/design.md) - Architecture, performance analysis, design decisions
- 📊 [Performance Report](docs/performance-sample.txt) - Real-world metrics and validation

**Project Status:** Phase 4 complete (Audio Analysis & ML Integration)
**Development Roadmap:** See [Project Plan](https://github.com/JohnBasrai/rtp-opus-streamer/issues/1)

## Architecture

```
┌─────────────────────────────────────────┐
│         Audio Source                    │
│       (WAV file / device)               │
└──────────────┬──────────────────────────┘
               │ 20ms PCM frames
               ↓
┌──────────────┴──────────────────────────┐
│         Opus Encoder                    │
│    (24 kbps, voice-optimized)           │
└──────────────┬──────────────────────────┘
               │ Compressed frames
               ↓
┌──────────────┴──────────────────────────┐
│         RTP Packetizer                  │
│       (RFC 3550, seq#, ts)              │
└──────────────┬──────────────────────────┘
               │ RTP packets
               ↓
         [ UDP Socket ]
               │
               ↓
┌──────────────┴──────────────────────────┐
│         RTP Receiver                    │
│    (validate, extract payload)          │
└──────────────┬──────────────────────────┘
               │ Opus frames
               ↓
┌──────────────┴──────────────────────────┐
│         Jitter Buffer                   │
│   (reorder, loss detect, delay)         │
└──────────────┬──────────────────────────┘
               │ Ordered frames
               ↓
┌──────────────┴──────────────────────────┐
│         Opus Decoder                    │
│         (to PCM)                        │
└──────────────┬──────────────────────────┘
               │ PCM samples
               ↓
┌──────────────┴──────────────────────────┐
│         Audio Sink                      │
│       (playback device)                 │
└─────────────────────────────────────────┘
```

## Implementation Phases

- [x] **Phase 1: Core Pipeline** (Week 1) - File → RTP → Playback ✅
  - Audio file reader, Opus encode/decode, RTP packetization, UDP transport, playback

- [x] **Phase 2: Network Resilience** (Week 2) - Robust packet handling ✅
  - Jitter buffer (60ms configurable), packet reordering, loss detection, statistics tracking, PLC

- [x] **Phase 3: Observability** (Week 3) - Metrics and measurement ✅
  - Prometheus-based metrics, latency measurement, and system observability

- [x] **Phase 4: Audio Analysis & ML Integration** (Week 4+) - Audio intelligence ✅
  - Real-time FFT spectral analysis, dominant frequency extraction, spectral energy/centroid (use `--analyze` flag)

## Building

**Prerequisites:**

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install libopus-dev libasound2-dev
```

**Linux (Fedora/RHEL):**
```bash
sudo dnf install opus-devel alsa-lib-devel
```

**macOS:**
```bash
brew install opus
```

**Windows:**
- Install Opus via vcpkg or download pre-built binaries
- WASAPI used for audio (no additional dependencies)

**Build:**
```bash
cargo build --release
```

## Running

### Basic Usage

**Terminal 1 - Start Receiver:**
```bash
./target/release/receiver --port 5004

# With custom jitter buffer depth (default: 60ms)
./target/release/receiver --port 5004 --buffer-depth-ms 100

# With real-time audio analysis (Phase 4)
./target/release/receiver --port 5004 --analyze
```

**Terminal 2 - Send Audio:**
```bash
./target/release/sender --input samples/voice.wav --remote 127.0.0.1:5004
```

### Testing with Generated Audio

You can create a test WAV file using various tools:

```bash
# Using sox (if installed)
sox -n -r 16000 -c 1 test.wav synth 5 sine 440

# Using ffmpeg (if installed)
ffmpeg -f lavfi -i "sine=frequency=440:duration=5:sample_rate=16000" -ac 1 test.wav
```

### Command Line Options

**Sender:**
```bash
sender --input <file.wav> --remote <ip:port> [--interval-ms <ms>]
```
- `--input`: Path to WAV file (any sample rate, mono or stereo).
            Currently optimized for voice (see samples/README.md for details).
- `--remote`: Destination IP:port (default: 127.0.0.1:5004)
- `--interval-ms`: Packet send interval in ms (default: 20ms for real-time)

**Receiver:**
```bash
receiver --port <port> [--buffer-depth-ms <ms>] [--analyze]
```
- `--port`: UDP port to listen on (default: 5004)
- `--buffer-depth-ms`: Jitter buffer depth in milliseconds (default: 60ms)
- `--analyze`: Enable real-time spectral analysis output (Phase 4)

### Example: Local Loopback Test

```bash
# Terminal 1
cargo run --bin receiver --release

# Terminal 2
cargo run --bin sender --release -- --input samples/voice.wav

# With audio analysis
cargo run --bin receiver --release -- --analyze
```

## Testing

```bash
# Unit tests
cargo test

# Integration tests (requires audio fixtures)
cargo test --test integration

# Benchmarks
cargo bench
```

## Key Design Choices (Summary)

**Frame Size: 20ms**
Opus supports 2.5, 5, 10, 20, 40, 60ms frames. Using 20ms balances:
- Latency: Lower frame size reduces algorithmic delay
- Efficiency: Higher frame size improves compression
- Network: 20ms = 50 packets/sec, manageable overhead

**Jitter Buffer: 60ms**
Typical networks show 10-30ms jitter. 60ms buffer provides:
- Headroom for variance (2-3σ coverage)
- Acceptable added latency
- Reordering window for out-of-sequence packets

**Codec Configuration: Voice-Optimized**
Current settings (16kHz, 24 kbps, VOIP mode) prioritize bandwidth efficiency for speech.
Music content will sound degraded. Future work will add configurable codec modes.

See `docs/design.md` for full analysis.

## Performance Targets

| Metric               | Target        |
|----------------------|---------------|
| Glass-to-glass       | < 150ms (p50) |
| CPU per stream       | < 2%          |
| Packet loss @ 5%     | Imperceptible |
| Max concurrent       | 50+ streams   |

## Extending

This is a reference implementation. Production deployments should consider:
- SRTP for encryption
- DTLS key exchange
- ICE/STUN/TURN for NAT traversal
- Scalability (multicast, forwarding servers)

## References

- [RFC 3550](https://www.rfc-editor.org/rfc/rfc3550): RTP (Real-time Transport Protocol)
- [RFC 6716](https://www.rfc-editor.org/rfc/rfc6716): Opus Audio Codec
- [RFC 3551](https://www.rfc-editor.org/rfc/rfc3551): RTP Profile for Audio/Video

## License

MIT OR Apache-2.0
