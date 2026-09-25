use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow propagation immediately wrapped in the same Result type.
    pub NEEDLESS_QUESTION_MARK {
        id: "needless-question-mark",
        summary: "Disallow propagation immediately wrapped in the same Result type",
        explanation: r#"
Propagating a Result payload and immediately rebuilding an identical successful Result performs two redundant operations.
Instead, you SHOULD return the original Result directly.
"#,
        example: {
            reported: r#"
function forward(result: Result<int32, string>): Result<int32, string> {
    return Result.ok(result?);
}
"#,
            accepted: r#"
function forward(result: Result<int32, string>): Result<int32, string> {
    return result;
}
"#,
        },
        provenance: [Clippy("needless_question_mark")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report successful Result construction around equivalent propagation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Result.ok calls with one propagated argument
    for expression in module.call_expressions() {
        let expression = expression?;
        if module.language_member(expression)? != Some(dir::LanguageItem::Result.member("ok")) {
            continue;
        }
        let dir::Expression::Call { arguments, .. } = view.get(expression) else {
            continue;
        };
        let [argument] = arguments.as_slice() else {
            continue;
        };
        let dir::Argument::Positional { value: propagated } = view.get(*argument) else {
            continue;
        };
        let dir::Expression::Maybe { left: result, .. } = view.get(*propagated) else {
            continue;
        };

        // require the rebuilt Result to terminate one callable path
        let Some(body) = module.enclosing_callable_body(expression.into_any()) else {
            continue;
        };
        let Some(returned) = module.callable_return_values(body) else {
            continue;
        };
        if !returned.contains(&expression) {
            continue;
        }

        // preserve any residual conversion encoded by a different Result type
        if module.adjusted_type_id(expression.into_any())?
            != module.adjusted_type_id(result.into_any())?
        {
            continue;
        }

        // remove both the successful constructor and propagation operator
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("Result is propagated and rebuilt unchanged", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, *result)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one direct Result replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    result: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(result.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping around the original Result
    let result = module.expression_source(result, dir::OperatorPrecedence::Lowest)?;
    let patch = Patch::replace(extent, result.into_owned());
    let suggestion = lint.fix("return the original Result", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept construction that converts the Result error type through From.
    #[test]
    fn test_accepts_residual_conversion() {
        let session = TestSession::dir(
            &NEEDLESS_QUESTION_MARK,
            r#"
import { From } from "tspp:convert";

struct Failure {
    message: string;
}

extension of Failure implements From<string> {
    static from(message: string): Failure {
        Failure { message: message }
    }
}

function convert(result: Result<int32, string>): Result<int32, Failure> {
    return Result.ok(result?);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept successful construction around a transformed payload.
    #[test]
    fn test_accepts_transformed_payload() {
        let session = TestSession::dir(
            &NEEDLESS_QUESTION_MARK,
            r#"
function increment(result: Result<int32, string>): Result<int32, string> {
    return Result.ok(result? + 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve propagation whose enclosing callable continues afterward.
    #[test]
    fn test_accepts_nonterminal_construction() {
        let session = TestSession::dir(
            &NEEDLESS_QUESTION_MARK,
            r#"
declare function consume(result: Result<int32, string>): void;

function forward(result: Result<int32, string>): Result<int32, string> {
    consume(Result.ok(result?));
    return Result.ok(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
