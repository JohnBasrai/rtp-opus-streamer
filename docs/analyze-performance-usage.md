# Performance Analysis Script - Usage Guide

## Overview

`scripts/analyze-performance.sh` parses Prometheus metrics from test runs and validates performance against design targets from `docs/design.md`.

## Quick Start

```bash
# 1. Run end-to-end test, capture output
./scripts/test-sender-receiver.sh > test-output.log

# 2. Analyze the results
./scripts/analyze-performance.sh test-output.log
```

## Sample Output

```
===================================================================
RTP Opus Streamer - Performance Analysis
===================================================================

───────────────────────────────────────────────────────────────
Phase 4: Audio Analysis Performance
───────────────────────────────────────────────────────────────

FFT Performance:
  Total FFTs executed:  385
  Average per FFT:      333 μs
  Analysis rate:        25.7 FFTs/second
  Target:               < 50 μs per FFT (release build)
  Status:               ⚠  Debug build (expect ~30-40 μs in release)

───────────────────────────────────────────────────────────────
Core Pipeline Performance
───────────────────────────────────────────────────────────────

Opus Decode:
  Total frames:         773
  Average per frame:    58 μs
  Status:               ✓ Excellent

Receiver Pipeline (decode → analysis → playback):
  Total frames:         773
  Average per frame:    310 μs
  Analysis overhead:    ~53% of pipeline (debug build)
  Status:               ✓ Well under latency budget

Jitter Buffer:
  Average delay:        0.1 ms
  Status:               ✓ Within configured depth (60ms)

───────────────────────────────────────────────────────────────
Network Performance
───────────────────────────────────────────────────────────────

Packet Statistics:
  Packets received:     773
  Packets lost:         2 (0.26%)
  Packets reordered:    2
  Packets late:         0
  Bytes received:       47163

  Status:               ✓ Excellent (< 1% loss)

───────────────────────────────────────────────────────────────
Design Targets Validation (from docs/design.md)
───────────────────────────────────────────────────────────────

✓ FFT execution:      Target < 50 μs (release)
                      Actual: 333 μs (debug)

✓ Analysis rate:      Target ~31 FFTs/second
                      Actual: 25.7 FFTs/second

✓ Latency budget:     Target < 150 ms end-to-end
                      Pipeline: 310 μs per frame

✓ Packet loss:        Target < 5% (with concealment)
                      Actual: 0.26%

───────────────────────────────────────────────────────────────
Notes
───────────────────────────────────────────────────────────────

• Measurements are from DEBUG build (unoptimized)
• Release build typically 5-10x faster for compute-intensive code
• Expected FFT performance in release: 30-40 μs per transform
• Localhost testing may show artifacts in transit time metrics

===================================================================
```

## What It Measures

### Phase 4: Audio Analysis
- FFT execution time (average per transform)
- Analysis rate (FFTs per second)
- Comparison against 50 μs target

### Core Pipeline
- Opus decode time per frame
- End-to-end receiver pipeline time
- Analysis overhead percentage
- Jitter buffer delays

### Network
- Packet statistics (received, lost, reordered, late)
- Packet loss percentage
- Comparison against 5% loss threshold

### Design Target Validation
- Validates all measurable targets from `docs/design.md`
- Shows ✓ for passing metrics
- Shows ⚠ for debug build caveats

## Requirements

**Dependencies:**
- `bc` - For floating point calculations
- `awk` - For parsing metrics
- `grep` - For extracting values

**Install on Ubuntu/Debian:**
```bash
sudo apt-get install bc gawk
```

**Install on macOS:**
```bash
brew install bc gawk
```

## Usage Patterns

### Quick validation during development
```bash
./scripts/test-sender-receiver.sh > /tmp/test.log
./scripts/analyze-performance.sh /tmp/test.log
```

### CI/regression testing
```bash
# Run tests and capture
./scripts/test-sender-receiver.sh > results/$(date +%Y%m%d-%H%M%S).log

# Analyze latest
./scripts/analyze-performance.sh results/*.log | tail -1
```

### Compare before/after changes
```bash
# Before changes
./scripts/test-sender-receiver.sh > before.log

# Make changes...

# After changes  
./scripts/test-sender-receiver.sh > after.log

# Compare
diff <(./scripts/analyze-performance.sh before.log) \
     <(./scripts/analyze-performance.sh after.log)
```

## Limitations

**Cannot measure:**
- Memory usage (requires OS-level tools like `valgrind`, `/proc`)
- CPU percentage (requires OS-level sampling)
- True glass-to-glass latency (requires instrumented timestamps)

**For these, use:**
- `valgrind --tool=massif` for memory profiling
- `perf` or `flamegraph` for CPU profiling
- Custom timestamp instrumentation for true latency

## Integration with Grafana

This script is **complementary** to Grafana, not a replacement:

| Tool | Purpose |
|------|---------|
| **Grafana** | Live monitoring, time-series trends, alerts |
| **analyze-performance.sh** | One-shot validation, CI regression testing |

Use Grafana for production monitoring, use this script for development validation and CI.

## Future Enhancements

Potential improvements (not implemented):
- Rust binary version for type safety and better parsing
- JSON output mode for CI integration
- Historical comparison (track metrics over time)
- Automated pass/fail thresholds for CI
- Memory and CPU profiling integration

## See Also

- `docs/design.md` - Performance targets and architecture
- `scripts/test-sender-receiver.sh` - End-to-end test script
- `CONTRIBUTING.md` - Testing guidelines
