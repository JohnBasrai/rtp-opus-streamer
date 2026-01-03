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

## Code Structure (Phase 2 Refactor)

### Library Architecture

**Before Phase 2:**
- Sender and receiver as binaries only
- RTP code duplicated (224 lines)
- No testability via library imports

**After Phase 2:**
```
rtp-opus-streamer/
├── common/          # Shared RTP packet code
│   └── src/
│       └── rtp.rs   # RtpPacket struct + methods
├── sender/
│   ├── src/
│   │   ├── lib.rs           # Public API
│   │   ├── bin/sender.rs    # CLI wrapper
│   │   └── ... (modules)
├── receiver/
│   ├── src/
│   │   ├── lib.rs               # Public API
│   │   ├── bin/receiver.rs      # CLI wrapper
│   │   ├── jitter_buffer.rs     # NEW
│   │   └── stats.rs             # NEW
└── tests/
    ├── network_simulator.rs     # NEW
    └── test_network_resilience.rs  # NEW
```

**Benefits:**
1. **Testability**: True end-to-end tests via library imports
2. **Code Reuse**: Common RTP eliminates duplication
3. **Modularity**: Clear separation of concerns
4. **Extensibility**: Easy to add features in Phase 3/4

**Library APIs:**
- `sender::stream_audio()`: Encode and transmit audio
- `receiver::receive_loop()`: Receive, buffer, decode, play
- `common::RtpPacket`: Shared packet structure

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

### 2. Jitter Buffer Strategy (Phase 2 Implementation)

**Fixed Buffer (Implemented):**
- Depth: 60ms (configurable via CLI)
- Packet reordering by sequence number
- Late packet detection and discard
- Priming period: wait for buffer fill before playout

**Design Rationale:**
- **Fixed vs Adaptive**: Fixed depth simplifies implementation and testing
- **60ms Choice**: Covers typical network jitter (10-30ms) with headroom
- **Reordering Window**: Sufficient for 20% out-of-order packets
- **Late Packet Handling**: Discard packets arriving after playout deadline

**Implementation Details:**
```rust
pub struct JitterBuffer {
    buffer: VecDeque<BufferedPacket>,  // Sorted by sequence
    next_sequence: u16,                  // Next expected packet
    is_primed: bool,                     // Buffer filled to depth
}
```

**Key Algorithms:**
1. **Insertion**: Binary search to maintain sequence order
2. **Playout**: Wait for priming, then release in sequence order
3. **Late Detection**: Sequence comparison accounting for wraparound

**Future: Adaptive in Phase 4**
- Dynamic depth adjustment based on observed jitter
- Tradeoff: Complexity vs latency optimization

### 3. Packet Loss Handling (Phase 2 Implementation)

**Loss Detection:**
- Sequence number gap analysis
- Accounts for wraparound at 65535
- Distinguishes between loss and reordering

**Concealment Strategy:**
- **Opus PLC (Packet Loss Concealment)**: Built-in decoder function
- Generates perceptually similar frames for lost packets
- Quality: Acceptable for up to 10% loss
- Limitation: No forward error correction (Phase 4)

**Statistics Tracking:**
- Total packets lost (via sequence gaps)
- Loss percentage over time
- Late packets (arrived after playout deadline)
- Periodic logging every 5 seconds

**Implementation:**
```rust
pub struct ReceiverStats {
    packets_received: u64,
    packets_lost: u64,      // Detected via sequence gaps
    packets_late: u64,       // Arrived too late
    packets_reordered: u64,  // Out of sequence
}
```

**Phase 4 (Future):**
- Opus in-band FEC (forward error correction)
- Redundant encoding: +10% bandwidth → 20% loss recovery
- RTCP feedback for sender-side adaptation

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
- RTP packet serialization (common crate)
- Jitter buffer reordering logic
- Statistics calculation accuracy
- Sequence wraparound handling

### Integration Tests (Phase 2)

**Network Simulator:**
- In-process network condition simulation
- Configurable packet loss (0-100%)
- Jitter injection (random delays)
- Packet reordering (out-of-sequence delivery)
- Deterministic testing (seeded RNG)

**End-to-End Tests:**
1. **Perfect Network**: Verify baseline codec quality
2. **10% Packet Loss**: Validate PLC effectiveness
3. **20% Reordering**: Jitter buffer reordering
4. **Combined Conditions**: Loss + jitter + reordering
5. **Sequence Wraparound**: Handle u16 wraparound at 65535

**Test Coverage:**
```rust
#[test]
fn test_end_to_end_with_loss() {
    // Sender → Simulator (10% loss) → Jitter Buffer → Decoder
    // Validates: PLC, stats tracking, in-order delivery
}
```

**Why In-Process Simulator:**
- Zero external dependencies (no Docker/toxiproxy)
- Fast, deterministic tests
- CI-friendly
- Demonstrates systems thinking

