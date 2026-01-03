//! Standardized CLI color policy.
//!
//! Mirrors common Rust tool behavior:
//! - Command-line choice wins.
//! - `NO_COLOR` disables colors.
//! - `EMACS` implies non-interactive output.
//! - In `Auto`, only enable colors when the target stream is a TTY.

use std::env;

/// When to emit ANSI colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorWhen {
    /// Enable colors only when output is a TTY.
    Auto,

    /// Always enable colors.
    Always,

    /// Never enable colors.
    Never,
}

impl ColorWhen {
    // ---
    /// Returns whether ANSI color should be used for the given stream.
    pub fn should_color(self, stream: atty::Stream) -> bool {
        match self {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => {
                if env::var_os("NO_COLOR").is_some() {
                    return false;
                }

                if env::var_os("EMACS").is_some() {
                    return false;
                }

                atty::is(stream)
            }
        }
    }

    /// Convenience helper for stdout.
    pub fn should_color_stdout(self) -> bool {
        self.should_color(atty::Stream::Stdout)
    }

    /// Convenience helper for stderr.
    pub fn should_color_stderr(self) -> bool {
        self.should_color(atty::Stream::Stderr)
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    fn with_env_var<K: AsRef<str>, V: AsRef<str>, F: FnOnce()>(key: K, val: V, f: F) {
        // ---
        let key = key.as_ref();
        let prev = env::var_os(key);
        env::set_var(key, val.as_ref());
        f();
        match prev {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn no_color_disables_in_auto() {
        // ---
        with_env_var("NO_COLOR", "1", || {
            assert!(!ColorWhen::Auto.should_color_stdout());
            assert!(!ColorWhen::Auto.should_color_stderr());
        });
    }

    #[test]
    fn emacs_disables_in_auto() {
        // ---
        with_env_var("EMACS", "t", || {
            assert!(!ColorWhen::Auto.should_color_stdout());
            assert!(!ColorWhen::Auto.should_color_stderr());
        });
    }

    #[test]
    fn explicit_always_wins() {
        // ---
        with_env_var("NO_COLOR", "1", || {
            assert!(ColorWhen::Always.should_color_stdout());
        });
    }

    #[test]
    fn explicit_never_wins() {
        // ---
        with_env_var("NO_COLOR", "", || {
            assert!(!ColorWhen::Never.should_color_stdout());
        });
    }
}
