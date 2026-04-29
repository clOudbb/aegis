//! Builtin command metadata registration.

use crate::error::Result;
use crate::executor::Executor;
use crate::flags::ConsoleFlags;

/// Register the builtin command set.
pub fn register_builtins(executor: &mut Executor) -> Result<()> {
    executor.register_builtin_command("echo", ConsoleFlags::empty(), "Print text")?;
    executor.register_builtin_command("help", ConsoleFlags::empty(), "Show help")?;
    executor.register_builtin_command("commands", ConsoleFlags::empty(), "List commands")?;
    executor.register_builtin_command("cvars", ConsoleFlags::empty(), "List cvars")?;
    executor.register_builtin_command("get", ConsoleFlags::empty(), "Read a cvar")?;
    executor.register_builtin_command("set", ConsoleFlags::empty(), "Write a cvar")?;
    Ok(())
}
