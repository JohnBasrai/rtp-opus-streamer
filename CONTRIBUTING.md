# Contributing to RTP Opus Streamer

Thanks for considering contributing!

Before submitting a pull request:

- Ensure all tests pass (`./scripts/test-all.sh`)
- Format your code (`cargo fmt`)
- Run clippy (`cargo clippy -- -D warnings`)
- If your change affects behavior, please update `CHANGELOG.md` under the [Unreleased] section
- Keep commits focused and descriptive

We follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/).

## Code Formatting

This project uses `rustfmt` for consistent code formatting. All code should be formatted before committing.

### Quick Commands
```bash
# Format all code
cargo fmt

# Check if code is formatted (used by CI)
cargo fmt --check

# Run the complete test suite (matches CI exactly)
./scripts/test-all.sh

# Run CI locally (requires 'act')
./scripts/ci-local.sh
```

### Visual Separators

Since `rustfmt` removes blank lines at the start of impl blocks, function bodies, and module blocks, we use comment separators for visual clarity:

```rust
// Module blocks
mod helpers {
    // ---
    use super::*;
    
    pub fn some_function() {
        // ---
        // function body
    }
}

// Struct definitions
pub struct RtpPacket {
    // ---
    pub sequence: u16,
    pub timestamp: u32,
    pub payload: Vec<u8>,
}

// Impl blocks
impl RtpPacket {
    // ---
    pub fn new() -> Self {
        // ---
        Self {
            sequence: 0,
            timestamp: 0,
            payload: Vec::new(),
        }
    }
}

// Regular functions
pub fn encode_audio() {
    // ---
    let encoder = OpusEncoder::new()
        .expect("failed to create encoder");
    // ...
}

// Struct literals (construction) - NO separator
let packet = RtpPacket {
    sequence: 1,
    timestamp: 320,
    payload: vec![1, 2, 3],
};

// Test modules
#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn test_something() {
        // ---
        // test body
    }
}
```

**Style Guidelines:**
1) Use `// ---` for visual separation in at a minimum **module blocks**, **impl blocks**, **struct definitions**, and **function bodies**
2) Place separators after the opening brace and before the first meaningful line
3) Between meaningful steps of logic processing (e.g., separating validation, encoding, and transmission)
4) For modules: place separator after `mod name {` and before imports/content
5) For impl blocks: place separator after `impl ... {` and before the first method
6) For struct definitions: place separator after `struct Name {` and before field declarations
7) For functions: place separator after function signature and before the main logic
8) Do NOT use separators inside struct literals (during construction)
9) Keep separators consistent across the codebase

**Note:** This project uses rustfmt's default configuration. The `// ---` separator pattern is a formatting convention to work around rustfmt's blank line removal in stable Rust.

## Documentation and Doc Comments

This project follows a **production-grade documentation standard** for Rust code.

### Required Doc Comments

Use Rust doc comments (`///`) for:

- Public structs
- Public enums
- Public functions
- Public modules that define architectural boundaries
- Core algorithms and protocol implementations

Doc comments should describe **intent, guarantees, and failure semantics** —
not restate what the code obviously does.

### Optional (Encouraged) Doc Comments

Doc comments or short block comments are encouraged for:

- Internal functions with performance or correctness implications
- Startup and initialization logic
- Configuration parsing and validation
- Code that enforces invariants or protocol requirements
- Network protocol implementations

### Not Required

Doc comments are not required for:

- Trivial helpers
- Simple getters or pass-through functions
- Test code (assert messages should be sufficient)
- Obvious glue code

### General Guidance

- Prefer documenting *why* over *how*
- Be explicit about failure behavior
- Keep comments accurate and up to date
- Avoid over-documenting trivial code
- Include examples in doc comments where helpful

Well-written doc comments are considered part of the code's correctness.

## Testing Guidelines

### Test Scripts

Test scripts in `scripts/` match the CI workflow exactly:

- **`test-all.sh`**: Runs formatting, clippy, build, and all tests (same as CI)
- **`ci-local.sh`**: Runs GitHub Actions locally using [act](https://github.com/nektos/act)

### Integration Tests

Integration tests should be placed in the `tests/` directory with the naming convention:
```
tests/test_<feature_name>.rs
```

Examples:
- `tests/test_core_pipeline.rs`
- `tests/test_network_resilience.rs`
- `tests/test_observability.rs`

### Unit Tests

Unit tests should be co-located with the code being tested using the standard `#[cfg(test)]` module pattern.

**Note:** This project uses binary crates (`sender` and `receiver`), so unit tests are embedded within the binary source files rather than in separate `lib.rs` files.

### Running Tests

```bash
# All tests (same as CI)
./scripts/test-all.sh

# Or manually:
cargo test --all

# Specific integration test
cargo test --test test_core_pipeline

# With logging output
RUST_LOG=debug cargo test -- --nocapture
```

## Local CI Testing

To verify your changes will pass CI before pushing:

```bash
# Install act (one-time setup)
# macOS:
brew install act

# Linux:
# See https://github.com/nektos/act#installation

# Run CI locally
./scripts/ci-local.sh
```

This runs the exact same workflow as GitHub Actions, catching issues before you push.
