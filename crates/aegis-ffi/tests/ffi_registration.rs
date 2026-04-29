//! Contract tests for C ABI plugin, command, and cvar registration.

use aegis_ffi::error::{AEGIS_ERROR_INTERNAL, AEGIS_ERROR_INVALID_ARGUMENT, AEGIS_OK};
use aegis_ffi::result::{AEGIS_EXECUTION_STATUS_FAILED, AEGIS_EXECUTION_STATUS_SUCCESS};
use aegis_ffi::string::AegisStringView;
use aegis_ffi::{
    AegisCommandCallback, AegisExecutionContextHandle, aegis_context_write_text, aegis_core_create,
    aegis_core_release, aegis_execute_line, aegis_plugin_release, aegis_register_command,
    aegis_register_cvar, aegis_register_plugin, aegis_result_output_payload_at,
    aegis_result_release, aegis_result_status_code,
};

fn release_core(handle: *mut aegis_ffi::handle::AegisCoreHandle) {
    // SAFETY: Test handles are released exactly once after creation.
    unsafe {
        aegis_core_release(handle);
    }
}

fn release_plugin(handle: *mut aegis_ffi::register::AegisPluginHandle) {
    // SAFETY: Test plugin handles are released exactly once after creation.
    unsafe {
        aegis_plugin_release(handle);
    }
}

fn release_result(handle: *mut aegis_ffi::result::AegisExecutionResultHandle) {
    // SAFETY: Test result handles are released exactly once after creation.
    unsafe {
        aegis_result_release(handle);
    }
}

fn register_plugin(
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    id: &str,
    name: &str,
    out_plugin: *mut *mut aegis_ffi::register::AegisPluginHandle,
) -> u32 {
    // SAFETY: Tests pass live core handles and valid output pointer storage.
    unsafe {
        aegis_register_plugin(
            core,
            AegisStringView::from_str(id),
            AegisStringView::from_str(name),
            AegisStringView::from_str("0.1.0"),
            out_plugin,
        )
    }
}

fn register_cvar(
    plugin: *mut aegis_ffi::register::AegisPluginHandle,
    name: &str,
    default_value: &str,
    description: &str,
) -> u32 {
    // SAFETY: Tests pass live plugin handles and valid string views.
    unsafe {
        aegis_register_cvar(
            plugin,
            AegisStringView::from_str(name),
            AegisStringView::from_str(default_value),
            0,
            AegisStringView::from_str(description),
        )
    }
}

fn register_command(
    plugin: *mut aegis_ffi::register::AegisPluginHandle,
    callback: AegisCommandCallback,
) -> u32 {
    register_command_with_userdata(plugin, callback, core::ptr::null_mut())
}

fn register_command_with_userdata(
    plugin: *mut aegis_ffi::register::AegisPluginHandle,
    callback: AegisCommandCallback,
    userdata: *mut core::ffi::c_void,
) -> u32 {
    // SAFETY: Tests pass live plugin handles and a callback valid for the core lifetime.
    unsafe {
        aegis_register_command(
            plugin,
            AegisStringView::from_str("host_hello"),
            0,
            AegisStringView::from_str("Host hello command"),
            Some(callback),
            userdata,
        )
    }
}

fn execute_line(
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    input: &str,
    out_result: *mut *mut aegis_ffi::result::AegisExecutionResultHandle,
) -> u32 {
    // SAFETY: Tests pass live core handles and valid output pointer storage.
    unsafe { aegis_execute_line(core, AegisStringView::from_str(input), out_result) }
}

fn result_status_code(result: *const aegis_ffi::result::AegisExecutionResultHandle) -> u32 {
    // SAFETY: Tests pass live result handles.
    unsafe { aegis_result_status_code(result) }
}

fn result_output_payload_at(
    result: *const aegis_ffi::result::AegisExecutionResultHandle,
    index: usize,
) -> AegisStringView {
    // SAFETY: Tests pass live result handles.
    unsafe { aegis_result_output_payload_at(result, index) }
}

unsafe extern "C-unwind" fn hello_command(
    context: *mut AegisExecutionContextHandle,
    _argc: usize,
    _argv: *const AegisStringView,
    _userdata: *mut core::ffi::c_void,
) -> u32 {
    // SAFETY: Aegis passes a live callback context for the duration of this callback.
    unsafe {
        aegis_context_write_text(
            context,
            AegisStringView::from_str("hello from swift-like host"),
        )
    }
}

#[derive(Default)]
struct CallbackState {
    argc: usize,
    first_arg_len: usize,
    userdata_seen: bool,
}

struct ReentrantState {
    core: *mut aegis_ffi::handle::AegisCoreHandle,
    nested_code: u32,
}

