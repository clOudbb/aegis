//! Execution context and output buffering.

use crate::output::OutputFrame;
use crate::sink::OutputSink;

/// Context passed to command handlers.
pub struct ExecutionContext {
    command_id: u64,
    sequence: u64,
    frames: Vec<OutputFrame>,
    sinks: Vec<OutputSink>,
}

impl ExecutionContext {
    /// Create an execution context for a command invocation.
    pub fn new(command_id: u64, sinks: Vec<OutputSink>) -> Self {
        Self {
            command_id,
            sequence: 0,
            frames: Vec::new(),
            sinks,
        }
    }

    /// Write a structured output frame.
    pub fn write_frame(&mut self, frame: OutputFrame) {
        self.sequence += 1;
        let frame = frame
            .with_command_id(self.command_id)
            .with_sequence(self.sequence);
        self.frames.push(frame.clone());

        for sink in &self.sinks {
            if let Err(error) = sink.dispatch(&frame) {
                self.sequence += 1;
                self.frames.push(
                    OutputFrame::diagnostic(error.message())
                        .with_command_id(self.command_id)
                        .with_sequence(self.sequence),
                );
            }
        }
    }

    /// Write text output.
    pub fn write_text(&mut self, payload: impl Into<String>) {
        self.write_frame(OutputFrame::text(payload));
    }

    /// Write error output.
    pub fn write_error(&mut self, payload: impl Into<String>) {
        self.write_frame(OutputFrame::error(payload));
    }

    /// Write warning output.
    pub fn write_warning(&mut self, payload: impl Into<String>) {
        self.write_frame(OutputFrame::warning(payload));
    }

    /// Consume the context and return collected frames.
    pub fn into_frames(self) -> Vec<OutputFrame> {
        self.frames
    }
}
