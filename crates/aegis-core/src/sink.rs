//! Output sink support.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::error::{AegisError, Result};
use crate::output::OutputFrame;

type SinkHandler = dyn Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static;

/// Synchronous output sink callback.
#[derive(Clone)]
pub struct OutputSink {
    handler: Arc<SinkHandler>,
}

impl OutputSink {
    /// Create an output sink from a callback.
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
        }
    }

    /// Observe a frame.
    pub fn dispatch(&self, frame: &OutputFrame) -> Result<()> {
        match catch_unwind(AssertUnwindSafe(|| (self.handler)(frame))) {
            Ok(result) => result,
            Err(_) => Err(AegisError::internal("output sink panicked")),
        }
    }
}
