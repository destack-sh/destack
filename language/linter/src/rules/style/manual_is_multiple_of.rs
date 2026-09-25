use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isMultipleOf over a remainder comparison with zero.
    pub MANUAL_IS_MULTIPLE_OF {
        id: "manual-is-multiple-of",
        summary: "Prefer isMultipleOf over a remainder comparison with zero",
        explanation: r#"
Comparing an integer remainder with zero manually tests divisibility.
Instead, you SHOULD call `.isMultipleOf()` on the dividend.

A zero divisor returns true exactly when the dividend is zero.
The smallest signed value with a divisor of `-1` returns true without overflowing.
"#,
        example: {
            reported: r#"
function aligned(value: uint32, alignment: uint32): boolean {
    return value % alignment === 0;
}
"#,
            accepted: r#"
function aligned(value: uint32, alignment: uint32): boolean {
    return value.isMultipleOf(alignment);
}
"#,
        },
        provenance: [Clippy("manual_is_multiple_of")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One manual divisibility predicate.
struct Divisibility {
    /// The dividend.
    dividend: dir::LocalNodeId<dir::Expression>,
    /// The divisor.
    divisor: dir::LocalNodeId<dir::Expression>,
    /// Whether the predicate is negated.
    is_negated: bool,
}

/// Report integer remainder comparisons with zero.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin equality comparisons
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !operator.is_equality() {
            continue;
        }
        let Some(divisibility) = divisibility(module, operator, left, right)? else {
            continue;
        };

        // replace the manual predicate with the integer method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("remainder comparison manually tests divisibility", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &divisibility)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one integer remainder compared with exact zero.
fn divisibility(
    module: &DirModule<'_>,
    operator: dir::BinaryOperator,
    left: &dir::BuiltinOperand,
    right: &dir::BuiltinOperand,
) -> Result<Option<Divisibility>, ProviderError> {
    let remainder = if module.integral_constant(right.source.local_id)? == Some(0) {
        left.source.local_id
    } else if module.integral_constant(left.source.local_id)? == Some(0) {
        right.source.local_id
    } else {
        return Ok(None);
    };
    let Some((dir::BinaryOperator::Remainder, operands @ [dividend, divisor])) =
        module.builtin_binary(remainder)?
    else {
        return Ok(None);
    };
    let is_integer = matches!(
        module.primitive_type(remainder.into_any())?,
        Some(dir::PrimitiveType::Integer(_))
    );
    if !operands.iter().all(dir::BuiltinOperand::is_integral) || !is_integer {
        return Ok(None);
    }

    Ok(Some(Divisibility {
        dividend: dividend.source.local_id,
        divisor: divisor.source.local_id,
        is_negated: operator.is_negative_equality(),
    }))
}

/// Build the canonical divisibility predicate.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    divisibility: &Divisibility,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let dividend_span = module.source_extent(divisibility.dividend.into_any())?;
    let divisor_span = module.source_extent(divisibility.divisor.into_any())?;
    if module.has_unretained_comment(span, &[dividend_span, divisor_span])? {
        return Ok(None);
    }

    // retain the dividend and divisor in one method call
    let dividend =
        module.expression_source(divisibility.dividend, dir::OperatorPrecedence::Postfix)?;
    let divisor = module.source(divisor_span)?;
    let negation = if divisibility.is_negated { "!" } else { "" };
    let replacement = format!("{negation}{dividend}.isMultipleOf({divisor})");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("call `.isMultipleOf()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a negated unsigned remainder comparison.
    #[test]
    fn test_replaces_negated_remainder_comparison() {
        let session = TestSession::dir(
            &MANUAL_IS_MULTIPLE_OF,
            r#"
function misaligned(value: uint32, alignment: uint32): boolean {
    return 0 != value % alignment;
}
"#,
        );

        session.assert_suggestions(
            r#"
function misaligned(value: uint32, alignment: uint32): boolean {
    return !value.isMultipleOf(alignment);
}
"#,
        );
    }

    /// Replace a signed remainder comparison.
    #[test]
    fn test_replaces_signed_remainder() {
        let session = TestSession::dir(
            &MANUAL_IS_MULTIPLE_OF,
            r#"
function aligned(value: int32, alignment: int32): boolean {
    return value % alignment === 0;
}
"#,
        );

        session.assert_suggestions(
            r#"
function aligned(value: int32, alignment: int32): boolean {
    return value.isMultipleOf(alignment);
}
"#,
        );
    }
}
