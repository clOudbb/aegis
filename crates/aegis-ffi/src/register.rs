//! C ABI registration facade for plugin-owned commands and cvars.

use std::panic::{AssertUnwindSafe, catch_unwind};

use aegis_core::context::ExecutionContext;
use aegis_core::error::{AegisError, Result as CoreResult};
use aegis_core::executor::{CommandStatus, Executor};
use aegis_core::flags::ConsoleFlags;
use aegis_core::parser::CommandArg;
use aegis_core::plugin::{PluginDescriptor, PluginId};

use crate::error::{
    AEGIS_ERROR_INTERNAL, AEGIS_ERROR_INVALID_ARGUMENT, AEGIS_ERROR_PANIC, AEGIS_OK,
    code_from_core_error,
};
use crate::handle::{AegisCoreHandle, AegisCoreState};
use crate::string::AegisStringView;

/// Opaque plugin ownership handle.
///
/// The handle does not own the core and does not unregister the plugin when
/// released. It becomes invalid immediately after the owning core is released.
pub struct AegisPluginHandle {
    core: std::sync::Weak<AegisCoreState>,
    core_id: usize,
    plugin_id: String,
}

/// Opaque execution context handle valid only during one command callback.
pub struct AegisExecutionContextHandle {
    context: *mut ExecutionContext,
}

/// C ABI command callback.
///
/// `context` is valid only during the callback. `argv` points to `argc`
/// borrowed UTF-8 string views that become invalid when the callback returns.
/// `userdata` is caller-owned and is never released by Aegis.
/// Rust panics from `extern "C-unwind"` callbacks are converted to command
/// failure. Non-Rust callbacks must not throw or unwind through this boundary.
/// Callbacks must not reenter the same core handle while executing.
pub type AegisCommandCallback = unsafe extern "C-unwind" fn(
    context: *mut AegisExecutionContextHandle,
    argc: usize,
    argv: *const AegisStringView,
    userdata: *mut core::ffi::c_void,
) -> u32;

#[derive(Clone, Copy)]
struct CallbackUserdata(*mut core::ffi::c_void);

// SAFETY: Aegis stores and passes this pointer back to the host without
// dereferencing it. The host is responsible for keeping pointed-to state valid
// and synchronized until the owning core is released.
unsafe impl Send for CallbackUserdata {}

// SAFETY: See the `Send` implementation; Aegis never dereferences the pointer.
unsafe impl Sync for CallbackUserdata {}

impl CallbackUserdata {
    fn as_ptr(self) -> *mut core::ffi::c_void {
        self.0
    }
}

/// Register a plugin identity and return a plugin ownership handle.
///
/// # Safety
///
/// `core` must be null or a live pointer returned by `aegis_core_create`.
/// `out_plugin` must be a valid writable pointer to one plugin handle pointer.
/// String views must point to memory valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_register_plugin(
    core: *mut AegisCoreHandle,
    id: AegisStringView,
    name: AegisStringView,
    version: AegisStringView,
    out_plugin: *mut *mut AegisPluginHandle,
) -> u32 {
    catch_registration_boundary(|| register_plugin_impl(core, id, name, version, out_plugin))
}

/// Release a plugin handle.
///
/// # Safety
///
/// `plugin` must be null or a live pointer returned by `aegis_register_plugin`.
/// Releasing the same plugin handle more than once is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_plugin_release(plugin: *mut AegisPluginHandle) {
    if plugin.is_null() {
        return;
    }

    // SAFETY: `plugin` must be a pointer returned by `aegis_register_plugin`
    // and must not have been released already. Null is handled above.
    unsafe {
        drop(Box::from_raw(plugin));
    }
}

/// Register a plugin-owned cvar.
///
/// # Safety
///
/// `plugin` must be null or a live pointer returned by `aegis_register_plugin`.
/// String views must point to memory valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_register_cvar(
    plugin: *mut AegisPluginHandle,
    name: AegisStringView,
    default_value: AegisStringView,
    flags: u32,
    description: AegisStringView,
) -> u32 {
    catch_registration_boundary(|| {
        register_cvar_impl(plugin, name, default_value, flags, description)
    })
}

/// Register a plugin-owned command callback.
///
/// # Safety
///
/// `plugin` must be null or a live pointer returned by `aegis_register_plugin`.
/// String views must point to memory valid for the duration of the call.
/// `callback` and `userdata` must remain valid until the owning core is
/// released. The callback must not call back into the same core handle
/// reentrantly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_register_command(
    plugin: *mut AegisPluginHandle,
    name: AegisStringView,
    flags: u32,
    description: AegisStringView,
    callback: Option<AegisCommandCallback>,
    userdata: *mut core::ffi::c_void,
) -> u32 {
    catch_registration_boundary(|| {
        register_command_impl(plugin, name, flags, description, callback, userdata)
    })
}

/// Write text output through a callback execution context.
///
/// # Safety
///
/// `context` must be null or the live callback context passed to the currently
/// executing command callback. `text` must point to memory valid for the
/// duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aegis_context_write_text(
    context: *mut AegisExecutionContextHandle,
    text: AegisStringView,
) -> u32 {
    catch_registration_boundary(|| context_write_text_impl(context, text))
}

