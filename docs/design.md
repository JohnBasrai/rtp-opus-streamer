# Design Document

## Problem Statement

Real-time audio streaming requires:
1. Low latency (< 150ms glass-to-glass)
2. Network resilience (handle 5-10% packet loss)
3. Quality under constraint (adapt to bandwidth)

## System Requirements

- Latency target: < 100ms (interactive voice)
- Loss tolerance: 10% without quality degradation
- Bitrate: 24-32 kbps (wideband speech)
- Platform: Linux, macOS, Windows

## Architecture

### Sender Pipeline

1. **Audio Capture**: Read 20ms frames from file/device
2. **Encoding**: Opus at 24 kbps, wideband (16kHz)
3. **Packetization**: RTP header + payload
4. **Transmission**: UDP socket

### Receiver Pipeline

1. **Reception**: UDP socket → packet validation
2. **Jitter Buffer**: Reorder, delay compensation
3. **Decoding**: Opus → PCM samples
4. **Playback**: Write to audio device

## Key Design Decisions

### 1. Codec Selection: Opus

**Alternatives Considered:**
- AAC: Higher quality at high bitrates, more complex
- MP3: Poor low-latency performance
- Speex: Legacy, Opus successor

**Why Opus:**
- Best latency (algorithmic delay: 22.5ms)
- Excellent loss concealment (PLC)
- Wide bitrate range (6-510 kbps)
- Royalty-free

### 2. Jitter Buffer Strategy

**Fixed Buffer:**
- Pro: Simple implementation
- Con: Wastes latency during good network conditions

**Adaptive Buffer:**
- Pro: Minimizes latency when possible
- Con: Complexity, transition artifacts

**Implemented: Fixed 60ms (Phase 2)**
- Rationale: Simplicity first
- Future: Adaptive in Phase 4

### 3. Packet Loss Handling

**Phase 2:**
- Detection: Sequence number gaps
- Concealment: Repeat last frame (simple PLC)

**Phase 4 (Future):**
- Opus in-band FEC (add redundancy)
- Tradeoff: +10% bandwidth for 20% loss recovery

## Performance Analysis

### Latency Budget

| Component        | Latency  |
|------------------|----------|
| Capture buffer   | 20ms     |
| Encoding         | 22.5ms   |
| Network (p50)    | 15ms     |
| Jitter buffer    | 60ms     |
| Decoding         | 22.5ms   |
| Playback buffer  | 20ms     |
| **Total**        | **160ms**|

Target: < 150ms → Optimize jitter buffer in Phase 4

### CPU Profiling

To be measured in Phase 1. Expected hotspots:
- Opus encode/decode (60-70%)
- RTP processing (20-30%)
- I/O (10%)

## Testing Strategy

### Unit Tests
- Codec wrapper correctness
- RTP packet serialization
- Jitter buffer logic

### Integration Tests
- End-to-end pipeline (loopback)
- Packet loss simulation (probabilistic drop)
- Latency measurement

### Manual Tests
- Cross-platform audio devices
- Network conditions (WiFi, LTE)

## Future Enhancements

1. **Phase 3: Observability**
   - Prometheus metrics endpoint
   - OpenTelemetry tracing
   - Quality dashboards

2. **Phase 4: Adaptive Behavior**
   - FEC (Opus in-band)
   - Bitrate adaptation (RTCP feedback)
   - DTX (Discontinuous Transmission)

3. **Beyond:**
   - WebRTC interop (DTLS-SRTP)
   - Multi-codec support (fallback)
   - Forwarding server (SFU architecture)
