use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `String.slice` over the legacy `String.substring` operation.
    pub PREFER_STRING_SLICE {
        id: "prefer-string-slice",
        summary: "Prefer `String.slice` over the legacy `String.substring` operation",
        explanation: r#"
`String.substring` swaps reversed bounds and clamps negative bounds, which differs from collection slicing.
Instead, you SHOULD use `String.slice` for the same start-inclusive, end-exclusive convention as arrays.

Review negative or dynamically ordered bounds because `slice` preserves their direction and interprets negative values from the end.
"#,
        example: {
            reported: r#"
function prefix(text: string, end: isize): string {
    return text.substring(0, end);
}
"#,
            accepted: r#"
function prefix(text: string, end: isize): string {
    return text.slice(0, end);
}
"#,
        },
        provenance: [Unicorn("prefer-string-slice")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical calls to the legacy string substring operation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical String.substring calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)?
            != Some(dir::LanguageItem::String.member("substring"))
        {
            continue;
        }

        // replace the selected member name for explicit review
        let span = module.main_span(call.callee.into_any())?;
        let mut diagnostic = lint.diagnostic("string uses the legacy substring operation", span);
        let suggestion = suggestion(module, lint, call.callee)?;
        diagnostic = diagnostic.suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one string slice replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    member: dir::LocalNodeId<dir::Expression>,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let span = module.main_span(member.into_any())?;
    let patch = Patch::replace(span, "slice");

    lint.suggestion("use String.slice", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Suggest slice without claiming reversed bounds are equivalent.
    #[test]
    fn test_suggests_reversed_bounds() {
        let session = TestSession::dir(
            &PREFER_STRING_SLICE,
            r#"
function section(text: string, start: isize, end: isize): string {
    return text.substring(end, start);
}
"#,
        );

        session.assert_suggestions(
            r#"
function section(text: string, start: isize, end: isize): string {
    return text.slice(end, start);
}
"#,
        );
    }

    /// Accept user methods with the same name.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &PREFER_STRING_SLICE,
            r#"
class Text {
    substring(start: isize): string {
        return "";
    }
}
function section(text: Text): string {
    return text.substring(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
