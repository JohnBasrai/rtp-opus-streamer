# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
- None

### Fixed
- None

## [0.1.0] - 2024-12-31

### Added
- Initial project structure
- Workspace configuration with sender and receiver binaries
- Architecture documentation
- Design document with technical rationale
- Four-phase implementation roadmap
