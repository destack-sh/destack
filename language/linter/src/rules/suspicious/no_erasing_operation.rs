use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow operations that erase their operand into a constant.
    pub NO_ERASING_OPERATION {
        id: "no-erasing-operation",
        summary: "Disallow operations that erase their operand into a constant",
        explanation: r#"
An integral operation with an erasing operand produces a constant whenever the operation completes.
Instead, you SHOULD correct the operator or constant operand that caused the value to be discarded.
"#,
        example: {
            reported: r#"
function erase(value: int32): int32 {
    return value * 0;
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
        fixable: None,
        check: DirModule(check),
    }
}

/// Report builtin integral operations whose constant operand erases another value.
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

        // select exact constants in erasing operand positions
        let left_constant = module.scalar_constant(left.source.local_id)?;
        let right_constant = module.scalar_constant(right.source.local_id)?;
        let left_constant = left_constant.and_then(|value| value.as_integral());
        let right_constant = right_constant.and_then(|value| value.as_integral());
        let is_erasing = matches!(
            (operator, left_constant, right_constant),
            (
                dir::BinaryOperator::Multiply | dir::BinaryOperator::ElementwiseAnd,
                Some(0),
                _,
            ) | (
                dir::BinaryOperator::Multiply | dir::BinaryOperator::ElementwiseAnd,
                _,
                Some(0),
            ) | (
                dir::BinaryOperator::Divide
                    | dir::BinaryOperator::ShiftLeft
                    | dir::BinaryOperator::ShiftRight
                    | dir::BinaryOperator::UnsignedShiftRight
                    | dir::BinaryOperator::Remainder,
                Some(0),
                _,
            ) | (dir::BinaryOperator::Remainder, _, Some(1))
                | (dir::BinaryOperator::Exponent, _, Some(0))
                | (dir::BinaryOperator::ElementwiseOr, Some(-1), _)
                | (dir::BinaryOperator::ElementwiseOr, _, Some(-1))
        );
        if !is_erasing {
            continue;
        }

        // report the complete constant operation
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("constant operand erases the other value", span)
            .help("correct the operator or the constant operand");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a leading zero division that may also trap.
    #[test]
    fn test_reports_leading_zero_division() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
function erase(value: int32): int32 {
    return 0 / value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-erasing-operation]: constant operand erases the other value
 ──▶ main.ds:2:12
  │
1 │ function erase(value: int32): int32 {
2 │     return 0 / value;
  │            ^^^^^^^^^
3 │ }
  │

 = help: correct the operator or the constant operand
"#,
        );
    }

    /// Report an erasing operation without dropping an effectful operand.
    #[test]
    fn test_reports_effectful_erased_operand_without_fix() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
declare function next(): int32;
function erase(): int32 {
    return next() & 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-erasing-operation]: constant operand erases the other value
 ──▶ main.ds:3:12
  │
1 │ declare function next(): int32;
2 │ function erase(): int32 {
3 │     return next() & 0;
  │            ^^^^^^^^^^
4 │ }
  │

 = help: correct the operator or the constant operand
"#,
        );
    }

    /// Report a remainder whose unit divisor erases the dividend.
    #[test]
    fn test_reports_unit_remainder() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
function erase(value: int32): int32 {
    return value % 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-erasing-operation]: constant operand erases the other value
 ──▶ main.ds:2:12
  │
1 │ function erase(value: int32): int32 {
2 │     return value % 1;
  │            ^^^^^^^^^
3 │ }
  │

 = help: correct the operator or the constant operand
"#,
        );
    }

    /// Report a zero exponent that erases the base.
    #[test]
    fn test_reports_zero_exponent() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
function erase(value: int32): int32 {
    return value ** 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-erasing-operation]: constant operand erases the other value
 ──▶ main.ds:2:12
  │
1 │ function erase(value: int32): int32 {
2 │     return value ** 0;
  │            ^^^^^^^^^^
3 │ }
  │

 = help: correct the operator or the constant operand
"#,
        );
    }

    /// Report an all-bits-set OR that erases the other operand.
    #[test]
    fn test_reports_all_bits_set_or() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
function erase(value: int32): int32 {
    return value | -1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-erasing-operation]: constant operand erases the other value
 ──▶ main.ds:2:12
  │
1 │ function erase(value: int32): int32 {
2 │     return value | -1;
  │            ^^^^^^^^^^
3 │ }
  │

 = help: correct the operator or the constant operand
"#,
        );
    }

    /// Preserve floating-point multiplication because NaN and infinity are observable.
    #[test]
    fn test_accepts_float_zero_multiplication() {
        let session = TestSession::dir(
            &NO_ERASING_OPERATION,
            r#"
function erase(value: float64): float64 {
    return value * 0.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
