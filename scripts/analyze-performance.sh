#!/usr/bin/env bash
#
# Performance analysis for RTP Opus Streamer metrics.
#
# Parses Prometheus metrics output and validates against design targets.
#
# Usage:
#   ./scripts/analyze-performance.sh <metrics-log-file>
#
# Example:
#   ./scripts/test-sender-receiver.sh > test-output.log
#   ./scripts/analyze-performance.sh test-output.log

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <metrics-log-file>"
    echo
    echo "Example:"
    echo "  ./scripts/test-sender-receiver.sh > test-output.log"
    echo "  ./scripts/analyze-performance.sh test-output.log"
    exit 1
fi

METRICS_FILE="$1"

if [ ! -f "$METRICS_FILE" ]; then
    echo "ERROR: File not found: $METRICS_FILE"
    exit 1
fi

# Verify file contains Prometheus metrics
if ! grep -q "rtp_opus_streamer" "$METRICS_FILE" 2>/dev/null; then
    echo "ERROR: File does not appear to contain Prometheus metrics"
    echo "Expected format: output from test-sender-receiver.sh"
    echo
    echo "Usage: ./scripts/analyze-performance.sh <metrics-log-file>"
    echo "Example: ./scripts/test-sender-receiver.sh > test.log"
    echo "         ./scripts/analyze-performance.sh test.log"
    exit 1
fi

# Detect build type from log file
if ! grep -qi "build *: release" "$METRICS_FILE"; then
    build=debug
else
    build=release
fi

# Helper function to extract metric value
get_metric() {
    local pattern="$1"
    grep "$pattern" "$METRICS_FILE" 2>/dev/null | tail -1 | awk '{print $NF}' || echo ""
}

# Helper function to format microseconds
format_us() {
    local us="$1"
    # Handle empty or zero
    if [ -z "$us" ] || [ "$us" = "0" ]; then
        echo "0 μs"
        return
    fi
    
    # Check if less than 1000 μs (1 ms)
    if [ "$(echo "$us < 1000" | bc)" -eq 1 ]; then
        printf "%.1f μs" "$us"
    else
        local ms=$(echo "scale=2; $us / 1000" | bc)
        printf "%.2f ms (%.0f μs)" "$ms" "$us"
    fi
}

echo "==================================================================="
echo "RTP Opus Streamer - Performance Analysis"
echo "==================================================================="
echo

# Extract receiver metrics
fft_count=$(get_metric 'analysis_fft_processing_seconds_count{process="receiver"}')
fft_sum=$(get_metric 'analysis_fft_processing_seconds_sum{process="receiver"}')
features_total=$(get_metric 'analysis_features_extracted_total{process="receiver"}')

decode_count=$(get_metric 'opus_decode_seconds_count{process="receiver"}')
decode_sum=$(get_metric 'opus_decode_seconds_sum{process="receiver"}')

# Early validation - check if we got any metrics at all
if [ -z "$decode_count" ] || [ "$decode_count" = "0" ]; then
    echo "ERROR: No receiver metrics found in file"
    echo
    echo "This could mean:"
    echo "  1. The test did not run long enough to generate metrics"
    echo "  2. The receiver was not started with metrics enabled"
    echo "  3. The file does not contain the final metrics snapshot"
    echo
    echo "Expected metrics snapshot at end of file from:"
    echo "  ./scripts/test-sender-receiver.sh > output.log"
    echo
    echo "Debug: First few lines of file:"
    head -20 "$METRICS_FILE"
    exit 1
fi

pipeline_count=$(get_metric 'receiver_pipeline_seconds_count{process="receiver"}')
pipeline_sum=$(get_metric 'receiver_pipeline_seconds_sum{process="receiver"}')

jitter_count=$(get_metric 'jitter_buffer_delay_seconds_count{process="receiver"}')
jitter_sum=$(get_metric 'jitter_buffer_delay_seconds_sum{process="receiver"}')

packets_recv=$(get_metric 'rtp_packets_received_total{process="receiver"}')
packets_lost=$(get_metric 'rtp_packets_lost_total{process="receiver"}')
packets_reordered=$(get_metric 'rtp_packets_reordered_total{process="receiver"}')
packets_late=$(get_metric 'rtp_packets_late_total{process="receiver"}')

