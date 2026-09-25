use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isPowerOfTwo over equivalent unsigned integer tests.
    pub MANUAL_IS_POWER_OF_TWO {
        id: "manual-is-power-of-two",
        summary: "Prefer isPowerOfTwo over equivalent unsigned integer tests",
        explanation: r#"
Testing `countOnes() === 1` or `value !== 0 && (value & (value - 1)) === 0` manually checks whether an unsigned integer is a power of two.
Instead, you SHOULD call `.isPowerOfTwo()` on the integer.
"#,
        example: {
            reported: r#"
function powerOfTwo(value: uint32): boolean {
    return value !== 0 && (value & (value - 1)) === 0;
}
"#,
            accepted: r#"
function powerOfTwo(value: uint32): boolean {
    return value.isPowerOfTwo();
}
"#,
        },
        provenance: [Clippy("manual_is_power_of_two")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report equivalent unsigned power-of-two predicates.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined integer predicates
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(value) = power_of_two_value(module, expression)? else {
            continue;
        };
        if !module
            .primitive_type(value.into_any())?
            .is_some_and(dir::PrimitiveType::is_unsigned_integer)
        {
            continue;
        }

        // retain the canonical predicate implementation
        let is_power_of_two = dir::LanguageItem::Integer.member("isPowerOfTwo");
        if module.is_within_language_member(expression.into_any(), is_power_of_two)? {
            continue;
        }

        // replace the complete predicate with the method call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("integer expression tests for a power of two", span);
        if let Some(suggestion) = suggestion(module, lint, expression, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the integer from one equivalent power-of-two predicate.
fn power_of_two_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };

    // recognize an exact set-bit count
    if operator.is_equality() && !operator.is_negative_equality() {
        for (call, one) in [(left, right), (right, left)] {
            let value = module.integral_constant(one.source.local_id)?;
            if value == Some(1)
                && let Some(receiver) = count_ones_receiver(module, call.source.local_id)?
            {
                return Ok(Some(receiver));
            }
        }
    }

    // require a nonzero guard around the clearing test
    if operator != dir::BinaryOperator::And {
        return Ok(None);
    }
    for (guard, clearing) in [(left, right), (right, left)] {
        let Some(guarded) = nonzero_value(module, guard.source.local_id)? else {
            continue;
        };
        let Some(cleared) = cleared_lowest_one_value(module, clearing.source.local_id)? else {
            continue;
        };
        if module.is_same_computation(guarded, cleared)? {
            return Ok(Some(guarded));
        }
    }

    Ok(None)
}

/// Select the unsigned integer from one exact positive or nonzero comparison.
fn nonzero_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let left_constant = module.integral_constant(left.source.local_id)?;
    let right_constant = module.integral_constant(right.source.local_id)?;

    // normalize the nonzero operand from either comparison order
    let value = match operator {
        _ if operator.is_negative_equality() && right_constant == Some(0) => {
            Some(left.source.local_id)
        }
        _ if operator.is_negative_equality() && left_constant == Some(0) => {
            Some(right.source.local_id)
        }
        dir::BinaryOperator::GreaterThan if right_constant == Some(0) => Some(left.source.local_id),
        dir::BinaryOperator::LessThan if left_constant == Some(0) => Some(right.source.local_id),
        _ => None,
    };

    Ok(value)
}

/// Select the integer from `(value & (value - 1)) === 0`.
fn cleared_lowest_one_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    if !operator.is_equality() || operator.is_negative_equality() {
        return Ok(None);
    }

    // normalize the bitwise expression away from zero
    let bitwise = if module.integral_constant(right.source.local_id)? == Some(0) {
        left.source.local_id
    } else if module.integral_constant(left.source.local_id)? == Some(0) {
        right.source.local_id
    } else {
        return Ok(None);
    };
    let Some((dir::BinaryOperator::ElementwiseAnd, [left, right])) =
        module.builtin_binary(bitwise)?
    else {
        return Ok(None);
    };

    // orient the original value beside its decrement
    for (value, decrement) in [(left, right), (right, left)] {
        let Some((dir::BinaryOperator::Subtract, [decremented, one])) =
            module.builtin_binary(decrement.source.local_id)?
        else {
            continue;
        };
        if module.integral_constant(one.source.local_id)? == Some(1)
            && module.is_same_operand(value, decremented)?
        {
            return Ok(Some(value.source.local_id));
        }
    }

    Ok(None)
}

/// Return the receiver of one canonical zero-argument countOnes call.
fn count_ones_receiver(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    if module.language_member(expression)? != Some(dir::LanguageItem::Integer.member("countOnes")) {
        return Ok(None);
    }
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if !call.arguments.is_empty() {
        return Ok(None);
    }

    Ok(Some(call.receiver))
}

/// Build the canonical power-of-two predicate call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(span, &[value_span])? {
        return Ok(None);
    }

    // retain the integer with postfix-safe grouping
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(span, format!("{value}.isPowerOfTwo()"));
    let suggestion = lint.suggestion("call `.isPowerOfTwo()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a guarded lowest-bit clearing test.
    #[test]
    fn test_replaces_cleared_lowest_one_test() {
        let session = TestSession::dir(
            &MANUAL_IS_POWER_OF_TWO,
            r#"
function powerOfTwo(value: uint32): boolean {
    return ((value - 1) & value) === 0 && 0 < value;
}
"#,
        );

        session.assert_suggestions(
            r#"
function powerOfTwo(value: uint32): boolean {
    return value.isPowerOfTwo();
}
"#,
        );
    }

    /// Replace an unsigned set-bit count comparison.
    #[test]
    fn test_replaces_count_ones_comparison() {
        let session = TestSession::dir(
            &MANUAL_IS_POWER_OF_TWO,
            r#"
function powerOfTwo(value: uint32): boolean {
    return 1 == value.countOnes();
}
"#,
        );

        session.assert_suggestions(
            r#"
function powerOfTwo(value: uint32): boolean {
    return value.isPowerOfTwo();
}
"#,
        );
    }

    /// Accept signed integers because their minimum value has exactly one set bit.
    #[test]
    fn test_accepts_signed_count_ones_comparison() {
        let session = TestSession::dir(
            &MANUAL_IS_POWER_OF_TWO,
            r#"
function powerOfTwo(value: int32): boolean {
    return value.countOnes() === 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a lowest-bit clearing test without the required nonzero guard.
    #[test]
    fn test_accepts_unguarded_clearing_test() {
        let session = TestSession::dir(
            &MANUAL_IS_POWER_OF_TWO,
            r#"
function maybePowerOfTwo(value: uint32): boolean {
    return (value & (value - 1)) === 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined countOnes methods.
    #[test]
    fn test_accepts_user_count_ones() {
        let session = TestSession::dir(
            &MANUAL_IS_POWER_OF_TWO,
            r#"
class Bits {
    countOnes(): uint32 {
        return 1;
    }
}
declare const bits: Bits;
const power = bits.countOnes() === 1;
"#,
        );

        session.assert_no_diagnostics();
    }
}
