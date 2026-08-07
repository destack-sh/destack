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
An integer operation with an identity operand returns the other operand unchanged and usually
remains after an incomplete simplification. Remove the operation so the value being produced is
explicit.
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
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report builtin integer operations with an identity operand.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin binary operations over integral values
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, operands @ [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            continue;
        }
        let left = left.source.local_id;
        let right = right.source.local_id;

        // select the operand retained by the exact identity
        let left_constant = module.scalar_constant(left)?;
        let right_constant = module.scalar_constant(right)?;
        let Some((value, identity)) =
            select_identity_operand(operator, left, left_constant, right, right_constant)
        else {
            continue;
        };
        if !module.is_repeatable_expression(identity)? {
            continue;
        }

        // require removal to preserve the checked result type
        if module.node_type_id(expression.into_any())? != module.node_type_id(value.into_any())? {
            continue;
        }

        // report and remove the checked identity operation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("operation has an identity operand", span);
        if let Some(suggestion) = suggestion(module, lint, expression, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the operand retained by one exact integral identity operation.
fn select_identity_operand(
    operator: dir::BinaryOperator,
    left: dir::LocalNodeId<dir::Expression>,
    left_constant: Option<dir::ScalarLiteral>,
    right: dir::LocalNodeId<dir::Expression>,
    right_constant: Option<dir::ScalarLiteral>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    let left_constant = left_constant.and_then(|value| value.as_integral());
    let right_constant = right_constant.and_then(|value| value.as_integral());

    match (operator, left_constant, right_constant) {
        (
            dir::BinaryOperator::Add
            | dir::BinaryOperator::ElementwiseOr
            | dir::BinaryOperator::ElementwiseXor,
            Some(0),
            _,
        ) => Some((right, left)),
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
        ) => Some((left, right)),
        (dir::BinaryOperator::Multiply, Some(1), _) => Some((right, left)),
        (
            dir::BinaryOperator::Multiply
            | dir::BinaryOperator::Divide
            | dir::BinaryOperator::Exponent,
            _,
            Some(1),
        ) => Some((left, right)),
        (dir::BinaryOperator::ElementwiseAnd, Some(-1), _) => Some((right, left)),
        (dir::BinaryOperator::ElementwiseAnd, _, Some(-1)) => Some((left, right)),
        _ => None,
    }
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
    let replacement = module.operand_source(value, dir::OperatorPrecedence::Postfix)?;
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

    /// Remove a zero shift without changing the checked integer type.
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

    /// Preserve an effectful call whose checked result is an identity value.
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
