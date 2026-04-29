//! Command parsing.
//!
//! The parser turns text into command invocations. It does not execute
//! commands, resolve registered names, or inspect the command registry.

use crate::error::{AegisError, Result};

/// Parsed command name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandName {
    original: String,
    canonical: String,
}

impl CommandName {
    /// Create a command name from user input.
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(AegisError::parse("command name is empty"));
        }

        if !input
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
        {
            return Err(AegisError::parse(
                "command name contains invalid characters",
            ));
        }

        Ok(Self {
            original: input.to_owned(),
            canonical: input.to_ascii_lowercase(),
        })
    }

    /// Return the original command token.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Return the canonical lookup name.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

/// Parsed command argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandArg {
    value: String,
}

impl CommandArg {
    /// Create an argument from a parsed value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Return the argument value.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// Parsed command invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandInvocation {
    command: CommandName,
    args: Vec<CommandArg>,
}

impl CommandInvocation {
    /// Return the parsed command name.
    pub fn command(&self) -> &CommandName {
        &self.command
    }

    /// Return parsed arguments.
    pub fn args(&self) -> &[CommandArg] {
        &self.args
    }
}

/// Parser for Aegis command input.
#[derive(Clone, Copy, Debug, Default)]
pub struct Parser;

impl Parser {
    /// Create a parser.
    pub const fn new() -> Self {
        Self
    }

    /// Parse a single command line.
    pub fn parse_line(&self, input: &str) -> Result<CommandInvocation> {
        let tokens = split_tokens(input)?;
        let Some((command, args)) = tokens.split_first() else {
            return Err(AegisError::parse("command line is empty"));
        };

        Ok(CommandInvocation {
            command: CommandName::parse(command)?,
            args: args
                .iter()
                .map(|arg| CommandArg::new(arg.as_str()))
                .collect(),
        })
    }

    /// Parse a script into command invocations.
    pub fn parse_script(&self, input: &str) -> Result<Vec<CommandInvocation>> {
        self.parse_script_commands(input, None)
    }

    pub(crate) fn parse_script_with_max_commands(
        &self,
        input: &str,
        max_commands: usize,
    ) -> Result<Vec<CommandInvocation>> {
        self.parse_script_commands(input, Some(max_commands))
    }

    fn parse_script_commands(
        &self,
        input: &str,
        max_commands: Option<usize>,
    ) -> Result<Vec<CommandInvocation>> {
        let mut invocations = Vec::new();

        for command in split_commands(input, max_commands)? {
            if command.trim().is_empty() {
                continue;
            }
            invocations.push(self.parse_line(command.trim())?);
        }

        Ok(invocations)
    }
}

fn split_tokens(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quote = false;
    let mut token_started = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                token_started = true;
            }
            '\\' if in_quote => {
                let Some(next) = chars.next() else {
                    return Err(AegisError::parse("escape sequence is incomplete"));
                };
                token_started = true;
                current.push(next);
            }
            ch if ch.is_whitespace() && !in_quote => {
                if token_started {
                    tokens.push(core::mem::take(&mut current));
                    token_started = false;
                }
            }
            _ => {
                token_started = true;
                current.push(ch);
            }
        }
    }

    if in_quote {
        return Err(AegisError::parse("quoted string is not closed"));
    }

    if token_started {
        tokens.push(current);
    }

    Ok(tokens)
}

fn split_commands(input: &str, max_commands: Option<usize>) -> Result<Vec<String>> {
    let mut commands = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\\' if in_quote => {
                current.push(ch);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '"' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            '/' if !in_quote && chars.peek() == Some(&'/') && is_comment_start(&current) => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
                if !current.trim().is_empty() {
                    push_command(&mut commands, &mut current, max_commands)?;
                }
            }
            ';' | '\n' if !in_quote => {
                if !current.trim().is_empty() {
                    push_command(&mut commands, &mut current, max_commands)?;
                }
            }
            _ => current.push(ch),
        }
    }

    if in_quote {
        return Err(AegisError::parse("quoted string is not closed"));
    }

    if !current.trim().is_empty() {
        push_command(&mut commands, &mut current, max_commands)?;
    }

    Ok(commands)
}

fn push_command(
    commands: &mut Vec<String>,
    current: &mut String,
    max_commands: Option<usize>,
) -> Result<()> {
    if let Some(max_commands) = max_commands
        && commands.len() >= max_commands
    {
        return Err(AegisError::script("script command count exceeds maximum"));
    }
    commands.push(core::mem::take(current));
    Ok(())
}

fn is_comment_start(current: &str) -> bool {
    current.is_empty() || current.chars().last().is_some_and(char::is_whitespace)
}
