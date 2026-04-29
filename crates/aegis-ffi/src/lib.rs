//! C ABI facade for Aegis.
//!
//! This crate owns all C-compatible exported symbols and ABI-facing types.
//! The safe Rust implementation lives in `aegis-core`.
//! Phase 0 exports only ABI version metadata and uses no unsafe blocks.
//! Future raw-pointer code must document each unsafe block with `SAFETY:`
//! invariants and boundary tests.
//!
//! # FFI behavior
//!
//! ABI structs use `#[repr(C)]`, include a `size` field, and avoid Rust-owned
//! containers such as `String`, `Vec`, `Result`, and `Option`.

#![deny(missing_docs)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use aegis_core::CORE_API_VERSION;
use aegis_core::script::{ScriptOptions, ScriptRunner};

pub mod error;
pub mod handle;
pub mod register;
pub mod result;
pub mod string;

pub use register::{
    AegisCommandCallback, AegisExecutionContextHandle, AegisPluginHandle, aegis_context_write_text,
    aegis_plugin_release, aegis_register_command, aegis_register_cvar, aegis_register_plugin,
};

use crate::error::{
    AEGIS_ERROR_INTERNAL, AEGIS_ERROR_INVALID_ARGUMENT, AEGIS_ERROR_PANIC, AEGIS_OK,
};
use crate::handle::AegisCoreHandle;
use crate::result::AegisExecutionResultHandle;
use crate::string::AegisStringView;

/// Current Aegis C ABI major version.
pub const AEGIS_ABI_MAJOR: u16 = 0;

/// Current Aegis C ABI minor version.
pub const AEGIS_ABI_MINOR: u16 = 1;

/// Current Aegis C ABI patch version.
pub const AEGIS_ABI_PATCH: u16 = 0;

/// C-compatible ABI version record returned by `aegis_abi_version`.
///
/// `size` is included so future callers can detect struct layout changes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AegisAbiVersion {
    /// Size of this struct in bytes.
    pub size: usize,
    /// Breaking ABI version.
    pub major: u16,
    /// Backward-compatible ABI feature version.
    pub minor: u16,
    /// Patch version for ABI-preserving fixes.
    pub patch: u16,
    /// Safe Rust core API version used by this FFI facade.
    pub core_api_version: u32,
}

impl AegisAbiVersion {
    /// Current ABI version descriptor.
    pub const CURRENT: Self = Self {
        size: core::mem::size_of::<Self>(),
        major: AEGIS_ABI_MAJOR,
        minor: AEGIS_ABI_MINOR,
        patch: AEGIS_ABI_PATCH,
        core_api_version: CORE_API_VERSION,
    };
}

/// Return the C ABI version supported by this library.
///
/// This function is exported for C-compatible consumers. It does not allocate,
/// does not retain caller-owned memory, and is safe to call from any thread.
#[unsafe(no_mangle)]
pub extern "C" fn aegis_abi_version() -> AegisAbiVersion {
    AegisAbiVersion::CURRENT
}

/// Create a core handle with builtin commands registered.
#[unsafe(no_mangle)]
pub extern "C" fn aegis_core_create() -> *mut AegisCoreHandle {
    match catch_unwind(|| Box::into_raw(Box::new(AegisCoreHandle::new()))) {
        Ok(handle) => handle,
        Err(_) => core::ptr::null_mut(),
    }
}

/// Release a core handle previously returned by `aegis_core_create`.
///
/// # Safety
///
/// `handle` must be null or a live pointer returned by `aegis_core_create`.
/// Passing any other pointer or releasing the same handle more than once is
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_core_release(handle: *mut AegisCoreHandle) {
    if handle.is_null() {
        return;
    }

    // SAFETY: `handle` must be a pointer returned by `aegis_core_create` and
    // must not have been released already. Null is handled above.
    unsafe {
        drop(Box::from_raw(handle));
    }
}

/// Execute one command line through a core handle.
///
/// # Safety
///
/// `core` must be null or a live pointer returned by `aegis_core_create`.
/// `out_result` must be a valid writable pointer to one result handle pointer.
/// `input` must point to memory valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_execute_line(
    core: *mut AegisCoreHandle,
    input: AegisStringView,
    out_result: *mut *mut AegisExecutionResultHandle,
) -> u32 {
    catch_ffi_boundary(|| execute_line_impl(core, input, out_result))
}

/// Execute one script through a core handle using default script options.
///
/// # Safety
///
/// `core` must be null or a live pointer returned by `aegis_core_create`.
/// `out_result` must be a valid writable pointer to one result handle pointer.
/// `source_name` and `script` must point to memory valid for the duration of
/// the call. `options_ptr` must be null in this ABI version.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_execute_script(
    core: *mut AegisCoreHandle,
    source_name: AegisStringView,
    script: AegisStringView,
    options_ptr: *const core::ffi::c_void,
    out_result: *mut *mut AegisExecutionResultHandle,
) -> u32 {
    catch_ffi_boundary(|| execute_script_impl(core, source_name, script, options_ptr, out_result))
}

