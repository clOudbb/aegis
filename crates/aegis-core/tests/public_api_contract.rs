//! Direct contract tests for public aegis-core APIs.

use core::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use aegis_core::authority::ExecutionAuthority;
use aegis_core::builtin::register_builtins;
use aegis_core::cancel::CancellationToken;
use aegis_core::context::ExecutionContext;
use aegis_core::cvar::ConsoleVar;
use aegis_core::error::{AegisError, AegisErrorCode, Result};
use aegis_core::executor::{CommandStatus, ExecutionResult, ExecutionStatus, Executor};
use aegis_core::flags::ConsoleFlags;
use aegis_core::hook::{
    ExecutionHookPoint, HookContext, HookDecision, HookDispatcher, HookMatcher,
};
use aegis_core::output::{OUTPUT_SCHEMA_VERSION, OutputChannel, OutputFrame, OutputFrameKind};
use aegis_core::parser::{CommandArg, CommandName, Parser};
use aegis_core::plugin::{PluginDescriptor, PluginId, PluginRegistry};
use aegis_core::query::{CompletionItem, CompletionKind, HelpTopic, HelpTopicKind};
use aegis_core::registry::{CommandMetadata, CommandRegistry};
use aegis_core::script::{ScriptExecutionResult, ScriptOptions, ScriptRunner};
use aegis_core::sink::OutputSink;

fn expect_error<T>(result: Result<T>, message: &str) -> Result<AegisError> {
    match result {
        Ok(_) => Err(AegisError::internal(message)),
        Err(error) => Ok(error),
    }
}

#[test]
fn execution_authority_defaults_to_cheats_disabled() {
    assert!(!ExecutionAuthority::default().cheats_enabled());
}

#[test]
fn execution_authority_can_enable_cheats() {
    assert!(ExecutionAuthority::with_cheats_enabled(true).cheats_enabled());
}

#[test]
fn cancellation_token_clone_observes_cancellation() {
    let token = CancellationToken::new();
    let clone = token.clone();

    token.cancel();

    assert!(clone.is_cancelled());
}

#[test]
fn command_name_parse_accepts_public_separators() -> Result<()> {
    let name = CommandName::parse("Host.Debug-test:value_1")?;

    assert_eq!(name.original(), "Host.Debug-test:value_1");
    assert_eq!(name.canonical(), "host.debug-test:value_1");
    Ok(())
}

#[test]
fn command_name_parse_rejects_empty_name() -> Result<()> {
    let error = expect_error(CommandName::parse(""), "empty command name should fail")?;

    assert_eq!(error.message(), "command name is empty");
    Ok(())
}

#[test]
fn command_arg_new_preserves_value() {
    let arg = CommandArg::new("hello");

    assert_eq!(arg.as_str(), "hello");
}

#[test]
fn parser_rejects_empty_line() -> Result<()> {
    let error = expect_error(Parser::new().parse_line("   "), "empty line should fail")?;

    assert_eq!(error.message(), "command line is empty");
    Ok(())
}

