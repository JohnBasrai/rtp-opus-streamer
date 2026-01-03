#!/usr/bin/env bash
#
# Manual end-to-end sender/receiver test.
#
# This script is NOT run in CI.
# It is intended for developers to manually verify:
#   - sender and receiver start correctly
#   - RTP audio flows end-to-end
#   - Prometheus metrics endpoints are exposed
#
# Usage:
#   ./scripts/test-sender-receiver.sh [DURATION_SECONDS]
#
# If DURATION_SECONDS is omitted, defaults to 15 seconds.
# Ctrl-C will interrupt the run and trigger final metrics scrape.
#

set -euo pipefail

# Defaults
DURATION="${1:-15}"
SENDER_METRICS_ADDR="127.0.0.1:9100"
RECEIVER_METRICS_ADDR="127.0.0.1:9200"
RTP_PORT="5004"
AUDIO_FILE="${AUDIO_FILE:-samples/test.wav}"

echo "=== Manual sender/receiver system test ==="
echo
echo "Audio file        : ${AUDIO_FILE}"
echo "RTP port          : ${RTP_PORT}"
echo "Sender metrics    : ${SENDER_METRICS_ADDR}"
echo "Receiver metrics  : ${RECEIVER_METRICS_ADDR}"
echo "Duration (sec)    : ${DURATION}"
echo

if [[ ! -f "${AUDIO_FILE}" ]]; then
    echo "ERROR: Audio file not found: ${AUDIO_FILE}"
    exit 1
fi

scrape_metrics() {
    echo
    echo "=== Metrics snapshot ==="

    echo
    echo "Sender metrics:"
    curl -sf "http://${SENDER_METRICS_ADDR}/metrics" | head -n 15 || true

    echo
    echo "Receiver metrics:"
    curl -sf "http://${RECEIVER_METRICS_ADDR}/metrics" | head -n 15 || true
}

cleanup() {
    echo
    echo "Stopping sender and receiver..."

    scrape_metrics

    kill "${SENDER_PID}" "${RECEIVER_PID}" 2>/dev/null || true
    echo "Exiting..."
}
trap cleanup EXIT

echo "Starting receiver..."
./target/debug/receiver \
    --port "${RTP_PORT}" \
    --metrics-bind "${RECEIVER_METRICS_ADDR}" &
RECEIVER_PID=$!

sleep 1

echo "Starting sender..."
./target/debug/sender \
    --input "${AUDIO_FILE}" \
    --remote "127.0.0.1:${RTP_PORT}" \
    --metrics-bind "${SENDER_METRICS_ADDR}" &
SENDER_PID=$!

sleep 2

# Initial sanity scrape
scrape_metrics

echo
echo "System running. Listening for audio..."
echo "Sleeping for ${DURATION} seconds (Ctrl-C to stop early)."
echo

sleep "${DURATION}"
