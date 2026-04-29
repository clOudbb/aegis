//! Execution authority supplied by the host.

/// Host-controlled execution authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionAuthority {
    cheats_enabled: bool,
}

impl ExecutionAuthority {
    /// Create authority with cheats disabled.
    pub const fn new() -> Self {
        Self {
            cheats_enabled: false,
        }
    }

    /// Create authority with explicit cheat mode.
    pub const fn with_cheats_enabled(cheats_enabled: bool) -> Self {
        Self { cheats_enabled }
    }

    /// Return whether cheat-protected commands and writes are allowed.
    pub const fn cheats_enabled(self) -> bool {
        self.cheats_enabled
    }
}

impl Default for ExecutionAuthority {
    fn default() -> Self {
        Self::new()
    }
}
