//! Contract tests for registry and executor public APIs.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use aegis_core::cvar::ConsoleVar;
use aegis_core::error::{AegisError, Result};
use aegis_core::executor::{CommandStatus, ExecutionStatus, Executor};
use aegis_core::flags::ConsoleFlags;
use aegis_core::output::OutputFrameKind;
use aegis_core::plugin::PluginDescriptor;
use aegis_core::registry::{CommandMetadata, CommandRegistry};

fn expect_error<T>(result: Result<T>, message: &str) -> Result<AegisError> {
    match result {
        Ok(_) => Err(AegisError::internal(message)),
        Err(error) => Ok(error),
    }
}

#[test]
fn console_flags_can_be_combined_additively() {
    let flags = ConsoleFlags::CHEAT | ConsoleFlags::ARCHIVE;

    assert!(flags.contains(ConsoleFlags::CHEAT));
    assert!(flags.contains(ConsoleFlags::ARCHIVE));
}

#[test]
fn registry_finds_command_case_insensitively() -> Result<()> {
    let mut registry = CommandRegistry::new();
    registry.register_metadata(CommandMetadata::new("Echo", "Print text")?)?;

    assert!(registry.contains_command("echo"));
    Ok(())
}

#[test]
fn registry_rejects_duplicate_canonical_command_name() -> Result<()> {
    let mut registry = CommandRegistry::new();
    registry.register_metadata(CommandMetadata::new("echo", "Print text")?)?;

    let error = expect_error(
        registry.register_metadata(CommandMetadata::new("ECHO", "Print again")?),
        "duplicate command registration should fail",
    )?;
    assert_eq!(error.message(), "command is already registered");
    Ok(())
}

#[test]
fn command_metadata_rejects_invalid_name() -> Result<()> {
    let error = expect_error(
        CommandMetadata::new("/debug", "Invalid command"),
        "invalid command metadata name should fail",
    )?;

    assert_eq!(error.message(), "command name contains invalid characters");
    Ok(())
}

#[test]
fn registry_preserves_command_flags() -> Result<()> {
    let mut registry = CommandRegistry::new();
    let metadata =
        CommandMetadata::new("debug_dump", "Dump debug state")?.with_flags(ConsoleFlags::CHEAT);

    registry.register_metadata(metadata)?;

    assert!(
        registry
            .get_command("DEBUG_DUMP")?
            .flags()
            .contains(ConsoleFlags::CHEAT)
    );
    Ok(())
}

#[test]
fn cvar_defaults_to_initial_value() -> Result<()> {
    let cvar = ConsoleVar::new(
        "developer",
        "0",
        ConsoleFlags::empty(),
        "Enable developer output",
    )?;

    assert_eq!(cvar.value(), "0");
    Ok(())
}

#[test]
fn cvar_name_is_canonicalized() -> Result<()> {
    let cvar = ConsoleVar::new(
        "Developer",
        "0",
        ConsoleFlags::empty(),
        "Enable developer output",
    )?;

    assert_eq!(cvar.name().canonical(), "developer");
    Ok(())
}

#[test]
fn cvar_flags_can_be_combined_independently_of_command_flags() -> Result<()> {
    let cvar = ConsoleVar::new(
        "password",
        "secret",
        ConsoleFlags::PROTECTED | ConsoleFlags::ARCHIVE,
        "Protected value",
    )?;

    assert!(cvar.flags().contains(ConsoleFlags::PROTECTED));
    assert!(cvar.flags().contains(ConsoleFlags::ARCHIVE));
    Ok(())
}

#[test]
fn executor_runs_builtin_echo() -> Result<()> {
    let executor = Executor::with_builtins();
    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_assigns_command_id_to_output_frames() -> Result<()> {
    let executor = Executor::with_builtins();
    let result = executor.execute_line("echo hello")?;

    assert!(result.frames().iter().all(|frame| frame.command_id() == 1));
    Ok(())
}

#[test]
fn executor_assigns_sequence_to_output_frames() -> Result<()> {
    let executor = Executor::with_builtins();
    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.frames()[0].sequence(), 1);
    Ok(())
}

