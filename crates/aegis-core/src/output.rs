//! Structured command output.
//!
//! Output frames are semantic records emitted by commands, scripts, and core
//! services. UI layers decide how to render them.

/// Stable output frame schema version.
pub const OUTPUT_SCHEMA_VERSION: u16 = 1;

/// Semantic output kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OutputFrameKind {
    /// Human-readable text.
    Text,
    /// UTF-8 JSON payload.
    Json,
    /// Tabular data payload.
    Table,
    /// Log payload.
    Log,
    /// Warning payload.
    Warning,
    /// Error payload.
    Error,
    /// Progress payload.
    Progress,
    /// State change payload.
    StateChanged,
    /// Diagnostic payload.
    Diagnostic,
}

/// Logical output channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OutputChannel {
    /// Primary user-facing output.
    Main,
    /// Diagnostic or developer-facing output.
    Debug,
    /// Core or host system output.
    System,
}

/// A structured output record emitted by Aegis core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputFrame {
    schema_version: u16,
    kind: OutputFrameKind,
    channel: OutputChannel,
    command_id: u64,
    sequence: u64,
    payload: String,
}

impl OutputFrame {
    /// Create a frame with explicit kind and channel.
    pub fn new(kind: OutputFrameKind, channel: OutputChannel, payload: impl Into<String>) -> Self {
        Self {
            schema_version: OUTPUT_SCHEMA_VERSION,
            kind,
            channel,
            command_id: 0,
            sequence: 0,
            payload: payload.into(),
        }
    }

    /// Create a main-channel text frame.
    pub fn text(payload: impl Into<String>) -> Self {
        Self::new(OutputFrameKind::Text, OutputChannel::Main, payload)
    }

    /// Create a main-channel warning frame.
    pub fn warning(payload: impl Into<String>) -> Self {
        Self::new(OutputFrameKind::Warning, OutputChannel::Main, payload)
    }

    /// Create a main-channel error frame.
    pub fn error(payload: impl Into<String>) -> Self {
        Self::new(OutputFrameKind::Error, OutputChannel::Main, payload)
    }

    /// Create a system diagnostic frame.
    pub fn diagnostic(payload: impl Into<String>) -> Self {
        Self::new(OutputFrameKind::Diagnostic, OutputChannel::System, payload)
    }

    /// Create a main-channel state-changed frame.
    pub fn state_changed(payload: impl Into<String>) -> Self {
        Self::new(OutputFrameKind::StateChanged, OutputChannel::Main, payload)
    }

    /// Return the schema version for this frame.
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Return the semantic kind for this frame.
    pub const fn kind(&self) -> OutputFrameKind {
        self.kind
    }

    /// Return the logical channel for this frame.
    pub const fn channel(&self) -> OutputChannel {
        self.channel
    }

    /// Return the command invocation id associated with this frame.
    pub const fn command_id(&self) -> u64 {
        self.command_id
    }

    /// Return the sequence number for this frame.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Return the UTF-8 payload for this frame.
    pub fn payload(&self) -> &str {
        &self.payload
    }

    /// Return this frame with a command invocation id assigned.
    pub const fn with_command_id(mut self, command_id: u64) -> Self {
        self.command_id = command_id;
        self
    }

    /// Return this frame with a stream sequence assigned.
    pub const fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }
}