/// Release an execution result handle returned by an execute function.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function. Passing any other pointer or releasing the same handle more than
/// once is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_release(result: *mut AegisExecutionResultHandle) {
    if result.is_null() {
        return;
    }

    // SAFETY: `result` must be a pointer returned by an Aegis execute function
    // and must not have been released already. Null is handled above.
    unsafe {
        drop(Box::from_raw(result));
    }
}

/// Return the result status code.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_status_code(
    result: *const AegisExecutionResultHandle,
) -> u32 {
    with_result(result, 0, AegisExecutionResultHandle::status_code)
}

/// Return the result error code.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_error_code(result: *const AegisExecutionResultHandle) -> u32 {
    with_result(result, 0, AegisExecutionResultHandle::error_code)
}

/// Return the number of output frames retained by a result handle.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_count(
    result: *const AegisExecutionResultHandle,
) -> usize {
    with_result(result, 0, AegisExecutionResultHandle::output_count)
}

/// Return the command id for an output frame by index.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_command_id_at(
    result: *const AegisExecutionResultHandle,
    index: usize,
) -> u64 {
    with_result(result, 0, |result| result.output_command_id_at(index))
}

/// Return the sequence number for an output frame by index.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_sequence_at(
    result: *const AegisExecutionResultHandle,
    index: usize,
) -> u64 {
    with_result(result, 0, |result| result.output_sequence_at(index))
}

/// Return the output channel code for an output frame by index.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_channel_at(
    result: *const AegisExecutionResultHandle,
    index: usize,
) -> u32 {
    with_result(result, 0, |result| result.output_channel_at(index))
}

/// Return the output kind code for an output frame by index.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_kind_at(
    result: *const AegisExecutionResultHandle,
    index: usize,
) -> u32 {
    with_result(result, 0, |result| result.output_kind_at(index))
}

/// Return the output payload view for an output frame by index.
///
/// # Safety
///
/// `result` must be null or a live pointer returned by an Aegis execute
/// function. The returned view is valid only until the result handle is
/// released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_result_output_payload_at(
    result: *const AegisExecutionResultHandle,
    index: usize,
) -> AegisStringView {
    with_result(result, AegisStringView::empty(), |result| {
        result.output_payload_at(index)
    })
}

fn execute_line_impl(
    core: *mut AegisCoreHandle,
    input: AegisStringView,
    out_result: *mut *mut AegisExecutionResultHandle,
) -> u32 {
    if out_result.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: `out_result` is checked for null above and points to caller-owned
    // storage for one result handle pointer for the duration of this call.
    unsafe {
        *out_result = core::ptr::null_mut();
    }

    if core.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }
    let Some(input) = input.as_str() else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    // SAFETY: `core` is checked for null above and must be a valid handle
    // created by `aegis_core_create` for the duration of this call.
    let core = unsafe { &*core };
    let executor = match core.executor() {
        Ok(executor) => executor,
        Err(_) => return AEGIS_ERROR_INTERNAL,
    };
    let result = match executor.execute_line(input) {
        Ok(result) => AegisExecutionResultHandle::from_execution_result(result),
        Err(error) => AegisExecutionResultHandle::from_error(error),
    };
    write_result(out_result, result);
    AEGIS_OK
}

fn execute_script_impl(
    core: *mut AegisCoreHandle,
    source_name: AegisStringView,
    script: AegisStringView,
    options_ptr: *const core::ffi::c_void,
    out_result: *mut *mut AegisExecutionResultHandle,
) -> u32 {
    if out_result.is_null() || !options_ptr.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: `out_result` is checked for null above and points to caller-owned
    // storage for one result handle pointer for the duration of this call.
    unsafe {
        *out_result = core::ptr::null_mut();
    }

    if core.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }
    let (Some(source_name), Some(script)) = (source_name.as_str(), script.as_str()) else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    // SAFETY: `core` is checked for null above and must be a valid handle
    // created by `aegis_core_create` for the duration of this call.
    let core = unsafe { &*core };
    let executor = match core.executor() {
        Ok(executor) => executor,
        Err(_) => return AEGIS_ERROR_INTERNAL,
    };
    let runner = ScriptRunner::new(&executor);
    let result = match runner.execute_script(source_name, script, ScriptOptions::default()) {
        Ok(result) => AegisExecutionResultHandle::from_script_result(result),
        Err(error) => AegisExecutionResultHandle::from_error(error),
    };
    write_result(out_result, result);
    AEGIS_OK
}

fn catch_ffi_boundary(run: impl FnOnce() -> u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(code) => code,
        Err(_) => AEGIS_ERROR_PANIC,
    }
}

fn write_result(
    out_result: *mut *mut AegisExecutionResultHandle,
    result: AegisExecutionResultHandle,
) {
    let result = Box::into_raw(Box::new(result));
    // SAFETY: callers reach this helper only after `out_result` has been
    // validated as non-null and initialized to null for this call.
    unsafe {
        *out_result = result;
    }
}

fn with_result<T>(
    result: *const AegisExecutionResultHandle,
    default: T,
    read: impl FnOnce(&AegisExecutionResultHandle) -> T,
) -> T {
    if result.is_null() {
        return default;
    }

    // SAFETY: The ABI contract requires non-null `result` to be a live pointer
    // returned by an Aegis execute function and not yet released.
    read(unsafe { &*result })
}
