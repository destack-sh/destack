use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow template expressions without text or conversion.
    pub NO_UNNECESSARY_TEMPLATE_EXPRESSION {
        id: "no-unnecessary-template-expression",
        summary: "Disallow template expressions without text or conversion",
        explanation: r#"
A template containing only one interpolation returns that value unchanged when checking proves no conversion is required.
Instead, you SHOULD use the interpolated value directly.
"#,
        example: {
            reported: r#"
function identity(value: string): string {
    return `${value}`;
}
"#,
            accepted: r#"
function identity(value: string): string {
    return value;
}
"#,
        },
        provenance: [TypeScriptEslint("no-unnecessary-template-expression")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report one-value templates that preserve the value type.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect interpolated templates with no literal text
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::TemplateExpression {
            value: dir::TemplateLiteral::InterpolatedString { chunks, arguments },
        } = node
        else {
            continue;
        };

        // require one interpolation and no authored text
        let [before, after] = chunks.as_slice() else {
            continue;
        };
        if !module.dir.strings.get(before.raw).is_empty()
            || !module.dir.strings.get(after.raw).is_empty()
        {
            continue;
        }
        let [argument] = arguments.as_slice() else {
            continue;
        };
        let Some(value) = view.get(*argument).value() else {
            continue;
        };

        // require the interpolation to preserve its type
        if module.adjusted_type(expression.into_any())? != module.adjusted_type(value.into_any())? {
            continue;
        }

        // replace the template while retaining its sole value
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("template contains only one unchanged value", span);
        if let Some(suggestion) = suggestion(module, lint, span, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one template with its retained value.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let value_span = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_span])? {
        return Ok(None);
    }

    // preserve the interpolated value under every surrounding operator
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, value);
    let suggestion = lint.fix("use the value directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a template that converts a numeric value to text.
    #[test]
    fn test_accepts_template_conversion() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TEMPLATE_EXPRESSION,
            r#"
function format(value: int32): string {
    return `${value}`;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a template containing literal text.
    #[test]
    fn test_accepts_template_text() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TEMPLATE_EXPRESSION,
            r#"
function format(value: string): string {
    return `value: ${value}`;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a fix when replacing the template would discard a comment.
    #[test]
    fn test_reports_commented_template_without_fix() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TEMPLATE_EXPRESSION,
            r#"
function identity(value: string): string {
    return `${/* retain */ value}`;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unnecessary-template-expression]: template contains only one unchanged value
 ──▶ main.ds:2:12
  │
1 │ function identity(value: string): string {
2 │     return `${/* retain */ value}`;
  │            ^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Preserve a conditional interpolation as one surrounding operand.
    #[test]
    fn test_groups_conditional_interpolation() {
        let session = TestSession::dir(
            &NO_UNNECESSARY_TEMPLATE_EXPRESSION,
            r#"
function choose(isFirst: boolean, first: string, second: string): boolean {
    return `${isFirst ? first : second}` === first;
}
"#,
        );

        session.assert_fixes(
            r#"
function choose(isFirst: boolean, first: string, second: string): boolean {
    return (isFirst ? first : second) === first;
}
"#,
        );
    }
}
