# RTP Opus Streamer

Real-time audio streaming using RTP transport (RFC 3550) and Opus encoding (RFC 6716). Demonstrates network resilience, observability, and adaptive behavior for low-latency audio applications.

**Project Status:** Phase 1 in development  
**Development Roadmap:** See [Epic Issue](#) (link after creating GitHub issue)

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

- [ ] **Phase 1: Core Pipeline** (Week 1) - File → RTP → Playback
  - Audio file reader, Opus encode/decode, RTP packetization, UDP transport, playback
  
- [ ] **Phase 2: Network Resilience** (Week 2) - Robust packet handling
  - Jitter buffer, packet loss detection, reordering, simple concealment
  
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

**Sender:**
```bash
./target/release/sender --input audio.wav --remote 127.0.0.1:5004
```

**Receiver:**
```bash
./target/release/receiver --port 5004 --device default
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
