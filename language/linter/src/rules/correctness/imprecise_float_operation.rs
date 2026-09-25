use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer numerically stable floating-point operations.
    pub IMPRECISE_FLOAT_OPERATION {
        id: "imprecise-float-operation",
        summary: "Prefer numerically stable floating-point operations",
        explanation: r#"
Expanded cube-root, logarithm, and exponential formulas lose precision through intermediate rounding and cancellation.
Instead, you SHOULD call the dedicated floating-point method.
"#,
        example: {
            reported: r#"
function offsetLog(value: float64): float64 {
    return (1.0 + value).log();
}
"#,
            accepted: r#"
function offsetLog(value: float64): float64 {
    return value.log1p();
}
"#,
        },
        provenance: [Clippy("imprecise_flops")],
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One expanded floating-point operation with a dedicated method.
struct FloatOperation {
    /// The receiver retained by the replacement.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The dedicated method name.
    method: &'static str,
    /// The diagnostic description.
    message: &'static str,
}

impl FloatOperation {
    /// Select one expanded cube-root, logarithm, or exponential operation.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        // recognize `(1 + value).log()` and `(value + 1).log()`
        if let Some(call) = module.member_call(expression)
            && !call.is_optional()
            && call.generic_arguments.is_empty()
            && call.arguments.is_empty()
            && module.language_member(expression)? == Some(dir::LanguageItem::Float.member("log"))
            && let Some((dir::BinaryOperator::Add, [left, right])) =
                module.builtin_binary(call.receiver)?
        {
            let receiver = if module.is_numeric_constant(left.source.local_id, 1.0)? {
                Some(right.source.local_id)
            } else if module.is_numeric_constant(right.source.local_id, 1.0)? {
                Some(left.source.local_id)
            } else {
                None
            };
            if let Some(receiver) = receiver {
                return Ok(Some(Self {
                    receiver,
                    method: "log1p",
                    message: "expanded logarithm loses precision near zero",
                }));
            }
        }

        // recognize `value.pow(1 / 3)`
        if let Some(call) = module.member_call(expression)
            && !call.is_optional()
            && call.generic_arguments.is_empty()
            && module.language_member(expression)? == Some(dir::LanguageItem::Float.member("pow"))
            && let [argument] = call.arguments
            && let Some(exponent) = module.view().get(*argument).value()
            && let Some((dir::BinaryOperator::Divide, [numerator, denominator])) =
                module.builtin_binary(exponent)?
            && matches!(
                module.scalar_constant(numerator.source.local_id)?,
                Some(dir::Literal::Float(value)) if value == 1.0
            )
            && matches!(
                module.scalar_constant(denominator.source.local_id)?,
                Some(dir::Literal::Float(value)) if value == 3.0
            )
        {
            return Ok(Some(Self {
                receiver: call.receiver,
                method: "cbrt",
                message: "fractional power is less accurate than cube root",
            }));
        }

        // recognize `value.exp() - 1`
        if let Some((dir::BinaryOperator::Subtract, [left, right])) =
            module.builtin_binary(expression)?
            && module.is_numeric_constant(right.source.local_id, 1.0)?
            && let Some(call) = module.member_call(left.source.local_id)
            && !call.is_optional()
            && call.generic_arguments.is_empty()
            && call.arguments.is_empty()
            && module.language_member(left.source.local_id)?
                == Some(dir::LanguageItem::Float.member("exp"))
        {
            return Ok(Some(Self {
                receiver: call.receiver,
                method: "expm1",
                message: "subtracting one from exp loses precision near zero",
            }));
        }

        Ok(None)
    }
}

/// Report expanded floating-point operations with more accurate methods.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect every expression as one possible expanded operation
    for (expression, _) in module.view().iter_nodes::<dir::Expression>() {
        let Some(operation) = FloatOperation::select(module, expression)? else {
            continue;
        };

        // replace the complete formula with its dedicated method
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic(operation.message, extent);
        if let Some(suggestion) = suggestion(module, lint, extent, &operation)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one expanded formula with its dedicated floating-point method.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    operation: &FloatOperation,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(operation.receiver.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the single evaluated receiver
    let receiver =
        module.expression_source(operation.receiver, dir::OperatorPrecedence::Postfix)?;
    let replacement = format!("{receiver}.{}()", operation.method);
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion(format!("call `.{}()`", operation.method), patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace offset logarithm with log1p.
    #[test]
    fn test_replaces_offset_logarithm() {
        let session = TestSession::dir(
            &IMPRECISE_FLOAT_OPERATION,
            r#"
function offsetLog(value: float64): float64 {
    return (1.0 + value).log();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[imprecise-float-operation]: expanded logarithm loses precision near zero
 ──▶ main.tspp:2:12
  │
1 │ function offsetLog(value: float64): float64 {
2 │     return (1.0 + value).log();
  │            ^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: call `.log1p()` (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function offsetLog(value: float64): float64 {
-   2│     return (1.0 + value).log();
+   2│     return value.log1p();
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function offsetLog(value: float64): float64 {
    return value.log1p();
}
"#,
        );
    }

    /// Replace fractional power with cube root.
    #[test]
    fn test_replaces_cube_root_power() {
        let session = TestSession::dir(
            &IMPRECISE_FLOAT_OPERATION,
            r#"
function root(value: float64): float64 {
    return value.pow(1.0 / 3.0);
}
"#,
        );

        session.assert_suggestions(
            r#"
function root(value: float64): float64 {
    return value.cbrt();
}
"#,
        );
    }

    /// Replace exponential subtraction with expm1.
    #[test]
    fn test_replaces_exponential_subtraction() {
        let session = TestSession::dir(
            &IMPRECISE_FLOAT_OPERATION,
            r#"
function growth(value: float64): float64 {
    return value.exp() - 1.0;
}
"#,
        );

        session.assert_suggestions(
            r#"
function growth(value: float64): float64 {
    return value.expm1();
}
"#,
        );
    }

    /// Accept nearby formulas with distinct behavior.
    #[test]
    fn test_accepts_distinct_float_operations() {
        let session = TestSession::dir(
            &IMPRECISE_FLOAT_OPERATION,
            r#"
function calculate(value: float64): float64 {
    const logarithm = (2.0 + value).log();
    const root = value.pow(1.0 / 4.0);
    return logarithm + root + value.exp() - 2.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
