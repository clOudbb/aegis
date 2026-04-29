//! Host-independent console core library.
//!
//! Aegis core owns command registration, parsing, execution context, event
//! dispatch, and structured output primitives for embeddable console hosts.
//! UI rendering, platform presentation, file loading, and host lifecycle
//! management belong outside this crate.
//!
//! # Safety
//!
//! This crate forbids unsafe Rust. FFI-specific code belongs in `aegis-ffi`.
//!
//! # Example
//!
//! ```
//! assert_eq!(aegis_core::CORE_API_VERSION, 1);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod authority;
pub mod builtin;
pub mod cancel;
pub mod context;
pub mod cvar;
pub mod error;
pub mod executor;
pub mod flags;
pub mod hook;
pub mod output;
pub mod parser;
pub mod plugin;
pub mod query;
pub mod registry;
pub mod script;
pub mod sink;

/// Version of the safe Rust core API contract.
///
/// This value is intentionally separate from Cargo package versions. It gives
/// downstream wrappers and the FFI facade a compact compatibility signal.
pub const CORE_API_VERSION: u32 = 1;
