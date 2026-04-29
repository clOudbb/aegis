//! Contract tests for plugin registration protocols.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use aegis_core::error::AegisError;
use aegis_core::error::Result;
use aegis_core::executor::{ExecutionStatus, Executor};
use aegis_core::flags::ConsoleFlags;
use aegis_core::hook::{ExecutionHookPoint, HookDecision, HookMatcher};
use aegis_core::output::OutputFrame;
use aegis_core::plugin::{PluginDescriptor, PluginRegistry};
use aegis_core::query::{CompletionItem, CompletionKind};
use aegis_core::script::{ScriptOptions, ScriptRunner};

#[test]
fn plugin_registry_accepts_unique_plugin_id() -> Result<()> {
    let mut registry = PluginRegistry::new();
    registry.register(PluginDescriptor::new("host.debug", "Host Debug"))?;

    assert!(registry.contains("host.debug"));
    Ok(())
}

#[test]
fn plugin_registry_rejects_duplicate_plugin_id_case_insensitively() -> Result<()> {
    let mut registry = PluginRegistry::new();
    registry.register(PluginDescriptor::new("host.debug", "Host Debug"))?;

    let error = match registry.register(PluginDescriptor::new("HOST.DEBUG", "Debug Again")) {
        Ok(()) => {
            return Err(aegis_core::error::AegisError::internal(
                "duplicate plugin registration should fail",
            ));
        }
        Err(error) => error,
    };
    assert_eq!(error.message(), "plugin is already registered");
    Ok(())
}

#[test]
fn plugin_can_register_command() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.debug", "Host Debug"),
        |plugin| {
            plugin.register_command(
                "host_ping",
                "Host ping",
                ConsoleFlags::empty(),
                |ctx, _args| {
                    ctx.write_frame(OutputFrame::text("pong"));
                    Ok(())
                },
            )
        },
    )?;

    let result = executor.execute_line("host_ping")?;
    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn plugin_can_register_cvar() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.settings", "Host Settings"),
        |plugin| plugin.register_cvar("host_mode", "normal", ConsoleFlags::empty(), "Host mode"),
    )?;

    let result = executor.execute_line("host_mode")?;
    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn plugin_output_sink_observes_frames() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_sink = Arc::clone(&observed);

    executor.register_plugin(
        PluginDescriptor::new("host.logging", "Host Logging"),
        |plugin| {
            plugin.register_output_sink("test_sink", move |_frame| {
                observed_sink.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        },
    )?;

    let result = executor.execute_line("echo hello")?;
    assert_eq!(result.status(), ExecutionStatus::Success);
    assert!(observed.load(Ordering::SeqCst) > 0);
    Ok(())
}

#[test]
fn plugin_before_execute_hook_can_block_command() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.policy", "Host Policy"),
        |plugin| {
            plugin.register_execution_hook(
                ExecutionHookPoint::BeforeExecute,
                HookMatcher::command("echo")?,
                |_context| Ok(HookDecision::block("echo disabled")),
            )
        },
    )?;

    let result = executor.execute_line("echo hello")?;
    assert_eq!(result.status(), ExecutionStatus::Blocked);
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn plugin_hook_panic_returns_error_without_crossing_public_api() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.audit", "Host Audit"),
        |plugin| {
            plugin.register_execution_hook(
                ExecutionHookPoint::BeforeExecute,
                HookMatcher::any(),
                |_context| -> Result<HookDecision> {
                    panic!("host hook panic");
                },
            )
        },
    )?;

    let error = match executor.execute_line("echo hello") {
        Ok(_) => {
            return Err(AegisError::internal(
                "hook panic should be converted to an error",
            ));
        }
        Err(error) => error,
    };

    assert!(error.message().contains("panicked"));
    Ok(())
}

#[test]
fn plugin_after_script_hook_error_does_not_discard_script_result() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.audit", "Host Audit"),
        |plugin| {
            plugin.register_execution_hook(
                ExecutionHookPoint::AfterScriptExecute,
                HookMatcher::any(),
                |_context| Err(AegisError::internal("audit failed")),
            )
        },
    )?;
    let runner = ScriptRunner::new(&executor);

    let result = runner.execute_script("test.cfg", "echo hello", ScriptOptions::default())?;

    assert_eq!(result.executed_commands(), 1);
    assert!(!result.is_failed());
    assert_eq!(result.diagnostics().len(), 1);
    Ok(())
}

#[test]
fn plugin_after_execute_hook_is_observe_only() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.audit", "Host Audit"),
        |plugin| {
            plugin.register_execution_hook(
                ExecutionHookPoint::AfterExecute,
                HookMatcher::any(),
                |_context| Ok(HookDecision::block("too late")),
            )
        },
    )?;

    let result = executor.execute_line("echo hello")?;
    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn plugin_before_script_execute_hook_can_block_script() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.policy", "Host Policy"),
        |plugin| {
            plugin.register_execution_hook(
                ExecutionHookPoint::BeforeScriptExecute,
                HookMatcher::any(),
                |_context| Ok(HookDecision::block("scripts disabled")),
            )
        },
    )?;
    let runner = ScriptRunner::new(&executor);

    let result = runner.execute_script("test.cfg", "echo hello", ScriptOptions::default())?;
    assert_eq!(result.executed_commands(), 0);
    assert!(result.is_blocked());
    Ok(())
}

#[test]
fn completion_item_preserves_insert_text() {
    let item = CompletionItem::new("echo", "echo", CompletionKind::Command);

    assert_eq!(item.insert_text(), "echo");
}

#[test]
fn executor_completion_returns_registered_commands() -> Result<()> {
    let executor = Executor::with_builtins();
    let items = executor.complete("ec")?;

    assert!(items.iter().any(|item| item.insert_text() == "echo"));
    Ok(())
}

#[test]
fn executor_completion_hides_hidden_cvars() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "internal_token",
        "secret",
        ConsoleFlags::HIDDEN,
        "Internal token",
    )?;
    let items = executor.complete("internal")?;

    assert!(items.is_empty());
    Ok(())
}

#[test]
fn executor_help_returns_topic_for_registered_command() -> Result<()> {
    let executor = Executor::with_builtins();
    let topic = executor.help("echo")?;

    assert_eq!(topic.name(), "echo");
    Ok(())
}
