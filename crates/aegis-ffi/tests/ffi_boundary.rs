//! Contract tests for C ABI boundary helpers.

use aegis_ffi::error::{
    AEGIS_ERROR_CANCELLED, AEGIS_ERROR_COMMAND_NOT_FOUND, AEGIS_ERROR_FFI, AEGIS_ERROR_INTERNAL,
    AEGIS_ERROR_INVALID_ARGUMENT, AEGIS_ERROR_PANIC, AEGIS_ERROR_PARSE,
    AEGIS_ERROR_PERMISSION_DENIED, AEGIS_ERROR_PLUGIN, AEGIS_ERROR_REGISTRY, AEGIS_ERROR_SCRIPT,
    AEGIS_ERROR_TIMEOUT, AEGIS_OK,
};
use aegis_ffi::result::{
    AEGIS_EXECUTION_STATUS_FAILED, AEGIS_OUTPUT_CHANNEL_DEBUG, AEGIS_OUTPUT_CHANNEL_MAIN,
    AEGIS_OUTPUT_CHANNEL_SYSTEM, AEGIS_OUTPUT_CHANNEL_UNKNOWN, AEGIS_OUTPUT_KIND_DIAGNOSTIC,
    AEGIS_OUTPUT_KIND_ERROR, AEGIS_OUTPUT_KIND_JSON, AEGIS_OUTPUT_KIND_LOG,
    AEGIS_OUTPUT_KIND_PROGRESS, AEGIS_OUTPUT_KIND_STATE_CHANGED, AEGIS_OUTPUT_KIND_TABLE,
    AEGIS_OUTPUT_KIND_TEXT, AEGIS_OUTPUT_KIND_UNKNOWN, AEGIS_OUTPUT_KIND_WARNING,
};
use aegis_ffi::string::AegisStringView;
use aegis_ffi::{
    aegis_core_create, aegis_core_release, aegis_execute_line, aegis_execute_script,
    aegis_result_error_code, aegis_result_output_channel_at, aegis_result_output_command_id_at,
    aegis_result_output_count, aegis_result_output_kind_at, aegis_result_output_payload_at,
    aegis_result_output_sequence_at, aegis_result_release, aegis_result_status_code,
};

fn release_core(handle: *mut aegis_ffi::handle::AegisCoreHandle) {
    // SAFETY: Test handles are released exactly once after creation.
    unsafe {
        aegis_core_release(handle);
    }
}

fn release_result(handle: *mut aegis_ffi::result::AegisExecutionResultHandle) {
    // SAFETY: Test result handles are released exactly once after creation.
    unsafe {
        aegis_result_release(handle);
    }
}

fn execute_line(
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    input: AegisStringView,
    out_result: *mut *mut aegis_ffi::result::AegisExecutionResultHandle,
) -> u32 {
    // SAFETY: Tests pass live handles and valid output pointer storage.
    unsafe { aegis_execute_line(core, input, out_result) }
}

fn execute_script(
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    source_name: AegisStringView,
    script: AegisStringView,
    out_result: *mut *mut aegis_ffi::result::AegisExecutionResultHandle,
) -> u32 {
    // SAFETY: Tests pass live handles, valid string views, null options, and
    // valid output pointer storage.
    unsafe { aegis_execute_script(core, source_name, script, core::ptr::null(), out_result) }
}

fn execute_script_with_options(
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    source_name: AegisStringView,
    script: AegisStringView,
    options_ptr: *const core::ffi::c_void,
    out_result: *mut *mut aegis_ffi::result::AegisExecutionResultHandle,
) -> u32 {
    // SAFETY: Tests pass live handles and deliberately vary options pointer
    // validity to exercise the ABI boundary.
    unsafe { aegis_execute_script(core, source_name, script, options_ptr, out_result) }
}

fn result_status_code(result: *const aegis_ffi::result::AegisExecutionResultHandle) -> u32 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_status_code(result) }
}

fn result_error_code(result: *const aegis_ffi::result::AegisExecutionResultHandle) -> u32 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_error_code(result) }
}

fn result_output_count(result: *const aegis_ffi::result::AegisExecutionResultHandle) -> usize {
    // SAFETY: Tests pass live result handles.
    unsafe { aegis_result_output_count(result) }
}

fn result_output_command_id_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> u64 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_output_command_id_at(result, index) }
}

fn result_output_sequence_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> u64 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_output_sequence_at(result, index) }
}

fn result_output_channel_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> u32 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_output_channel_at(result, index) }
}

fn result_output_kind_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> u32 {
    // SAFETY: Tests pass null or live result handles.
    unsafe { aegis_result_output_kind_at(result, index) }
}

fn result_output_payload_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> AegisStringView {
    // SAFETY: Tests pass live result handles.
    unsafe { aegis_result_output_payload_at(result, index) }
}

