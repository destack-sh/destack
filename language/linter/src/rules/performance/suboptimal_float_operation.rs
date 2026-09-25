use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer faster floating-point operations when their precision is sufficient.
    pub SUBOPTIMAL_FLOAT_OPERATION {
        id: "suboptimal-float-operation",
        summary: "Prefer faster floating-point operations when their precision is sufficient",
        explanation: r#"
General floating-point formulas can require extra instructions or rounding steps when a dedicated operation exists.
Instead, you SHOULD call the dedicated floating-point method.
"#,
        example: {
            reported: r#"
function root(value: float64): float64 {
    return value ** 0.5;
}
"#,
            accepted: r#"
function root(value: float64): float64 {
    return value.sqrt();
}
"#,
        },
        provenance: [Clippy("suboptimal_flops")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One floating-point formula with a dedicated method.
enum FloatOperation {
    /// A zero-argument method call.
    Unary {
        /// The retained receiver.
        receiver: dir::LocalNodeId<dir::Expression>,
        /// The dedicated method name.
        method: &'static str,
        /// The diagnostic description.
        message: &'static str,
    },
    /// A fused multiply-add call.
    Fma {
        /// The multiplied receiver.
        receiver: dir::LocalNodeId<dir::Expression>,
        /// The multiplier.
        multiplier: dir::LocalNodeId<dir::Expression>,
        /// Whether the multiplier is negated.
        is_multiplier_negated: bool,
        /// The added value.
        addend: dir::LocalNodeId<dir::Expression>,
        /// Whether the added value is negated.
        is_addend_negated: bool,
        /// Whether the replacement changes operand evaluation order.
        is_reordered: bool,
    },
}

/// Report general floating-point formulas with dedicated operations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect every expression as one possible floating-point formula
    for (expression, _) in module.view().iter_nodes::<dir::Expression>() {
        let Some(operation) = select_float_operation(module, expression)? else {
            continue;
        };

        // replace the complete formula with its dedicated method
        let extent = module.source_extent(expression.into_any())?;
        let message = operation.message();
        let mut diagnostic = lint.diagnostic(message, extent);
        if let Some(suggestion) = suggestion(module, lint, extent, &operation)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

impl FloatOperation {
    /// Return the diagnostic description for this operation.
    fn message(&self) -> &'static str {
        match self {
            Self::Unary { message, .. } => message,
            Self::Fma { .. } => "multiplication followed by addition uses two rounding steps",
        }
    }
}

/// Select one formula with a dedicated floating-point operation.
fn select_float_operation(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<FloatOperation>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    if !matches!(
        module.primitive_type(expression.into_any())?,
        Some(dir::PrimitiveType::Float(_))
    ) {
        return Ok(None);
    }

    // recognize powers with dedicated roots and exponentials
    if operator == dir::BinaryOperator::Exponent {
        if module.is_numeric_constant(right.source.local_id, 0.5)? {
            return Ok(Some(FloatOperation::Unary {
                receiver: left.source.local_id,
                method: "sqrt",
                message: "general exponentiation computes a square root",
            }));
        }
        if module.is_numeric_constant(left.source.local_id, 2.0)? {
            return Ok(Some(FloatOperation::Unary {
                receiver: right.source.local_id,
                method: "exp2",
                message: "general exponentiation computes a base-two exponential",
            }));
        }
        if module.language_member(left.source.local_id)?
            == Some(dir::LanguageItem::Math.member("E"))
        {
            return Ok(Some(FloatOperation::Unary {
                receiver: right.source.local_id,
                method: "exp",
                message: "general exponentiation computes a natural exponential",
            }));
        }
    }

    // recognize a natural logarithm divided by one canonical base logarithm
    if operator == dir::BinaryOperator::Divide
        && let Some(receiver) = select_natural_log_receiver(module, left.source.local_id)?
    {
        let member = module.language_member(right.source.local_id)?;
        let method = if member == Some(dir::LanguageItem::Math.member("LN2")) {
            Some(("log2", "logarithm division computes a base-two logarithm"))
        } else if member == Some(dir::LanguageItem::Math.member("LN10")) {
            Some(("log10", "logarithm division computes a base-ten logarithm"))
        } else {
            None
        };
        if let Some((method, message)) = method {
            return Ok(Some(FloatOperation::Unary {
                receiver,
                method,
                message,
            }));
        }
    }

    // recognize each additive arrangement of one floating-point product
    let left_product = select_float_product(module, left.source.local_id)?;
    let right_product = select_float_product(module, right.source.local_id)?;
    let fma = match (operator, left_product, right_product) {
        (dir::BinaryOperator::Add, Some([receiver, multiplier]), _) => Some(FloatOperation::Fma {
            receiver,
            multiplier,
            is_multiplier_negated: false,
            addend: right.source.local_id,
            is_addend_negated: false,
            is_reordered: false,
        }),
        (dir::BinaryOperator::Subtract, Some([receiver, multiplier]), _) => {
            Some(FloatOperation::Fma {
                receiver,
                multiplier,
                is_multiplier_negated: false,
                addend: right.source.local_id,
                is_addend_negated: true,
                is_reordered: false,
            })
        }
        (dir::BinaryOperator::Add, _, Some([receiver, multiplier])) => Some(FloatOperation::Fma {
            receiver,
            multiplier,
            is_multiplier_negated: false,
            addend: left.source.local_id,
            is_addend_negated: false,
            is_reordered: true,
        }),
        (dir::BinaryOperator::Subtract, _, Some([receiver, multiplier])) => {
            Some(FloatOperation::Fma {
                receiver,
                multiplier,
                is_multiplier_negated: true,
                addend: left.source.local_id,
                is_addend_negated: false,
                is_reordered: true,
            })
        }
        _ => None,
    };
    Ok(fma)
}

/// Select the operands of one builtin floating-point product.
fn select_float_product(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<[dir::LocalNodeId<dir::Expression>; 2]>, ProviderError> {
    let is_float = matches!(
        module.primitive_type(expression.into_any())?,
        Some(dir::PrimitiveType::Float(_))
    );
    if !is_float {
        return Ok(None);
    }
    let Some((dir::BinaryOperator::Multiply, operands)) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let operands = [operands[0].source.local_id, operands[1].source.local_id];

    Ok(Some(operands))
}

/// Select the receiver of one direct natural logarithm call.
fn select_natural_log_receiver(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || !call.arguments.is_empty()
        || module.language_member(expression)? != Some(dir::LanguageItem::Float.member("log"))
    {
        return Ok(None);
    }

    Ok(Some(call.receiver))
}

/// Replace one formula with its dedicated floating-point call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    operation: &FloatOperation,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let (retained, replacement, method) = match operation {
        FloatOperation::Unary {
            receiver, method, ..
        } => {
            let receiver_span = module.source_extent(receiver.into_any())?;
            let receiver = module.expression_source(*receiver, dir::OperatorPrecedence::Postfix)?;

            (
                vec![receiver_span],
                format!("{receiver}.{method}()"),
                *method,
            )
        }
        FloatOperation::Fma {
            receiver,
            multiplier,
            is_multiplier_negated,
            addend,
            is_addend_negated,
            is_reordered,
        } => {
            // preserve the original evaluation order
            let preserves_order = !*is_reordered
                || (module.is_speculatable_expression(*receiver)?
                    && module.is_speculatable_expression(*multiplier)?
                    && module.is_speculatable_expression(*addend)?);
            if !preserves_order {
                return Ok(None);
            }

            let receiver_span = module.source_extent(receiver.into_any())?;
            let multiplier_span = module.source_extent(multiplier.into_any())?;
            let addend_span = module.source_extent(addend.into_any())?;
            let receiver = module.expression_source(*receiver, dir::OperatorPrecedence::Postfix)?;
            let multiplier_precedence = if *is_multiplier_negated {
                dir::OperatorPrecedence::Postfix
            } else {
                dir::OperatorPrecedence::Prefix
            };
            let addend_precedence = if *is_addend_negated {
                dir::OperatorPrecedence::Postfix
            } else {
                dir::OperatorPrecedence::Prefix
            };
            let multiplier = module.expression_source(*multiplier, multiplier_precedence)?;
            let addend = module.expression_source(*addend, addend_precedence)?;
            let multiplier = if *is_multiplier_negated {
                format!("-{multiplier}")
            } else {
                multiplier.into_owned()
            };
            let addend = if *is_addend_negated {
                format!("-{addend}")
            } else {
                addend.into_owned()
            };
            let retained = vec![receiver_span, multiplier_span, addend_span];

            (
                retained,
                format!("{receiver}.fma({multiplier}, {addend})"),
                "fma",
            )
        }
    };
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion(format!("call `.{method}()`"), patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a half power with square root.
    #[test]
    fn test_replaces_square_root_power() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function root(value: float64): float64 {
    return value ** 0.5;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[suboptimal-float-operation]: general exponentiation computes a square root
 ──▶ main.tspp:2:12
  │
1 │ function root(value: float64): float64 {
2 │     return value ** 0.5;
  │            ^^^^^^^^^^^^
3 │ }
  │

 = suggestion: call `.sqrt()` (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function root(value: float64): float64 {
-   2│     return value ** 0.5;
+   2│     return value.sqrt();
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function root(value: float64): float64 {
    return value.sqrt();
}
"#,
        );
    }

    /// Replace canonical exponential powers.
    #[test]
    fn test_replaces_exponential_powers() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function exponentials(value: float64): float64 {
    return 2.0 ** value + Math.E ** value;
}
"#,
        );

        session.assert_suggestions(
            r#"
function exponentials(value: float64): float64 {
    return value.exp2() + value.exp();
}
"#,
        );
    }

    /// Replace logarithm division with base-specific methods.
    #[test]
    fn test_replaces_logarithm_division() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function logarithms(value: float64): float64 {
    return value.log() / Math.LN2 + value.log() / Math.LN10;
}
"#,
        );

        session.assert_suggestions(
            r#"
function logarithms(value: float64): float64 {
    return value.log2() + value.log10();
}
"#,
        );
    }

    /// Replace adjacent multiplication and addition with fma.
    #[test]
    fn test_replaces_multiply_add() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function scale(value: float64, multiplier: float64, addend: float64): float64 {
    return value * multiplier + addend;
}
function subtractAddend(value: float64, multiplier: float64, addend: float64): float64 {
    return value * multiplier - addend;
}
function addProduct(value: float64, multiplier: float64, addend: float64): float64 {
    return addend + value * multiplier;
}
function subtractProduct(value: float64, multiplier: float64, addend: float64): float64 {
    return addend - value * multiplier;
}
"#,
        );

        session.assert_suggestions(
            r#"
function scale(value: float64, multiplier: float64, addend: float64): float64 {
    return value.fma(multiplier, addend);
}
function subtractAddend(value: float64, multiplier: float64, addend: float64): float64 {
    return value.fma(multiplier, -addend);
}
function addProduct(value: float64, multiplier: float64, addend: float64): float64 {
    return value.fma(multiplier, addend);
}
function subtractProduct(value: float64, multiplier: float64, addend: float64): float64 {
    return value.fma(-multiplier, addend);
}
"#,
        );
    }

    /// Parenthesize an already negated fused multiplier.
    #[test]
    fn test_parenthesizes_negated_multiplier() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function calculate(value: float64, multiplier: float64, addend: float64): float64 {
    return addend - value * -multiplier;
}
"#,
        );

        session.assert_suggestions(
            r#"
function calculate(value: float64, multiplier: float64, addend: float64): float64 {
    return value.fma(-(-multiplier), addend);
}
"#,
        );
    }

    /// Preserve effect order when the product follows the addend.
    #[test]
    fn test_preserves_reordered_effects() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
declare function next(): float64;

function calculate(value: float64, multiplier: float64): float64 {
    return next() + value * multiplier;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[suboptimal-float-operation]: multiplication followed by addition uses two rounding steps
 ──▶ main.tspp:4:12
  │
2 │
3 │ function calculate(value: float64, multiplier: float64): float64 {
4 │     return next() + value * multiplier;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept formulas without an equivalent dedicated operation.
    #[test]
    fn test_accepts_distinct_float_operations() {
        let session = TestSession::dir(
            &SUBOPTIMAL_FLOAT_OPERATION,
            r#"
function calculate(value: float64, multiplier: float64, addend: float64): float64 {
    const power = value ** 0.25;
    const logarithm = value.log() / 3.0;
    const widened = 2 * 3 + value;
    return addend + value + multiplier + power + logarithm + widened;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
