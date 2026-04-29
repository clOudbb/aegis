//! Registry query data types.
//!
//! Query APIs expose command and cvar metadata to downstream consumers without
//! imposing a UI, formatter, or completion rendering model.

use crate::flags::ConsoleFlags;

/// Completion item kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionKind {
    /// Command completion.
    Command,
    /// Console variable completion.
    CVar,
    /// Argument completion.
    Argument,
}

/// One completion candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionItem {
    label: String,
    insert_text: String,
    kind: CompletionKind,
}

impl CompletionItem {
    /// Create a completion item.
    pub fn new(label: &str, insert_text: &str, kind: CompletionKind) -> Self {
        Self {
            label: label.to_owned(),
            insert_text: insert_text.to_owned(),
            kind,
        }
    }

    /// Return the display label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Return text to insert into the input.
    pub fn insert_text(&self) -> &str {
        &self.insert_text
    }

    /// Return completion kind.
    pub const fn kind(&self) -> CompletionKind {
        self.kind
    }
}

/// Help topic kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpTopicKind {
    /// Command help topic.
    Command,
    /// Console variable help topic.
    CVar,
}

/// Help metadata for a registered command or cvar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpTopic {
    name: String,
    description: String,
    kind: HelpTopicKind,
    flags: ConsoleFlags,
    owner_plugin_id: Option<String>,
}

impl HelpTopic {
    /// Create a help topic.
    pub fn new(
        name: &str,
        description: &str,
        kind: HelpTopicKind,
        flags: ConsoleFlags,
        owner_plugin_id: Option<String>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            kind,
            flags,
            owner_plugin_id,
        }
    }

    /// Return the topic name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the topic description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Return topic kind.
    pub const fn kind(&self) -> HelpTopicKind {
        self.kind
    }

    /// Return topic flags.
    pub const fn flags(&self) -> ConsoleFlags {
        self.flags
    }

    /// Return owning plugin id when present.
    pub fn owner_plugin_id(&self) -> Option<&str> {
        self.owner_plugin_id.as_deref()
    }
}