fn assert_string_view_eq(view: AegisStringView, expected: &str) {
    assert!(!view.ptr.is_null());
    assert_eq!(view.len, expected.len());
    // SAFETY: The result handle retaining this payload is still live for the
    // duration of each assertion call.
    let bytes = unsafe { core::slice::from_raw_parts(view.ptr, view.len) };
    assert_eq!(bytes, expected.as_bytes());
}

#[test]
fn ffi_ok_code_is_zero() {
    assert_eq!(AEGIS_OK, 0);
}

#[test]
fn ffi_invalid_argument_code_is_stable() {
    assert_eq!(AEGIS_ERROR_INVALID_ARGUMENT, 400);
}

#[test]
fn ffi_error_codes_cover_core_error_model() {
    assert_eq!(AEGIS_ERROR_PARSE, 100);
    assert_eq!(AEGIS_ERROR_REGISTRY, 200);
    assert_eq!(AEGIS_ERROR_COMMAND_NOT_FOUND, 300);
    assert_eq!(AEGIS_ERROR_PERMISSION_DENIED, 500);
    assert_eq!(AEGIS_ERROR_CANCELLED, 600);
    assert_eq!(AEGIS_ERROR_TIMEOUT, 700);
    assert_eq!(AEGIS_ERROR_SCRIPT, 800);
    assert_eq!(AEGIS_ERROR_PLUGIN, 900);
    assert_eq!(AEGIS_ERROR_INTERNAL, 1_000);
    assert_eq!(AEGIS_ERROR_FFI, 1_100);
    assert_eq!(AEGIS_ERROR_PANIC, 10_000);
}

#[test]
fn ffi_output_channel_codes_are_stable() {
    assert_eq!(AEGIS_OUTPUT_CHANNEL_UNKNOWN, 0);
    assert_eq!(AEGIS_OUTPUT_CHANNEL_MAIN, 1);
    assert_eq!(AEGIS_OUTPUT_CHANNEL_DEBUG, 2);
    assert_eq!(AEGIS_OUTPUT_CHANNEL_SYSTEM, 3);
}

#[test]
fn ffi_output_kind_codes_are_stable() {
    assert_eq!(AEGIS_OUTPUT_KIND_UNKNOWN, 0);
    assert_eq!(AEGIS_OUTPUT_KIND_TEXT, 1);
    assert_eq!(AEGIS_OUTPUT_KIND_JSON, 2);
    assert_eq!(AEGIS_OUTPUT_KIND_TABLE, 3);
    assert_eq!(AEGIS_OUTPUT_KIND_LOG, 4);
    assert_eq!(AEGIS_OUTPUT_KIND_WARNING, 5);
    assert_eq!(AEGIS_OUTPUT_KIND_ERROR, 6);
    assert_eq!(AEGIS_OUTPUT_KIND_PROGRESS, 7);
    assert_eq!(AEGIS_OUTPUT_KIND_STATE_CHANGED, 8);
    assert_eq!(AEGIS_OUTPUT_KIND_DIAGNOSTIC, 9);
}

#[test]
fn string_view_accepts_non_empty_utf8() {
    let input = "echo";
    let view = AegisStringView::from_str(input);

    assert_eq!(view.len, 4);
}

#[test]
fn string_view_empty_is_null_with_zero_length() {
    let view = AegisStringView::empty();

    assert!(view.ptr.is_null());
    assert_eq!(view.len, 0);
}

#[test]
fn ffi_core_create_returns_non_null_handle() {
    let handle = aegis_core_create();
    assert!(!handle.is_null());

    release_core(handle);
}

#[test]
fn ffi_core_release_tolerates_null_handle() {
    release_core(core::ptr::null_mut());
}

#[test]
fn ffi_result_release_tolerates_null_handle() {
    release_result(core::ptr::null_mut());
}

#[test]
fn ffi_execute_line_returns_result_handle_for_echo() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert!(!result.is_null());

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_rejects_null_core_handle() {
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core::ptr::null_mut(), input, &mut result);

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());
}

#[test]
fn ffi_execute_line_rejects_null_out_result() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let code = execute_line(core, input, core::ptr::null_mut());

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);

    release_core(core);
}

#[test]
fn ffi_execute_line_rejects_invalid_utf8_input() {
    let core = aegis_core_create();
    let invalid = [0xff_u8];
    let input = AegisStringView {
        ptr: invalid.as_ptr(),
        len: invalid.len(),
    };
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());

    release_core(core);
}

