//! Synchronous command executor.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::authority::ExecutionAuthority;
use crate::builtin;
use crate::context::ExecutionContext;
use crate::cvar::ConsoleVar;
use crate::error::{AegisError, Result};
use crate::flags::ConsoleFlags;
use crate::hook::{ExecutionHookPoint, HookContext, HookDecision, HookDispatcher, HookMatcher};
use crate::output::OutputFrame;
use crate::parser::{CommandArg, CommandInvocation, Parser};
use crate::plugin::{PluginDescriptor, PluginId, PluginRegistrar, PluginRegistry};
use crate::query::{CompletionItem, CompletionKind, HelpTopic, HelpTopicKind};
use crate::registry::{CommandMetadata, CommandRegistry};
use crate::sink::OutputSink;

/// Status returned by a command handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandStatus {
    /// Command completed successfully.
    Success,
    /// Command completed with a command-level failure.
    Failed,
}

impl From<()> for CommandStatus {
    fn from((): ()) -> Self {
        Self::Success
    }
}

/// Overall execution status for a command invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionStatus {
    /// Invocation succeeded.
    Success,
    /// Invocation failed without crashing the core.
    Failed,
    /// Invocation was blocked by a hook or policy.
    Blocked,
}

/// Result of executing one command line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    status: ExecutionStatus,
    error: Option<AegisError>,
    frames: Vec<OutputFrame>,
}

impl ExecutionResult {
    /// Create an execution result.
    pub fn new(status: ExecutionStatus, frames: Vec<OutputFrame>) -> Self {
        Self {
            status,
            error: None,
            frames,
        }
    }

    /// Return this result with a structured failure reason.
    pub fn with_error(mut self, error: AegisError) -> Self {
        self.error = Some(error);
        self
    }

    /// Return execution status.
    pub const fn status(&self) -> ExecutionStatus {
        self.status
    }

    /// Return collected output frames.
    pub fn frames(&self) -> &[OutputFrame] {
        &self.frames
    }

    /// Return the structured failure reason when present.
    pub fn error(&self) -> Option<&AegisError> {
        self.error.as_ref()
    }

    /// Consume and return frames.
    pub fn into_frames(self) -> Vec<OutputFrame> {
        self.frames
    }

    pub(crate) fn push_frame(&mut self, frame: OutputFrame) {
        self.frames.push(frame);
    }
}

type CommandHandler =
    Arc<dyn Fn(&mut ExecutionContext, &[CommandArg]) -> Result<CommandStatus> + Send + Sync>;

#[derive(Clone)]
struct CommandEntry {
    handler: CommandHandler,
}

/// Synchronous console command executor.
pub struct Executor {
    registry: RefCell<CommandRegistry>,
    handlers: RefCell<BTreeMap<String, CommandEntry>>,
    plugins: PluginRegistry,
    plugin_output_sink_keys: BTreeSet<(String, String)>,
    hook_dispatcher: HookDispatcher,
    parser: Parser,
    authority: ExecutionAuthority,
    direct_output_sinks: Vec<OutputSink>,
    plugin_output_sinks: Vec<OutputSink>,
}

impl Executor {
    /// Create an empty executor.
    pub fn new() -> Self {
        Self {
            registry: RefCell::new(CommandRegistry::new()),
            handlers: RefCell::new(BTreeMap::new()),
            plugins: PluginRegistry::new(),
            plugin_output_sink_keys: BTreeSet::new(),
            hook_dispatcher: HookDispatcher::new(),
            parser: Parser::new(),
            authority: ExecutionAuthority::new(),
            direct_output_sinks: Vec::new(),
            plugin_output_sinks: Vec::new(),
        }
    }

    /// Create an executor with builtin commands registered.
    pub fn with_builtins() -> Self {
        let mut executor = Self::new();
        let _ = builtin::register_builtins(&mut executor);
        executor
    }

    /// Return the execution authority.
    pub const fn authority(&self) -> ExecutionAuthority {
        self.authority
    }

