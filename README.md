# RTP Opus Streamer

Real-time audio streaming using RTP transport (RFC 3550) and Opus encoding (RFC 6716). Demonstrates network resilience, observability, and adaptive behavior for low-latency audio applications.

**Project Status:** Phase 2 complete  
**Development Roadmap:** See [Project Plan](rtp-opus-streamer-project-notes.md)

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
│          (24 kbps)                      │
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
  
- [ ] **Phase 3: Observability** (Week 3) - Metrics and measurement
  - Prometheus metrics, latency measurement, quality metrics, logging
  
- [ ] **Phase 4: Adaptive Behavior** (Week 4+) - Production-quality features
  - Forward Error Correction, adaptive bitrate, congestion control, multi-stream

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
```

**Terminal 2 - Send Audio:**
```bash
./target/release/sender --input audio.wav --remote 127.0.0.1:5004
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
- `--input`: Path to WAV file (any sample rate, mono or stereo)
- `--remote`: Destination IP:port (default: 127.0.0.1:5004)
- `--interval-ms`: Packet send interval in ms (default: 20ms for real-time)

**Receiver:**
```bash
receiver --port <port>
```
- `--port`: UDP port to listen on (default: 5004)

### Example: Local Loopback Test

```bash
# Terminal 1
cargo run --bin receiver --release

# Terminal 2
cargo run --bin sender --release -- --input voice.wav
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

## Design Decisions

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
