# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-01-02

**Phase 3: Observability - Metrics and Measurement Infrastructure**

  * **Added**
    * Prometheus metrics for sender and receiver
    * Shared `common::observability` layer
    * Metrics HTTP endpoint for long-lived processes
    * Manual sender/receiver test script
  * **Changed**
    * Standardized CLI color handling
    * Clarified sender/receiver lifecycle as long-running processes
  * **Internal**
    * Enforced EMBP boundaries via `common/lib.rs`
    * CI and CLI ergonomics cleanup (post-merge)

_No “Unreleased” leftovers after this._

## [0.2.0] - 2024-12-31

### Added

**Phase 2: Network Resilience**
- Common RTP library crate (eliminates code duplication)
- Jitter buffer with configurable depth (default 60ms)
- Packet reordering based on sequence numbers
- Statistics tracking (loss rate, jitter, reordering events)
- Enhanced packet loss concealment using Opus PLC
- Sender and receiver restructured as libraries for testability
- Network simulator for integration testing
- End-to-end integration tests with simulated network conditions
- Late packet detection and handling

**Phase 1: Core Pipeline**
- WAV file reading with automatic resampling to 16kHz mono
- Opus audio codec integration (24 kbps, 20ms frames)
- RTP packet structure (RFC 3550 compliant)
- UDP network transmission with async I/O
- Audio playback via cpal with real-time streaming
- Integration tests in receiver/tests/ (codec and protocol validation)
- Comprehensive error handling with resilient operation
- Full documentation following production standards

**Infrastructure**
- GitHub Actions CI workflow
- CONTRIBUTING.md with code style guidelines
- Visual separator formatting convention
- Test script matching CI workflow (`test-all.sh`)
- Local CI testing with `ci-local.sh` (requires act)

### Changed
- Sender and receiver now expose library APIs
- RTP packet code moved to common crate
- Receiver binary path changed to src/bin/receiver.rs
- Sender binary path changed to src/bin/sender.rs

### Fixed
- None

## [0.1.0] - 2024-12-31

### Added
- Initial project structure
- Workspace configuration with sender and receiver binaries
- Architecture documentation
- Design document with technical rationale
- Four-phase implementation roadmap
