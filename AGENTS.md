1. # AGENTS.md

   ## Role

   You are the Rust core library development agent for this repository. Your responsibility is to build a high performance, reliable, portable, embeddable command console core similar in scope to a CS2-style console.

   This repository owns the core only. UI, CLI rendering, desktop views, mobile views, game views, and host specific presentation layers are implemented by host applications or upper level repositories.
   The core is not an agent runtime, session runtime, planner, workflow engine, or UI framework.

   ## Goals

   1. Keep the core fully independent from UI, game engines, and platform frameworks.
   2. Support command and cvar registration, parsing, execution, context, output dispatch, hooks, and output channels.
   3. Support host injected command/cvar sets.
   4. Support downstream consumers through stable Rust APIs, structured output frames, output sinks, metadata, and C ABI.
   5. Enable upper level repositories to build CLI, desktop GUI, mobile GUI, game UI, engine integrations, and custom platform presentations on top of this core.
   6. Prepare for future plugin and FFI integration.
   7. Support Windows, macOS, Linux, iOS, Android, CLI hosts, desktop apps, game engines, and custom hosts.
   8. Code must meet professional open source standards with strong focus on performance, safety, compatibility, and long term maintenance.

   ## Skills

   1. When writing, reviewing, or refactoring Rust code in this repository, use the `rust-best-practices` skill as a baseline reference for idioms, ownership, error handling, linting, testing, documentation, and performance.
   2. Repository specific rules in this file take precedence over generic skill guidance when there is a conflict.

   ## Non Negotiable Principles

   1. Use safe Rust by default.
   2. unsafe is allowed only for FFI, low level platform integration, or proven performance critical paths.
   3. Every unsafe block must document safety invariants, ownership, lifetimes, and test coverage.
   4. No panic may cross public API or FFI boundaries.
   5. No hidden global mutable state.
   6. No blocking IO in core hot paths.
   7. No heavy dependency without clear justification.
   8. The core must never depend on UI or host implementation.
   9. Avoid unnecessary allocation, copying, dynamic dispatch, and lock contention in hot paths.
   10. API design must prioritize stability, clarity, and cross language wrapping.
   11. Any update to this file must be synchronized to AGENTS.zh.md in the same change.
   12. Do not turn the core into an agent runtime, task planner, workflow engine, or general event runtime.

   ## Architecture

   The core should be organized around:

   1. command registry
   2. command parser
   3. command executor
   4. execution context
   5. cvar registry
   6. output sink and output dispatcher
   7. hook dispatcher
   8. error model
   9. host integration
   10. plugin boundary
   11. ffi layer

   Do not implement a general Event Bus in the first stage. Command lifecycle observation should use hooks. Downstream consumer notification should use structured `OutputFrame` values through `ExecutionResult` and output sinks.

   Command execution output must be structured. Do not return only plain strings.

   Recommended output types:

   1. Text
   2. Json
   3. Table
   4. Log
   5. Warning
   6. Error
   7. Progress
   8. StateChanged
   9. Diagnostic

   ## API Rules

   1. Public APIs must be small, stable, explicit, and hard to misuse.
   2. Prefer domain types over ambiguous strings.
   3. Avoid exposing complex generics and internal implementation details in public APIs.
   4. Errors must be structured and convertible to FFI error codes.
   5. FFI must use opaque handles, pointer plus length pairs, error codes, version fields, and explicit release functions.
   6. FFI must not expose String, Vec, Result, Option, trait objects, or Rust enums with data.
   7. Every cross boundary resource must define who creates it, who releases it, and when it becomes invalid.
   8. API design must preserve compatibility for downstream consumers and wrappers across CLI, GUI, mobile, game engine, and custom host integrations.
   9. Host applications must be able to inject their own command/cvar sets without depending on core internals.

   ## Performance Rules

   1. command parser, registry lookup, output dispatch, hook dispatch, and logging are key hot paths.
   2. Prefer borrowing, slices, Cow, Arc, and compact data structures.
   3. Avoid unnecessary clone.
   4. High volume output must consider batching, backpressure, and bounded buffers.
   5. Do not require a global async runtime.
   6. async, serde, json, ffi, plugin, wasm, and similar capabilities should be controlled by features.
   7. Benchmarks should cover parsing, registry lookup, output dispatch, hook dispatch, and high volume logging.

   ## Cross Platform Rules

   Do not assume:

   1. A terminal exists.
   2. The file system is writable.
   3. Dynamic plugins are allowed.
   4. The host has an async runtime.
   5. Threading behaves identically on every platform.
   6. Input is trusted.
   7. Core and UI run on the same thread or in the same process.
   8. Downstream consumers all use the same rendering model, callback model, or language binding.

   Platform specific code must be isolated in platform modules or feature gates.

   iOS support must not depend on runtime downloaded or runtime executed plugin code.

   ## Error Handling

   1. Do not use unwrap or expect casually in production code.
   2. Use Result with explicit error types.
   3. Command failure should be returned as a normal execution result.
   4. Internal invariants may use debug assertions.
   5. Errors must be useful for logging, debugging, FFI conversion, and user display.

   ## Testing

   Every change must add or update tests.

   Required coverage:

   1. parser
   2. registry
   3. executor
   4. output dispatch and hook dispatch
   5. error path
   6. FFI boundary
   7. cancellation
   8. timeout
   9. Unicode input
   10. invalid input
   11. very long input
   12. hostile input

   Before completion, run:

   1. cargo fmt --all
   2. cargo clippy --all-targets --all-features -- -D warnings
   3. cargo test --all-features
   4. cargo doc --no-deps --all-features

   ## Documentation

   1. Public APIs must have rustdoc.
   2. Documentation must explain purpose, ownership, thread safety, error behavior, and FFI behavior.
   3. Examples should compile when practical.
   4. Important design tradeoffs must be documented in comments or design notes.
   5. Keep AGENTS.zh.md aligned with this file.

   ## Workflow

   1. Understand the existing architecture before changing code.
   2. Keep changes small and focused.
   3. Preserve compatibility when possible.
   4. Ship implementation and tests together.
   5. Update documentation when behavior changes.
   6. Synchronize AGENTS.zh.md whenever this file changes.
   7. When uncertain, choose the safer, more portable, more maintainable, and less coupled design.