#[test]
fn ffi_execute_line_rejects_impossibly_large_string_view() {
    let core = aegis_core_create();
    let input = AegisStringView {
        ptr: core::ptr::NonNull::<u8>::dangling().as_ptr(),
        len: usize::MAX,
    };
    let mut result =
        core::ptr::NonNull::<aegis_ffi::result::AegisExecutionResultHandle>::dangling().as_ptr();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());

    release_core(core);
}

#[test]
fn ffi_execute_script_returns_result_handle_for_echo_script() {
    let core = aegis_core_create();
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("echo one\necho two");
    let mut result = core::ptr::null_mut();
    let code = execute_script(core, source, script, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert!(!result.is_null());

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_script_rejects_null_core_handle() {
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("echo one");
    let mut result = core::ptr::null_mut();
    let code = execute_script(core::ptr::null_mut(), source, script, &mut result);

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());
}

#[test]
fn ffi_execute_script_rejects_null_out_result() {
    let core = aegis_core_create();
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("echo one");
    let code = execute_script(core, source, script, core::ptr::null_mut());

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);

    release_core(core);
}

#[test]
fn ffi_execute_script_rejects_non_null_options_pointer() {
    let core = aegis_core_create();
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("echo one");
    let mut result = core::ptr::null_mut();
    let options = 1_u8;
    let code = execute_script_with_options(
        core,
        source,
        script,
        &options as *const u8 as *const core::ffi::c_void,
        &mut result,
    );

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());

    release_core(core);
}

#[test]
fn ffi_execute_script_clears_out_result_when_options_pointer_is_rejected() {
    let core = aegis_core_create();
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("echo one");
    let mut result =
        core::ptr::NonNull::<aegis_ffi::result::AegisExecutionResultHandle>::dangling().as_ptr();
    let options = 1_u8;
    let code = execute_script_with_options(
        core,
        source,
        script,
        &options as *const u8 as *const core::ffi::c_void,
        &mut result,
    );

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());

    release_core(core);
}

#[test]
fn ffi_execute_script_rejects_invalid_utf8_source_name() {
    let core = aegis_core_create();
    let invalid = [0xff_u8];
    let source = AegisStringView {
        ptr: invalid.as_ptr(),
        len: invalid.len(),
    };
    let script = AegisStringView::from_str("echo one");
    let mut result = core::ptr::null_mut();
    let code = execute_script(core, source, script, &mut result);

    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);
    assert!(result.is_null());

    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_count() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_output_count(result), 1);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_command_id() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_output_command_id_at(result, 0), 1);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_sequence() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_output_sequence_at(result, 0), 1);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_channel() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_ne!(result_output_channel_at(result, 0), 0);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_kind() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_output_kind_at(result, 0), AEGIS_OUTPUT_KIND_TEXT);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_exposes_output_payload() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_string_view_eq(result_output_payload_at(result, 0), "hello");

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_result_accessors_tolerate_null_handle() {
    assert_eq!(result_status_code(core::ptr::null()), 0);
    assert_eq!(result_error_code(core::ptr::null()), 0);
    assert_eq!(result_output_count(core::ptr::null()), 0);
    assert_eq!(result_output_command_id_at(core::ptr::null(), 0), 0);
    assert_eq!(result_output_sequence_at(core::ptr::null(), 0), 0);
    assert_eq!(result_output_channel_at(core::ptr::null(), 0), 0);
    assert_eq!(result_output_kind_at(core::ptr::null(), 0), 0);
    assert!(result_output_payload_at(core::ptr::null(), 0).ptr.is_null());
}

#[test]
fn ffi_execute_line_parse_error_uses_failed_status_and_error_code() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("/echo");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_FAILED);
    assert_eq!(result_error_code(result), AEGIS_ERROR_PARSE);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_line_unknown_command_sets_command_not_found_error_code() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("missing_command");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_FAILED);
    assert_eq!(result_error_code(result), AEGIS_ERROR_COMMAND_NOT_FOUND);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_execute_script_failure_sets_script_error_code() {
    let core = aegis_core_create();
    let source = AegisStringView::from_str("test.cfg");
    let script = AegisStringView::from_str("missing_command");
    let mut result = core::ptr::null_mut();
    let code = execute_script(core, source, script, &mut result);

    assert_eq!(code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_FAILED);
    assert_eq!(result_error_code(result), AEGIS_ERROR_SCRIPT);

    release_result(result);
    release_core(core);
}

#[test]
fn ffi_result_output_payload_at_tolerates_out_of_bounds_index() {
    let core = aegis_core_create();
    let input = AegisStringView::from_str("echo hello");
    let mut result = core::ptr::null_mut();
    let code = execute_line(core, input, &mut result);

    assert_eq!(code, AEGIS_OK);

    let payload = result_output_payload_at(result, 999);
    assert!(payload.ptr.is_null());
    assert_eq!(payload.len, 0);

    release_result(result);
    release_core(core);
}
