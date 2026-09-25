use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer divideCeil over an adjusted integer numerator.
    pub MANUAL_DIV_CEIL {
        id: "manual-div-ceil",
        summary: "Prefer divideCeil over an adjusted integer numerator",
        explanation: r#"
Adding one less than the divisor before integer division manually computes a ceiling quotient and may overflow the intermediate sum.
Instead, you SHOULD call `.divideCeil()` on the dividend.
"#,
        example: {
            reported: r#"
function chunks(length: uint32, width: uint32): uint32 {
    return (length + width - 1) / width;
}
"#,
            accepted: r#"
function chunks(length: uint32, width: uint32): uint32 {
    return length.divideCeil(width);
}
"#,
        },
        provenance: [Clippy("manual_div_ceil")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One manual ceiling division.
struct CeilingDivision {
    /// The dividend.
    dividend: dir::LocalNodeId<dir::Expression>,
    /// The divisor.
    divisor: dir::LocalNodeId<dir::Expression>,
}

/// Report canonical unsigned ceiling-division arithmetic.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin unsigned division
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::BinaryOperator::Divide, operands @ [numerator, divisor])) =
            module.builtin_binary(expression)?
        else {
            continue;
        };
        let is_unsigned = module
            .primitive_type(expression.into_any())?
            .is_some_and(dir::PrimitiveType::is_unsigned_integer);
        if !operands.iter().all(dir::BuiltinOperand::is_integral) || !is_unsigned {
            continue;
        }
        let Some(dividend) = dividend(module, numerator.source.local_id, divisor)? else {
            continue;
        };
        let division = CeilingDivision {
            dividend,
            divisor: divisor.source.local_id,
        };

        // replace the overflowing arithmetic with the canonical method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("arithmetic manually computes ceiling division", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &division)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the dividend from one canonical adjusted numerator.
fn dividend(
    module: &DirModule<'_>,
    numerator: dir::LocalNodeId<dir::Expression>,
    divisor: &dir::BuiltinOperand,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(numerator)? else {
        return Ok(None);
    };

    // recognize `(dividend + divisor) - 1`
    if operator == dir::BinaryOperator::Subtract
        && module.integral_constant(right.source.local_id)? == Some(1)
    {
        let Some((dir::BinaryOperator::Add, [first, second])) =
            module.builtin_binary(left.source.local_id)?
        else {
            return Ok(None);
        };
        for (dividend, added_divisor) in [(first, second), (second, first)] {
            if module.is_same_operand(added_divisor, divisor)? {
                return Ok(Some(dividend.source.local_id));
            }
        }
    }

    // recognize `dividend + (divisor - 1)` in either order
    if operator != dir::BinaryOperator::Add {
        return Ok(None);
    }
    for (candidate, adjustment) in [(left, right), (right, left)] {
        let Some((dir::BinaryOperator::Subtract, [adjusted_divisor, one])) =
            module.builtin_binary(adjustment.source.local_id)?
        else {
            continue;
        };
        if module.integral_constant(one.source.local_id)? == Some(1)
            && module.is_same_operand(adjusted_divisor, divisor)?
        {
            return Ok(Some(candidate.source.local_id));
        }
    }

    Ok(None)
}

/// Build a canonical ceiling-division call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    division: &CeilingDivision,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let dividend_span = module.source_extent(division.dividend.into_any())?;
    let divisor_span = module.source_extent(division.divisor.into_any())?;
    if module.has_unretained_comment(span, &[dividend_span, divisor_span])? {
        return Ok(None);
    }

    // retain the dividend and divisor in one method call
    let dividend = module.expression_source(division.dividend, dir::OperatorPrecedence::Postfix)?;
    let divisor = module.source(divisor_span)?;
    let patch = Patch::replace(span, format!("{dividend}.divideCeil({divisor})"));
    let suggestion = lint.fix("call `.divideCeil()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace grouped and commuted ceiling-division numerators.
    #[test]
    fn test_replaces_ceiling_division_adjustments() {
        let session = TestSession::dir(
            &MANUAL_DIV_CEIL,
            r#"
function commuted(length: uint32, width: uint32): uint32 {
    return (width + length - 1) / width;
}
function grouped(length: uint32, width: uint32): uint32 {
    return (length + (width - 1)) / width;
}
"#,
        );

        session.assert_fixes(
            r#"
function commuted(length: uint32, width: uint32): uint32 {
    return length.divideCeil(width);
}
function grouped(length: uint32, width: uint32): uint32 {
    return length.divideCeil(width);
}
"#,
        );
    }

    /// Accept ordinary integer division.
    #[test]
    fn test_accepts_unadjusted_division() {
        let session = TestSession::dir(
            &MANUAL_DIV_CEIL,
            r#"
function chunks(length: uint32, width: uint32): uint32 {
    return length / width;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
