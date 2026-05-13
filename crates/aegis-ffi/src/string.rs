//! C ABI string view helpers.

/// Borrowed UTF-8 bytes passed across the C ABI.
///
/// The caller owns the pointed-to memory and must keep it valid for the
/// duration of the ABI call. Strings are not required to be null-terminated.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AegisStringView {
    /// Pointer to UTF-8 bytes.
    pub ptr: *const u8,
    /// Length in bytes.
    pub len: usize,
}

impl AegisStringView {
    /// Return an empty string view.
    pub const fn empty() -> Self {
        Self {
            ptr: core::ptr::null(),
            len: 0,
        }
    }

    /// Create a string view from a Rust string slice.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &str) -> Self {
        Self {
            ptr: input.as_ptr(),
            len: input.len(),
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        if self.ptr.is_null() {
            return (self.len == 0).then_some("");
        }
        if self.len > isize::MAX as usize {
            return None;
        }

        // SAFETY: The ABI contract requires callers to pass a pointer valid for
        // `len` bytes for the duration of the call. Invalid UTF-8 is rejected
        // by returning `None`.
        let bytes = unsafe { core::slice::from_raw_parts(self.ptr, self.len) };
        core::str::from_utf8(bytes).ok()
    }
}
