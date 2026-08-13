use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isolateLowestOne over its manual bitwise form.
    pub MANUAL_ISOLATE_LOWEST_ONE {
        id: "manual-isolate-lowest-one",
        summary: "Prefer isolateLowestOne over its manual bitwise form",
        explanation: r#"
Combining a signed integer with its negation isolates its least-significant one bit.
Instead, you SHOULD call `.isolateLowestOne()` on the integer.
"#,
        example: {
            reported: r#"
function lowest(value: int32): int32 {
    return value & -value;
}
"#,
            accepted: r#"
function lowest(value: int32): int32 {
    return value.isolateLowestOne();
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report signed integers combined with their negation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin bitwise conjunctions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::BinaryOperator::ElementwiseAnd, [left, right])) =
            module.builtin_binary(expression)?
        else {
            continue;
        };
        let Some(value) = negated_pair(module, left, right)? else {
            continue;
        };
        if !matches!(
            module.primitive_type(value.into_any())?,
            Some(dir::PrimitiveType::Integer(integer)) if integer.is_signed()
        ) {
            continue;
        }

        // replace the complete bitwise idiom with the integer method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("bitwise expression manually isolates the lowest one", span);
        if let Some(suggestion) = suggestion(module, lint, expression, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the repeated value from `value & -value` in either order.
fn negated_pair(
    module: &DirModule<'_>,
    left: &dir::BuiltinOperand,
    right: &dir::BuiltinOperand,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    for (value, negated) in [(left, right), (right, left)] {
        let Some((dir::UnaryOperator::Negate, operand)) =
            module.builtin_unary(negated.source.local_id)?
        else {
            continue;
        };
        if module.is_same_operand(value, operand)? {
            return Ok(Some(value.source.local_id));
        }
    }

    Ok(None)
}

/// Build the canonical lowest-one isolation call.
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

    // retain one evaluation of the signed integer
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(span, format!("{value}.isolateLowestOne()"));
    let suggestion = lint.fix("call `.isolateLowestOne()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a commuted lowest-one isolation expression.
    #[test]
    fn test_replaces_commuted_isolation() {
        let session = TestSession::dir(
            &MANUAL_ISOLATE_LOWEST_ONE,
            r#"
function lowest(value: int32): int32 {
    return -value & value;
}
"#,
        );

        session.assert_fixes(
            r#"
function lowest(value: int32): int32 {
    return value.isolateLowestOne();
}
"#,
        );
    }

    /// Accept unrelated bitwise conjunctions.
    #[test]
    fn test_accepts_unrelated_conjunction() {
        let session = TestSession::dir(
            &MANUAL_ISOLATE_LOWEST_ONE,
            r#"
function mask(left: int32, right: int32): int32 {
    return left & -right;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
