//! Stable C ABI error codes.

use aegis_core::error::{AegisError, AegisErrorCode};

/// Successful ABI call.
pub const AEGIS_OK: u32 = 0;
/// Input could not be parsed.
pub const AEGIS_ERROR_PARSE: u32 = 100;
/// Registry operation failed.
pub const AEGIS_ERROR_REGISTRY: u32 = 200;
/// Command lookup failed.
pub const AEGIS_ERROR_COMMAND_NOT_FOUND: u32 = 300;
/// ABI or command argument was invalid.
pub const AEGIS_ERROR_INVALID_ARGUMENT: u32 = 400;
/// Permission policy denied execution.
pub const AEGIS_ERROR_PERMISSION_DENIED: u32 = 500;
/// Operation was cancelled cooperatively.
pub const AEGIS_ERROR_CANCELLED: u32 = 600;
/// Operation timed out cooperatively.
pub const AEGIS_ERROR_TIMEOUT: u32 = 700;
/// Script execution failed.
pub const AEGIS_ERROR_SCRIPT: u32 = 800;
/// Plugin operation failed.
pub const AEGIS_ERROR_PLUGIN: u32 = 900;
/// Internal invariant failed.
pub const AEGIS_ERROR_INTERNAL: u32 = 1_000;
/// FFI boundary operation failed.
pub const AEGIS_ERROR_FFI: u32 = 1_100;
/// Panic was caught at an FFI boundary.
pub const AEGIS_ERROR_PANIC: u32 = 10_000;

pub(crate) fn code_from_core_error(error: &AegisError) -> u32 {
    match error.code() {
        AegisErrorCode::ParseError => AEGIS_ERROR_PARSE,
        AegisErrorCode::RegistryError => AEGIS_ERROR_REGISTRY,
        AegisErrorCode::CommandNotFound => AEGIS_ERROR_COMMAND_NOT_FOUND,
        AegisErrorCode::InvalidArgument => AEGIS_ERROR_INVALID_ARGUMENT,
        AegisErrorCode::PermissionDenied => AEGIS_ERROR_PERMISSION_DENIED,
        AegisErrorCode::Cancelled => AEGIS_ERROR_CANCELLED,
        AegisErrorCode::Timeout => AEGIS_ERROR_TIMEOUT,
        AegisErrorCode::ScriptError => AEGIS_ERROR_SCRIPT,
        AegisErrorCode::PluginError => AEGIS_ERROR_PLUGIN,
        AegisErrorCode::InternalError => AEGIS_ERROR_INTERNAL,
        AegisErrorCode::FfiError => AEGIS_ERROR_FFI,
        _ => AEGIS_ERROR_INTERNAL,
    }
}
