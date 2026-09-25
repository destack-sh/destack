use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow overflow tests that perform the overflowing operation first.
    pub NO_OVERFLOW_CHECK_AFTER_OVERFLOW {
        id: "no-overflow-check-after-overflow",
        summary: "Disallow overflow tests that perform the overflowing operation first",
        explanation: r#"
Comparing an unsigned arithmetic result with an operand observes overflow only after it has occurred.
Instead, you MUST use checked arithmetic or the overflow flag returned by an overflowing operation.
"#,
        example: {
            reported: r#"
function overflows(left: uint32, right: uint32): boolean {
    return left + right < left;
}
"#,
            accepted: r#"
function overflows(left: uint32, right: uint32): boolean {
    return left.checkedAdd(right) === undefined;
}
"#,
        },
        provenance: [Clippy("panicking_overflow_checks")],
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One arithmetic operation tested after it may have overflowed.
struct OverflowTest {
    /// The arithmetic receiver.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The other arithmetic operand.
    argument: dir::LocalNodeId<dir::Expression>,
    /// The overflow-checked arithmetic method.
    method: &'static str,
}

impl OverflowTest {
    /// Select one exact unsigned overflow idiom.
    fn select(
        module: &DirModule<'_>,
        comparison: dir::BinaryOperator,
        left: &dir::BuiltinOperand,
        right: &dir::BuiltinOperand,
    ) -> Result<Option<Self>, ProviderError> {
        // consider the arithmetic result on either comparison side
        let candidates = match comparison {
            dir::BinaryOperator::LessThan => [
                (dir::BinaryOperator::LessThan, left.source.local_id, right),
                (
                    dir::BinaryOperator::GreaterThan,
                    right.source.local_id,
                    left,
                ),
            ],
            dir::BinaryOperator::GreaterThan => [
                (
                    dir::BinaryOperator::GreaterThan,
                    left.source.local_id,
                    right,
                ),
                (dir::BinaryOperator::LessThan, right.source.local_id, left),
            ],
            _ => return Ok(None),
        };

        // require one canonical comparison against the minuend or either addend
        for (comparison, result, compared) in candidates {
            let Some((arithmetic, operands @ [first, second])) = module.builtin_binary(result)?
            else {
                continue;
            };
            let is_unsigned = module
                .primitive_type(result.into_any())?
                .is_some_and(dir::PrimitiveType::is_unsigned_integer);
            if !operands.iter().all(dir::BuiltinOperand::is_integral) || !is_unsigned {
                continue;
            }
            let method = match (arithmetic, comparison) {
                (dir::BinaryOperator::Add, dir::BinaryOperator::LessThan) => "checkedAdd",
                (dir::BinaryOperator::Subtract, dir::BinaryOperator::GreaterThan) => {
                    "checkedSubtract"
                }
                _ => continue,
            };
            let is_compared = module.is_same_operand(first, compared)?
                || arithmetic == dir::BinaryOperator::Add
                    && module.is_same_operand(second, compared)?;
            if is_compared {
                return Ok(Some(Self {
                    receiver: first.source.local_id,
                    argument: second.source.local_id,
                    method,
                }));
            }
        }

        Ok(None)
    }
}

/// Report unsigned overflow tests that first evaluate the fallible operation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin relational comparisons
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((comparison, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        let Some(test) = OverflowTest::select(module, comparison, left, right)? else {
            continue;
        };

        // replace the post-operation comparison with an overflow-checked method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("overflow is tested after arithmetic", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &test)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build a checked-arithmetic overflow predicate.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    test: &OverflowTest,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let receiver_span = module.source_extent(test.receiver.into_any())?;
    let argument_span = module.source_extent(test.argument.into_any())?;
    if module.has_unretained_comment(span, &[receiver_span, argument_span])? {
        return Ok(None);
    }

    // retain both arithmetic operands in their order
    let receiver = module.expression_source(test.receiver, dir::OperatorPrecedence::Postfix)?;
    let argument = module.source(argument_span)?;
    let method = test.method;
    let replacement = format!("{receiver}.{method}({argument}) === undefined");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("use overflow-checked arithmetic", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an unsigned addition overflow comparison.
    #[test]
    fn test_replaces_addition_overflow_test() {
        let session = TestSession::dir(
            &NO_OVERFLOW_CHECK_AFTER_OVERFLOW,
            r#"
function overflows(left: uint32, right: uint32): boolean {
    return left + right < right;
}
"#,
        );

        session.assert_suggestions(
            r#"
function overflows(left: uint32, right: uint32): boolean {
    return left.checkedAdd(right) === undefined;
}
"#,
        );
    }

    /// Preserve arithmetic operand evaluation order when testing the second addend.
    #[test]
    fn test_preserves_addition_evaluation_order() {
        let session = TestSession::dir(
            &NO_OVERFLOW_CHECK_AFTER_OVERFLOW,
            r#"
declare function next(): uint32;

function overflows(limit: uint32): boolean {
    return next() + limit < limit;
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): uint32;

function overflows(limit: uint32): boolean {
    return next().checkedAdd(limit) === undefined;
}
"#,
        );
    }

    /// Replace an unsigned subtraction underflow comparison.
    #[test]
    fn test_replaces_subtraction_underflow_test() {
        let session = TestSession::dir(
            &NO_OVERFLOW_CHECK_AFTER_OVERFLOW,
            r#"
function underflows(left: uint32, right: uint32): boolean {
    return left - right > left;
}
"#,
        );

        session.assert_suggestions(
            r#"
function underflows(left: uint32, right: uint32): boolean {
    return left.checkedSubtract(right) === undefined;
}
"#,
        );
    }

    /// Accept signed comparisons because the unsigned idiom does not apply.
    #[test]
    fn test_accepts_signed_arithmetic() {
        let session = TestSession::dir(
            &NO_OVERFLOW_CHECK_AFTER_OVERFLOW,
            r#"
function comparison(left: int32, right: int32): boolean {
    return left + right < left;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
