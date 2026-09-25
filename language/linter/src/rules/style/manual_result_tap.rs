use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Result tap methods when mapping observes and returns its payload.
    pub MANUAL_RESULT_TAP {
        id: "manual-result-tap",
        summary: "Prefer Result tap methods when mapping observes and returns its payload",
        explanation: r#"
Mapping a Result payload only to observe it before returning it unchanged obscures the side effect.
Instead, you SHOULD use `tap` for successful values and `tapErr` for errors.
"#,
        example: {
            reported: r#"
declare function record(value: &readonly int32): void;

function inspect(result: Result<int32, string>): Result<int32, string> {
    return result.map((value) => {
        record(value);
        value
    });
}
"#,
            accepted: r#"
declare function record(value: &readonly int32): void;

function inspect(result: Result<int32, string>): Result<int32, string> {
    return result.tap((value) => {
        record(value);
    });
}
"#,
        },
        provenance: [Clippy("manual_inspect")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report Result maps that observe and return their payload unchanged.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Result mapping calls with one callback
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() {
            continue;
        }
        let method = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::Result.member("map") => "tap",
            Some(member) if member == dir::LanguageItem::Result.member("mapErr") => "tapErr",
            _ => continue,
        };
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: callback } = view.get(*argument) else {
            continue;
        };

        // require one callback accepted by the corresponding tap method
        let Some(lambda) = module.lambda(*callback) else {
            continue;
        };
        if lambda.signature.parameters.len() != 1 {
            continue;
        }
        let Some(removed) = module.removable_parameter_return(*callback)? else {
            continue;
        };

        // require the identity mapping to preserve the Result type
        if module.adjusted_type_id(expression.into_any())?
            != module.adjusted_type_id(call.receiver.into_any())?
        {
            continue;
        }

        // replace the mapping method and remove the identity tail
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Result map only observes its payload", span);
        if call.generic_arguments.is_empty()
            && let Some(suggestion) = suggestion(module, lint, call.callee, removed, method)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one Result tap rewrite.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    callee: dir::LocalNodeId<dir::Expression>,
    removed: dir::LocalNodeId<dir::Expression>,
    method: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let removed = module.statement_removal_span(removed)?;
    if module.has_unretained_comment(removed, &[])? {
        return Ok(None);
    }

    // replace the method and remove the returned payload line
    let member = module.main_span(callee.into_any())?;
    let mut file = FilePatch::new(member.file);
    file.replace(member, method);
    file.delete(removed);
    file.sort();
    let suggestion = lint.fix("observe the Result payload with a tap method", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace error observation with tapErr.
    #[test]
    fn test_replaces_error_observation() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
declare function record(error: &readonly int32): void;

function inspect(result: Result<string, int32>): Result<string, int32> {
    return result.mapErr((error) => {
        record(error);
        error
    });
}
"#,
        );

        session.assert_fixes(
            r#"
declare function record(error: &readonly int32): void;

function inspect(result: Result<string, int32>): Result<string, int32> {
    return result.tapErr((error) => {
        record(error);
    });
}
"#,
        );
    }

    /// Remove an explicit identity return after observation.
    #[test]
    fn test_replaces_explicit_return() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
declare function record(value: &readonly int32): void;

function inspect(result: Result<int32, string>): Result<int32, string> {
    return result.map((value) => {
        record(value);
        return value;
    });
}
"#,
        );

        session.assert_fixes(
            r#"
declare function record(value: &readonly int32): void;

function inspect(result: Result<int32, string>): Result<int32, string> {
    return result.tap((value) => {
        record(value);
    });
}
"#,
        );
    }

    /// Accept a callback that transforms its payload.
    #[test]
    fn test_accepts_transformation() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
function increment(result: Result<int32, string>): Result<int32, string> {
    return result.map((value) => {
        value;
        value + 1
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an observation that requires mutable payload access.
    #[test]
    fn test_accepts_mutable_observation() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
function append(result: Result<int32[], string>): Result<int32[], string> {
    return result.map((values) => {
        values.push(1);
        values
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an observation that requires the owned managed value.
    #[test]
    fn test_accepts_owned_observation() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
class User {}

declare function observe(user: ^User): void;

function inspect(result: Result<^User, string>): Result<^User, string> {
    return result.map((user) => {
        observe(user);
        user
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a callback whose declared parameter type cannot be retargeted.
    #[test]
    fn test_accepts_declared_parameter() {
        let session = TestSession::dir(
            &MANUAL_RESULT_TAP,
            r#"
declare function record(value: &readonly int32): void;

function inspect(result: Result<int32, string>): Result<int32, string> {
    return result.map((value: int32) => {
        record(value);
        value
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
