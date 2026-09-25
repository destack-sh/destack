use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer isolateLowestOne over its manual wrapping bitwise form.
    pub MANUAL_ISOLATE_LOWEST_ONE {
        id: "manual-isolate-lowest-one",
        summary: "Prefer isolateLowestOne over its manual wrapping bitwise form",
        explanation: r#"
Combining an integer with its wrapping negation isolates its least-significant one bit.
Instead, you SHOULD call `.isolateLowestOne()` on the integer.
"#,
        example: {
            reported: r#"
function lowest(value: uint32): uint32 {
    return value & value.wrappingNegate();
}
"#,
            accepted: r#"
function lowest(value: uint32): uint32 {
    return value.isolateLowestOne();
}
"#,
        },
        provenance: [Clippy("manual_isolate_lowest_one")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report integers combined with their wrapping negation.
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
        let Some(value) = select_wrapping_negated_value(module, left, right)? else {
            continue;
        };
        if !matches!(
            module.primitive_type(value.into_any())?,
            Some(dir::PrimitiveType::Integer(_))
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

/// Select the repeated value from `value & value.wrappingNegate()` in either order.
fn select_wrapping_negated_value(
    module: &DirModule<'_>,
    left: &dir::BuiltinOperand,
    right: &dir::BuiltinOperand,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    for (value, negated) in [(left, right), (right, left)] {
        let negated = negated.source.local_id;
        let Some(call) = module.member_call(negated) else {
            continue;
        };
        if module.language_member(negated)?
            == Some(dir::LanguageItem::Integer.member("wrappingNegate"))
            && !call.is_optional()
            && call.generic_arguments.is_empty()
            && call.arguments.is_empty()
            && module.is_same_computation(value.source.local_id, call.receiver)?
        {
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

    // retain one evaluation of the integer
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
    return value.wrappingNegate() & value;
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
function mask(left: uint32, right: uint32): uint32 {
    return left & right.wrappingNegate();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept signed negation because it traps for the minimum value.
    #[test]
    fn test_accepts_signed_negation() {
        let session = TestSession::dir(
            &MANUAL_ISOLATE_LOWEST_ONE,
            r#"
function lowest(value: int32): int32 {
    return value & -value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
