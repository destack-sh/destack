use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow filterMap callbacks with a definitely defined result.
    pub UNNECESSARY_FILTER_MAP {
        id: "unnecessary-filter-map",
        summary: "Disallow filterMap callbacks with a definitely defined result",
        explanation: r#"
A `filterMap` callback with a definitely defined result cannot filter any element.
Instead, you SHOULD call `map` to express the unconditional transformation.
"#,
        example: {
            reported: r#"
function increment(values: int32[]): int32[] {
    return values.filterMap((value) => value + 1);
}
"#,
            accepted: r#"
function increment(values: int32[]): int32[] {
    return values.map((value) => value + 1);
}
"#,
        },
        provenance: [Clippy("unnecessary_filter_map")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical Array.filterMap calls whose callback cannot return undefined.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array.filterMap calls with one callback
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Array.member("filterMap"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: callback } = view.get(*argument) else {
            continue;
        };

        // retain filterMap when the callback can produce undefined
        if module.produces_undefined(*callback)? {
            continue;
        }

        // replace only the canonical operation name
        let span = module.source_extent(expression.into_any())?;
        let member = module.main_span(call.callee.into_any())?;
        let suggestion = suggestion(lint, member)?;
        let diagnostic = lint
            .diagnostic("filterMap callback always returns a defined value", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace filterMap with map.
fn suggestion(
    lint: &Lint,
    member: tspp_source::Span,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let patch = Patch::replace(member, "map");

    lint.suggestion("use an unconditional map", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace filterMap for a definitely defined lambda result.
    #[test]
    fn test_reports_defined_filter_map() {
        let session = TestSession::dir(
            &UNNECESSARY_FILTER_MAP,
            r#"
function increment(values: int32[]): int32[] {
    return values.filterMap((value) => value + 1);
}
"#,
        );
        session.assert_diagnostics(
            r#"
warning[unnecessary-filter-map]: filterMap callback always returns a defined value
 ──▶ main.tspp:2:12
  │
1 │ function increment(values: int32[]): int32[] {
2 │     return values.filterMap((value) => value + 1);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: use an unconditional map (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function increment(values: int32[]): int32[] {
-   2│     return values.filterMap((value) => value + 1);
+   2│     return values.map((value) => value + 1);
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function increment(values: int32[]): int32[] {
    return values.map((value) => value + 1);
}
"#,
        );
    }

    /// Accept a callback whose result can be undefined.
    #[test]
    fn test_accepts_optional_result() {
        let session = TestSession::dir(
            &UNNECESSARY_FILTER_MAP,
            r#"
function positives(values: int32[]): int32[] {
    return values.filterMap((value) => value > 0 ? value : undefined);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a callback whose explicit returns include undefined.
    #[test]
    fn test_accepts_optional_explicit_return() {
        let session = TestSession::dir(
            &UNNECESSARY_FILTER_MAP,
            r#"
function positives(values: int32[]): int32[] {
    return values.filterMap((value) => {
        if (value > 0) {
            return value;
        }

        return undefined;
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a named callback declaring an optional result.
    #[test]
    fn test_accepts_named_callback_declaring_an_optional_result() {
        let session = TestSession::dir(
            &UNNECESSARY_FILTER_MAP,
            r#"
function increment(value: int32, index: isize): int32 | undefined {
    return value + 1;
}

function increments(values: int32[]): int32[] {
    return values.filterMap(increment);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined filterMap method.
    #[test]
    fn test_accepts_user_filter_map() {
        let session = TestSession::dir(
            &UNNECESSARY_FILTER_MAP,
            r#"
class Values {
    filterMap(mapper: (value: int32) => int32): this {
        return this;
    }
}

function increment(values: Values): Values {
    return values.filterMap((value) => value + 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