    /// Set host execution authority.
    pub fn set_authority(&mut self, authority: ExecutionAuthority) {
        self.authority = authority;
    }

    /// Register a command.
    pub fn register_command<F, S>(
        &mut self,
        name: &str,
        flags: ConsoleFlags,
        description: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&mut ExecutionContext, &[CommandArg]) -> Result<S> + Send + Sync + 'static,
        S: Into<CommandStatus> + 'static,
    {
        self.register_command_with_owner(None, name, flags, description, handler)
    }

    pub(crate) fn register_command_with_owner<F, S>(
        &mut self,
        owner_plugin_id: Option<String>,
        name: &str,
        flags: ConsoleFlags,
        description: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&mut ExecutionContext, &[CommandArg]) -> Result<S> + Send + Sync + 'static,
        S: Into<CommandStatus> + 'static,
    {
        let canonical = crate::parser::CommandName::parse(name)?
            .canonical()
            .to_owned();
        let mut metadata = CommandMetadata::new(name, description)?.with_flags(flags);
        if let Some(owner_plugin_id) = owner_plugin_id {
            metadata = metadata.with_owner_plugin_id(owner_plugin_id);
        }
        self.registry.borrow_mut().register_metadata(metadata)?;
        let handler = Arc::new(move |ctx: &mut ExecutionContext, args: &[CommandArg]| {
            handler(ctx, args).map(Into::into)
        });
        self.handlers
            .borrow_mut()
            .insert(canonical, CommandEntry { handler });
        Ok(())
    }

    /// Register builtin command metadata.
    pub fn register_builtin_command(
        &mut self,
        name: &str,
        flags: ConsoleFlags,
        description: &str,
    ) -> Result<()> {
        self.registry
            .borrow_mut()
            .register_metadata(CommandMetadata::new(name, description)?.with_flags(flags))
    }

    /// Register a cvar.
    pub fn register_cvar(
        &mut self,
        name: &str,
        default_value: &str,
        flags: ConsoleFlags,
        description: &str,
    ) -> Result<()> {
        self.register_cvar_value(ConsoleVar::new(name, default_value, flags, description)?)
    }

    pub(crate) fn register_cvar_value(&mut self, cvar: ConsoleVar) -> Result<()> {
        self.registry.borrow_mut().register_cvar(cvar)
    }