#[test]
fn parser_rejects_incomplete_quoted_escape() -> Result<()> {
    let error = expect_error(
        Parser::new().parse_line(r#"echo "abc\"#),
        "incomplete escape should fail",
    )?;

    assert_eq!(error.message(), "escape sequence is incomplete");
    Ok(())
}

#[test]
fn output_frame_new_uses_explicit_kind_and_channel() {
    let frame = OutputFrame::new(OutputFrameKind::Json, OutputChannel::Debug, "{}");

    assert_eq!(frame.schema_version(), OUTPUT_SCHEMA_VERSION);
    assert_eq!(frame.kind(), OutputFrameKind::Json);
    assert_eq!(frame.channel(), OutputChannel::Debug);
    assert_eq!(frame.payload(), "{}");
}

#[test]
fn output_frame_builders_cover_public_kinds_and_channels() {
    assert_eq!(
        OutputFrame::warning("warn").kind(),
        OutputFrameKind::Warning
    );
    assert_eq!(OutputFrame::error("err").kind(), OutputFrameKind::Error);
    assert_eq!(
        OutputFrame::diagnostic("diag").channel(),
        OutputChannel::System
    );
    assert_eq!(
        OutputFrame::state_changed("state").kind(),
        OutputFrameKind::StateChanged
    );
}

#[test]
fn output_frame_supports_remaining_semantic_kinds() {
    assert_eq!(
        OutputFrame::new(OutputFrameKind::Table, OutputChannel::Main, "table").kind(),
        OutputFrameKind::Table
    );
    assert_eq!(
        OutputFrame::new(OutputFrameKind::Log, OutputChannel::Debug, "log").kind(),
        OutputFrameKind::Log
    );
    assert_eq!(
        OutputFrame::new(OutputFrameKind::Progress, OutputChannel::Main, "50").kind(),
        OutputFrameKind::Progress
    );
}

#[test]
fn output_frame_withers_assign_command_id_and_sequence() {
    let frame = OutputFrame::text("hello")
        .with_command_id(42)
        .with_sequence(7);

    assert_eq!(frame.command_id(), 42);
    assert_eq!(frame.sequence(), 7);
}

#[test]
fn error_constructors_cover_public_error_codes() {
    let cases = [
        (AegisError::parse("parse"), AegisErrorCode::ParseError, 100),
        (
            AegisError::registry("registry"),
            AegisErrorCode::RegistryError,
            200,
        ),
        (
            AegisError::command_not_found("missing"),
            AegisErrorCode::CommandNotFound,
            300,
        ),
        (
            AegisError::invalid_argument("invalid"),
            AegisErrorCode::InvalidArgument,
            400,
        ),
        (
            AegisError::permission_denied("denied"),
            AegisErrorCode::PermissionDenied,
            500,
        ),
        (
            AegisError::cancelled("cancelled"),
            AegisErrorCode::Cancelled,
            600,
        ),
        (AegisError::timeout("timeout"), AegisErrorCode::Timeout, 700),
        (
            AegisError::script("script"),
            AegisErrorCode::ScriptError,
            800,
        ),
        (
            AegisError::plugin("plugin"),
            AegisErrorCode::PluginError,
            900,
        ),
        (
            AegisError::internal("internal"),
            AegisErrorCode::InternalError,
            1_000,
        ),
        (AegisError::ffi("ffi"), AegisErrorCode::FfiError, 1_100),
    ];

    for (error, code, numeric_code) in cases {
        assert_eq!(error.code(), code);
        assert_eq!(code.as_u32(), numeric_code);
    }
}

#[test]
fn error_new_and_display_preserve_message() {
    let error = AegisError::new(AegisErrorCode::FfiError, "ffi failed");

    assert_eq!(error.message(), "ffi failed");
    assert_eq!(error.to_string(), "ffi failed");
}

#[test]
fn console_flags_empty_has_no_bits() {
    let flags = ConsoleFlags::empty();

    assert!(flags.is_empty());
    assert_eq!(flags.bits(), 0);
}

#[test]
fn console_flags_from_bits_retain_preserves_unknown_bits() {
    let flags = ConsoleFlags::from_bits_retain(1 << 31);

    assert_eq!(flags.bits(), 1 << 31);
}

#[test]
fn console_flags_bitor_assign_adds_bits() {
    let mut flags = ConsoleFlags::ARCHIVE;

    flags |= ConsoleFlags::NOTIFY;

    assert!(flags.contains(ConsoleFlags::ARCHIVE));
    assert!(flags.contains(ConsoleFlags::NOTIFY));
}

#[test]
fn console_var_preserves_metadata_and_owner() -> Result<()> {
    let cvar = ConsoleVar::new("developer", "0", ConsoleFlags::ARCHIVE, "Developer mode")?
        .with_owner_plugin_id("host.settings");

    assert_eq!(cvar.default_value(), "0");
    assert_eq!(cvar.description(), "Developer mode");
    assert_eq!(cvar.owner_plugin_id(), Some("host.settings"));
    Ok(())
}

#[test]
fn command_metadata_preserves_description_flags_and_owner() -> Result<()> {
    let metadata = CommandMetadata::new("debug_dump", "Dump debug state")?
        .with_flags(ConsoleFlags::CHEAT)
        .with_owner_plugin_id("host.debug");

    assert_eq!(metadata.description(), "Dump debug state");
    assert!(metadata.flags().contains(ConsoleFlags::CHEAT));
    assert_eq!(metadata.owner_plugin_id(), Some("host.debug"));
    Ok(())
}

#[test]
fn registry_exposes_cvar_queries_and_iterators() -> Result<()> {
    let mut registry = CommandRegistry::new();
    registry.register_cvar(ConsoleVar::new(
        "developer",
        "0",
        ConsoleFlags::empty(),
        "Developer mode",
    )?)?;

    assert!(registry.contains_cvar("Developer"));
    assert_eq!(registry.get_cvar("developer")?.value(), "0");
    assert_eq!(registry.cvars().count(), 1);
    assert_eq!(registry.commands().count(), 0);
    Ok(())
}

#[test]
fn registry_rejects_command_and_cvar_name_conflicts() -> Result<()> {
    let mut cvar_first = CommandRegistry::new();
    cvar_first.register_cvar(ConsoleVar::new(
        "developer",
        "0",
        ConsoleFlags::empty(),
        "Developer mode",
    )?)?;
    let error = expect_error(
        cvar_first.register_metadata(CommandMetadata::new("Developer", "Command")?),
        "command should not shadow cvar",
    )?;
    assert_eq!(error.message(), "name is already registered as cvar");

    let mut command_first = CommandRegistry::new();
    command_first.register_metadata(CommandMetadata::new("developer", "Command")?)?;
    let error = expect_error(
        command_first.register_cvar(ConsoleVar::new(
            "Developer",
            "0",
            ConsoleFlags::empty(),
            "Developer mode",
        )?),
        "cvar should not shadow command",
    )?;
    assert_eq!(error.message(), "name is already registered as command");
    Ok(())
}

#[test]
fn registry_missing_cvar_returns_command_not_found() -> Result<()> {
    let registry = CommandRegistry::new();
    let error = expect_error(registry.get_cvar("developer"), "missing cvar should fail")?;

    assert_eq!(error.code(), AegisErrorCode::CommandNotFound);
    assert!(!registry.contains_cvar("/invalid"));
    Ok(())
}

#[test]
fn execution_context_write_helpers_collect_typed_frames() {
    let mut context = ExecutionContext::new(7, Vec::new());

    context.write_text("hello");
    context.write_error("error");
    context.write_warning("warning");

    let frames = context.into_frames();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].kind(), OutputFrameKind::Text);
    assert_eq!(frames[1].kind(), OutputFrameKind::Error);
    assert_eq!(frames[2].kind(), OutputFrameKind::Warning);
    assert!(frames.iter().all(|frame| frame.command_id() == 7));
    assert_eq!(frames[2].sequence(), 3);
}

