use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer integer rotation operations over equivalent shift expressions.
    pub MANUAL_ROTATE {
        id: "manual-rotate",
        summary: "Prefer integer rotation operations over equivalent shift expressions",
        explanation: r#"
Combining complementary shifts of one unsigned integer with bitwise OR performs a bit rotation.
Instead, you SHOULD call `.rotateLeft()` or `.rotateRight()` with the corresponding amount.
"#,
        example: {
            reported: r#"
function rotate(value: uint32): uint32 {
    return (value >> 8) | (value << 24);
}
"#,
            accepted: r#"
function rotate(value: uint32): uint32 {
    return value.rotateRight(8);
}
"#,
        },
        provenance: [Clippy("manual_rotate")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report complementary unsigned shifts combined with bitwise OR.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined integral bitwise OR operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::BinaryOperator::ElementwiseOr, operands @ [left, right])) =
            module.builtin_binary(expression)?
        else {
            continue;
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            continue;
        }

        // select complementary shifts of one duplicable unsigned value
        let Some(left_shift) = select_shift(module, left.source.local_id)? else {
            continue;
        };
        let Some(right_shift) = select_shift(module, right.source.local_id)? else {
            continue;
        };
        if left_shift.direction == right_shift.direction
            || !module.is_same_computation(left_shift.value, right_shift.value)?
        {
            continue;
        }
        let Some(dir::PrimitiveType::Integer(integer)) =
            module.primitive_type(left_shift.value.into_any())?
        else {
            continue;
        };
        let Some(width) = integer.width() else {
            continue;
        };
        let width = i64::from(width);
        if integer.is_signed()
            || left_shift.amount >= width
            || right_shift.amount >= width
            || left_shift.amount + right_shift.amount != width
        {
            continue;
        }

        // select the authored amount for the named rotation direction
        let rotation = if left_shift.direction == ShiftDirection::Right {
            left_shift
        } else {
            right_shift
        };

        // replace the complete shift expression with a rotation call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("complementary shifts implement a rotation", span);
        if let Some(suggestion) = suggestion(module, lint, expression, rotation)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One selected constant shift.
#[derive(Debug, Clone, Copy)]
struct Shift {
    /// The shifted integer value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The authored shift amount expression.
    amount_expression: dir::LocalNodeId<dir::Expression>,
    /// The exact shift amount.
    amount: i64,
    /// The shift direction.
    direction: ShiftDirection,
}

/// One bit-shift direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftDirection {
    /// Shift toward more significant bits.
    Left,
    /// Shift toward less significant bits.
    Right,
}

/// Select one builtin unsigned constant shift.
fn select_shift(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<Shift>, ProviderError> {
    let Some((operator, [value, amount])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let direction = match operator {
        dir::BinaryOperator::ShiftLeft => ShiftDirection::Left,
        dir::BinaryOperator::ShiftRight => ShiftDirection::Right,
        _ => return Ok(None),
    };
    let value = value.source.local_id;
    let amount_expression = amount.source.local_id;
    let Some(amount) = module.integral_constant(amount_expression)? else {
        return Ok(None);
    };
    if amount <= 0 {
        return Ok(None);
    }

    Ok(Some(Shift {
        value,
        amount_expression,
        amount,
        direction,
    }))
}

/// Build the canonical rotation call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    shift: Shift,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(shift.value.into_any())?;
    let amount_span = module.source_extent(shift.amount_expression.into_any())?;
    if module.has_unretained_comment(span, &[value_span, amount_span])? {
        return Ok(None);
    }

    // retain the value and selected shift amount
    let value = module.expression_source(shift.value, dir::OperatorPrecedence::Postfix)?;
    let amount = module.source(amount_span)?;
    let method = match shift.direction {
        ShiftDirection::Left => "rotateLeft",
        ShiftDirection::Right => "rotateRight",
    };
    let patch = Patch::replace(span, format!("{value}.{method}({amount})"));
    let suggestion = lint.fix(format!("call `.{method}()`"), patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace complementary shifts in either authored order.
    #[test]
    fn test_replaces_manual_rotate_right() {
        let session = TestSession::dir(
            &MANUAL_ROTATE,
            r#"
function rotate(value: uint16): uint16 {
    return (value >> 12) | (value << (2 + 2));
}
"#,
        );

        session.assert_fixes(
            r#"
function rotate(value: uint16): uint16 {
    return value.rotateRight(12);
}
"#,
        );
    }

    /// Accept signed shifts because right shift propagates the sign bit.
    #[test]
    fn test_accepts_signed_shift_pair() {
        let session = TestSession::dir(
            &MANUAL_ROTATE,
            r#"
function rotate(value: int32): int32 {
    return (value >> 8) | (value << 24);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept noncomplementary and effectful shift pairs.
    #[test]
    fn test_accepts_nonrotations() {
        let session = TestSession::dir(
            &MANUAL_ROTATE,
            r#"
declare function next(): uint32;
function different(value: uint32): uint32 {
    return (value >> 8) | (value << 8);
}
function effectful(): uint32 {
    return (next() >> 8) | (next() << 24);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
