use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, IntegerStep, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer strict comparison over offsetting an operand by one.
    pub MANUAL_STRICT_COMPARISON {
        id: "manual-strict-comparison",
        summary: "Prefer strict comparison over offsetting an operand by one",
        explanation: r#"
An inclusive integer comparison with an operand offset by one expresses strict ordering through extra arithmetic.
Instead, you SHOULD compare the original operands with `<` or `>`.

The rewritten comparison does not preserve an overflow trap produced by the offset at an integer boundary.
"#,
        example: {
            reported: r#"
function isAfter(left: int32, right: int32): boolean {
    return left >= right + 1;
}
"#,
            accepted: r#"
function isAfter(left: int32, right: int32): boolean {
    return left > right;
}
"#,
        },
        provenance: [Clippy("int_plus_one")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One inclusive comparison expressed through a unit offset.
struct StrictComparison {
    /// The unmodified left operand.
    left: dir::LocalNodeId<dir::Expression>,
    /// The unmodified right operand.
    right: dir::LocalNodeId<dir::Expression>,
    /// The equivalent strict comparison operator.
    operator: dir::BinaryOperator,
}

/// Report inclusive integer comparisons that offset one operand by one.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined comparison operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(comparison) = strict_comparison(module, expression)? else {
            continue;
        };

        // replace the offset comparison with its strict form
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("inclusive comparison offsets an operand", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &comparison)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the original operands and strict operator from one comparison.
fn strict_comparison(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<StrictComparison>, ProviderError> {
    let Some((operator, [left, right])) = module.integral_binary(expression)? else {
        return Ok(None);
    };

    // remove the unit offset that expresses the strict ordering
    let left_step = module.integer_step(left)?;
    let right_step = module.integer_step(right)?;
    let (left, right, operator) = match (operator, left_step, right_step) {
        (dir::BinaryOperator::GreaterThanOrEqual, Some(IntegerStep::Decrement(left)), _) => {
            (left, right, dir::BinaryOperator::GreaterThan)
        }
        (dir::BinaryOperator::GreaterThanOrEqual, _, Some(IntegerStep::Increment(right))) => {
            (left, right, dir::BinaryOperator::GreaterThan)
        }
        (dir::BinaryOperator::LessThanOrEqual, Some(IntegerStep::Increment(left)), _) => {
            (left, right, dir::BinaryOperator::LessThan)
        }
        (dir::BinaryOperator::LessThanOrEqual, _, Some(IntegerStep::Decrement(right))) => {
            (left, right, dir::BinaryOperator::LessThan)
        }
        _ => return Ok(None),
    };

    Ok(Some(StrictComparison {
        left,
        right,
        operator,
    }))
}

/// Build the corresponding strict comparison.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    comparison: &StrictComparison,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let left_span = module.source_extent(comparison.left.into_any())?;
    let right_span = module.source_extent(comparison.right.into_any())?;
    if module.has_unretained_comment(span, &[left_span, right_span])? {
        return Ok(None);
    }

    // retain both operands around the strict operator
    let left = module.expression_source(comparison.left, dir::OperatorPrecedence::Comparison)?;
    let right = module.expression_source(comparison.right, dir::OperatorPrecedence::Comparison)?;
    let replacement = format!("{left} {} {right}", comparison.operator.text());
    let replacement = if module.source_parentheses(expression.into_any()).is_some() {
        format!("({replacement})")
    } else {
        replacement
    };
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("use a strict comparison", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace every orientation of an inclusive unit-offset comparison.
    #[test]
    fn test_replaces_unit_offset_comparisons() {
        let session = TestSession::dir(
            &MANUAL_STRICT_COMPARISON,
            r#"
function compare(left: int32, right: int32): boolean {
    const first = left >= right + 1;
    const second = left - 1 >= right;
    const third = left + 1 <= right;
    const fourth = left <= right - 1;
    const fifth = left >= 1 + right;
    const sixth = left <= -1 + right;
    const seventh = left >= right - -1;
    const eighth = left <= right + -1;

    return first || second || third || fourth || fifth || sixth || seventh || eighth;
}
"#,
        );

        session.assert_suggestions(
            r#"
function compare(left: int32, right: int32): boolean {
    const first = left > right;
    const second = left > right;
    const third = left < right;
    const fourth = left < right;
    const fifth = left > right;
    const sixth = left < right;
    const seventh = left > right;
    const eighth = left < right;

    return first || second || third || fourth || fifth || sixth || seventh || eighth;
}
"#,
        );
    }

    /// Accept strict, floating-point, non-unit, and user-defined comparisons.
    #[test]
    fn test_accepts_other_comparisons() {
        let session = TestSession::dir(
            &MANUAL_STRICT_COMPARISON,
            r#"
struct Count {}

extension of Count implements Add<Count>, Compare<Count> {
    type Output = Count;

    add(other: Count): Count {
        return other;
    }

    equal(other: &readonly Count): boolean {
        return true;
    }

    partialCompare(other: &readonly Count): Ordering | null {
        return Ordering.Equal;
    }

    compare(other: &readonly Count): Ordering {
        return Ordering.Equal;
    }
}

function accepted(
    left: int32,
    right: int32,
    floating: float64,
    first: Count,
    second: Count,
): boolean {
    const strict = left > right;
    const nonUnit = left >= right + 2;
    const floats = floating >= floating + 1;
    const custom = first >= second + second;

    return strict || nonUnit || floats || custom;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a comparison using canonical standard-library bigint operators.
    #[test]
    fn test_replaces_bigint_comparison() {
        let session = TestSession::dir(
            &MANUAL_STRICT_COMPARISON,
            r#"
function isAfter(left: bigint, right: bigint): boolean {
    return left >= right + 1n;
}
"#,
        );

        session.assert_suggestions(
            r#"
function isAfter(left: bigint, right: bigint): boolean {
    return left > right;
}
"#,
        );
    }
}