**Future: Production Testing**
- Real network conditions (WiFi, LTE)
- Tools: toxiproxy, tc (traffic control)
- Long-duration stability tests

### Manual Tests
- Cross-platform audio devices
- Network conditions (WiFi, LTE)
- Multi-hour streaming stability

## Phase 3: Observability (Completed)

Phase 3 introduces first-class observability to `rtp-opus-streamer`. The goal is not ad-hoc logging, but **continuous visibility into system health, network behavior, and audio pipeline performance** for long-running sender and receiver processes.

### Design Goals

1. **Low-overhead instrumentation**
   - Metrics must not meaningfully impact the real-time audio path
   - Hot paths are instrumented carefully and sparingly

2. **Process-oriented observability**
   - Sender and receiver are treated as long-lived services, not short-lived commands
   - Metrics reflect system behavior over time, not per-invocation summaries

3. **Operational clarity**
   - Metrics should answer:
     - Is audio flowing?
     - Is the network degrading?
     - Is latency accumulating?
     - Are packets being lost, reordered, or delayed?

4. **Production-aligned tooling**
   - Prometheus-compatible metrics
   - HTTP-based scraping model
   - No custom collectors or proprietary formats

---

### Observability Architecture

Both sender and receiver expose metrics via an embedded HTTP endpoint suitable for Prometheus scraping.

```

┌───────────────────────────────┐
│   Sender / Receiver Process   │
│                               │
│  ┌─────────────────────────┐  │
│  │ Audio + RTP Pipeline    │  │
│  └───────────┬─────────────┘  │
│              │                │
│  ┌───────────▼─────────────┐  │
│  │ Observability Layer     │  │
│  │ (metrics + registry)    │  │
│  └───────────┬─────────────┘  │
│              │                │
│  ┌───────────▼─────────────┐  │
│  │ HTTP Metrics Endpoint   │  │
│  └───────────┬─────────────┘  │
│              │                │
└──────────────┼────────────────┘
│
Prometheus Scraper

```
Prometheus scrapes metrics from the HTTP endpoint exposed by each process.


This architecture ensures:
- Instrumentation is **centralized**
- Metrics are **consistent** across binaries
- Observability concerns do **not leak** into core audio logic

---

### Shared Observability Layer

A shared observability module is used by both sender and receiver to enforce consistency and
avoid duplication.

**Responsibilities:**
- Metrics registration and lifecycle
- Common counters, gauges, and histograms
- Encapsulation of Prometheus client details

**Non-Goals:**
- No business logic
- No audio or RTP semantics
- No dependency on CLI parsing or runtime configuration

This aligns with Explicit Module Boundary Pattern (EMBP) principles:
- Observability is a *service module*, not a cross-cutting concern
- Sender and receiver depend on it explicitly, not implicitly

---

### Metrics Philosophy

Metrics are designed around **questions**, not internal data structures.

Examples of questions Phase 3 metrics answer:
- Are packets being lost or merely reordered?
- Is jitter increasing over time?
- Is end-to-end latency stable or drifting?
- Is the system keeping up with real-time constraints?

Metric cardinality is intentionally kept low to ensure:
- Predictable memory usage
- Prometheus scalability
- Safe long-running operation

Detailed metric names and types are intentionally omitted from this document; they are considered an implementation detail rather than a design contract to preserve refactoring freedom and avoid over-specifying the public observability contract.

---

### Manual Testing Strategy (Phase 3)

While Phase 2 emphasized deterministic, in-process testing, Phase 3 adds a **manual test script**
to validate observability under real execution conditions.

Rationale:
- Observability correctness depends on *time*, *duration*, and *steady-state behavior*
- These properties are difficult to validate in unit or short-lived integration tests
- Manual tests complement (not replace) automated coverage

The manual workflow validates:
- Metrics endpoint availability
- Counter monotonicity
- Gauge stability
- Histogram population over time
- Behavior under real packet loss and jitter

---

### Why Observability Was Added in Phase 3

Observability is intentionally **not** a Phase 1 or Phase 2 concern.

By Phase 3:
- The audio pipeline is stable
- Network behavior is well-defined
- Failure modes are understood

This makes Phase 3 the first point where metrics provide **signal instead of noise**.

Adding observability earlier would have produced misleading data while core behavior was still in flux.

## Future Enhancements

1. **Phase 4: Adaptive Behavior**
   - Opus in-band FEC (forward error correction)
   - Sender-side bitrate adaptation
   - RTCP-based feedback loops
   - Dynamic jitter buffer depth adjustment

2. **Beyond Phase 4**
   - WebRTC interop (DTLS-SRTP)
   - Multi-codec support and negotiation
   - Forwarding server (SFU-style architecture)
   - Multi-stream and multi-receiver topologies
