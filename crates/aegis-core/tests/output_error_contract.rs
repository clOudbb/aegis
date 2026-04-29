//! Contract tests for output and error public APIs.

use aegis_core::error::{AegisError, AegisErrorCode};
use aegis_core::output::{OutputChannel, OutputFrame, OutputFrameKind};

#[test]
fn text_frame_uses_schema_version_one() {
    let frame = OutputFrame::text("hello");

    assert_eq!(frame.schema_version(), 1);
}

#[test]
fn text_frame_preserves_payload() {
    let frame = OutputFrame::text("hello");

    assert_eq!(frame.payload(), "hello");
}

#[test]
fn text_frame_uses_text_kind() {
    let frame = OutputFrame::text("hello");

    assert_eq!(frame.kind(), OutputFrameKind::Text);
}

#[test]
fn text_frame_defaults_to_main_channel() {
    let frame = OutputFrame::text("hello");

    assert_eq!(frame.channel(), OutputChannel::Main);
}

#[test]
fn text_frame_defaults_to_no_command_id() {
    let frame = OutputFrame::text("hello");

    assert_eq!(frame.command_id(), 0);
}

#[test]
fn parse_error_reports_parse_error_code() {
    let error = AegisError::parse("unterminated quote");

    assert_eq!(error.code(), AegisErrorCode::ParseError);
}

#[test]
fn parse_error_preserves_message() {
    let error = AegisError::parse("unterminated quote");

    assert_eq!(error.message(), "unterminated quote");
}

#[test]
fn command_not_found_has_stable_numeric_code() {
    assert_eq!(AegisErrorCode::CommandNotFound.as_u32(), 300);
}
