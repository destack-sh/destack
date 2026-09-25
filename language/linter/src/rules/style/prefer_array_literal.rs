use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer an empty array literal over the canonical Array constructor.
    pub PREFER_ARRAY_LITERAL {
        id: "prefer-array-literal",
        summary: "Prefer an empty array literal over the canonical Array constructor",
        explanation: r#"
The canonical zero-argument `Array.new` call constructs the same empty array as `[]`.
Instead, you SHOULD use the array literal.
"#,
        example: {
            reported: r#"
function values(): int32[] {
    return Array.new();
}
"#,
            accepted: r#"
function values(): int32[] {
    return [];
}
"#,
        },
        provenance: [
            Eslint("no-array-constructor"),
            TypeScriptEslint("no-array-constructor"),
            Unicorn("no-new-array"),
        ],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report inferred empty Array constructions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical zero-argument Array.new calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };

        // require the canonical array factory
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("new")) {
            continue;
        }

        // require an inferred empty construction
        if !call.generic_arguments.is_empty() || !call.arguments.is_empty() {
            continue;
        }
        if matches!(
            view.get(call.receiver),
            dir::Expression::Instantiation { .. }
        ) {
            continue;
        }

        // replace the contextually typed factory call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("empty array uses the Array.new factory", span);
        if let Some(suggestion) = suggestion(module, lint, span)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one inferred empty factory call with an empty literal.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    if module.has_unretained_comment(extent, &[])? {
        return Ok(None);
    }

    let patch = Patch::replace(extent, "[]");
    let suggestion = lint.fix("use an empty array literal", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a capacity-preserving Array factory.
    #[test]
    fn test_accepts_array_with_capacity() {
        let session = TestSession::dir(
            &PREFER_ARRAY_LITERAL,
            r#"
function values(capacity: usize): int32[] {
    return Array.withCapacity(capacity);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an Array factory call with an explicit element type.
    #[test]
    fn test_accepts_explicit_array_element_type() {
        let session = TestSession::dir(
            &PREFER_ARRAY_LITERAL,
            r#"
function values(): int32[] {
    return Array<int32>.new();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a fix when replacing the factory would discard a comment.
    #[test]
    fn test_reports_commented_array_factory_without_fix() {
        let session = TestSession::dir(
            &PREFER_ARRAY_LITERAL,
            r#"
function values(): int32[] {
    return Array.new(/* empty */);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-array-literal]: empty array uses the Array.new factory
 ──▶ main.tspp:2:12
  │
1 │ function values(): int32[] {
2 │     return Array.new(/* empty */);
  │            ^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Ignore call-shaped decorator data.
    #[test]
    fn test_accepts_decorator_application() {
        let session = TestSession::dir(
            &PREFER_ARRAY_LITERAL,
            r#"
@derive(Clone)
struct Status {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
