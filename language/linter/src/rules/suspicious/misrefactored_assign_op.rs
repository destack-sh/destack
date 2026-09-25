use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow compound assignments that repeat the assigned place.
    pub MISREFACTORED_ASSIGN_OP {
        id: "misrefactored-assign-op",
        summary: "Disallow compound assignments that repeat the assigned place",
        explanation: r#"
A compound assignment already reads its target, so repeating that target in the assigned operation applies it twice.
Instead, you SHOULD retain only the other operand on the right-hand side.
"#,
        example: {
            reported: r#"
function add(value: int32, amount: int32): int32 {
    let result = value;
    result += result + amount;
    return result;
}
"#,
            accepted: r#"
function add(value: int32, amount: int32): int32 {
    let result = value;
    result += amount;
    return result;
}
"#,
        },
        provenance: [Clippy("misrefactored_assign_op")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One redundant compound-assignment operation.
struct RepeatedOperand {
    /// The complete repeated operation.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The operand retained by the correction.
    retained: dir::LocalNodeId<dir::Expression>,
}

/// Report compound assignments whose right side repeats the target operation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct compound assignments
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        let Ok(operator) = dir::BinaryOperator::try_from(assignment.operator) else {
            continue;
        };
        let Some(repeated) =
            repeated_operand(module, assignment.target, assignment.value, operator)?
        else {
            continue;
        };

        // remove the target repeated by the assigned operation
        let span = module.source_extent(repeated.expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("compound assignment repeats its target operation", span);
        if let Some(suggestion) = suggestion(module, lint, &repeated)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the redundant target operand from one matching builtin operation.
fn repeated_operand(
    module: &DirModule<'_>,
    target: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) -> Result<Option<RepeatedOperand>, ProviderError> {
    let Some((value_operator, [left, right])) = module.builtin_binary(value)? else {
        return Ok(None);
    };
    if value_operator != operator {
        return Ok(None);
    }

    // accept the target on the left for every compound operation
    if module.is_same_computation(target, left.source.local_id)? {
        return Ok(Some(RepeatedOperand {
            expression: value,
            retained: right.source.local_id,
        }));
    }

    // accept the target on the right only for commutative operations
    let is_commutative = matches!(
        operator,
        dir::BinaryOperator::Add
            | dir::BinaryOperator::Multiply
            | dir::BinaryOperator::ElementwiseAnd
            | dir::BinaryOperator::ElementwiseXor
            | dir::BinaryOperator::ElementwiseOr
    );
    if is_commutative && module.is_same_computation(target, right.source.local_id)? {
        return Ok(Some(RepeatedOperand {
            expression: value,
            retained: left.source.local_id,
        }));
    }

    Ok(None)
}

/// Build the corrected compound-assignment operand.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    repeated: &RepeatedOperand,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(repeated.expression.into_any())?;
    let retained = module.source_extent(repeated.retained.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain only the operand not supplied by the assignment target
    let retained =
        module.expression_source(repeated.retained, dir::OperatorPrecedence::Assignment)?;
    let patch = Patch::replace(extent, retained);
    let suggestion = lint.suggestion("remove the repeated target operand", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a target repeated on the left.
    #[test]
    fn test_replaces_left_target() {
        TestSession::assert_example(&MISREFACTORED_ASSIGN_OP);
    }

    /// Remove a target repeated on the right of a commutative operation.
    #[test]
    fn test_replaces_right_target() {
        let session = TestSession::dir(
            &MISREFACTORED_ASSIGN_OP,
            r#"
function add(value: int32, amount: int32): int32 {
    let result = value;
    result += amount + result;
    return result;
}
"#,
        );

        session.assert_suggestions(
            r#"
function add(value: int32, amount: int32): int32 {
    let result = value;
    result += amount;
    return result;
}
"#,
        );
    }

    /// Accept an ordinary compound assignment.
    #[test]
    fn test_accepts_distinct_operand() {
        let session = TestSession::dir(
            &MISREFACTORED_ASSIGN_OP,
            r#"
function add(value: int32, amount: int32): int32 {
    let result = value;
    result += amount;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a different operation on the right-hand side.
    #[test]
    fn test_accepts_different_operation() {
        let session = TestSession::dir(
            &MISREFACTORED_ASSIGN_OP,
            r#"
function combine(value: int32, amount: int32): int32 {
    let result = value;
    result += result * amount;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept the target on the right of a noncommutative operation.
    #[test]
    fn test_accepts_right_target_for_noncommutative_operation() {
        let session = TestSession::dir(
            &MISREFACTORED_ASSIGN_OP,
            r#"
function subtract(value: int32, amount: int32): int32 {
    let result = value;
    result -= amount - result;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
