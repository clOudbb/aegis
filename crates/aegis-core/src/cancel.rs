//! Cooperative cancellation primitives.
//!
//! Cancellation in Aegis is cooperative. The core checks this token at script
//! boundaries and host command handlers may also inspect cloned tokens in
//! future integrations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared cooperative cancellation token.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a token in the active state.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Return whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