    /// Register a plugin and install capabilities through a restricted registrar.
    pub fn register_plugin<F>(&mut self, descriptor: PluginDescriptor, register: F) -> Result<()>
    where
        F: FnOnce(&mut PluginRegistrar<'_>) -> Result<()>,
    {
        let plugin_id = PluginId::parse(descriptor.id().original())?;
        let registry_snapshot = self.registry.borrow().clone();
        let handlers_snapshot = self.handlers.borrow().clone();
        let plugins_snapshot = self.plugins.clone();
        let plugin_output_sink_keys_snapshot = self.plugin_output_sink_keys.clone();
        let hook_dispatcher_snapshot = self.hook_dispatcher.clone();
        let direct_output_sinks_snapshot = self.direct_output_sinks.clone();
        let plugin_output_sinks_snapshot = self.plugin_output_sinks.clone();

        self.plugins.register(descriptor)?;

        let mut registrar = PluginRegistrar::new(plugin_id, self);
        if let Err(error) = register(&mut registrar) {
            *self.registry.borrow_mut() = registry_snapshot;
            *self.handlers.borrow_mut() = handlers_snapshot;
            self.plugins = plugins_snapshot;
            self.plugin_output_sink_keys = plugin_output_sink_keys_snapshot;
            self.hook_dispatcher = hook_dispatcher_snapshot;
            self.direct_output_sinks = direct_output_sinks_snapshot;
            self.plugin_output_sinks = plugin_output_sinks_snapshot;
            return Err(error);
        }

        Ok(())
    }

    pub(crate) fn register_output_sink_with_owner<F>(
        &mut self,
        plugin_id: &str,
        sink_id: &str,
        sink: F,
    ) -> Result<()>
    where
        F: Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static,
    {
        let key = (plugin_id.to_owned(), sink_id.to_owned());
        if !self.plugin_output_sink_keys.insert(key) {
            return Err(AegisError::registry("output sink is already registered"));
        }

        self.plugin_output_sinks.push(OutputSink::new(sink));
        Ok(())
    }

    pub(crate) fn register_execution_hook_with_owner<F>(
        &mut self,
        plugin_id: &str,
        point: ExecutionHookPoint,
        matcher: HookMatcher,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&HookContext) -> Result<HookDecision> + Send + Sync + 'static,
    {
        self.hook_dispatcher.register_with_owner(
            Some(plugin_id.to_owned()),
            point,
            matcher,
            handler,
        )
    }

    pub(crate) fn dispatch_execution_hook(&self, context: &HookContext) -> Result<HookDecision> {
        self.hook_dispatcher.dispatch(context)
    }

    /// Return whether a plugin id has been registered.
    pub fn contains_plugin(&self, plugin_id: &str) -> bool {
        self.plugins.contains(plugin_id)
    }

    /// Register a command owned by an existing plugin.
    pub fn register_plugin_command<F, S>(
        &mut self,
        plugin_id: &str,
        name: &str,
        flags: ConsoleFlags,
        description: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&mut ExecutionContext, &[CommandArg]) -> Result<S> + Send + Sync + 'static,
        S: Into<CommandStatus> + 'static,
    {
        let plugin_id = self.plugins.get(plugin_id)?.id().canonical().to_owned();
        self.register_command_with_owner(Some(plugin_id), name, flags, description, handler)
    }

    /// Register a cvar owned by an existing plugin.
    pub fn register_plugin_cvar(
        &mut self,
        plugin_id: &str,
        name: &str,
        default_value: &str,
        flags: ConsoleFlags,
        description: &str,
    ) -> Result<()> {
        let plugin_id = self.plugins.get(plugin_id)?.id().canonical().to_owned();
        let cvar = ConsoleVar::new(name, default_value, flags, description)?
            .with_owner_plugin_id(plugin_id);
        self.register_cvar_value(cvar)
    }

    /// Set a direct host output sink, replacing previously configured direct sinks.
    pub fn set_output_sink<F>(&mut self, sink: F)
    where
        F: Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static,
    {
        self.direct_output_sinks.clear();
        self.direct_output_sinks.push(OutputSink::new(sink));
    }

    /// Add a direct host output sink.
    pub fn add_output_sink<F>(&mut self, sink: F)
    where
        F: Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static,
    {
        self.direct_output_sinks.push(OutputSink::new(sink));
    }

    /// Execute one command line.
    pub fn execute_line(&self, input: &str) -> Result<ExecutionResult> {
        let invocation = self.parser.parse_line(input)?;
        self.execute_invocation(&invocation, 1)
    }

    /// Execute a parsed invocation with an explicit result-local command id.
    pub fn execute_invocation(
        &self,
        invocation: &CommandInvocation,
        command_id: u64,
    ) -> Result<ExecutionResult> {
        let mut context = ExecutionContext::new(command_id, self.output_sinks());
        let name = invocation.command().canonical();

        let before = self.dispatch_execution_hook(&HookContext::command(
            ExecutionHookPoint::BeforeExecute,
            name,
        ))?;
        if before.is_blocked() {
            let reason = before.reason().unwrap_or("command execution was blocked");
            context.write_warning(reason);
            return Ok(
                ExecutionResult::new(ExecutionStatus::Blocked, context.into_frames())
                    .with_error(AegisError::permission_denied(reason)),
            );
        }

        let mut result = self.execute_invocation_inner(invocation, context);
        self.dispatch_after_execute_hook(name, command_id, &mut result);
        Ok(result)
    }

    fn execute_invocation_inner(
        &self,
        invocation: &CommandInvocation,
        mut context: ExecutionContext,
    ) -> ExecutionResult {
        let name = invocation.command().canonical();

        if let Some(metadata) = self.command_metadata(name) {
            return self.execute_command(invocation, metadata, context);
        }

        if self.registry.borrow().contains_cvar(name) {
            let status = self.execute_cvar(invocation, &mut context);
            return ExecutionResult::new(status, context.into_frames());
        }

        let message = format!("command not found: {}", invocation.command().original());
        context.write_error(message.clone());
        ExecutionResult::new(ExecutionStatus::Failed, context.into_frames())
            .with_error(AegisError::command_not_found(message))
    }

    fn output_sinks(&self) -> Vec<OutputSink> {
        let mut sinks =
            Vec::with_capacity(self.direct_output_sinks.len() + self.plugin_output_sinks.len());
        sinks.extend(self.direct_output_sinks.iter().cloned());
        sinks.extend(self.plugin_output_sinks.iter().cloned());
        sinks
    }

    /// Return a snapshot of command metadata.
    pub fn command_metadata(&self, name: &str) -> Option<CommandMetadata> {
        self.registry.borrow().get_command(name).ok().cloned()
    }

    /// Return a snapshot of registered commands.
    pub fn commands(&self) -> Vec<CommandMetadata> {
        self.registry.borrow().commands().cloned().collect()
    }

    /// Return a snapshot of registered cvars.
    pub fn cvars(&self) -> Vec<ConsoleVar> {
        self.registry.borrow().cvars().cloned().collect()
    }

    /// Return completion candidates for a command/cvar prefix.
    pub fn complete(&self, prefix: &str) -> Result<Vec<CompletionItem>> {
        if !prefix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
        {
            return Err(AegisError::parse(
                "completion prefix contains invalid characters",
            ));
        }

        let prefix = prefix.to_ascii_lowercase();
        let registry = self.registry.borrow();
        let mut items = Vec::new();

        for command in registry
            .commands()
            .filter(|command| !command.flags().contains(ConsoleFlags::HIDDEN))
            .filter(|command| command.name().canonical().starts_with(prefix.as_str()))
        {
            let name = command.name().canonical();
            items.push(CompletionItem::new(name, name, CompletionKind::Command));
        }

        for cvar in registry
            .cvars()
            .filter(|cvar| !cvar.flags().contains(ConsoleFlags::HIDDEN))
            .filter(|cvar| cvar.name().canonical().starts_with(prefix.as_str()))
        {
            let name = cvar.name().canonical();
            items.push(CompletionItem::new(name, name, CompletionKind::CVar));
        }

        Ok(items)
    }

    /// Return help metadata for a registered command or cvar.
    pub fn help(&self, name: &str) -> Result<HelpTopic> {
        let registry = self.registry.borrow();

        if let Ok(command) = registry.get_command(name)
            && !command.flags().contains(ConsoleFlags::HIDDEN)
        {
            return Ok(HelpTopic::new(
                command.name().canonical(),
                command.description(),
                HelpTopicKind::Command,
                command.flags(),
                command.owner_plugin_id().map(str::to_owned),
            ));
        }

        if let Ok(cvar) = registry.get_cvar(name)
            && !cvar.flags().contains(ConsoleFlags::HIDDEN)
        {
            return Ok(HelpTopic::new(
                cvar.name().canonical(),
                cvar.description(),
                HelpTopicKind::CVar,
                cvar.flags(),
                cvar.owner_plugin_id().map(str::to_owned),
            ));
        }

        Err(AegisError::command_not_found("help topic not found"))
    }

    fn execute_command(
        &self,
        invocation: &CommandInvocation,
        metadata: CommandMetadata,
        mut context: ExecutionContext,
    ) -> ExecutionResult {
        if metadata.flags().contains(ConsoleFlags::CHEAT) && !self.authority.cheats_enabled() {
            let error = AegisError::permission_denied(
                "cheat-protected command cannot run while cheats are disabled",
            );
            context.write_error(error.message());
            return ExecutionResult::new(ExecutionStatus::Failed, context.into_frames())
                .with_error(error);
        }

        let mut execution_error = None;
        let status = match metadata.name().canonical() {
            "echo" => builtin_echo(&mut context, invocation.args()),
            "commands" => {
                self.builtin_commands(&mut context);
                CommandStatus::Success
            }
            "cvars" => {
                self.builtin_cvars(&mut context);
                CommandStatus::Success
            }
            "help" => {
                self.builtin_help(&mut context, invocation.args());
                CommandStatus::Success
            }
            "get" => self.builtin_get(&mut context, invocation.args()),
            "set" => self.builtin_set(&mut context, invocation.args()),
            command_name => {
                let handler = self
                    .handlers
                    .borrow()
                    .get(command_name)
                    .map(|entry| Arc::clone(&entry.handler));
                let Some(handler) = handler else {
                    let error = AegisError::internal("command handler is not registered");
                    context.write_error(error.message());
                    return ExecutionResult::new(ExecutionStatus::Failed, context.into_frames())
                        .with_error(error);
                };
                match catch_unwind(AssertUnwindSafe(|| {
                    handler(&mut context, invocation.args())
                })) {
                    Ok(Ok(status)) => status,
                    Ok(Err(error)) => {
                        context.write_error(error.message());
                        execution_error = Some(error);
                        CommandStatus::Failed
                    }
                    Err(_) => {
                        let error = AegisError::internal("command handler panicked");
                        context.write_error(error.message());
                        execution_error = Some(error);
                        CommandStatus::Failed
                    }
                }
            }
        };

        let execution_status = match status {
            CommandStatus::Success => ExecutionStatus::Success,
            CommandStatus::Failed => ExecutionStatus::Failed,
        };
        let mut result = ExecutionResult::new(execution_status, context.into_frames());
        if let Some(error) = execution_error {
            result = result.with_error(error);
        } else if execution_status == ExecutionStatus::Failed {
            result = result.with_error(AegisError::invalid_argument("command failed"));
        }
        result
    }

    fn dispatch_after_execute_hook(
        &self,
        name: &str,
        command_id: u64,
        result: &mut ExecutionResult,
    ) {
        match self.dispatch_execution_hook(&HookContext::command(
            ExecutionHookPoint::AfterExecute,
            name,
        )) {
            Ok(decision) if decision.is_blocked() => {
                let next_sequence = result.frames().len() as u64 + 1;
                result.push_frame(
                    OutputFrame::diagnostic(
                        decision
                            .reason()
                            .unwrap_or("after-execute hook block was ignored"),
                    )
                    .with_command_id(command_id)
                    .with_sequence(next_sequence),
                );
            }
            Ok(_) => {}
            Err(error) => {
                let next_sequence = result.frames().len() as u64 + 1;
                result.push_frame(
                    OutputFrame::diagnostic(error.message())
                        .with_command_id(command_id)
                        .with_sequence(next_sequence),
                );
            }
        }
    }

    fn execute_cvar(
        &self,
        invocation: &CommandInvocation,
        context: &mut ExecutionContext,
    ) -> ExecutionStatus {
        match invocation.args() {
            [] => self.read_cvar(context, invocation.command().canonical()),
            [value] => self.write_cvar(context, invocation.command().canonical(), value.as_str()),
            _ => {
                context.write_error("cvar write accepts exactly one value");
                ExecutionStatus::Failed
            }
        }
    }

    fn read_cvar(&self, context: &mut ExecutionContext, name: &str) -> ExecutionStatus {
        let registry = self.registry.borrow();
        let cvar = match registry.get_cvar(name) {
            Ok(cvar) => cvar,
            Err(error) => {
                context.write_error(error.message());
                return ExecutionStatus::Failed;
            }
        };
        let value = display_cvar_value(cvar);
        context.write_text(format!("{} = {}", cvar.name().canonical(), value));
        ExecutionStatus::Success
    }

    fn write_cvar(
        &self,
        context: &mut ExecutionContext,
        name: &str,
        value: &str,
    ) -> ExecutionStatus {
        let mut registry = self.registry.borrow_mut();
        let cvar = match registry.get_cvar_mut(name) {
            Ok(cvar) => cvar,
            Err(error) => {
                context.write_error(error.message());
                return ExecutionStatus::Failed;
            }
        };

        if cvar.flags().contains(ConsoleFlags::CHEAT) && !self.authority.cheats_enabled() {
            context.write_error("cheat-protected cvar cannot change while cheats are disabled");
            return ExecutionStatus::Failed;
        }
        if cvar.flags().contains(ConsoleFlags::READ_ONLY) {
            context.write_error("cvar is read-only");
            return ExecutionStatus::Failed;
        }
        if cvar.flags().contains(ConsoleFlags::PRINTABLE_ONLY)
            && value.chars().any(char::is_control)
        {
            context.write_error("cvar value must contain printable characters only");
            return ExecutionStatus::Failed;
        }

        cvar.set_value(value);
        context.write_text(format!(
            "{} = {}",
            cvar.name().canonical(),
            display_cvar_value(cvar)
        ));
        if cvar.flags().contains(ConsoleFlags::NOTIFY) {
            context.write_frame(OutputFrame::state_changed(format!(
                "{} changed",
                cvar.name().canonical()
            )));
        }
        ExecutionStatus::Success
    }

    fn builtin_commands(&self, context: &mut ExecutionContext) {
        for command in self
            .registry
            .borrow()
            .commands()
            .filter(|command| !command.flags().contains(ConsoleFlags::HIDDEN))
        {
            context.write_text(format!(
                "{} - {}",
                command.name().canonical(),
                command.description()
            ));
        }
    }

    fn builtin_cvars(&self, context: &mut ExecutionContext) {
        for cvar in self
            .registry
            .borrow()
            .cvars()
            .filter(|cvar| !cvar.flags().contains(ConsoleFlags::HIDDEN))
        {
            context.write_text(format!(
                "{} - {}",
                cvar.name().canonical(),
                cvar.description()
            ));
        }
    }

    fn builtin_help(&self, context: &mut ExecutionContext, args: &[CommandArg]) {
        let Some(name) = args.first() else {
            context.write_text("usage: help <command-or-cvar>");
            return;
        };

        if let Ok(command) = self.registry.borrow().get_command(name.as_str()) {
            if !command.flags().contains(ConsoleFlags::HIDDEN) {
                context.write_text(format!(
                    "{} - {}",
                    command.name().canonical(),
                    command.description()
                ));
            }
            return;
        }

        if let Ok(cvar) = self.registry.borrow().get_cvar(name.as_str()) {
            if !cvar.flags().contains(ConsoleFlags::HIDDEN) {
                context.write_text(format!(
                    "{} - {}",
                    cvar.name().canonical(),
                    cvar.description()
                ));
            }
            return;
        }

        context.write_error("help topic not found");
    }

    fn builtin_get(&self, context: &mut ExecutionContext, args: &[CommandArg]) -> CommandStatus {
        let Some(name) = args.first() else {
            context.write_error("get requires a cvar name");
            return CommandStatus::Failed;
        };
        match self.read_cvar(context, name.as_str()) {
            ExecutionStatus::Success => CommandStatus::Success,
            ExecutionStatus::Failed | ExecutionStatus::Blocked => CommandStatus::Failed,
        }
    }

    fn builtin_set(&self, context: &mut ExecutionContext, args: &[CommandArg]) -> CommandStatus {
        let [name, value] = args else {
            context.write_error("set requires a cvar name and one value");
            return CommandStatus::Failed;
        };
        match self.write_cvar(context, name.as_str(), value.as_str()) {
            ExecutionStatus::Success => CommandStatus::Success,
            ExecutionStatus::Failed | ExecutionStatus::Blocked => CommandStatus::Failed,
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_echo(context: &mut ExecutionContext, args: &[CommandArg]) -> CommandStatus {
    let mut text = String::new();
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            text.push(' ');
        }
        text.push_str(arg.as_str());
    }
    context.write_text(text);
    CommandStatus::Success
}

fn display_cvar_value(cvar: &ConsoleVar) -> &str {
    if cvar.flags().contains(ConsoleFlags::PROTECTED) {
        "***"
    } else {
        cvar.value()
    }
}
