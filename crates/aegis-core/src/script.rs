//! Sequential script runner.
//!
//! The script runner executes host-provided text. It intentionally performs no
//! file IO and uses no async runtime, so desktop, mobile, CLI, and game hosts
//! can decide how script content is loaded and scheduled.

use core::time::Duration;
use std::time::Instant;

use crate::cancel::CancellationToken;
use crate::error::{AegisError, Result};
use crate::executor::{ExecutionResult, ExecutionStatus, Executor};
use crate::hook::{ExecutionHookPoint, HookContext};
use crate::parser::Parser;

/// Policy used when one command in a script fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScriptFailurePolicy {
    /// Continue processing remaining commands after failures.
    ContinueOnError,
    /// Stop at the first failed command.
    StopOnError,
    /// Continue processing remaining commands and report collected failures.
    CollectErrors,
}

/// Script execution options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptOptions {
    failure_policy: ScriptFailurePolicy,
    max_commands: usize,
    max_input_bytes: usize,
    timeout: Option<Duration>,
}

impl ScriptOptions {
    /// Return the failure policy.
    pub const fn failure_policy(&self) -> ScriptFailurePolicy {
        self.failure_policy
    }

    /// Return the maximum number of commands accepted in one script.
    pub const fn max_commands(&self) -> usize {
        self.max_commands
    }

    /// Return the maximum input length accepted in bytes.
    pub const fn max_input_bytes(&self) -> usize {
        self.max_input_bytes
    }

    /// Return the optional total script timeout.
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Return a copy with a different failure policy.
    pub const fn with_failure_policy(mut self, policy: ScriptFailurePolicy) -> Self {
        self.failure_policy = policy;
        self
    }

    /// Return a copy with a different timeout.
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Return a copy with a different command limit.
    pub const fn with_max_commands(mut self, max_commands: usize) -> Self {
        self.max_commands = max_commands;
        self
    }

    /// Return a copy with a different input byte limit.
    pub const fn with_max_input_bytes(mut self, max_input_bytes: usize) -> Self {
        self.max_input_bytes = max_input_bytes;
        self
    }
}

impl Default for ScriptOptions {
    fn default() -> Self {
        Self {
            failure_policy: ScriptFailurePolicy::StopOnError,
            max_commands: 1024,
            max_input_bytes: 64 * 1024,
            timeout: None,
        }
    }
}

/// Result of executing a script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptExecutionResult {
    source_name: String,
    command_results: Vec<ExecutionResult>,
    errors: Vec<AegisError>,
    diagnostics: Vec<AegisError>,
    blocked: bool,
}

impl ScriptExecutionResult {
    /// Create a script execution result.
    pub fn new(
        source_name: impl Into<String>,
        command_results: Vec<ExecutionResult>,
        errors: Vec<AegisError>,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            command_results,
            errors,
            diagnostics: Vec::new(),
            blocked: false,
        }
    }

    /// Create a blocked script result.
    pub fn blocked(source_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            source_name: source_name.into(),
            command_results: Vec::new(),
            errors: vec![AegisError::permission_denied(reason)],
            diagnostics: Vec::new(),
            blocked: true,
        }
    }

    /// Return the host-provided script source name.
    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    /// Return command results retained by the selected failure policy.
    pub fn command_results(&self) -> &[ExecutionResult] {
        &self.command_results
    }

    /// Return collected script-level errors.
    pub fn errors(&self) -> &[AegisError] {
        &self.errors
    }

    /// Return diagnostics that did not affect script execution status.
    pub fn diagnostics(&self) -> &[AegisError] {
        &self.diagnostics
    }

    pub(crate) fn push_diagnostic(&mut self, error: AegisError) {
        self.diagnostics.push(error);
    }

    /// Return the number of command results retained by the script runner.
    pub fn executed_commands(&self) -> usize {
        self.command_results.len()
    }

    /// Return whether the script observed any failure.
    pub fn is_failed(&self) -> bool {
        self.blocked
            || !self.errors.is_empty()
            || self
                .command_results
                .iter()
                .any(|result| result.status() != ExecutionStatus::Success)
    }

    /// Return whether the script was blocked before execution.
    pub const fn is_blocked(&self) -> bool {
        self.blocked
    }
}

