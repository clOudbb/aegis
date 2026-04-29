//! Console variable model.

use crate::error::Result;
use crate::flags::ConsoleFlags;
use crate::parser::CommandName;

/// String-backed console variable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsoleVar {
    name: CommandName,
    default_value: String,
    value: String,
    flags: ConsoleFlags,
    description: String,
    owner_plugin_id: Option<String>,
}

impl ConsoleVar {
    /// Create a console variable.
    pub fn new(
        name: &str,
        default_value: &str,
        flags: ConsoleFlags,
        description: &str,
    ) -> Result<Self> {
        Ok(Self {
            name: CommandName::parse(name)?,
            default_value: default_value.to_owned(),
            value: default_value.to_owned(),
            flags,
            description: description.to_owned(),
            owner_plugin_id: None,
        })
    }

    /// Return the parsed cvar name.
    pub fn name(&self) -> &CommandName {
        &self.name
    }

    /// Return the default value.
    pub fn default_value(&self) -> &str {
        &self.default_value
    }

    /// Return the current value.
    pub fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
    }

    /// Return cvar flags.
    pub const fn flags(&self) -> ConsoleFlags {
        self.flags
    }

    /// Return the cvar description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Return the owner plugin id when registered through a plugin.
    pub fn owner_plugin_id(&self) -> Option<&str> {
        self.owner_plugin_id.as_deref()
    }

    /// Return this cvar with an owner plugin id.
    pub fn with_owner_plugin_id(mut self, owner_plugin_id: impl Into<String>) -> Self {
        self.owner_plugin_id = Some(owner_plugin_id.into());
        self
    }
}