unsafe extern "C-unwind" fn inspect_command(
    context: *mut AegisExecutionContextHandle,
    argc: usize,
    argv: *const AegisStringView,
    userdata: *mut core::ffi::c_void,
) -> u32 {
    if userdata.is_null() {
        return aegis_ffi::error::AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: Test passes a live `CallbackState` pointer as userdata.
    let state = unsafe { &mut *(userdata as *mut CallbackState) };
    state.argc = argc;
    state.userdata_seen = true;
    if argc > 0 && !argv.is_null() {
        // SAFETY: Aegis passes an argv array valid for `argc` elements during callback.
        let first = unsafe { *argv };
        state.first_arg_len = first.len;
    }

    // SAFETY: Aegis passes a live callback context for the duration of this callback.
    unsafe { aegis_context_write_text(context, AegisStringView::from_str("inspected")) }
}

unsafe extern "C-unwind" fn reentrant_execute_command(
    _context: *mut AegisExecutionContextHandle,
    _argc: usize,
    _argv: *const AegisStringView,
    userdata: *mut core::ffi::c_void,
) -> u32 {
    if userdata.is_null() {
        return aegis_ffi::error::AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: Test passes a live `ReentrantState` pointer as userdata.
    let state = unsafe { &mut *(userdata as *mut ReentrantState) };
    let mut result = core::ptr::null_mut();
    // SAFETY: The test deliberately exercises reentrant use of the same live
    // core handle. The nested call must fail instead of deadlocking.
    state.nested_code = unsafe {
        aegis_execute_line(
            state.core,
            AegisStringView::from_str("echo nested"),
            &mut result,
        )
    };
    release_result(result);
    AEGIS_OK
}

#[allow(clippy::panic)]
unsafe extern "C-unwind" fn panicking_command(
    _context: *mut AegisExecutionContextHandle,
    _argc: usize,
    _argv: *const AegisStringView,
    _userdata: *mut core::ffi::c_void,
) -> u32 {
    panic!("ffi callback panic");
}

#[test]
fn ffi_register_plugin_returns_plugin_handle() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();

    let code = register_plugin(core, "host.debug", "Host Debug", &mut plugin);

    assert_eq!(code, AEGIS_OK);
    assert!(!plugin.is_null());

    release_plugin(plugin);
    release_core(core);
}

#[test]
fn ffi_plugin_handle_after_core_release_returns_invalid_argument() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.debug", "Host Debug", &mut plugin),
        AEGIS_OK
    );

    release_core(core);

    let code = register_cvar(plugin, "developer", "0", "Enable developer output");
    assert_eq!(code, AEGIS_ERROR_INVALID_ARGUMENT);

    release_plugin(plugin);
}

#[test]
fn ffi_register_cvar_under_plugin_allows_read_by_name() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.settings", "Host Settings", &mut plugin),
        AEGIS_OK
    );

    let code = register_cvar(plugin, "developer", "0", "Enable developer output");
    assert_eq!(code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "developer", &mut result);

    assert_eq!(execute_code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_SUCCESS);

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}

#[test]
fn ffi_register_command_under_plugin_allows_execute_line() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.commands", "Host Commands", &mut plugin),
        AEGIS_OK
    );

    let code = register_command(plugin, hello_command as AegisCommandCallback);
    assert_eq!(code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "host_hello", &mut result);

    assert_eq!(execute_code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_SUCCESS);

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}

#[test]
fn ffi_command_callback_output_is_visible_in_result() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.commands", "Host Commands", &mut plugin),
        AEGIS_OK
    );
    let register_code = register_command(plugin, hello_command as AegisCommandCallback);
    assert_eq!(register_code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "host_hello", &mut result);
    assert_eq!(execute_code, AEGIS_OK);

    let payload = result_output_payload_at(result, 0);
    assert_eq!(payload.len, "hello from swift-like host".len());

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}

#[test]
fn ffi_command_callback_receives_args_and_userdata() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.commands", "Host Commands", &mut plugin),
        AEGIS_OK
    );
    let mut state = CallbackState::default();
    let register_code = register_command_with_userdata(
        plugin,
        inspect_command as AegisCommandCallback,
        &mut state as *mut CallbackState as *mut core::ffi::c_void,
    );
    assert_eq!(register_code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "host_hello alpha beta", &mut result);
    assert_eq!(execute_code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_SUCCESS);
    assert!(state.userdata_seen);
    assert_eq!(state.argc, 2);
    assert_eq!(state.first_arg_len, "alpha".len());

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}

#[test]
fn ffi_command_callback_reentrant_same_core_call_fails_without_deadlock() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.commands", "Host Commands", &mut plugin),
        AEGIS_OK
    );
    let mut state = ReentrantState {
        core,
        nested_code: AEGIS_OK,
    };
    let register_code = register_command_with_userdata(
        plugin,
        reentrant_execute_command as AegisCommandCallback,
        &mut state as *mut ReentrantState as *mut core::ffi::c_void,
    );
    assert_eq!(register_code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "host_hello", &mut result);
    assert_eq!(execute_code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_SUCCESS);
    assert_eq!(state.nested_code, AEGIS_ERROR_INTERNAL);

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}

#[test]
#[allow(clippy::panic)]
fn ffi_command_callback_panic_becomes_failed_result() {
    let core = aegis_core_create();
    let mut plugin = core::ptr::null_mut();
    assert_eq!(
        register_plugin(core, "host.commands", "Host Commands", &mut plugin),
        AEGIS_OK
    );
    let register_code = register_command(plugin, panicking_command as AegisCommandCallback);
    assert_eq!(register_code, AEGIS_OK);

    let mut result = core::ptr::null_mut();
    let execute_code = execute_line(core, "host_hello", &mut result);
    assert_eq!(execute_code, AEGIS_OK);
    assert_eq!(result_status_code(result), AEGIS_EXECUTION_STATUS_FAILED);

    release_result(result);
    release_plugin(plugin);
    release_core(core);
}