/// Synchronous script runner over an executor.
pub struct ScriptRunner<'executor> {
    executor: &'executor Executor,
    parser: Parser,
}

impl<'executor> ScriptRunner<'executor> {
    /// Create a script runner for an executor.
    pub const fn new(executor: &'executor Executor) -> Self {
        Self {
            executor,
            parser: Parser::new(),
        }
    }

    /// Execute a host-provided script text.
    pub fn execute_script(
        &self,
        source_name: &str,
        script: &str,
        options: ScriptOptions,
    ) -> Result<ScriptExecutionResult> {
        self.execute_script_with_cancellation(
            source_name,
            script,
            options,
            &CancellationToken::new(),
        )
    }

    /// Execute a host-provided script text with cooperative cancellation.
    pub fn execute_script_with_cancellation(
        &self,
        source_name: &str,
        script: &str,
        options: ScriptOptions,
        cancellation: &CancellationToken,
    ) -> Result<ScriptExecutionResult> {
        let before = self.executor.dispatch_execution_hook(&HookContext::script(
            ExecutionHookPoint::BeforeScriptExecute,
            source_name,
        ))?;
        if before.is_blocked() {
            return Ok(ScriptExecutionResult::blocked(
                source_name,
                before.reason().unwrap_or("script execution was blocked"),
            ));
        }

        if script.len() > options.max_input_bytes() {
            return Err(AegisError::script(
                "script input exceeds maximum byte length",
            ));
        }

        let invocations = self
            .parser
            .parse_script_with_max_commands(script, options.max_commands())?;

        let started_at = Instant::now();
        let mut command_results = Vec::with_capacity(invocations.len());
        let mut errors = Vec::new();

        for (index, invocation) in invocations.iter().enumerate() {
            if let Some(error) = checkpoint(&options, cancellation, started_at) {
                errors.push(error);
                break;
            }

            let result = self
                .executor
                .execute_invocation(invocation, (index + 1) as u64)?;
            let failed = result.status() != ExecutionStatus::Success;

            match options.failure_policy() {
                ScriptFailurePolicy::StopOnError if failed => {
                    errors.push(command_failure_error(
                        result.status(),
                        invocation.command().canonical(),
                    ));
                    command_results.push(result);
                    break;
                }
                ScriptFailurePolicy::ContinueOnError => {
                    command_results.push(result);
                }
                ScriptFailurePolicy::CollectErrors => {
                    if failed {
                        errors.push(command_failure_error(
                            result.status(),
                            invocation.command().canonical(),
                        ));
                    }
                    command_results.push(result);
                }
                ScriptFailurePolicy::StopOnError => {
                    command_results.push(result);
                }
            }

            if let Some(error) = checkpoint(&options, cancellation, started_at) {
                errors.push(error);
                break;
            }
        }

        let mut result = ScriptExecutionResult::new(source_name, command_results, errors);

        if let Err(error) = self.executor.dispatch_execution_hook(&HookContext::script(
            ExecutionHookPoint::AfterScriptExecute,
            source_name,
        )) {
            result.push_diagnostic(error);
        }

        Ok(result)
    }
}

fn checkpoint(
    options: &ScriptOptions,
    cancellation: &CancellationToken,
    started_at: Instant,
) -> Option<AegisError> {
    if cancellation.is_cancelled() {
        return Some(AegisError::cancelled("script execution was cancelled"));
    }

    if let Some(timeout) = options.timeout()
        && started_at.elapsed() >= timeout
    {
        return Some(AegisError::timeout("script execution timed out"));
    }

    None
}

fn command_failure_error(status: ExecutionStatus, command: &str) -> AegisError {
    let reason = match status {
        ExecutionStatus::Success => "succeeded",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Blocked => "was blocked",
    };
    AegisError::script(format!("command `{command}` {reason}"))
}