#[test]
fn executor_returns_error_for_unknown_command() -> Result<()> {
    let executor = Executor::with_builtins();
    let result = executor.execute_line("missing_command")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn executor_help_missing_topic_returns_failed_status_and_error() -> Result<()> {
    let executor = Executor::with_builtins();
    let result = executor.execute_line("help missing_command")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert_eq!(
        result.error().map(AegisError::code),
        Some(aegis_core::error::AegisErrorCode::CommandNotFound)
    );
    Ok(())
}

#[test]
fn executor_reads_cvar_by_name() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "developer",
        "0",
        ConsoleFlags::empty(),
        "Enable developer output",
    )?;

    let result = executor.execute_line("developer")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_writes_cvar_by_name_and_value() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "developer",
        "0",
        ConsoleFlags::empty(),
        "Enable developer output",
    )?;

    let result = executor.execute_line("developer 1")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_rejects_unquoted_cvar_value_with_spaces() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "player_name",
        "player",
        ConsoleFlags::empty(),
        "Player name",
    )?;

    let result = executor.execute_line("player_name hello world")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn executor_cvar_failure_preserves_specific_error() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar("build_number", "1", ConsoleFlags::READ_ONLY, "Build number")?;

    let result = executor.execute_line("build_number 2")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert_eq!(
        result.error().map(AegisError::message),
        Some("cvar is read-only")
    );
    Ok(())
}

#[test]
fn executor_builtin_get_failure_preserves_specific_error() -> Result<()> {
    let executor = Executor::with_builtins();

    let result = executor.execute_line("get missing_cvar")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert_eq!(
        result.error().map(AegisError::message),
        Some("cvar is not registered")
    );
    Ok(())
}

#[test]
fn executor_accepts_quoted_cvar_value_with_spaces() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "player_name",
        "player",
        ConsoleFlags::empty(),
        "Player name",
    )?;

    let result = executor.execute_line(r#"player_name "hello world""#)?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_blocks_read_only_cvar_write() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar("build_number", "1", ConsoleFlags::READ_ONLY, "Build number")?;

    let result = executor.execute_line("build_number 2")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn executor_blocks_cheat_cvar_write_when_cheats_are_disabled() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar("god_mode", "0", ConsoleFlags::CHEAT, "Enable god mode")?;

    let result = executor.execute_line("god_mode 1")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn executor_reports_cheat_before_read_only_for_cvar_write() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "server_locked",
        "0",
        ConsoleFlags::CHEAT | ConsoleFlags::READ_ONLY,
        "Server locked value",
    )?;

    let result = executor.execute_line("server_locked 1")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert!(
        result
            .frames()
            .iter()
            .any(|frame| frame.payload().contains("cheat"))
    );
    Ok(())
}

#[test]
fn executor_masks_protected_cvar_read_output() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "password",
        "secret",
        ConsoleFlags::PROTECTED,
        "Protected value",
    )?;

    let result = executor.execute_line("password")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert!(
        result
            .frames()
            .iter()
            .all(|frame| !frame.payload().contains("secret"))
    );
    assert!(
        result
            .frames()
            .iter()
            .any(|frame| frame.payload().contains("***"))
    );
    Ok(())
}

#[test]
fn executor_blocks_cheat_command_when_cheats_are_disabled() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_command(
        "debug_dump",
        ConsoleFlags::CHEAT,
        "Dump debug state",
        |_ctx, _args| Ok(CommandStatus::Success),
    )?;

    let result = executor.execute_line("debug_dump")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn executor_rejects_non_printable_cvar_write() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "player_name",
        "player",
        ConsoleFlags::PRINTABLE_ONLY,
        "Player name",
    )?;

    let result = executor.execute_line("player_name \u{0007}")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    Ok(())
}

#[test]
fn hidden_cvar_is_omitted_from_cvars_builtin() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "internal_token",
        "secret",
        ConsoleFlags::HIDDEN,
        "Internal token",
    )?;

    let result = executor.execute_line("cvars")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert!(
        result
            .frames()
            .iter()
            .all(|frame| !frame.payload().contains("internal_token"))
    );
    Ok(())
}

