use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require an explicit array join separator.
    pub REQUIRE_ARRAY_JOIN_SEPARATOR {
        id: "require-array-join-separator",
        summary: "Require an explicit array join separator",
        explanation: r#"
`Array.join()` silently selects a comma as its separator.
Instead, you SHOULD pass the intended separator explicitly.
"#,
        example: {
            reported: r#"
function commaSeparated(values: string[]): string {
    return values.join();
}
"#,
            accepted: r#"
function commaSeparated(values: string[]): string {
    return values.join(",");
}
"#,
        },
        provenance: [Unicorn("require-array-join-separator")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical zero-argument Array join calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Array join calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if !call.arguments.is_empty()
            || module.language_member(expression)? != Some(dir::LanguageItem::Array.member("join"))
        {
            continue;
        }

        // make the implicit comma visible without discarding comments
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("array join uses the implicit comma separator", span);
        if let Some(suggestion) = suggest_separator(module, lint, expression)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Add the default comma as an explicit separator.
fn suggest_separator(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve comments inside the empty argument container
    let arguments = module.source_region(expression.into_any(), NodeSpanRegion::Arguments)?;
    if module.has_unretained_comment(arguments, &[])? {
        return Ok(None);
    }

    // replace the empty argument container
    let patch = Patch::replace(arguments, r#"(",")"#);
    let suggestion = lint.fix("specify the comma separator", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a join call with an explicit separator.
    #[test]
    fn test_accepts_explicit_separator() {
        let session = TestSession::dir(
            &REQUIRE_ARRAY_JOIN_SEPARATOR,
            r#"
function lines(values: string[]): string {
    return values.join("\n");
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user method named join.
    #[test]
    fn test_accepts_user_join_method() {
        let session = TestSession::dir(
            &REQUIRE_ARRAY_JOIN_SEPARATOR,
            r#"
class Values {
    join(): string {
        return "custom";
    }
}

function join(values: Values): string {
    return values.join();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a comment inside the empty argument list by omitting the fix.
    #[test]
    fn test_reports_commented_join_without_fix() {
        let session = TestSession::dir(
            &REQUIRE_ARRAY_JOIN_SEPARATOR,
            r#"
function commaSeparated(values: string[]): string {
    return values.join(/* retain */);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-array-join-separator]: array join uses the implicit comma separator
 ──▶ main.tspp:2:12
  │
1 │ function commaSeparated(values: string[]): string {
2 │     return values.join(/* retain */);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
