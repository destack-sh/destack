use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer unary negation over multiplying or dividing by -1.
    pub PREFER_UNARY_NEGATION {
        id: "prefer-unary-negation",
        summary: "Prefer unary negation over multiplying or dividing by -1",
        explanation: r#"
Multiplying or dividing a builtin numeric value by negative one performs the same operation as unary negation.
Instead, you SHOULD negate the value directly.
"#,
        example: {
            reported: r#"
function negate(value: int32): int32 {
    return value * -1;
}
"#,
            accepted: r#"
function negate(value: int32): int32 {
    return -value;
}
"#,
        },
        provenance: [Clippy("neg_multiply"), Unicorn("prefer-unary-minus")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report builtin multiplication or division by negative one.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined numeric multiplication and division
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((operator, operands @ [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(
            operator,
            dir::BinaryOperator::Multiply | dir::BinaryOperator::Divide
        ) {
            continue;
        }
        if !operands.iter().all(|operand| {
            operand
                .scalar_families
                .as_ref()
                .is_some_and(|set| set.is_numeric())
        }) {
            continue;
        }

        // select the other value beside an exact negative one
        let left = left.source.local_id;
        let right = right.source.local_id;
        let left_constant = module.scalar_constant(left)?;
        let right_constant = module.scalar_constant(right)?;
        let (value, message) = match operator {
            dir::BinaryOperator::Multiply if is_negative_one(right_constant) => {
                (left, "numeric value is multiplied by negative one")
            }
            dir::BinaryOperator::Multiply if is_negative_one(left_constant) => {
                (right, "numeric value is multiplied by negative one")
            }
            dir::BinaryOperator::Divide if is_negative_one(right_constant) => {
                (left, "numeric value is divided by negative one")
            }
            _ => continue,
        };
        if module.node_type_id(expression.into_any())? != module.node_type_id(value.into_any())? {
            continue;
        }

        // replace the arithmetic expression with direct negation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(suggestion) = suggestion(module, lint, expression, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one scalar constant is numeric negative one.
fn is_negative_one(constant: Option<dir::Literal>) -> bool {
    matches!(
        constant,
        Some(dir::Literal::Integer(-1) | dir::Literal::Bigint(-1) | dir::Literal::Float(-1.0))
    )
}

/// Build a precedence-safe unary negation.
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

    // group the retained value for prefix precedence
    let value = module.expression_source(value, dir::OperatorPrecedence::Prefix)?;
    let patch = Patch::replace(span, format!("-{value}"));
    let suggestion = lint.fix("use unary negation", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Negate a builtin integer value from either multiplication side.
    #[test]
    fn test_replaces_negative_one_multiplication() {
        let session = TestSession::dir(
            &PREFER_UNARY_NEGATION,
            r#"
function negate(left: int32, right: int32): int32 {
    return -1 * (left + right);
}
"#,
        );

        session.assert_fixes(
            r#"
function negate(left: int32, right: int32): int32 {
    return -(left + right);
}
"#,
        );
    }

    /// Negate a builtin floating-point value.
    #[test]
    fn test_replaces_float_negative_one_multiplication() {
        let session = TestSession::dir(
            &PREFER_UNARY_NEGATION,
            r#"
function negate(value: float64): float64 {
    return value * -1.0;
}
"#,
        );

        session.assert_fixes(
            r#"
function negate(value: float64): float64 {
    return -value;
}
"#,
        );
    }

    /// Negate a builtin numeric dividend divided by negative one.
    #[test]
    fn test_replaces_negative_one_division() {
        let session = TestSession::dir(
            &PREFER_UNARY_NEGATION,
            r#"
function negate(value: int32): int32 {
    return value / -1;
}
"#,
        );

        session.assert_fixes(
            r#"
function negate(value: int32): int32 {
    return -value;
}
"#,
        );
    }

    /// Accept negative one divided by another value.
    #[test]
    fn test_accepts_negative_one_dividend() {
        let session = TestSession::dir(
            &PREFER_UNARY_NEGATION,
            r#"
function reciprocal(value: float64): float64 {
    return -1.0 / value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept multiplication selected through a user-defined protocol.
    #[test]
    fn test_accepts_overloaded_multiplication() {
        let session = TestSession::dir(
            &PREFER_UNARY_NEGATION,
            r#"
import { Multiply } from "tspp:ops";

struct Measure {}
extension of Measure implements Multiply<int32> {
    type Output = Measure;
    multiply(other: int32): Measure {
        return this;
    }
}

declare const value: Measure;
const negated = value * -1;
"#,
        );

        session.assert_no_diagnostics();
    }
}
