use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer nullish coalescing when a nullish value selects a fallback.
    pub PREFER_NULLISH_COALESCING {
        id: "prefer-nullish-coalescing",
        summary: "Prefer nullish coalescing when a nullish value selects a fallback",
        explanation: r#"
A conditional that returns a value when it is non-nullish and a fallback otherwise manually implements nullish coalescing.
Instead, you SHOULD use the `??` operator.
"#,
        example: {
            reported: r#"
function select(value: string | undefined, fallback: string): string {
    return value !== undefined ? value : fallback;
}
"#,
            accepted: r#"
function select(value: string | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-nullish-coalescing")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report conditionals equivalent to nullish coalescing.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect conditionals that test one value for null or undefined
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            condition,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some(then_expression) = module.sole_value_expression(*then_expression) else {
            continue;
        };
        let Some(else_expression) = module.sole_value_expression(*else_expression) else {
            continue;
        };
        let Some(test) = module.nullish_test(condition)? else {
            continue;
        };
        let (selected, fallback) = if test.is_defined {
            (then_expression, else_expression)
        } else {
            (else_expression, then_expression)
        };
        if !module.is_same_computation(test.value, selected)?
            || !module.is_duplicable_expression(test.value)?
        {
            continue;
        }

        // replace the complete conditional with one coalescing expression
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("conditional selects a fallback for a nullish value", span);
        if let Some(fix) = fix(module, lint, expression, test.value, fallback)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one nullish coalescing expression.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    fallback: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(value.into_any())?;
    let fallback_span = module.source_extent(fallback.into_any())?;
    if module.has_unretained_comment(extent, &[value_span, fallback_span])? {
        return Ok(None);
    }

    // retain both operands with coalescing-safe grouping
    let value = module.expression_source(value, dir::OperatorPrecedence::NullishCoalescing)?;
    let fallback =
        module.expression_source(fallback, dir::OperatorPrecedence::NullishCoalescing)?;
    let patch = Patch::replace(extent, format!("{value} ?? {fallback}"));
    let fix = lint.fix("use nullish coalescing", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a defined-value ternary with nullish coalescing.
    #[test]
    fn test_replaces_defined_ternary() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | undefined, fallback: string): string {
    return value !== undefined ? value : fallback;
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Replace a value-producing if expression with nullish coalescing.
    #[test]
    fn test_replaces_if_expression() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | undefined, fallback: string): string {
    return if (value !== undefined) {
        value
    } else {
        fallback
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Replace a ternary whose undefined branch comes first.
    #[test]
    fn test_replaces_undefined_ternary() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | undefined, fallback: string): string {
    return undefined === value ? fallback : value;
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Replace a strict null check when the value cannot be undefined.
    #[test]
    fn test_replaces_null_ternary() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | null, fallback: string): string {
    return value !== null ? value : fallback;
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | null, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Replace one loose comparison that covers both nullish values.
    #[test]
    fn test_replaces_loose_nullish_ternary() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | null | undefined, fallback: string): string {
    return value != null ? value : fallback;
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | null | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Replace paired strict comparisons that cover both nullish values.
    #[test]
    fn test_replaces_complete_strict_nullish_ternary() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(value: string | null | undefined, fallback: string): string {
    return value !== null && value !== undefined ? value : fallback;
}
"#,
        );

        session.assert_fixes(
            r#"
function select(value: string | null | undefined, fallback: string): string {
    return value ?? fallback;
}
"#,
        );
    }

    /// Accept a strict check that leaves another nullish value unchanged.
    #[test]
    fn test_accepts_incomplete_strict_nullish_check() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(
    value: string | null | undefined,
    fallback: string,
): string | null {
    return value !== undefined ? value : fallback;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a ternary that tests a different value.
    #[test]
    fn test_accepts_different_selected_value() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
function select(
    tested: string | undefined,
    selected: string,
    fallback: string,
): string {
    return tested !== undefined ? selected : fallback;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a repeated call whose results may differ.
    #[test]
    fn test_accepts_effectful_value() {
        let session = TestSession::dir(
            &PREFER_NULLISH_COALESCING,
            r#"
declare function next(): string | undefined;

function select(fallback: string): string {
    return next() !== undefined ? next()! : fallback;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
