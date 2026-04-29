//! Structured error model.
//!
//! Errors in this module are designed to be useful for Rust callers and
//! directly mappable to future C ABI error codes.

/// Result alias for Aegis core operations.
pub type Result<T> = core::result::Result<T, AegisError>;

/// Stable core error code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AegisErrorCode {
    /// Input could not be parsed.
    ParseError,
    /// Registry operation failed.
    RegistryError,
    /// Command lookup failed.
    CommandNotFound,
    /// Command arguments were invalid.
    InvalidArgument,
    /// Permission policy denied execution.
    PermissionDenied,
    /// Operation was cancelled cooperatively.
    Cancelled,
    /// Operation timed out cooperatively.
    Timeout,
    /// Script execution failed.
    ScriptError,
    /// Plugin operation failed.
    PluginError,
    /// Internal invariant failed.
    InternalError,
    /// FFI boundary operation failed.
    FfiError,
}

impl AegisErrorCode {
    /// Return the stable numeric code for this error.
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::ParseError => 100,
            Self::RegistryError => 200,
            Self::CommandNotFound => 300,
            Self::InvalidArgument => 400,
            Self::PermissionDenied => 500,
            Self::Cancelled => 600,
            Self::Timeout => 700,
            Self::ScriptError => 800,
            Self::PluginError => 900,
            Self::InternalError => 1_000,
            Self::FfiError => 1_100,
        }
    }
}

/// Structured core error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AegisError {
    code: AegisErrorCode,
    message: String,
}

impl AegisError {
    /// Create a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::ParseError, message)
    }

    /// Create a registry error.
    pub fn registry(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::RegistryError, message)
    }

    /// Create a command-not-found error.
    pub fn command_not_found(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::CommandNotFound, message)
    }

    /// Create an invalid-argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::InvalidArgument, message)
    }

    /// Create a permission-denied error.
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::PermissionDenied, message)
    }

    /// Create a cancelled error.
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::Cancelled, message)
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::Timeout, message)
    }

    /// Create a script error.
    pub fn script(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::ScriptError, message)
    }

    /// Create a plugin error.
    pub fn plugin(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::PluginError, message)
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::InternalError, message)
    }

    /// Create an FFI boundary error.
    pub fn ffi(message: impl Into<String>) -> Self {
        Self::new(AegisErrorCode::FfiError, message)
    }

    /// Create an error from an explicit code.
    pub fn new(code: AegisErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Return the structured error code.
    pub const fn code(&self) -> AegisErrorCode {
        self.code
    }

    /// Return the human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl core::fmt::Display for AegisError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AegisError {}