#[test]
fn execution_context_records_diagnostic_when_sink_fails() {
    let sink = OutputSink::new(|_frame| Err(AegisError::internal("sink failed")));
    let mut context = ExecutionContext::new(9, vec![sink]);

    context.write_frame(OutputFrame::text("hello"));

    let frames = context.into_frames();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1].kind(), OutputFrameKind::Diagnostic);
    assert_eq!(frames[1].payload(), "sink failed");
    assert_eq!(frames[1].command_id(), 9);
}

#[test]
fn output_sink_dispatch_invokes_callback() -> Result<()> {
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_sink = Arc::clone(&observed);
    let sink = OutputSink::new(move |_frame| {
        observed_sink.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    sink.dispatch(&OutputFrame::text("hello"))?;

    assert_eq!(observed.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn output_sink_dispatch_returns_handler_error() -> Result<()> {
    let sink = OutputSink::new(|_frame| Err(AegisError::internal("sink failed")));
    let error = expect_error(
        sink.dispatch(&OutputFrame::text("hello")),
        "sink error should propagate",
    )?;

    assert_eq!(error.message(), "sink failed");
    Ok(())
}

#[test]
fn execution_result_accessors_preserve_frames_and_error() {
    let frame = OutputFrame::text("hello");
    let result = ExecutionResult::new(ExecutionStatus::Failed, vec![frame.clone()])
        .with_error(AegisError::invalid_argument("bad args"));

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert_eq!(result.frames(), &[frame]);
    assert_eq!(result.error().map(AegisError::message), Some("bad args"));
    assert_eq!(result.into_frames().len(), 1);
}

#[test]
fn executor_new_starts_without_builtins() -> Result<()> {
    let executor = Executor::new();
    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Failed);
    assert_eq!(
        result.error().map(AegisError::code),
        Some(AegisErrorCode::CommandNotFound)
    );
    Ok(())
}

#[test]
fn executor_default_matches_empty_executor() {
    let executor = Executor::default();

    assert!(executor.commands().is_empty());
}

#[test]
fn executor_can_enable_cheat_command_with_authority() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.set_authority(ExecutionAuthority::with_cheats_enabled(true));
    executor.register_command(
        "debug_dump",
        ConsoleFlags::CHEAT,
        "Dump debug state",
        |ctx, _args| {
            ctx.write_text("debug");
            Ok(())
        },
    )?;

    let result = executor.execute_line("debug_dump")?;

    assert_eq!(
        executor.authority(),
        ExecutionAuthority::with_cheats_enabled(true)
    );
    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_can_enable_cheat_cvar_write_with_authority() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.set_authority(ExecutionAuthority::with_cheats_enabled(true));
    executor.register_cvar("god_mode", "0", ConsoleFlags::CHEAT, "God mode")?;

    let result = executor.execute_line("god_mode 1")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert_eq!(executor.cvars()[0].value(), "1");
    Ok(())
}

#[test]
fn executor_register_command_accepts_unit_status() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_command(
        "host_ping",
        ConsoleFlags::empty(),
        "Host ping",
        |ctx, _args| {
            ctx.write_text("pong");
            Ok(())
        },
    )?;

    let result = executor.execute_line("host_ping")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    Ok(())
}

#[test]
fn executor_custom_command_receives_args() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_command(
        "first_arg",
        ConsoleFlags::empty(),
        "Print first arg",
        |ctx, args| {
            if let Some(arg) = args.first() {
                ctx.write_text(arg.as_str());
            }
            Ok(CommandStatus::Success)
        },
    )?;

    let result = executor.execute_line("first_arg alpha")?;

    assert_eq!(result.frames()[0].payload(), "alpha");
    Ok(())
}

