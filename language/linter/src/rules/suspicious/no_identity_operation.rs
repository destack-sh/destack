use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

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
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin binary operations over integral values
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = node
        else {
            continue;
        };

        // select the operand retained by the exact identity
        let left_constant = module.scalar_constant(*left)?;
        let right_constant = module.scalar_constant(*right)?;
        let Some(value) =
            select_identity_operand(*operator, *left, left_constant, *right, right_constant)
        else {
            continue;
        };

        // require compiler-defined integral behavior
        let Some(operands) = module.builtin_operands(expression.into_any())? else {
            continue;
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            continue;
        }
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
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let is_left_zero = left_constant.is_some_and(|value| value.as_integral() == Some(0));
    let is_right_zero = right_constant.is_some_and(|value| value.as_integral() == Some(0));
    let is_left_one = left_constant.is_some_and(|value| value.as_integral() == Some(1));
    let is_right_one = right_constant.is_some_and(|value| value.as_integral() == Some(1));
    let is_left_negative_one = left_constant.is_some_and(|value| value.as_integral() == Some(-1));
    let is_right_negative_one = right_constant.is_some_and(|value| value.as_integral() == Some(-1));

    match operator {
        dir::BinaryOperator::Add
        | dir::BinaryOperator::ElementwiseOr
        | dir::BinaryOperator::ElementwiseXor
            if is_left_zero =>
        {
            Some(right)
        }
        dir::BinaryOperator::Add
        | dir::BinaryOperator::Subtract
        | dir::BinaryOperator::ShiftLeft
        | dir::BinaryOperator::ShiftRight
        | dir::BinaryOperator::UnsignedShiftRight
        | dir::BinaryOperator::ElementwiseOr
        | dir::BinaryOperator::ElementwiseXor
            if is_right_zero =>
        {
            Some(left)
        }
        dir::BinaryOperator::Multiply if is_left_one => Some(right),
        dir::BinaryOperator::Multiply
        | dir::BinaryOperator::Divide
        | dir::BinaryOperator::Exponent
            if is_right_one =>
        {
            Some(left)
        }
        dir::BinaryOperator::ElementwiseAnd if is_left_negative_one => Some(right),
        dir::BinaryOperator::ElementwiseAnd if is_right_negative_one => Some(left),
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
    let value = module.source_extent(value.into_any())?;

    // do not discard comments outside the retained operand
    if module.has_unretained_comment(extent, &[value])? {
        return Ok(None);
    }

    // replace the operation with its exact authored operand
    let replacement = module.source(value)?.to_string();
    let mut file = FilePatch::new(extent.file);
    file.replace(extent, replacement);
    let patches = PatchSet::single(file);
    let suggestion = lint.fix("remove the identity operation", patches)?;

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
}
