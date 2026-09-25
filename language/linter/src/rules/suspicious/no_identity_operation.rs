use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow arithmetic with an operand that cannot change the result.
    pub NO_IDENTITY_OPERATION {
        id: "no-identity-operation",
        summary: "Disallow arithmetic with an operand that cannot change the result",
        explanation: r#"
An integral identity operation such as `value + 0` returns the other operand unchanged.
Instead, you SHOULD remove the operation.
"#,
        example: {
            reported: r#"
function retain(value: int32): int32 {
    return value + 0;
}
"#,
            accepted: r#"
function retain(value: int32): int32 {
    return value;
}
"#,
        },
        provenance: [Clippy("identity_op")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One integral identity operation and its retained value.
struct IdentityOperation {
    /// The operand retained by the correction.
    value: dir::LocalNodeId<dir::Expression>,
    /// The identity operand removed by the correction.
    identity: dir::LocalNodeId<dir::Expression>,
}

impl IdentityOperation {
    /// Select one exact integral identity operation.
    fn select(
        module: &DirModule<'_>,
        operator: dir::BinaryOperator,
        [left, right]: [dir::LocalNodeId<dir::Expression>; 2],
    ) -> Result<Option<Self>, ProviderError> {
        let left_constant = module.integral_constant(left)?;
        let right_constant = module.integral_constant(right)?;

        // retain the conventional spelling of the lowest bit mask
        if operator == dir::BinaryOperator::ShiftLeft
            && left_constant == Some(1)
            && right_constant == Some(0)
        {
            return Ok(None);
        }

        // select identities shared by signed and unsigned integers
        let selected = match (operator, left_constant, right_constant) {
            (
                dir::BinaryOperator::Add
                | dir::BinaryOperator::ElementwiseOr
                | dir::BinaryOperator::ElementwiseXor,
                Some(0),
                _,
            ) => Some(Self {
                value: right,
                identity: left,
            }),
            (
                dir::BinaryOperator::Add
                | dir::BinaryOperator::Subtract
                | dir::BinaryOperator::ShiftLeft
                | dir::BinaryOperator::ShiftRight
                | dir::BinaryOperator::UnsignedShiftRight
                | dir::BinaryOperator::ElementwiseOr
                | dir::BinaryOperator::ElementwiseXor,
                _,
                Some(0),
            ) => Some(Self {
                value: left,
                identity: right,
            }),
            (dir::BinaryOperator::Multiply, Some(1), _) => Some(Self {
                value: right,
                identity: left,
            }),
            (
                dir::BinaryOperator::Multiply
                | dir::BinaryOperator::Divide
                | dir::BinaryOperator::Exponent,
                _,
                Some(1),
            ) => Some(Self {
                value: left,
                identity: right,
            }),
            (dir::BinaryOperator::Remainder, Some(left_value), Some(right_value))
                if left_value != 0 && left_value.unsigned_abs() < right_value.unsigned_abs() =>
            {
                Some(Self {
                    value: left,
                    identity: right,
                })
            }
            _ => None,
        };
        if selected.is_some() {
            return Ok(selected);
        }

        // select the width-specific all-ones identity
        let selected = if operator == dir::BinaryOperator::ElementwiseAnd
            && module.is_all_ones_constant(left)?
        {
            Some(Self {
                value: right,
                identity: left,
            })
        } else if operator == dir::BinaryOperator::ElementwiseAnd
            && module.is_all_ones_constant(right)?
        {
            Some(Self {
                value: left,
                identity: right,
            })
        } else {
            None
        };

        Ok(selected)
    }
}

/// Report builtin integer operations with an identity operand.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin and standard-library integral operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, operands)) = module.integral_binary(expression)? else {
            continue;
        };

        // select the operand retained by the exact identity
        let Some(identity) = IdentityOperation::select(module, operator, operands)? else {
            continue;
        };
        if !module.is_speculatable_expression(identity.identity)? {
            continue;
        }

        // require removal to preserve the result type at its use
        if module.adjusted_type_id(expression.into_any())?
            != module.adjusted_type_id(identity.value.into_any())?
        {
            continue;
        }

        // report and remove the identity operation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("operation has an identity operand", span);
        if let Some(suggestion) = suggestion(module, lint, expression, identity.value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Build an automatic replacement with the retained operand.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(value.into_any())?;

    // do not discard comments outside the retained operand
    if module.has_unretained_comment(extent, &[value_span])? {
        return Ok(None);
    }

    // retain the value with grouping valid under every surrounding operator
    let replacement = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("remove the identity operation", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a leading bigint multiplicative identity.
    #[test]
    fn test_removes_leading_bigint_identity() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: bigint): bigint {
    return 1n * value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-identity-operation]: operation has an identity operand
 ──▶ main.ds:2:12
  │
1 │ function retain(value: bigint): bigint {
2 │     return 1n * value;
  │            ^^^^^^^^^^
3 │ }
  │

 = fix: remove the identity operation
--- a/main.ds
+++ b/main.ds

    1│ function retain(value: bigint): bigint {
-   2│     return 1n * value;
+   2│     return value;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function retain(value: bigint): bigint {
    return value;
}
"#,
        );
    }

    /// Remove a zero shift without changing the integer type.
    #[test]
    fn test_removes_zero_shift() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: int32): int32 {
    return value << 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function retain(value: int32): int32 {
    return value;
}
"#,
        );
    }

    /// Remove a signed all-bits-set AND identity.
    #[test]
    fn test_removes_all_bits_set_identity() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: int32): int32 {
    return value & -1;
}
"#,
        );

        session.assert_fixes(
            r#"
function retain(value: int32): int32 {
    return value;
}
"#,
        );
    }

    /// Remove an unsigned all-bits-set AND identity.
    #[test]
    fn test_removes_unsigned_all_bits_set_identity() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: uint8): uint8 {
    return value & 255;
}
"#,
        );

        session.assert_fixes(
            r#"
function retain(value: uint8): uint8 {
    return value;
}
"#,
        );
    }

    /// Remove a bigint all-bits-set AND identity.
    #[test]
    fn test_removes_bigint_all_bits_set_identity() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: bigint): bigint {
    return value & -1n;
}
"#,
        );

        session.assert_fixes(
            r#"
function retain(value: bigint): bigint {
    return value;
}
"#,
        );
    }

    /// Preserve an unsigned mask narrower than its operand.
    #[test]
    fn test_accepts_partial_unsigned_mask() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function mask(value: uint16): uint16 {
    return value & 255;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove a constant remainder smaller than its divisor.
    #[test]
    fn test_removes_unchanged_remainder() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
const value = 3 % 5;
"#,
        );

        session.assert_fixes(
            r#"
const value = 3;
"#,
        );
    }

    /// Preserve the conventional lowest-bit mask spelling.
    #[test]
    fn test_accepts_lowest_bit_mask() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
const lowest = 1 << 0;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve floating-point operations whose signed-zero behavior is observable.
    #[test]
    fn test_accepts_float_zero_addition() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
function retain(value: float64): float64 {
    return value + 0.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an effectful call whose result is an identity value.
    #[test]
    fn test_accepts_effectful_identity_operand() {
        let session = TestSession::dir(
            &NO_IDENTITY_OPERATION,
            r#"
declare function observe(): 0;
function retain(value: int32): int32 {
    return observe() + value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