#[test]
fn executor_add_output_sink_preserves_multiple_direct_sinks() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    let first_sink = Arc::clone(&first);
    let second_sink = Arc::clone(&second);
    executor.add_output_sink(move |_frame| {
        first_sink.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    executor.add_output_sink(move |_frame| {
        second_sink.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    let result = executor.execute_line("echo hello")?;

    assert_eq!(result.status(), ExecutionStatus::Success);
    assert_eq!(first.load(Ordering::SeqCst), 1);
    assert_eq!(second.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn executor_exposes_command_and_cvar_snapshots() -> Result<()> {
    let mut executor = Executor::new();
    executor.register_command(
        "host_ping",
        ConsoleFlags::empty(),
        "Host ping",
        |_ctx, _args| Ok(()),
    )?;
    executor.register_cvar("developer", "0", ConsoleFlags::empty(), "Developer mode")?;

    assert_eq!(
        executor.command_metadata("host_ping").map(|metadata| {
            (
                metadata.name().canonical().to_owned(),
                metadata.description().to_owned(),
            )
        }),
        Some(("host_ping".to_owned(), "Host ping".to_owned()))
    );
    assert_eq!(executor.commands().len(), 1);
    assert_eq!(executor.cvars().len(), 1);
    Ok(())
}

#[test]
fn executor_completion_rejects_invalid_prefix() -> Result<()> {
    let executor = Executor::with_builtins();
    let error = expect_error(
        executor.complete("/"),
        "invalid completion prefix should fail",
    )?;

    assert_eq!(
        error.message(),
        "completion prefix contains invalid characters"
    );
    Ok(())
}

#[test]
fn executor_help_returns_cvar_topic() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar("developer", "0", ConsoleFlags::ARCHIVE, "Developer mode")?;

    let topic = executor.help("developer")?;

    assert_eq!(topic.kind(), HelpTopicKind::CVar);
    assert!(topic.flags().contains(ConsoleFlags::ARCHIVE));
    Ok(())
}

#[test]
fn executor_help_hides_hidden_topic() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_cvar(
        "internal_token",
        "secret",
        ConsoleFlags::HIDDEN,
        "Internal token",
    )?;

    let error = expect_error(
        executor.help("internal_token"),
        "hidden help topic should not resolve",
    )?;

    assert_eq!(error.message(), "help topic not found");
    Ok(())
}

#[test]
fn executor_register_plugin_capabilities_require_existing_plugin() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let error = expect_error(
        executor.register_plugin_cvar(
            "host.missing",
            "host_mode",
            "normal",
            ConsoleFlags::empty(),
            "Host mode",
        ),
        "missing plugin cvar owner should fail",
    )?;

    assert_eq!(error.message(), "plugin is not registered");
    Ok(())
}

#[test]
fn executor_register_plugin_command_records_owner() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.direct", "Host Direct"),
        |_plugin| Ok(()),
    )?;
    executor.register_plugin_command(
        "host.direct",
        "host_ping",
        ConsoleFlags::empty(),
        "Host ping",
        |_ctx, _args| Ok(CommandStatus::Success),
    )?;

    let metadata = executor.command_metadata("host_ping");

    assert_eq!(
        metadata.and_then(|metadata| metadata.owner_plugin_id().map(str::to_owned)),
        Some("host.direct".to_owned())
    );
    Ok(())
}

