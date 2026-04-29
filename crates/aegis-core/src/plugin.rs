//! Lightweight plugin registration protocol.
//!
//! A plugin is an explicit host-registered ownership unit. It is not a dynamic
//! library, manifest, sandbox, or extension runtime.

use std::collections::BTreeMap;

use crate::context::ExecutionContext;
use crate::cvar::ConsoleVar;
use crate::error::{AegisError, Result};
use crate::executor::{CommandStatus, Executor};
use crate::flags::ConsoleFlags;
use crate::hook::{ExecutionHookPoint, HookContext, HookDecision, HookMatcher};
use crate::output::OutputFrame;
use crate::parser::{CommandArg, CommandName};

/// Canonical plugin namespace identifier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PluginId {
    original: String,
    canonical: String,
}

impl PluginId {
    /// Parse and validate a plugin id.
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(AegisError::registry("plugin id is empty"));
        }

        if !input
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
        {
            return Err(AegisError::registry(
                "plugin id contains invalid characters",
            ));
        }

        Ok(Self {
            original: input.to_owned(),
            canonical: input.to_ascii_lowercase(),
        })
    }

    fn from_raw(input: &str) -> Self {
        Self {
            original: input.to_owned(),
            canonical: input.to_ascii_lowercase(),
        }
    }

    /// Return the original plugin id spelling.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Return the canonical lowercase plugin id.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

/// Plugin identity and display metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDescriptor {
    id: PluginId,
    name: String,
}

impl PluginDescriptor {
    /// Create a plugin descriptor.
    ///
    /// Validation happens when the descriptor is registered so descriptor
    /// construction stays ergonomic for host setup code.
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: PluginId::from_raw(id),
            name: name.to_owned(),
        }
    }

    /// Return the plugin id.
    pub fn id(&self) -> &PluginId {
        &self.id
    }

    /// Return the plugin display name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Registered plugin descriptor store.
#[derive(Clone, Debug, Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, PluginDescriptor>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin descriptor.
    pub fn register(&mut self, mut descriptor: PluginDescriptor) -> Result<()> {
        let parsed_id = PluginId::parse(descriptor.id().original())?;
        let canonical = parsed_id.canonical().to_owned();

        if self.plugins.contains_key(&canonical) {
            return Err(AegisError::registry("plugin is already registered"));
        }

        descriptor.id = parsed_id;
        self.plugins.insert(canonical, descriptor);
        Ok(())
    }

    /// Return whether a plugin id is registered.
    pub fn contains(&self, id: &str) -> bool {
        PluginId::parse(id).is_ok_and(|plugin_id| self.plugins.contains_key(plugin_id.canonical()))
    }

    /// Return a plugin descriptor by id.
    pub fn get(&self, id: &str) -> Result<&PluginDescriptor> {
        let plugin_id = PluginId::parse(id)?;
        self.plugins
            .get(plugin_id.canonical())
            .ok_or_else(|| AegisError::registry("plugin is not registered"))
    }

    /// Return all registered plugin descriptors.
    pub fn plugins(&self) -> impl Iterator<Item = &PluginDescriptor> {
        self.plugins.values()
    }
}

/// Restricted registrar handed to a plugin registration closure.
///
/// Plugin registration is transactional at the executor level. If the
/// registration closure returns an error, capabilities registered through the
/// same registrar are rolled back before the error is returned.
pub struct PluginRegistrar<'executor> {
    plugin_id: PluginId,
    executor: &'executor mut Executor,
}

impl<'executor> PluginRegistrar<'executor> {
    pub(crate) fn new(plugin_id: PluginId, executor: &'executor mut Executor) -> Self {
        Self {
            plugin_id,
            executor,
        }
    }

    /// Return the plugin id associated with this registrar.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Register a plugin-owned command.
    pub fn register_command<F, S>(
        &mut self,
        name: &str,
        description: &str,
        flags: ConsoleFlags,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&mut ExecutionContext, &[CommandArg]) -> Result<S> + Send + Sync + 'static,
        S: Into<CommandStatus> + 'static,
    {
        self.executor.register_command_with_owner(
            Some(self.plugin_id.canonical().to_owned()),
            name,
            flags,
            description,
            handler,
        )
    }

    /// Register a plugin-owned cvar.
    pub fn register_cvar(
        &mut self,
        name: &str,
        default_value: &str,
        flags: ConsoleFlags,
        description: &str,
    ) -> Result<()> {
        let cvar = ConsoleVar::new(name, default_value, flags, description)?
            .with_owner_plugin_id(self.plugin_id.canonical());
        self.executor.register_cvar_value(cvar)
    }

    /// Register a plugin-owned output sink.
    pub fn register_output_sink<F>(&mut self, id: &str, sink: F) -> Result<()>
    where
        F: Fn(&OutputFrame) -> Result<()> + Send + Sync + 'static,
    {
        let sink_id = CommandName::parse(id)?.canonical().to_owned();
        self.executor
            .register_output_sink_with_owner(self.plugin_id.canonical(), &sink_id, sink)
    }

    /// Register a plugin-owned execution hook.
    pub fn register_execution_hook<F>(
        &mut self,
        point: ExecutionHookPoint,
        matcher: HookMatcher,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&HookContext) -> Result<HookDecision> + Send + Sync + 'static,
    {
        self.executor.register_execution_hook_with_owner(
            self.plugin_id.canonical(),
            point,
            matcher,
            handler,
        )
    }
}