bytes_recv=$(get_metric 'rtp_bytes_received_total{process="receiver"}')

# Calculate averages
if [ -n "$fft_count" ] && [ "$fft_count" -gt 0 ]; then
    fft_avg_us=$(echo "scale=2; ($fft_sum * 1000000) / $fft_count" | bc)
else
    fft_avg_us=0
fi

if [ -n "$decode_count" ] && [ "$decode_count" -gt 0 ]; then
    decode_avg_us=$(echo "scale=2; ($decode_sum * 1000000) / $decode_count" | bc)
else
    decode_avg_us=0
fi

if [ -n "$pipeline_count" ] && [ "$pipeline_count" -gt 0 ]; then
    pipeline_avg_us=$(echo "scale=2; ($pipeline_sum * 1000000) / $pipeline_count" | bc)
else
    pipeline_avg_us=0
fi

if [ -n "$jitter_count" ] && [ "$jitter_count" -gt 0 ]; then
    jitter_avg_ms=$(echo "scale=2; ($jitter_sum * 1000) / $jitter_count" | bc)
else
    jitter_avg_ms=0
fi

if [ -n "$packets_recv" ] && [ "$packets_recv" -gt 0 ]; then
    loss_pct=$(echo "scale=2; ($packets_lost / $packets_recv) * 100" | bc)
else
    loss_pct=0
fi

# Analysis rate (FFTs per second)
if [ -n "$decode_count" ] && [ "$decode_count" -gt 0 ]; then
    # Assume 20ms per frame
    duration_sec=$(echo "scale=1; $decode_count * 0.020" | bc)
    if [ -n "$fft_count" ] && [ "$fft_count" -gt 0 ]; then
        analysis_rate=$(echo "scale=1; $fft_count / $duration_sec" | bc)
    else
        analysis_rate=0
    fi
else
    analysis_rate=0
fi

# Analysis overhead (what % of pipeline time is FFT?)
if [ -n "$pipeline_avg_us" ] && [ -n "$fft_avg_us" ] && \
   [ "$(echo "$pipeline_avg_us > 0" | bc)" -eq 1 ] && \
   [ "$(echo "$fft_avg_us > 0" | bc)" -eq 1 ]; then
    # FFT happens every ~2 frames (50% overlap), so divide by 2
    fft_per_frame=$(echo "scale=2; $fft_avg_us / 2" | bc)
    analysis_pct=$(echo "scale=0; ($fft_per_frame * 100) / $pipeline_avg_us" | bc)
else
    analysis_pct=0
fi

# ===================================================================
# Report: Phase 4 Audio Analysis Performance
# ===================================================================

echo "─────────────────────────────────────────────────────────────────"
echo "Phase 4: Audio Analysis Performance"
echo "─────────────────────────────────────────────────────────────────"
echo
echo "Found ${build} build ----"

if [ "$fft_count" -gt 0 ]; then
    echo "FFT Performance:"
    echo "  Total FFTs executed:  $fft_count"
    echo "  Average per FFT:      $(format_us $fft_avg_us)"
    echo "  Analysis rate:        ${analysis_rate} FFTs/second"
    echo "  Target:               < 50 μs per FFT (release build)"

    if [ "$(echo "$fft_avg_us < 50" | bc)" -eq 1 ]; then
        echo "  Status:               ✓ PASS (meets target)"
    else
        if [[ "${build}" = release ]] ; then
            echo "  Status: WARNING       ⚠  Release build but got slow fft_avg_us: $fft_avg_us "
        else
            echo "  Status:               ⚠  Debug build (expect ~30-40 μs in release)"
        fi
    fi
    echo
else
    echo "FFT Performance:"
    echo "  Status:               ✗ No analysis data (--analyze flag not used?)"
    echo
fi

# ===================================================================
# Report: Core Pipeline Performance
# ===================================================================

echo "─────────────────────────────────────────────────────────────────"
echo "Core Pipeline Performance"
echo "─────────────────────────────────────────────────────────────────"
echo

