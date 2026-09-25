use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow template interpolation in regular strings.
    pub NO_TEMPLATE_CURLY_IN_STRING {
        id: "no-template-curly-in-string",
        summary: "Disallow template interpolation in regular strings",
        explanation: r#"
`${...}` inside a regular string produces literal text because regular strings do not interpolate placeholders.
Instead, you SHOULD use a template literal when interpolation is intended.
"#,
        example: {
            reported: r#"
function greet(name: string): string {
    return "hello, ${name}";
}
"#,
            accepted: r#"
function greet(name: string): string {
    return `hello, ${name}`;
}
"#,
        },
        provenance: [Eslint("no-template-curly-in-string")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report interpolation syntax authored inside regular string literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored string literals containing interpolation syntax
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Literal(dir::Literal::String(_))) {
            continue;
        }

        // require unescaped interpolation syntax in the authored string
        let span = module.source_extent(expression.into_any())?;
        let source = module.source(span)?;
        if !has_interpolation(source) {
            continue;
        }

        let mut diagnostic = lint.diagnostic("regular string contains interpolation syntax", span);
        if let Some(suggestion) = suggestion(lint, span, source)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace regular string delimiters with template delimiters when safe.
fn suggestion(
    lint: &Lint,
    extent: Span,
    source: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    if !source.starts_with('"') || !source.ends_with('"') || source.contains('`') {
        return Ok(None);
    }

    let mut file = FilePatch::new(extent.file);
    file.replace(Span::at(extent.file, extent.start, 1), "`");
    file.replace(Span::at(extent.file, extent.end - 1, 1), "`");
    file.sort();
    let suggestion = lint.suggestion("use a template literal", file)?;

    Ok(Some(suggestion))
}

/// Return whether source text contains unescaped template interpolation.
fn has_interpolation(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;

    // find each interpolation marker and count its preceding escape characters
    while index + 1 < bytes.len() {
        if bytes[index] != b'$' || bytes[index + 1] != b'{' {
            index += 1;
            continue;
        }

        // count escapes immediately preceding the marker
        let mut slash = index;
        while slash > 0 && bytes[slash - 1] == b'\\' {
            slash -= 1;
        }
        if (index - slash).is_multiple_of(2) {
            return true;
        }

        // continue after the escaped marker prefix
        index += 2;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept ordinary string text without interpolation syntax.
    #[test]
    fn test_accepts_regular_string() {
        let session = TestSession::dir(
            &NO_TEMPLATE_CURLY_IN_STRING,
            r#"
function greet(): string {
    return "hello, name";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an escaped interpolation marker.
    #[test]
    fn test_accepts_escaped_interpolation() {
        let session = TestSession::dir(
            &NO_TEMPLATE_CURLY_IN_STRING,
            r#"
function source(): string {
    return "\${name}";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a suggestion when template delimiters cannot preserve the text.
    #[test]
    fn test_reports_interpolation_with_backtick_without_suggestion() {
        let session = TestSession::dir(
            &NO_TEMPLATE_CURLY_IN_STRING,
            r#"
function source(name: string): string {
    return "`${name}`";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-template-curly-in-string]: regular string contains interpolation syntax
 ──▶ main.tspp:2:12
  │
1 │ function source(name: string): string {
2 │     return "`${name}`";
  │            ^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report interpolation preceded by an even number of escape characters.
    #[test]
    fn test_reports_even_escaped_interpolation() {
        let session = TestSession::dir(
            &NO_TEMPLATE_CURLY_IN_STRING,
            r#"
function source(name: string): string {
    return "\\\\${name}";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-template-curly-in-string]: regular string contains interpolation syntax
 ──▶ main.tspp:2:12
  │
1 │ function source(name: string): string {
2 │     return "\\\\${name}";
  │            ^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: use a template literal (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function source(name: string): string {
-   2│     return "\\\\${name}";
+   2│     return `\\\\${name}`;
    3│ }
"#,
        );
    }
}
