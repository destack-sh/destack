use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer signum over equivalent sign branching.
    pub MANUAL_SIGNUM {
        id: "manual-signum",
        summary: "Prefer signum over equivalent sign branching",
        explanation: r#"
Branching between `-1`, `0`, and `1` from integer comparisons manually computes a sign value.
Instead, you SHOULD call `.signum()` on the compared integer.
"#,
        example: {
            reported: r#"
function sign(value: int32): int32 {
    return value > 0 ? 1 : value < 0 ? -1 : 0;
}
"#,
            accepted: r#"
function sign(value: int32): int32 {
    return value.signum();
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One nested ternary sign branch.
struct SignBranch {
    /// The compared integer.
    value: dir::LocalNodeId<dir::Expression>,
    /// The normalized comparison operator.
    operator: dir::BinaryOperator,
    /// The constant selected when the comparison succeeds.
    then_value: i64,
    /// The constant selected when the comparison fails.
    else_value: i64,
}

/// Report ternaries that manually compute integer signum.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect ternary expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let Some(value) = signum_value(module, node)? else {
            continue;
        };
        if !module.is_duplicable_expression(value)? {
            continue;
        }

        // replace the complete sign branch
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("branching manually computes integer signum", span);
        if let Some(suggestion) = suggestion(module, lint, expression, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the integer from one canonical signed or unsigned sign branch.
fn signum_value(
    module: &DirModule<'_>,
    expression: &dir::Expression,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
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
    let Some(first) = module.integer_comparison(condition, 0)? else {
        return Ok(None);
    };
    let Some(dir::PrimitiveType::Integer(integer)) =
        module.primitive_type(first.value.into_any())?
    else {
        return Ok(None);
    };

    // recognize the two-way unsigned sign branch
    if !integer.is_signed() {
        let branches = [
            (dir::BinaryOperator::GreaterThan, 1, 0),
            (dir::BinaryOperator::LessThanOrEqual, 0, 1),
            (dir::BinaryOperator::NotEqual, 1, 0),
            (dir::BinaryOperator::NotEqualStrict, 1, 0),
            (dir::BinaryOperator::EqualStrict, 0, 1),
            (dir::BinaryOperator::Equal, 0, 1),
        ];
        let then_value = module.integral_constant(*then_expression)?;
        let else_value = module.integral_constant(*else_expression)?;
        let matches = branches.iter().any(|(operator, branch_then, branch_else)| {
            first.operator == *operator
                && then_value == Some(*branch_then)
                && else_value == Some(*branch_else)
        });

        return Ok(matches.then_some(first.value));
    }

    // require the opposite sign test in the remaining branch
    let Some(inner) = sign_branch(module, *else_expression)? else {
        return Ok(None);
    };
    if !module.is_same_computation(first.value, inner.value)? {
        return Ok(None);
    }
    let then_value = module.integral_constant(*then_expression)?;
    let matches = matches!(
        (
            first.operator,
            then_value,
            inner.operator,
            inner.then_value,
            inner.else_value,
        ),
        (
            dir::BinaryOperator::GreaterThan,
            Some(1),
            dir::BinaryOperator::LessThan,
            -1,
            0,
        ) | (
            dir::BinaryOperator::GreaterThan,
            Some(1),
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
            0,
            -1,
        ) | (
            dir::BinaryOperator::LessThan,
            Some(-1),
            dir::BinaryOperator::GreaterThan,
            1,
            0,
        ) | (
            dir::BinaryOperator::LessThan,
            Some(-1),
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
            0,
            1,
        ) | (
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
            Some(0),
            dir::BinaryOperator::GreaterThan,
            1,
            -1,
        ) | (
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
            Some(0),
            dir::BinaryOperator::LessThan,
            -1,
            1,
        )
    );

    Ok(matches.then_some(first.value))
}

/// Select one ternary that compares an integer with zero and returns constants.
fn sign_branch(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<SignBranch>, ProviderError> {
    let dir::Expression::If {
        form: dir::IfForm::Ternary,
        condition,
        then_expression,
        else_expression: Some(else_expression),
    } = module.view().get(expression)
    else {
        return Ok(None);
    };
    let Some(condition) = condition.as_expression() else {
        return Ok(None);
    };
    let Some(comparison) = module.integer_comparison(condition, 0)? else {
        return Ok(None);
    };
    let (Some(then_value), Some(else_value)) = (
        module.integral_constant(*then_expression)?,
        module.integral_constant(*else_expression)?,
    ) else {
        return Ok(None);
    };

    Ok(Some(SignBranch {
        value: comparison.value,
        operator: comparison.operator,
        then_value,
        else_value,
    }))
}

/// Build the canonical integer signum call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[value_extent])? {
        return Ok(None);
    }

    // retain one evaluation of the compared integer
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, format!("{value}.signum()"));
    let suggestion = lint.fix("call `.signum()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a positive-first signed sign branch.
    #[test]
    fn test_replaces_signed_sign_branch() {
        TestSession::assert_example(&MANUAL_SIGNUM);
    }

    /// Replace a negative-first signed sign branch.
    #[test]
    fn test_replaces_reversed_signed_sign_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function sign(value: int32): int32 {
    return value < 0 ? -1 : value > 0 ? 1 : 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function sign(value: int32): int32 {
    return value.signum();
}
"#,
        );
    }

    /// Replace a signed sign branch that checks zero second.
    #[test]
    fn test_replaces_zero_second_signed_sign_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function sign(value: int32): int32 {
    return value > 0 ? 1 : value === 0 ? 0 : -1;
}
"#,
        );

        session.assert_fixes(
            r#"
function sign(value: int32): int32 {
    return value.signum();
}
"#,
        );
    }

    /// Replace a signed sign branch that checks zero first.
    #[test]
    fn test_replaces_zero_first_signed_sign_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function sign(value: int32): int32 {
    return value === 0 ? 0 : value < 0 ? -1 : 1;
}
"#,
        );

        session.assert_fixes(
            r#"
function sign(value: int32): int32 {
    return value.signum();
}
"#,
        );
    }

    /// Replace an unsigned sign branch.
    #[test]
    fn test_replaces_unsigned_sign_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function sign(value: uint32): uint32 {
    return value == 0 ? 0 : 1;
}
"#,
        );

        session.assert_fixes(
            r#"
function sign(value: uint32): uint32 {
    return value.signum();
}
"#,
        );
    }

    /// Replace an unsigned branch that tests nonzero explicitly.
    #[test]
    fn test_replaces_unsigned_nonzero_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function sign(value: uint32): uint32 {
    return value !== 0 ? 1 : 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function sign(value: uint32): uint32 {
    return value.signum();
}
"#,
        );
    }

    /// Accept sign branches whose result magnitude differs.
    #[test]
    fn test_accepts_nonunit_branch() {
        let session = TestSession::dir(
            &MANUAL_SIGNUM,
            r#"
function classify(value: int32): int32 {
    return value > 0 ? 2 : value < 0 ? -1 : 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