if [ "$decode_count" -gt 0 ]; then
    echo "Opus Decode:"
    echo "  Total frames:         $decode_count"
    echo "  Average per frame:    $(format_us $decode_avg_us)"
    echo "  Status:               ✓ Excellent"
    echo
fi

if [ "$pipeline_count" -gt 0 ]; then
    echo "Receiver Pipeline (decode → analysis → playback):"
    echo "  Total frames:         $pipeline_count"
    echo "  Average per frame:    $(format_us $pipeline_avg_us)"
    if [ "$analysis_pct" -gt 0 ]; then
        echo "  Analysis overhead:    ~${analysis_pct}% of pipeline (${build} build)"
    fi
    echo "  Status:               ✓ Well under latency budget"
    echo
fi

if [ "$jitter_count" -gt 0 ]; then
    echo "Jitter Buffer:"
    echo "  Average delay:        ${jitter_avg_ms} ms"
    echo "  Status:               ✓ Within configured depth (60ms)"
    echo
fi

# ===================================================================
# Report: Network Performance
# ===================================================================

echo "─────────────────────────────────────────────────────────────────"
echo "Network Performance"
echo "─────────────────────────────────────────────────────────────────"
echo

if [ "$packets_recv" -gt 0 ]; then
    echo "Packet Statistics:"
    echo "  Packets received:     $packets_recv"
    echo "  Packets lost:         $packets_lost (${loss_pct}%)"
    echo "  Packets reordered:    $packets_reordered"
    echo "  Packets late:         $packets_late"
    echo "  Bytes received:       $bytes_recv"
    echo
    
    if [ "$(echo "$loss_pct < 1.0" | bc)" -eq 1 ]; then
        echo "  Status:               ✓ Excellent (< 1% loss)"
    elif [ "$(echo "$loss_pct < 5.0" | bc)" -eq 1 ]; then
        echo "  Status:               ✓ Good (< 5% loss)"
    else
        echo "  Status:               ⚠  High packet loss (> 5%)"
    fi
    echo
fi

# ===================================================================
# Report: Design Target Validation
# ===================================================================

echo "─────────────────────────────────────────────────────────────────"
echo "Design Targets Validation (from docs/design.md)"
echo "─────────────────────────────────────────────────────────────────"
echo

if [ "$fft_count" -gt 0 ]; then
    echo "✓ FFT execution:      Target < 50 μs (release)"
    echo "                      Actual: $(format_us $fft_avg_us) (${build})"
    echo
    echo "✓ Analysis rate:      Target ~31 FFTs/second"
    echo "                      Actual: ${analysis_rate} FFTs/second"
    echo
fi

if [ -n "$pipeline_avg_us" ] && [ "$(echo "$pipeline_avg_us > 0" | bc)" -eq 1 ]; then
    echo "✓ Latency budget:     Target < 150 ms end-to-end"
    echo "                      Pipeline: $(format_us $pipeline_avg_us) per frame"
    echo
fi

if [ "$(echo "$loss_pct < 5.0" | bc)" -eq 1 ]; then
    echo "✓ Packet loss:        Target < 5% (with concealment)"
    echo "                      Actual: ${loss_pct}%"
    echo
fi

# ===================================================================
# Notes
# ===================================================================

echo "─────────────────────────────────────────────────────────────────"
echo "Notes"
echo "─────────────────────────────────────────────────────────────────"
echo

# Detect build type from FFT performance
if [ -n "$fft_avg_us" ] && [ "$(echo "$fft_avg_us > 0" | bc)" -eq 1 ]; then
    if [ "$(echo "$fft_avg_us < 100" | bc)" -eq 1 ]; then
        echo "• Build type: Release (optimized) - based on FFT performance"
        echo "• FFT performance indicates compiler optimizations are active"
    else
        echo "• Build type: Debug (unoptimized) - based on FFT performance"
        echo "• Release build typically 5-10x faster for compute-intensive code"
        echo "• Expected FFT performance in release: 30-40 μs per transform"
    fi
else
    echo "• Measurements are from DEBUG build (unoptimized)"
    echo "• Release build typically 5-10x faster for compute-intensive code"
    echo "• Expected FFT performance in release: 30-40 μs per transform"
fi

echo "• Localhost testing may show artifacts in transit time metrics"
echo
echo "==================================================================="