#[test]
fn notify_cvar_write_emits_state_changed_frame() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "developer",
        "0",
        ConsoleFlags::NOTIFY,
        "Enable developer output",
    )?;

    let result = executor.execute_line("developer 1")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert!(
        result
            .frames()
            .iter()
            .any(|frame| frame.kind() == OutputFrameKind::StateChanged)
    );
    Ok(())
}

#[test]
fn executor_dispatches_output_to_sink() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_sink = Arc::clone(&observed);
    executor.set_output_sink(move |_frame| {
        observed_sink.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert_eq!(observed.load(Ordering::SeqCst), result.frames().len());
    Ok(())
}

#[test]
fn set_output_sink_preserves_plugin_output_sinks() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let plugin_observed = Arc::new(AtomicUsize::new(0));
    let plugin_sink = Arc::clone(&plugin_observed);
    executor.register_plugin(
        PluginDescriptor::new("host.logging", "Host Logging"),
        |plugin| {
            plugin.register_output_sink("plugin_sink", move |_frame| {
                plugin_sink.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        },
    )?;

    let direct_observed = Arc::new(AtomicUsize::new(0));
    let direct_sink = Arc::clone(&direct_observed);
    executor.set_output_sink(move |_frame| {
        direct_sink.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert!(plugin_observed.load(Ordering::SeqCst) > 0);
    assert!(direct_observed.load(Ordering::SeqCst) > 0);
    Ok(())
}

#[test]
fn plugin_registration_rolls_back_capabilities_on_error() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let error = expect_error(
        executor.register_plugin(
            PluginDescriptor::new("host.partial", "Host Partial"),
            |plugin| {
                plugin.register_cvar(
                    "partial_value",
                    "0",
                    ConsoleFlags::empty(),
                    "Partially registered value",
                )?;
                Err(AegisError::plugin("plugin setup failed"))
            },
        ),
        "plugin registration failure should be returned",
    )?;

    assert_eq!(error.message(), "plugin setup failed");
    assert!(!executor.contains_plugin("host.partial"));
    assert!(
        !executor
            .cvars()
            .iter()
            .any(|cvar| cvar.name().canonical() == "partial_value")
    );
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn plugin_registration_panic_returns_error_and_rolls_back_capabilities() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let registration = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.register_plugin(
            PluginDescriptor::new("host.panicking", "Host Panicking"),
            |plugin| {
                plugin.register_cvar(
                    "panic_value",
                    "0",
                    ConsoleFlags::empty(),
                    "Partially registered value",
                )?;
                panic!("plugin setup panic");
            },
        )
    }));
    let registration = match registration {
        Ok(registration) => registration,
        Err(_) => {
            return Err(AegisError::internal(
                "plugin registration panic should not cross public API",
            ));
        }
    };
    let error = expect_error(
        registration,
        "plugin registration panic should be converted to an error",
    )?;

    assert_eq!(error.message(), "plugin registration panicked");
    assert!(!executor.contains_plugin("host.panicking"));
    assert!(
        !executor
            .cvars()
            .iter()
            .any(|cvar| cvar.name().canonical() == "panic_value")
    );
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn executor_converts_panic_in_command_handler_to_failed_result() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_command(
        "panic_command",
        ConsoleFlags::empty(),
        "Panic command",
        |_ctx, _args| -> Result<CommandStatus> {
            panic!("host command panic");
        },
    )?;

    let result = executor.execute_line("panic_command")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert!(
        result
            .frames()
            .iter()
            .any(|frame| frame.payload().contains("panicked"))
    );
    Ok(())
}

#[test]
#[allow(clippy::panic)]
fn executor_converts_panic_in_output_sink_to_diagnostic_frame() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.set_output_sink(|_frame| -> Result<()> {
        panic!("host sink panic");
    });

    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert_eq!(result.frames()[0].sequence(), 1);
    assert_eq!(result.frames()[1].sequence(), 2);
    assert!(
        result
            .frames()
            .iter()
            .any(|frame| frame.payload().contains("panicked"))
    );
    Ok(())
}
