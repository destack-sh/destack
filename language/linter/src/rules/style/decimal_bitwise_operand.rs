use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow opaque decimal masks in bitwise operations.
    pub DECIMAL_BITWISE_OPERAND {
        id: "decimal-bitwise-operand",
        summary: "Disallow opaque decimal masks in bitwise operations",
        explanation: r#"
Multi-digit decimal masks obscure the bit pattern used by a bitwise operation.
Instead, you SHOULD write the mask in binary, hexadecimal, or octal notation.
"#,
        example: {
            reported: r#"
declare const value: uint32;
const masked = value & 240;
"#,
            accepted: r#"
declare const value: uint32;
const masked = value & 0xf0;
"#,
        },
        provenance: [Clippy("decimal_bitwise_operands")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report nontrivial decimal literals used as builtin bit masks.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin mask operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, operands)) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(
            operator,
            dir::BinaryOperator::ElementwiseAnd
                | dir::BinaryOperator::ElementwiseOr
                | dir::BinaryOperator::ElementwiseXor
        ) {
            continue;
        }

        // report every opaque decimal mask operand
        for operand in operands {
            let Some(literal) = decimal_mask(module, operand.source.local_id)? else {
                continue;
            };
            let span = module.source_extent(literal.into_any())?;
            output.report(lint.diagnostic("decimal literal obscures a bit mask", span));
        }
    }

    Ok(output)
}

/// Select one nontrivial decimal integer literal through transparent operations.
fn decimal_mask(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();

    // descend through compiler-defined unary operations and casts
    let expression = match view.get(expression) {
        dir::Expression::Unary { right, .. } => {
            if module.builtin_unary(expression)?.is_none() {
                return Ok(None);
            }

            return decimal_mask(module, *right);
        }
        dir::Expression::As {
            expression: value, ..
        } => return decimal_mask(module, *value),
        dir::Expression::Literal(dir::Literal::Integer(value) | dir::Literal::Bigint(value)) => {
            let magnitude = value.unsigned_abs();
            if magnitude <= 9
                || magnitude.is_power_of_two()
                || magnitude.wrapping_add(1).is_power_of_two()
            {
                return Ok(None);
            }

            expression
        }
        _ => return Ok(None),
    };

    // retain only decimal source notation
    let span = module.source_extent(expression.into_any())?;
    let token = module.token(span)?;
    if !matches!(
        token.literal(),
        Some(dir::TokenLiteral::Int {
            base: dir::NumberBase::Decimal,
            ..
        })
    ) {
        return Ok(None);
    }

    Ok(Some(expression))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report opaque decimal masks on either side of a bitwise operation.
    #[test]
    fn test_reports_decimal_masks() {
        let session = TestSession::dir(
            &DECIMAL_BITWISE_OPERAND,
            r#"
declare const value: uint32;
const masked = 240 & value;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[decimal-bitwise-operand]: decimal literal obscures a bit mask
 ──▶ main.tspp:2:16
  │
1 │ declare const value: uint32;
2 │ const masked = 240 & value;
  │                ^^^
  │
"#,
        );
    }

    /// Accept explicit masks and familiar single-bit constants.
    #[test]
    fn test_accepts_readable_masks() {
        let session = TestSession::dir(
            &DECIMAL_BITWISE_OPERAND,
            r#"
declare const value: uint32;
const hexadecimal = value & 0xf0;
const binary = value | 0b1010;
const singleDigit = value ^ 7;
const singleBit = value & 16;
const lowBits = value & 31;
"#,
        );

        session.assert_no_diagnostics();
    }
}
