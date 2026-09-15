use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer multiplication over squaring with exponentiation.
    pub PREFER_MULTIPLICATION_OVER_POWER {
        id: "prefer-multiplication-over-power",
        summary: "Prefer multiplication over squaring with exponentiation",
        explanation: r#"
Exponentiation performs a general power operation when an expression is only squared.
Instead, you SHOULD multiply the base by itself when its evaluation may be duplicated.
"#,
        example: {
            reported: r#"
function square(value: float64): float64 {
    return value ** 2;
}
"#,
            accepted: r#"
function square(value: float64): float64 {
    return value * value;
}
"#,
        },
        provenance: [Clippy("suboptimal_flops")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report builtin exponentiation that squares one duplicable base.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin square operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::BinaryOperator::Exponent, [base, exponent])) =
            module.builtin_binary(expression)?
        else {
            continue;
        };
        let base = base.source.local_id;
        let exponent = exponent.source.local_id;
        if module.integral_constant(exponent)? != Some(2)
            || !module.is_duplicable_expression(base)?
        {
            continue;
        }

        // replace general exponentiation with direct multiplication
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("general exponentiation only squares its base", span);
        if let Some(fix) = fix(module, lint, expression, base, exponent)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one square operation with direct multiplication.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    base: dir::LocalNodeId<dir::Expression>,
    exponent: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let base_extent = module.source_extent(base.into_any())?;
    let exponent_extent = module.source_extent(exponent.into_any())?;
    if module.has_unretained_comment(extent, &[base_extent, exponent_extent])? {
        return Ok(None);
    }

    // group the repeated base beneath multiplication
    let base = module.expression_source(base, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{base} * {base}"));
    let fix = lint.fix("multiply the base by itself", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a builtin floating-point square.
    #[test]
    fn test_replaces_float_square() {
        let session = TestSession::dir(
            &PREFER_MULTIPLICATION_OVER_POWER,
            r#"
function square(value: float64): float64 {
    return value ** 2;
}
"#,
        );

        session.assert_fixes(
            r#"
function square(value: float64): float64 {
    return value * value;
}
"#,
        );
    }

    /// Group a potentially trapping compound base beneath multiplication.
    #[test]
    fn test_groups_compound_base() {
        let session = TestSession::dir(
            &PREFER_MULTIPLICATION_OVER_POWER,
            r#"
function square(left: int32, right: int32): int32 {
    return (left + right) ** 2;
}
"#,
        );

        session.assert_fixes(
            r#"
function square(left: int32, right: int32): int32 {
    return (left + right) * (left + right);
}
"#,
        );
    }

    /// Accept a base whose evaluation has effects.
    #[test]
    fn test_accepts_effectful_base() {
        let session = TestSession::dir(
            &PREFER_MULTIPLICATION_OVER_POWER,
            r#"
declare function next(): float64;

function square(): float64 {
    return next() ** 2;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an exponent other than two.
    #[test]
    fn test_accepts_other_exponent() {
        let session = TestSession::dir(
            &PREFER_MULTIPLICATION_OVER_POWER,
            r#"
function cube(value: float64): float64 {
    return value ** 3;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