fn register_plugin_impl(
    core: *mut AegisCoreHandle,
    id: AegisStringView,
    name: AegisStringView,
    version: AegisStringView,
    out_plugin: *mut *mut AegisPluginHandle,
) -> u32 {
    if out_plugin.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: `out_plugin` is checked for null above and points to caller-owned
    // storage for one plugin handle pointer for the duration of this call.
    unsafe {
        *out_plugin = core::ptr::null_mut();
    }

    if core.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }
    let (Some(id), Some(name), Some(_version)) = (id.as_str(), name.as_str(), version.as_str())
    else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    let plugin_id = match PluginId::parse(id) {
        Ok(plugin_id) => plugin_id,
        Err(error) => return code_from_core_error(&error),
    };

    // SAFETY: `core` is checked for null above and must be a valid handle
    // created by `aegis_core_create` for the duration of this call.
    let core_ref = unsafe { &*core };
    let mut executor = match core_ref.executor() {
        Ok(executor) => executor,
        Err(_) => return AEGIS_ERROR_INTERNAL,
    };
    if let Err(error) = executor.register_plugin(PluginDescriptor::new(id, name), |_plugin| Ok(()))
    {
        return code_from_core_error(&error);
    }

    let plugin = Box::new(AegisPluginHandle {
        core: core_ref.downgrade(),
        core_id: core_ref.id(),
        plugin_id: plugin_id.canonical().to_owned(),
    });

    // SAFETY: `out_plugin` was validated and initialized above.
    unsafe {
        *out_plugin = Box::into_raw(plugin);
    }
    AEGIS_OK
}

fn register_cvar_impl(
    plugin: *mut AegisPluginHandle,
    name: AegisStringView,
    default_value: AegisStringView,
    flags: u32,
    description: AegisStringView,
) -> u32 {
    let (Some(name), Some(default_value), Some(description)) =
        (name.as_str(), default_value.as_str(), description.as_str())
    else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    with_plugin_core(plugin, |executor, plugin_id| {
        executor
            .register_plugin_cvar(
                plugin_id,
                name,
                default_value,
                ConsoleFlags::from_bits_retain(flags),
                description,
            )
            .map_err(|error| code_from_core_error(&error))
    })
}

fn register_command_impl(
    plugin: *mut AegisPluginHandle,
    name: AegisStringView,
    flags: u32,
    description: AegisStringView,
    callback: Option<AegisCommandCallback>,
    userdata: *mut core::ffi::c_void,
) -> u32 {
    let Some(callback) = callback else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };
    let (Some(name), Some(description)) = (name.as_str(), description.as_str()) else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    let userdata = CallbackUserdata(userdata);
    with_plugin_core(plugin, |core, plugin_id| {
        core.register_plugin_command(
            plugin_id,
            name,
            ConsoleFlags::from_bits_retain(flags),
            description,
            move |context, args| execute_command_callback(context, args, callback, userdata),
        )
        .map_err(|error| code_from_core_error(&error))
    })
}

fn execute_command_callback(
    context: &mut ExecutionContext,
    args: &[CommandArg],
    callback: AegisCommandCallback,
    userdata: CallbackUserdata,
) -> CoreResult<CommandStatus> {
    let argv: Vec<AegisStringView> = args
        .iter()
        .map(|arg| AegisStringView::from_str(arg.as_str()))
        .collect();
    let argv_ptr = if argv.is_empty() {
        core::ptr::null()
    } else {
        argv.as_ptr()
    };
    let mut context_handle = AegisExecutionContextHandle {
        context: context as *mut ExecutionContext,
    };

    let callback_result = catch_unwind(AssertUnwindSafe(|| unsafe {
        // SAFETY: The callback pointer and userdata are provided by the host
        // and must remain valid until core release. The temporary context and
        // argv views are valid for this synchronous callback invocation only.
        callback(&mut context_handle, argv.len(), argv_ptr, userdata.as_ptr())
    }))
    .unwrap_or(AEGIS_ERROR_PANIC);

    if callback_result == AEGIS_OK {
        Ok(CommandStatus::Success)
    } else {
        Err(AegisError::ffi(format!(
            "ffi command callback failed with code {callback_result}"
        )))
    }
}

fn context_write_text_impl(
    context: *mut AegisExecutionContextHandle,
    text: AegisStringView,
) -> u32 {
    if context.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }
    let Some(text) = text.as_str() else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };

    // SAFETY: `context` is checked for null above. The callback contract makes
    // the nested execution context pointer valid only for the current callback.
    let context = unsafe { &mut *context };
    if context.context.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: `context.context` is checked for null above and is created from a
    // live `&mut ExecutionContext` immediately before invoking the callback.
    unsafe {
        (*context.context).write_text(text);
    }
    AEGIS_OK
}

fn with_plugin_core(
    plugin: *mut AegisPluginHandle,
    use_core: impl FnOnce(&mut Executor, &str) -> Result<(), u32>,
) -> u32 {
    if plugin.is_null() {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    // SAFETY: `plugin` is checked for null above and must be a live plugin
    // handle returned by `aegis_register_plugin`.
    let plugin = unsafe { &*plugin };
    let Some(core) = plugin.core.upgrade() else {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    };
    if core.id() != plugin.core_id {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    let mut executor = match core.executor() {
        Ok(executor) => executor,
        Err(_) => return AEGIS_ERROR_INTERNAL,
    };
    if !executor.contains_plugin(&plugin.plugin_id) {
        return AEGIS_ERROR_INVALID_ARGUMENT;
    }

    use_core(&mut executor, &plugin.plugin_id).map_or_else(|code| code, |()| AEGIS_OK)
}

fn catch_registration_boundary(run: impl FnOnce() -> u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(code) => code,
        Err(_) => AEGIS_ERROR_PANIC,
    }
}