#[test]
fn executor_register_plugin_cvar_records_owner() -> Result<()> {
    let mut executor = Executor::with_builtins();
    executor.register_plugin(
        PluginDescriptor::new("host.direct", "Host Direct"),
        |_plugin| Ok(()),
    )?;
    executor.register_plugin_cvar(
        "host.direct",
        "host_mode",
        "normal",
        ConsoleFlags::empty(),
        "Host mode",
    )?;

    assert_eq!(executor.cvars()[0].owner_plugin_id(), Some("host.direct"));
    Ok(())
}

#[test]
fn register_builtins_adds_known_commands() -> Result<()> {
    let mut executor = Executor::new();

    register_builtins(&mut executor)?;

    assert!(executor.command_metadata("echo").is_some());
    assert!(executor.command_metadata("help").is_some());
    Ok(())
}

#[test]
fn hook_point_block_permissions_are_stable() {
    assert!(ExecutionHookPoint::BeforeExecute.allows_block());
    assert!(ExecutionHookPoint::BeforeScriptExecute.allows_block());
    assert!(!ExecutionHookPoint::AfterExecute.allows_block());
    assert!(!ExecutionHookPoint::AfterScriptExecute.allows_block());
}

#[test]
fn hook_decision_allow_and_block_report_state() {
    let allow = HookDecision::allow();
    let block = HookDecision::block("disabled");

    assert!(allow.is_allowed());
    assert!(!allow.is_blocked());
    assert!(block.is_blocked());
    assert_eq!(block.reason(), Some("disabled"));
}

#[test]
fn hook_matcher_command_canonicalizes_name() -> Result<()> {
    let matcher = HookMatcher::command("Echo")?;

    assert_eq!(matcher.command_name(), Some("echo"));
    Ok(())
}

#[test]
fn hook_matcher_command_rejects_invalid_name() -> Result<()> {
    let error = expect_error(HookMatcher::command("/echo"), "invalid matcher should fail")?;

    assert_eq!(error.message(), "command name contains invalid characters");
    Ok(())
}

#[test]
fn hook_context_command_and_script_expose_fields() {
    let command = HookContext::command(ExecutionHookPoint::BeforeExecute, "echo");
    let script = HookContext::script(ExecutionHookPoint::BeforeScriptExecute, "test.cfg");

    assert_eq!(command.point(), ExecutionHookPoint::BeforeExecute);
    assert_eq!(command.command_name(), Some("echo"));
    assert_eq!(script.source_name(), Some("test.cfg"));
}

