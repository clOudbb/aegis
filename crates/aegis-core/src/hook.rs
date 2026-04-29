//! Command and script execution hooks.
//!
//! Hooks observe command lifecycle points. The first version supports blocking
//! only before command or script execution; after hooks are observe-only.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::error::{AegisError, Result};
use crate::parser::CommandName;

/// Execution lifecycle point for hooks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionHookPoint {
    /// Before one command invocation executes.
    BeforeExecute,
    /// After one command invocation executes.
    AfterExecute,
    /// Before one script starts executing.
    BeforeScriptExecute,
    /// After one script finishes executing.
    AfterScriptExecute,
}

impl ExecutionHookPoint {
    /// Return whether this hook point is allowed to block execution.
    pub const fn allows_block(self) -> bool {
        matches!(self, Self::BeforeExecute | Self::BeforeScriptExecute)
    }
}

/// Decision returned by an execution hook.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDecision {
    blocked_reason: Option<String>,
}

impl HookDecision {
    /// Allow execution to continue.
    pub const fn allow() -> Self {
        Self {
            blocked_reason: None,
        }
    }

    /// Block execution with a human-readable reason.
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            blocked_reason: Some(reason.into()),
        }
    }

    /// Return whether execution is allowed.
    pub const fn is_allowed(&self) -> bool {
        self.blocked_reason.is_none()
    }

    /// Return whether execution is blocked.
    pub const fn is_blocked(&self) -> bool {
        self.blocked_reason.is_some()
    }

    /// Return the block reason when present.
    pub fn reason(&self) -> Option<&str> {
        self.blocked_reason.as_deref()
    }
}

/// Hook matching rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookMatcher {
    command_name: Option<String>,
}

impl HookMatcher {
    /// Match every hook context.
    pub const fn any() -> Self {
        Self { command_name: None }
    }

    /// Match a canonical command or cvar name.
    pub fn command(name: &str) -> Result<Self> {
        Ok(Self {
            command_name: Some(CommandName::parse(name)?.canonical().to_owned()),
        })
    }

    /// Return the command name matched by this matcher.
    pub fn command_name(&self) -> Option<&str> {
        self.command_name.as_deref()
    }

    fn matches(&self, context: &HookContext) -> bool {
        self.command_name
            .as_deref()
            .is_none_or(|expected| context.command_name() == Some(expected))
    }
}

/// Context passed to execution hooks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookContext {
    point: ExecutionHookPoint,
    command_name: Option<String>,
    source_name: Option<String>,
}

impl HookContext {
    /// Create a hook context for one command invocation.
    pub fn command(point: ExecutionHookPoint, command_name: &str) -> Self {
        Self {
            point,
            command_name: Some(command_name.to_owned()),
            source_name: None,
        }
    }

    /// Create a hook context for one script invocation.
    pub fn script(point: ExecutionHookPoint, source_name: &str) -> Self {
        Self {
            point,
            command_name: None,
            source_name: Some(source_name.to_owned()),
        }
    }

    /// Return the lifecycle point.
    pub const fn point(&self) -> ExecutionHookPoint {
        self.point
    }

    /// Return the command name for command hooks.
    pub fn command_name(&self) -> Option<&str> {
        self.command_name.as_deref()
    }

    /// Return the script source name for script hooks.
    pub fn source_name(&self) -> Option<&str> {
        self.source_name.as_deref()
    }
}

type HookHandler = dyn Fn(&HookContext) -> Result<HookDecision> + Send + Sync + 'static;

#[derive(Clone)]
struct HookEntry {
    point: ExecutionHookPoint,
    matcher: HookMatcher,
    owner_plugin_id: Option<String>,
    handler: Arc<HookHandler>,
}

/// Synchronous execution hook dispatcher.
#[derive(Clone, Default)]
pub struct HookDispatcher {
    entries: Vec<HookEntry>,
}

impl HookDispatcher {
    /// Create an empty hook dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an execution hook.
    pub fn register<F>(
        &mut self,
        point: ExecutionHookPoint,
        matcher: HookMatcher,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&HookContext) -> Result<HookDecision> + Send + Sync + 'static,
    {
        self.register_with_owner(None, point, matcher, handler)
    }

    pub(crate) fn register_with_owner<F>(
        &mut self,
        owner_plugin_id: Option<String>,
        point: ExecutionHookPoint,
        matcher: HookMatcher,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&HookContext) -> Result<HookDecision> + Send + Sync + 'static,
    {
        self.entries.push(HookEntry {
            point,
            matcher,
            owner_plugin_id,
            handler: Arc::new(handler),
        });
        Ok(())
    }

    /// Dispatch hooks matching the context.
    pub fn dispatch(&self, context: &HookContext) -> Result<HookDecision> {
        for entry in &self.entries {
            let _owner_plugin_id = entry.owner_plugin_id.as_deref();
            if entry.point != context.point() || !entry.matcher.matches(context) {
                continue;
            }

            let decision = match catch_unwind(AssertUnwindSafe(|| (entry.handler)(context))) {
                Ok(result) => result?,
                Err(_) => return Err(AegisError::internal("execution hook panicked")),
            };
            if decision.is_blocked() && context.point().allows_block() {
                return Ok(decision);
            }
        }

        Ok(HookDecision::allow())
    }
}
