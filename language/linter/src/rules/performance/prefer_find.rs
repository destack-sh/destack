use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer find when consuming the first filtered element.
    pub PREFER_FIND {
        id: "prefer-find",
        summary: "Prefer find when consuming the first filtered element",
        explanation: r#"
Filtering an array before reading the first result allocates an intermediate array and evaluates the predicate for every input.
Instead, you SHOULD call `find` to stop at the first matching element.

An effectful predicate can run fewer times after this replacement.
"#,
        example: {
            reported: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
            accepted: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report filtered arrays whose first element is consumed optionally.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect checked calls that optionally consume one first element
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let filter = call.receiver;

        // require optional first-element behavior from the canonical Array member
        let consumer = module.language_member(expression)?;
        let at = dir::LanguageItem::Array.member("at");
        let first = dir::LanguageItem::Array.member("first");
        let is_first = if consumer == Some(at) {
            let [argument] = call.arguments else {
                continue;
            };
            let Some(index) = view.get(*argument).value() else {
                continue;
            };

            matches!(
                module.scalar_constant(index)?,
                Some(dir::ScalarLiteral::Integer(0))
            )
        } else if consumer == Some(first) {
            call.arguments.is_empty()
        } else {
            false
        };
        if !is_first {
            continue;
        }

        // require the receiver to be one canonical Array.filter call
        let Some(filter_call) = module.member_call(filter) else {
            continue;
        };
        let filter_language_member = dir::LanguageItem::Array.member("filter");
        if module.language_member(filter)? != Some(filter_language_member) {
            continue;
        }

        // report the allocation and offer the short-circuiting search
        let span = module.span(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("filtered array is only used for its first element", span);
        if let Some(suggestion) = suggestion(module, lint, expression, filter, filter_call.callee)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Suggest replacing one filtered first-element read with `find`.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    filter: dir::LocalNodeId<dir::Expression>,
    filter_member: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let expression = module.source_extent(expression.into_any())?;
    let filter = module.source_extent(filter.into_any())?;
    if !expression.contains_span(filter) {
        return Err(ProviderError::internal(
            "filtered first-element extent does not contain its filter call",
        ));
    }
    if module.has_unretained_comment(expression, &[filter])? {
        return Ok(None);
    }

    // replace the method name and remove the optional first-element suffix
    let filter_member = module.main_span(filter_member.into_any())?;
    let suffix = Span::new(expression.file, filter.end, expression.end);
    let mut file = FilePatch::new(expression.file);
    file.replace(filter_member, "find");
    file.delete(suffix);
    file.sort();
    let suggestion = lint.suggestion("search the array directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report `first()` after the canonical Array filter.
    #[test]
    fn test_reports_filter_first() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).first();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-find]: filtered array is only used for its first element
 ──▶ main.ds:2:12
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     return values.filter((value) => value > 0).first();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: search the array directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function firstPositive(values: int32[]): int32 | undefined {
-   2│     return values.filter((value) => value > 0).first();
+   2│     return values.find((value) => value > 0);
"#,
        );
        session.assert_suggestions(
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        );
    }

    /// Accept checked indexing because it traps instead of returning undefined.
    #[test]
    fn test_accepts_filter_index_zero() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[]): int32 {
    return values.filter((value) => value > 0)[0];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept optional lookup at any position other than the first.
    #[test]
    fn test_accepts_filter_at_nonzero_index() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
function secondPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an optional receiver when replacing the filtered read.
    #[test]
    fn test_reports_optional_filter() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[] | undefined): int32 | undefined {
    return values?.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_suggestions(
            r#"
function firstPositive(values: int32[] | undefined): int32 | undefined {
    return values?.find((value) => value > 0);
}
"#,
        );
    }

    /// Accept a user-defined method named filter.
    #[test]
    fn test_accepts_user_filter_method() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
class Values {
    filter(predicate: (value: int32) => boolean): this {
        return this;
    }

    at(index: number): int32 | undefined {
        return undefined;
    }
}

function firstPositive(values: Values): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user extension whose method shares the canonical Array member name.
    #[test]
    fn test_accepts_user_array_filter_extension() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
import { Array } from "destack:collections";

extension<T> of Array<T> {
    filter(predicate: (value: T) => boolean, trace: boolean): Array<T> {
        return this;
    }
}

function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0, true).at(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments outside the retained predicate by omitting the suggestion.
    #[test]
    fn test_reports_commented_filter_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values
        .filter((value) => value > 0) /* retain */
        .first();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-find]: filtered array is only used for its first element
 ──▶ main.ds:2:12
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     return values
  │            ^^^^^^
3 │         .filter((value) => value > 0) /* retain */
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         .first();
  │         ^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Ignore call-shaped newtype construction.
    #[test]
    fn test_accepts_newtype_construction() {
        let session = TestSession::dir(
            &PREFER_FIND,
            r#"
newtype Status = { kind: "ready"; value: int32 } | { kind: "pending" };

function ready(value: int32): Status {
    return Status({ kind: "ready", value });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