#[test]
fn hook_dispatcher_runs_matching_hook() -> Result<()> {
    let mut dispatcher = HookDispatcher::new();
    dispatcher.register(
        ExecutionHookPoint::BeforeExecute,
        HookMatcher::command("echo")?,
        |_context| Ok(HookDecision::block("echo disabled")),
    )?;

    let decision = dispatcher.dispatch(&HookContext::command(
        ExecutionHookPoint::BeforeExecute,
        "echo",
    ))?;

    assert_eq!(decision.reason(), Some("echo disabled"));
    Ok(())
}

#[test]
fn hook_dispatcher_skips_nonmatching_hook() -> Result<()> {
    let mut dispatcher = HookDispatcher::new();
    dispatcher.register(
        ExecutionHookPoint::BeforeExecute,
        HookMatcher::command("echo")?,
        |_context| Ok(HookDecision::block("echo disabled")),
    )?;

    let decision = dispatcher.dispatch(&HookContext::command(
        ExecutionHookPoint::BeforeExecute,
        "status",
    ))?;

    assert!(decision.is_allowed());
    Ok(())
}

#[test]
fn hook_dispatcher_ignores_block_from_after_hook() -> Result<()> {
    let mut dispatcher = HookDispatcher::new();
    dispatcher.register(
        ExecutionHookPoint::AfterExecute,
        HookMatcher::any(),
        |_context| Ok(HookDecision::block("too late")),
    )?;

    let decision = dispatcher.dispatch(&HookContext::command(
        ExecutionHookPoint::AfterExecute,
        "echo",
    ))?;

    assert!(decision.is_allowed());
    Ok(())
}

#[test]
fn hook_dispatcher_propagates_handler_error() -> Result<()> {
    let mut dispatcher = HookDispatcher::new();
    dispatcher.register(
        ExecutionHookPoint::BeforeExecute,
        HookMatcher::any(),
        |_context| Err(AegisError::internal("hook failed")),
    )?;
    let error = expect_error(
        dispatcher.dispatch(&HookContext::command(
            ExecutionHookPoint::BeforeExecute,
            "echo",
        )),
        "hook error should propagate",
    )?;

    assert_eq!(error.message(), "hook failed");
    Ok(())
}

#[test]
fn completion_item_exposes_label_insert_text_and_kind() {
    let item = CompletionItem::new("arg", "--arg", CompletionKind::Argument);

    assert_eq!(item.label(), "arg");
    assert_eq!(item.insert_text(), "--arg");
    assert_eq!(item.kind(), CompletionKind::Argument);
}

#[test]
fn help_topic_exposes_all_metadata() {
    let topic = HelpTopic::new(
        "host_ping",
        "Host ping",
        HelpTopicKind::Command,
        ConsoleFlags::CHEAT,
        Some("host.debug".to_owned()),
    );

    assert_eq!(topic.name(), "host_ping");
    assert_eq!(topic.description(), "Host ping");
    assert_eq!(topic.kind(), HelpTopicKind::Command);
    assert!(topic.flags().contains(ConsoleFlags::CHEAT));
    assert_eq!(topic.owner_plugin_id(), Some("host.debug"));
}

#[test]
fn plugin_id_parse_canonicalizes_and_rejects_invalid_ids() -> Result<()> {
    let plugin_id = PluginId::parse("Host.Debug")?;
    let error = expect_error(
        PluginId::parse("host/debug"),
        "invalid plugin id should fail",
    )?;

    assert_eq!(plugin_id.original(), "Host.Debug");
    assert_eq!(plugin_id.canonical(), "host.debug");
    assert_eq!(error.message(), "plugin id contains invalid characters");
    Ok(())
}

#[test]
fn plugin_id_parse_rejects_empty_id() -> Result<()> {
    let error = expect_error(PluginId::parse(""), "empty plugin id should fail")?;

    assert_eq!(error.message(), "plugin id is empty");
    Ok(())
}

