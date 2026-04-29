//! Console flags shared by commands and cvars.

use core::ops::{BitOr, BitOrAssign};

/// Additive command/cvar flags.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConsoleFlags {
    bits: u32,
}

impl ConsoleFlags {
    /// No flags.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Persistable value hint.
    pub const ARCHIVE: Self = Self { bits: 1 << 0 };
    /// Requires host authority with cheats enabled.
    pub const CHEAT: Self = Self { bits: 1 << 1 };
    /// CVar can be read but not written.
    pub const READ_ONLY: Self = Self { bits: 1 << 2 };
    /// CVar output must mask the real value.
    pub const PROTECTED: Self = Self { bits: 1 << 3 };
    /// Hidden from list/help/completion query surfaces.
    pub const HIDDEN: Self = Self { bits: 1 << 4 };
    /// Emits a state-changed output frame after successful cvar write.
    pub const NOTIFY: Self = Self { bits: 1 << 5 };
    /// CVar writes must contain printable text only.
    pub const PRINTABLE_ONLY: Self = Self { bits: 1 << 6 };

    /// Return raw flag bits.
    pub const fn bits(self) -> u32 {
        self.bits
    }

    /// Create flags from raw bits. Unknown bits are preserved for forward compatibility.
    pub const fn from_bits_retain(bits: u32) -> Self {
        Self { bits }
    }

    /// Return whether all bits in `other` are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    /// Return whether no flags are set.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

impl BitOr for ConsoleFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl BitOrAssign for ConsoleFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}
