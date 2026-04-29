//! Contract tests for parser public APIs.

use aegis_core::parser::Parser;

#[test]
fn parse_line_canonicalizes_command_name_to_lowercase() -> aegis_core::error::Result<()> {
    let invocation = Parser::new().parse_line("Echo hello")?;

    assert_eq!(invocation.command().canonical(), "echo");
    Ok(())
}

#[test]
fn parse_line_preserves_original_command_name() -> aegis_core::error::Result<()> {
    let invocation = Parser::new().parse_line("Echo hello")?;

    assert_eq!(invocation.command().original(), "Echo");
    Ok(())
}

#[test]
fn parse_line_preserves_unicode_argument() -> aegis_core::error::Result<()> {
    let invocation = Parser::new().parse_line("echo 你好")?;

    assert_eq!(invocation.args()[0].as_str(), "你好");
    Ok(())
}

#[test]
fn parse_line_rejects_slash_in_command_name() -> aegis_core::error::Result<()> {
    let error = match Parser::new().parse_line("/echo hello") {
        Ok(_) => {
            return Err(aegis_core::error::AegisError::internal(
                "slash-prefixed command name should be rejected",
            ));
        }
        Err(error) => error,
    };

    assert_eq!(error.message(), "command name contains invalid characters");
    Ok(())
}

#[test]
fn parse_line_preserves_quoted_whitespace() -> aegis_core::error::Result<()> {
    let invocation = Parser::new().parse_line(r#"echo "hello world""#)?;

    assert_eq!(invocation.args()[0].as_str(), "hello world");
    Ok(())
}

#[test]
fn parse_line_preserves_explicit_empty_quoted_argument() -> aegis_core::error::Result<()> {
    let invocation = Parser::new().parse_line(r#"echo """#)?;

    assert_eq!(invocation.args().len(), 1);
    assert_eq!(invocation.args()[0].as_str(), "");
    Ok(())
}

#[test]
fn parse_script_splits_newlines_and_semicolons() -> aegis_core::error::Result<()> {
    let invocations = Parser::new().parse_script("echo one; echo two\necho three")?;

    assert_eq!(invocations.len(), 3);
    Ok(())
}

#[test]
fn parse_script_ignores_line_comments() -> aegis_core::error::Result<()> {
    let invocations = Parser::new().parse_script("echo one // comment\necho two")?;

    assert_eq!(invocations.len(), 2);
    Ok(())
}

#[test]
fn parse_script_ignores_full_line_comments() -> aegis_core::error::Result<()> {
    let invocations = Parser::new().parse_script("// comment\necho two")?;

    assert_eq!(invocations.len(), 1);
    Ok(())
}

#[test]
fn parse_script_keeps_double_slash_inside_argument() -> aegis_core::error::Result<()> {
    let invocations = Parser::new().parse_script("echo http://example.com")?;

    assert_eq!(invocations[0].args()[0].as_str(), "http://example.com");
    Ok(())
}

#[test]
fn parse_script_keeps_escaped_quote_inside_quoted_argument() -> aegis_core::error::Result<()> {
    let invocations = Parser::new().parse_script(r#"echo "hello \"world\""; echo done"#)?;

    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].args()[0].as_str(), r#"hello "world""#);
    Ok(())
}

#[test]
fn parse_line_rejects_unclosed_quote() -> aegis_core::error::Result<()> {
    let error = match Parser::new().parse_line(r#"echo "unterminated"#) {
        Ok(_) => {
            return Err(aegis_core::error::AegisError::internal(
                "unclosed quote should be rejected",
            ));
        }
        Err(error) => error,
    };

    assert_eq!(error.message(), "quoted string is not closed");
    Ok(())
}
