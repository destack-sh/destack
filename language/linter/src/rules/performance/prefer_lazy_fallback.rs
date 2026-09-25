use tspp_dir as dir;
use tspp_source::FilePatch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer lazy fallbacks when eager arguments perform avoidable work.
    pub PREFER_LAZY_FALLBACK {
        id: "prefer-lazy-fallback",
        summary: "Prefer lazy fallbacks when eager arguments perform avoidable work",
        explanation: r#"
An eager `Result` fallback is evaluated even when the result already contains a value.
Instead, you SHOULD pass a callback to `unwrapOrElse` when evaluating the fallback can trap or perform work.
"#,
        example: {
            reported: r#"
declare function recover(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(recover());
}
"#,
            accepted: r#"
declare function recover(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => recover());
}
"#,
        },
        provenance: [Clippy("or_fun_call")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report eager Result fallbacks whose evaluation can be avoided.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Result.unwrapOr calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || !call.generic_arguments.is_empty()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Result.member("unwrapOr"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: fallback } = view.get(*argument) else {
            continue;
        };
        if module.is_speculatable_expression(*fallback)? {
            continue;
        }

        // defer the fallback behind Result.unwrapOrElse
        let span = module.source_extent(fallback.into_any())?;
        let mut diagnostic = lint.diagnostic("fallback is evaluated before it is needed", span);
        let extent = module.source_extent(expression.into_any())?;
        let fallback_extent = module.source_extent(fallback.into_any())?;
        if !module.has_unretained_comment(extent, &[fallback_extent])? {
            let mut patch = FilePatch::new(extent.file);
            patch.replace(module.main_span(call.callee.into_any())?, "unwrapOrElse");
            let is_object = matches!(
                view.get(*fallback),
                dir::Expression::ObjectExpression { .. }
            );
            patch.insert(
                fallback_extent.start,
                if is_object { "() => (" } else { "() => " },
            );
            if is_object {
                patch.insert(fallback_extent.end, ")");
            }
            patch.sort();
            let suggestion = lint.suggestion("evaluate the fallback only when needed", patch)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Defer an effectful Result fallback.
    #[test]
    fn test_defers_effectful_fallback() {
        let session = TestSession::dir(
            &PREFER_LAZY_FALLBACK,
            r#"
declare function recover(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(recover());
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function recover(): int32;

function value(result: Result<int32, string>): int32 {
    return result.unwrapOrElse(() => recover());
}
"#,
        );
    }

    /// Parenthesize an object literal when deferring its construction.
    #[test]
    fn test_defers_object_fallback() {
        let session = TestSession::dir(
            &PREFER_LAZY_FALLBACK,
            r#"
function value(result: Result<{ value: int32 }, string>): { value: int32 } {
    return result.unwrapOr({ value: 0 });
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<{ value: int32 }, string>): { value: int32 } {
    return result.unwrapOrElse(() => ({ value: 0 }));
}
"#,
        );
    }

    /// Accept a trivial eager Result fallback.
    #[test]
    fn test_accepts_trivial_fallback() {
        let session = TestSession::dir(
            &PREFER_LAZY_FALLBACK,
            r#"
function value(result: Result<int32, string>): int32 {
    return result.unwrapOr(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined unwrapOr method.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &PREFER_LAZY_FALLBACK,
            r#"
class Value {
    unwrapOr(value: int32): int32 {
        return value;
    }
}

declare function recover(): int32;

function value(result: Value): int32 {
    return result.unwrapOr(recover());
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
