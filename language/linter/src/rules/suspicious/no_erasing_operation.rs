use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow operations that erase their operand into a constant.
    pub NO_ERASING_OPERATION {
        id: "no-erasing-operation",
        summary: "Disallow operations that erase their operand into a constant",
        explanation: "An integral operation with an erasing operand produces a constant regardless of the other value, apart from operations that may trap. Such expressions usually use the wrong operator or operand and should be corrected explicitly.",
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
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin binary operations over integral values
    for expression in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = view.get(expression)
        else {
            continue;
        };

        // select exact constants in erasing operand positions
        let left_constant = module.scalar_constant(*left)?;
        let right_constant = module.scalar_constant(*right)?;
        let is_left_zero = left_constant.is_some_and(|value| value.as_integral() == Some(0));
        let is_right_zero = right_constant.is_some_and(|value| value.as_integral() == Some(0));
        let is_right_one = right_constant.is_some_and(|value| value.as_integral() == Some(1));
        let is_left_negative_one =
            left_constant.is_some_and(|value| value.as_integral() == Some(-1));
        let is_right_negative_one =
            right_constant.is_some_and(|value| value.as_integral() == Some(-1));
        let is_erasing = match operator {
            dir::BinaryOperator::Multiply | dir::BinaryOperator::ElementwiseAnd => {
                is_left_zero || is_right_zero
            }
            dir::BinaryOperator::Divide
            | dir::BinaryOperator::ShiftLeft
            | dir::BinaryOperator::ShiftRight
            | dir::BinaryOperator::UnsignedShiftRight => is_left_zero,
            dir::BinaryOperator::Remainder => is_left_zero || is_right_one,
            dir::BinaryOperator::Exponent => is_right_zero,
            dir::BinaryOperator::ElementwiseOr => is_left_negative_one || is_right_negative_one,
            _ => false,
        };
        if !is_erasing {
            continue;
        }

        // require compiler-defined integral behavior
        let Some(operands) = module.builtin_operands(expression.into_any())? else {
            continue;
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            continue;
        }

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
        let session = TestSession::new(
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
        let session = TestSession::new(
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
        let session = TestSession::new(
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
        let session = TestSession::new(
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
        let session = TestSession::new(
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
        let session = TestSession::new(
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
