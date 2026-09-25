use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer absDiff over equivalent branching subtraction.
    pub MANUAL_ABS_DIFF {
        id: "manual-abs-diff",
        summary: "Prefer absDiff over equivalent branching subtraction",
        explanation: r#"
Choosing the subtraction order with a comparison manually computes an unsigned absolute difference.
Instead, you SHOULD call `.absDiff()` on one operand.
"#,
        example: {
            reported: r#"
function distance(left: uint32, right: uint32): uint32 {
    return left > right ? left - right : right - left;
}
"#,
            accepted: r#"
function distance(left: uint32, right: uint32): uint32 {
    return left.absDiff(right);
}
"#,
        },
        provenance: [Clippy("manual_abs_diff")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One manually computed unsigned absolute difference.
struct AbsoluteDifference {
    /// The first operand.
    left: dir::LocalNodeId<dir::Expression>,
    /// The second operand.
    right: dir::LocalNodeId<dir::Expression>,
}

/// Report ternaries that select an unsigned subtraction order.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect ternary expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let Some(difference) = absolute_difference(module, node)? else {
            continue;
        };

        // replace the comparison and both subtractions
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("branching subtraction manually computes absDiff", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &difference)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one canonical unsigned absolute-difference ternary.
fn absolute_difference(
    module: &DirModule<'_>,
    expression: &dir::Expression,
) -> Result<Option<AbsoluteDifference>, ProviderError> {
    let dir::Expression::If {
        form: dir::IfForm::Ternary,
        condition,
        then_expression,
        else_expression: Some(else_expression),
    } = expression
    else {
        return Ok(None);
    };
    let Some(condition) = condition.as_expression() else {
        return Ok(None);
    };
    let Some((comparison, [left, right])) = module.builtin_binary(condition)? else {
        return Ok(None);
    };
    let (greater, lesser) = match comparison {
        dir::BinaryOperator::GreaterThan | dir::BinaryOperator::GreaterThanOrEqual => (left, right),
        dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual => (right, left),
        _ => return Ok(None),
    };

    // require opposite subtraction orders in the corresponding branches
    let Some((dir::BinaryOperator::Subtract, [then_left, then_right])) =
        module.builtin_binary(*then_expression)?
    else {
        return Ok(None);
    };
    let Some((dir::BinaryOperator::Subtract, [else_left, else_right])) =
        module.builtin_binary(*else_expression)?
    else {
        return Ok(None);
    };
    if !module.is_same_operand(greater, then_left)?
        || !module.is_same_operand(lesser, then_right)?
        || !module.is_same_operand(lesser, else_left)?
        || !module.is_same_operand(greater, else_right)?
        || !module
            .primitive_type(left.source.local_id.into_any())?
            .is_some_and(dir::PrimitiveType::is_unsigned_integer)
    {
        return Ok(None);
    }

    Ok(Some(AbsoluteDifference {
        left: left.source.local_id,
        right: right.source.local_id,
    }))
}

/// Build the canonical absolute-difference call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    difference: &AbsoluteDifference,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let left_span = module.source_extent(difference.left.into_any())?;
    let right_span = module.source_extent(difference.right.into_any())?;
    if module.has_unretained_comment(span, &[left_span, right_span])? {
        return Ok(None);
    }

    // retain one evaluation of each operand
    let left = module.expression_source(difference.left, dir::OperatorPrecedence::Postfix)?;
    let right = module.source(right_span)?;
    let patch = Patch::replace(span, format!("{left}.absDiff({right})"));
    let suggestion = lint.fix("call `.absDiff()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an unsigned absolute-difference ternary.
    #[test]
    fn test_replaces_absolute_difference() {
        let session = TestSession::dir(
            &MANUAL_ABS_DIFF,
            r#"
function distance(left: uint32, right: uint32): uint32 {
    return left >= right ? left - right : right - left;
}
"#,
        );

        session.assert_fixes(
            r#"
function distance(left: uint32, right: uint32): uint32 {
    return left.absDiff(right);
}
"#,
        );
    }

    /// Accept signed subtraction because absDiff returns the corresponding unsigned type.
    #[test]
    fn test_accepts_signed_difference() {
        let session = TestSession::dir(
            &MANUAL_ABS_DIFF,
            r#"
function distance(left: int32, right: int32): int32 {
    return left > right ? left - right : right - left;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
