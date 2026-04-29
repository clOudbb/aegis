//! Command and cvar registry.

use std::collections::BTreeMap;

use crate::cvar::ConsoleVar;
use crate::error::{AegisError, Result};
use crate::flags::ConsoleFlags;
use crate::parser::CommandName;

/// Public command metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandMetadata {
    name: CommandName,
    description: String,
    flags: ConsoleFlags,
    owner_plugin_id: Option<String>,
}

impl CommandMetadata {
    /// Create command metadata with no flags.
    pub fn new(name: &str, description: &str) -> Result<Self> {
        Ok(Self {
            name: CommandName::parse(name)?,
            description: description.to_owned(),
            flags: ConsoleFlags::empty(),
            owner_plugin_id: None,
        })
    }

    /// Return this metadata with flags assigned.
    pub const fn with_flags(mut self, flags: ConsoleFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Return this metadata with an owner plugin id.
    pub fn with_owner_plugin_id(mut self, owner_plugin_id: impl Into<String>) -> Self {
        self.owner_plugin_id = Some(owner_plugin_id.into());
        self
    }

    /// Return the command name.
    pub fn name(&self) -> &CommandName {
        &self.name
    }

    /// Return the command description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Return command flags.
    pub const fn flags(&self) -> ConsoleFlags {
        self.flags
    }

    /// Return the owner plugin id when registered through a plugin.
    pub fn owner_plugin_id(&self) -> Option<&str> {
        self.owner_plugin_id.as_deref()
    }
}

/// Shared command/cvar registry.
#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<String, CommandMetadata>,
    cvars: BTreeMap<String, ConsoleVar>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register command metadata.
    pub fn register_metadata(&mut self, metadata: CommandMetadata) -> Result<()> {
        let parsed_name = CommandName::parse(metadata.name().original())?;
        let canonical = parsed_name.canonical().to_owned();
        if self.commands.contains_key(&canonical) {
            return Err(AegisError::registry("command is already registered"));
        }
        if self.cvars.contains_key(&canonical) {
            return Err(AegisError::registry("name is already registered as cvar"));
        }
        self.commands.insert(canonical, metadata);
        Ok(())
    }

    /// Register a cvar.
    pub fn register_cvar(&mut self, cvar: ConsoleVar) -> Result<()> {
        let canonical = cvar.name().canonical().to_owned();
        if self.cvars.contains_key(&canonical) {
            return Err(AegisError::registry("cvar is already registered"));
        }
        if self.commands.contains_key(&canonical) {
            return Err(AegisError::registry(
                "name is already registered as command",
            ));
        }
        self.cvars.insert(canonical, cvar);
        Ok(())
    }

    /// Return whether a command exists.
    pub fn contains_command(&self, name: &str) -> bool {
        self.command_key(name)
            .is_ok_and(|canonical| self.commands.contains_key(canonical.as_str()))
    }

    /// Get command metadata by name.
    pub fn get_command(&self, name: &str) -> Result<&CommandMetadata> {
        let canonical = self.command_key(name)?;
        self.commands
            .get(canonical.as_str())
            .ok_or_else(|| AegisError::command_not_found("command is not registered"))
    }

    /// Get cvar by name.
    pub fn get_cvar(&self, name: &str) -> Result<&ConsoleVar> {
        let canonical = self.command_key(name)?;
        self.cvars
            .get(canonical.as_str())
            .ok_or_else(|| AegisError::command_not_found("cvar is not registered"))
    }

    pub(crate) fn get_cvar_mut(&mut self, name: &str) -> Result<&mut ConsoleVar> {
        let canonical = self.command_key(name)?;
        self.cvars
            .get_mut(canonical.as_str())
            .ok_or_else(|| AegisError::command_not_found("cvar is not registered"))
    }

    /// Return all command metadata.
    pub fn commands(&self) -> impl Iterator<Item = &CommandMetadata> {
        self.commands.values()
    }

    /// Return all cvars.
    pub fn cvars(&self) -> impl Iterator<Item = &ConsoleVar> {
        self.cvars.values()
    }

    /// Return whether a cvar exists.
    pub fn contains_cvar(&self, name: &str) -> bool {
        self.command_key(name)
            .is_ok_and(|canonical| self.cvars.contains_key(canonical.as_str()))
    }

    fn command_key(&self, name: &str) -> Result<String> {
        Ok(CommandName::parse(name)?.canonical().to_owned())
    }
}
