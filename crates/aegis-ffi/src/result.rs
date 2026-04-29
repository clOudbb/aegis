//! Opaque execution result handle and output accessors.

use aegis_core::error::AegisError;
use aegis_core::executor::{ExecutionResult, ExecutionStatus};
use aegis_core::output::{OutputChannel, OutputFrame, OutputFrameKind};
use aegis_core::script::ScriptExecutionResult;

use crate::error::code_from_core_error;
use crate::string::AegisStringView;

/// Unknown output channel code.
pub const AEGIS_OUTPUT_CHANNEL_UNKNOWN: u32 = 0;
/// Main output channel code.
pub const AEGIS_OUTPUT_CHANNEL_MAIN: u32 = 1;
/// Debug output channel code.
pub const AEGIS_OUTPUT_CHANNEL_DEBUG: u32 = 2;
/// System output channel code.
pub const AEGIS_OUTPUT_CHANNEL_SYSTEM: u32 = 3;

/// Unknown output kind code.
pub const AEGIS_OUTPUT_KIND_UNKNOWN: u32 = 0;
/// Text output kind code.
pub const AEGIS_OUTPUT_KIND_TEXT: u32 = 1;
/// JSON output kind code.
pub const AEGIS_OUTPUT_KIND_JSON: u32 = 2;
/// Table output kind code.
pub const AEGIS_OUTPUT_KIND_TABLE: u32 = 3;
/// Log output kind code.
pub const AEGIS_OUTPUT_KIND_LOG: u32 = 4;
/// Warning output kind code.
pub const AEGIS_OUTPUT_KIND_WARNING: u32 = 5;
/// Error output kind code.
pub const AEGIS_OUTPUT_KIND_ERROR: u32 = 6;
/// Progress output kind code.
pub const AEGIS_OUTPUT_KIND_PROGRESS: u32 = 7;
/// State changed output kind code.
pub const AEGIS_OUTPUT_KIND_STATE_CHANGED: u32 = 8;
/// Diagnostic output kind code.
pub const AEGIS_OUTPUT_KIND_DIAGNOSTIC: u32 = 9;

/// Unknown execution status code.
pub const AEGIS_EXECUTION_STATUS_UNKNOWN: u32 = 0;
/// Successful execution status code.
pub const AEGIS_EXECUTION_STATUS_SUCCESS: u32 = 1;
/// Failed execution status code.
pub const AEGIS_EXECUTION_STATUS_FAILED: u32 = 2;
/// Blocked execution status code.
pub const AEGIS_EXECUTION_STATUS_BLOCKED: u32 = 3;

/// Opaque execution result handle used by C ABI consumers.
pub struct AegisExecutionResultHandle {
    status_code: u32,
    error_code: u32,
    frames: Vec<OutputFrame>,
}

impl AegisExecutionResultHandle {
    pub(crate) fn from_execution_result(result: ExecutionResult) -> Self {
        let error_code = result.error().map_or(0, code_from_core_error);
        Self {
            status_code: status_code_from_execution_status(result.status()),
            error_code,
            frames: result.into_frames(),
        }
    }

    pub(crate) fn from_script_result(result: ScriptExecutionResult) -> Self {
        let status_code = if result.is_blocked() {
            AEGIS_EXECUTION_STATUS_BLOCKED
        } else if result.is_failed() {
            AEGIS_EXECUTION_STATUS_FAILED
        } else {
            AEGIS_EXECUTION_STATUS_SUCCESS
        };

        let mut frames = Vec::new();
        for command_result in result.command_results() {
            frames.extend(command_result.frames().iter().cloned());
        }
        for error in result.errors() {
            frames.push(OutputFrame::error(error.message()).with_sequence(frames.len() as u64 + 1));
        }
        for diagnostic in result.diagnostics() {
            frames.push(
                OutputFrame::diagnostic(diagnostic.message())
                    .with_sequence(frames.len() as u64 + 1),
            );
        }

        Self {
            status_code,
            error_code: result
                .errors()
                .first()
                .map_or(0, crate::error::code_from_core_error),
            frames,
        }
    }

    pub(crate) fn from_error(error: AegisError) -> Self {
        Self {
            status_code: AEGIS_EXECUTION_STATUS_FAILED,
            error_code: code_from_core_error(&error),
            frames: vec![OutputFrame::error(error.message()).with_sequence(1)],
        }
    }

    pub(crate) fn status_code(&self) -> u32 {
        self.status_code
    }

    pub(crate) fn error_code(&self) -> u32 {
        self.error_code
    }

    pub(crate) fn output_count(&self) -> usize {
        self.frames.len()
    }

    pub(crate) fn output_command_id_at(&self, index: usize) -> u64 {
        self.frames
            .get(index)
            .map_or(0, aegis_core::output::OutputFrame::command_id)
    }

    pub(crate) fn output_sequence_at(&self, index: usize) -> u64 {
        self.frames
            .get(index)
            .map_or(0, aegis_core::output::OutputFrame::sequence)
    }

    pub(crate) fn output_channel_at(&self, index: usize) -> u32 {
        self.frames
            .get(index)
            .map_or(AEGIS_OUTPUT_CHANNEL_UNKNOWN, |frame| {
                output_channel_code(frame.channel())
            })
    }

    pub(crate) fn output_kind_at(&self, index: usize) -> u32 {
        self.frames
            .get(index)
            .map_or(AEGIS_OUTPUT_KIND_UNKNOWN, |frame| {
                output_kind_code(frame.kind())
            })
    }

    pub(crate) fn output_payload_at(&self, index: usize) -> AegisStringView {
        self.frames
            .get(index)
            .map_or_else(AegisStringView::empty, |frame| {
                AegisStringView::from_str(frame.payload())
            })
    }
}

fn status_code_from_execution_status(status: ExecutionStatus) -> u32 {
    match status {
        ExecutionStatus::Success => AEGIS_EXECUTION_STATUS_SUCCESS,
        ExecutionStatus::Failed => AEGIS_EXECUTION_STATUS_FAILED,
        ExecutionStatus::Blocked => AEGIS_EXECUTION_STATUS_BLOCKED,
    }
}

fn output_channel_code(channel: OutputChannel) -> u32 {
    match channel {
        OutputChannel::Main => AEGIS_OUTPUT_CHANNEL_MAIN,
        OutputChannel::Debug => AEGIS_OUTPUT_CHANNEL_DEBUG,
        OutputChannel::System => AEGIS_OUTPUT_CHANNEL_SYSTEM,
        _ => AEGIS_OUTPUT_CHANNEL_UNKNOWN,
    }
}

fn output_kind_code(kind: OutputFrameKind) -> u32 {
    match kind {
        OutputFrameKind::Text => AEGIS_OUTPUT_KIND_TEXT,
        OutputFrameKind::Json => AEGIS_OUTPUT_KIND_JSON,
        OutputFrameKind::Table => AEGIS_OUTPUT_KIND_TABLE,
        OutputFrameKind::Log => AEGIS_OUTPUT_KIND_LOG,
        OutputFrameKind::Warning => AEGIS_OUTPUT_KIND_WARNING,
        OutputFrameKind::Error => AEGIS_OUTPUT_KIND_ERROR,
        OutputFrameKind::Progress => AEGIS_OUTPUT_KIND_PROGRESS,
        OutputFrameKind::StateChanged => AEGIS_OUTPUT_KIND_STATE_CHANGED,
        OutputFrameKind::Diagnostic => AEGIS_OUTPUT_KIND_DIAGNOSTIC,
        _ => AEGIS_OUTPUT_KIND_UNKNOWN,
    }
}
