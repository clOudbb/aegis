//! Contract tests for script runner public APIs.

use core::time::Duration;
use std::thread;

use aegis_core::cancel::CancellationToken;
use aegis_core::error::{AegisError, Result};
use aegis_core::executor::{CommandStatus, Executor};
use aegis_core::flags::ConsoleFlags;
use aegis_core::script::{ScriptFailurePolicy, ScriptOptions, ScriptRunner};

#[test]
fn script_options_default_policy_stops_on_error() {
    let options = ScriptOptions::default();

    assert_eq!(options.failure_policy(), ScriptFailurePolicy::StopOnError);
}

#[test]
fn script_options_default_max_commands_is_bounded() {
    let options = ScriptOptions::default();

    assert_eq!(options.max_commands(), 1024);
}

#[test]
fn script_options_default_timeout_is_disabled() {
    let options = ScriptOptions::default();

    assert_eq!(options.timeout(), None);
}

#[test]
fn script_options_can_set_timeout() {
    let options = ScriptOptions::default().with_timeout(Duration::from_millis(10));

    assert_eq!(options.timeout(), Some(Duration::from_millis(10)));
}

#[test]
fn script_runner_executes_commands_in_order() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let result =
        runner.execute_script("test.cfg", "echo one\necho two", ScriptOptions::default())?;

    assert_eq!(result.executed_commands(), 2);
    Ok(())
}

#[test]
fn script_runner_stops_on_unknown_command_by_default() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let result = runner.execute_script(
        "test.cfg",
        "echo one\nmissing\necho two",
        ScriptOptions::default(),
    )?;

    assert_eq!(result.executed_commands(), 2);
    assert!(result.is_failed());
    Ok(())
}

#[test]
fn script_runner_rejects_scripts_over_max_command_limit() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let options = ScriptOptions::default().with_max_commands(1);
    let error = match runner.execute_script("test.cfg", "echo one\necho two", options) {
        Ok(_) => {
            return Err(AegisError::internal(
                "script over command limit should fail",
            ));
        }
        Err(error) => error,
    };

    assert_eq!(error.message(), "script command count exceeds maximum");
    Ok(())
}

#[test]
fn script_runner_continue_on_error_executes_remaining_commands() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let options =
        ScriptOptions::default().with_failure_policy(ScriptFailurePolicy::ContinueOnError);
    let result = runner.execute_script("test.cfg", "missing\necho two", options)?;

    assert_eq!(result.executed_commands(), 2);
    Ok(())
}

#[test]
fn script_runner_collect_errors_marks_script_failed_when_any_command_fails() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let options = ScriptOptions::default().with_failure_policy(ScriptFailurePolicy::CollectErrors);
    let result = runner.execute_script("test.cfg", "missing\necho two", options)?;

    assert_eq!(result.executed_commands(), 2);
    assert!(result.is_failed());
    assert_eq!(result.errors().len(), 1);
    Ok(())
}

#[test]
fn script_runner_observes_pre_cancelled_token() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let token = CancellationToken::new();
    token.cancel();

    let result = runner.execute_script_with_cancellation(
        "test.cfg",
        "echo one",
        ScriptOptions::default(),
        &token,
    )?;

    assert_eq!(result.executed_commands(), 0);
    assert!(result.is_failed());
    Ok(())
}

#[test]
fn script_runner_stops_after_command_requests_cancellation() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let token = CancellationToken::new();
    let command_token = token.clone();
    executor.register_command(
        "cancel_now",
        ConsoleFlags::empty(),
        "Cancel script",
        move |_ctx, _args| {
            command_token.cancel();
            Ok(CommandStatus::Success)
        },
    )?;
    let runner = ScriptRunner::new(&executor);

    let result = runner.execute_script_with_cancellation(
        "test.cfg",
        "cancel_now\necho skipped",
        ScriptOptions::default(),
        &token,
    )?;

    assert_eq!(result.executed_commands(), 1);
    assert!(result.is_failed());
    Ok(())
}

#[test]
fn script_runner_observes_pre_expired_timeout() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let options = ScriptOptions::default().with_timeout(Duration::from_nanos(0));

    let result = runner.execute_script_with_cancellation(
        "test.cfg",
        "echo one",
        options,
        &CancellationToken::new(),
    )?;

    assert_eq!(result.executed_commands(), 0);
    assert!(result.is_failed());
    Ok(())
}

#[test]
fn script_runner_stops_after_command_exceeds_timeout() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_command(
        "slow_command",
        ConsoleFlags::empty(),
        "Slow command",
        |_ctx, _args| {
            thread::sleep(Duration::from_millis(5));
            Ok(CommandStatus::Success)
        },
    )?;
    let runner = ScriptRunner::new(&executor);
    let options = ScriptOptions::default().with_timeout(Duration::from_millis(1));

    let result = runner.execute_script_with_cancellation(
        "test.cfg",
        "slow_command\necho skipped",
        options,
        &CancellationToken::new(),
    )?;

    assert_eq!(result.executed_commands(), 1);
    assert!(result.is_failed());
    Ok(())
}