#[test]
fn plugin_descriptor_preserves_raw_id_until_registration() {
    let descriptor = PluginDescriptor::new("Host.Debug", "Host Debug");

    assert_eq!(descriptor.id().original(), "Host.Debug");
    assert_eq!(descriptor.name(), "Host Debug");
}

#[test]
fn plugin_registry_get_and_plugins_iterate_descriptors() -> Result<()> {
    let mut registry = PluginRegistry::new();
    registry.register(PluginDescriptor::new("Host.Debug", "Host Debug"))?;

    assert_eq!(registry.get("host.debug")?.name(), "Host Debug");
    assert_eq!(registry.plugins().count(), 1);
    assert!(!registry.contains("host/debug"));
    Ok(())
}

#[test]
fn plugin_registry_get_rejects_missing_plugin() -> Result<()> {
    let registry = PluginRegistry::new();
    let error = expect_error(registry.get("host.debug"), "missing plugin should fail")?;

    assert_eq!(error.message(), "plugin is not registered");
    Ok(())
}

#[test]
fn plugin_registrar_exposes_canonical_plugin_id() -> Result<()> {
    let mut executor = Executor::with_builtins();

    executor.register_plugin(
        PluginDescriptor::new("Host.Debug", "Host Debug"),
        |plugin| {
            assert_eq!(plugin.plugin_id().canonical(), "host.debug");
            Ok(())
        },
    )?;

    assert!(executor.contains_plugin("host.debug"));
    Ok(())
}

#[test]
fn plugin_registrar_rejects_duplicate_output_sink_id() -> Result<()> {
    let mut executor = Executor::with_builtins();
    let error = expect_error(
        executor.register_plugin(
            PluginDescriptor::new("host.debug", "Host Debug"),
            |plugin| {
                plugin.register_output_sink("host_sink", |_frame| Ok(()))?;
                plugin.register_output_sink("HOST_SINK", |_frame| Ok(()))
            },
        ),
        "duplicate plugin sink should fail",
    )?;

    assert_eq!(error.message(), "output sink is already registered");
    assert!(!executor.contains_plugin("host.debug"));
    Ok(())
}

#[test]
fn script_options_can_set_input_byte_limit() {
    let options = ScriptOptions::default().with_max_input_bytes(32);

    assert_eq!(options.max_input_bytes(), 32);
}

#[test]
fn script_runner_rejects_scripts_over_input_byte_limit() -> Result<()> {
    let executor = Executor::with_builtins();
    let runner = ScriptRunner::new(&executor);
    let options = ScriptOptions::default().with_max_input_bytes(4);
    let error = expect_error(
        runner.execute_script("test.cfg", "echo hello", options),
        "oversized script should fail",
    )?;

    assert_eq!(error.message(), "script input exceeds maximum byte length");
    Ok(())
}

#[test]
fn script_execution_result_new_exposes_source_results_and_errors() {
    let command_result = ExecutionResult::new(ExecutionStatus::Success, Vec::new());
    let error = AegisError::script("script failed");
    let result = ScriptExecutionResult::new(
        "test.cfg",
        vec![command_result.clone()],
        vec![error.clone()],
    );

    assert_eq!(result.source_name(), "test.cfg");
    assert_eq!(result.command_results(), &[command_result]);
    assert_eq!(result.errors(), &[error]);
    assert!(result.diagnostics().is_empty());
}

#[test]
fn script_execution_result_blocked_preserves_reason() {
    let result = ScriptExecutionResult::blocked("test.cfg", "scripts disabled");

    assert_eq!(result.source_name(), "test.cfg");
    assert!(result.is_blocked());
    assert_eq!(result.errors()[0].message(), "scripts disabled");
}

#[test]
fn script_options_builder_methods_can_be_chained() {
    let options = ScriptOptions::default()
        .with_failure_policy(aegis_core::script::ScriptFailurePolicy::ContinueOnError)
        .with_timeout(Duration::from_millis(5))
        .with_max_commands(8)
        .with_max_input_bytes(16);

    assert_eq!(options.max_commands(), 8);
    assert_eq!(options.max_input_bytes(), 16);
    assert_eq!(options.timeout(), Some(Duration::from_millis(5)));
}
