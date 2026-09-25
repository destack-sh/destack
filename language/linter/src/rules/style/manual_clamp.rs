use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer clamp over nested bound comparisons.
    pub MANUAL_CLAMP {
        id: "manual-clamp",
        summary: "Prefer clamp over nested bound comparisons",
        explanation: r#"
Nested comparisons that select the lower bound, upper bound, or original value manually clamp a number.
Instead, you SHOULD call `.clamp()` on the value.

`.clamp()` rejects inverted bounds and NaN bounds.
"#,
        example: {
            reported: r#"
function bounded(value: int32, minimum: int32, maximum: int32): int32 {
    return value < minimum ? minimum : value > maximum ? maximum : value;
}
"#,
            accepted: r#"
function bounded(value: int32, minimum: int32, maximum: int32): int32 {
    return value.clamp(minimum, maximum);
}
"#,
        },
        provenance: [Clippy("manual_clamp")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One manual clamp expression.
struct Clamp {
    /// The repeated value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The lower bound.
    minimum: dir::LocalNodeId<dir::Expression>,
    /// The upper bound.
    maximum: dir::LocalNodeId<dir::Expression>,
}

/// One comparison that selects a clamp bound.
struct ClampBound {
    /// The repeated value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The selected bound.
    bound: dir::LocalNodeId<dir::Expression>,
    /// The side of the interval selected by the comparison.
    kind: ClampBoundKind,
    /// Whether the comparison includes equality.
    is_inclusive: bool,
}

/// One clamp interval side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClampBoundKind {
    /// The minimum bound.
    Minimum,
    /// The maximum bound.
    Maximum,
}

/// Report nested ternaries that clamp one numeric value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect outer ternary expressions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let Some(clamp) = clamp(module, node)? else {
            continue;
        };

        // replace both comparisons with one clamp call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("nested comparisons manually clamp a value", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &clamp)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one canonical two-bound clamp ternary.
fn clamp(
    module: &DirModule<'_>,
    expression: &dir::Expression,
) -> Result<Option<Clamp>, ProviderError> {
    let dir::Expression::If {
        form: dir::IfForm::Ternary,
        condition,
        then_expression: outer_result,
        else_expression: Some(inner),
    } = expression
    else {
        return Ok(None);
    };
    let Some(outer_condition) = condition.as_expression() else {
        return Ok(None);
    };
    let Some(outer) = clamp_bound(module, outer_condition, *outer_result)? else {
        return Ok(None);
    };

    // require the false branch to apply the complementary bound
    let dir::Expression::If {
        form: dir::IfForm::Ternary,
        condition,
        then_expression: inner_result,
        else_expression: Some(value_result),
    } = module.view().get(*inner)
    else {
        return Ok(None);
    };
    let Some(inner_condition) = condition.as_expression() else {
        return Ok(None);
    };
    let Some(inner) = clamp_bound(module, inner_condition, *inner_result)? else {
        return Ok(None);
    };

    // require each branch to return the corresponding duplicable operand
    let primitive = module.primitive_type(outer.value.into_any())?;
    let is_supported = match primitive {
        Some(dir::PrimitiveType::Integer(_)) => true,
        Some(dir::PrimitiveType::Float(_)) => !outer.is_inclusive && !inner.is_inclusive,
        _ => false,
    };
    if outer.kind == inner.kind
        || !module.is_same_computation(outer.value, inner.value)?
        || !module.is_same_computation(outer.value, *value_result)?
        || !is_supported
    {
        return Ok(None);
    }
    let (minimum, maximum) = match outer.kind {
        ClampBoundKind::Minimum => (outer.bound, inner.bound),
        ClampBoundKind::Maximum => (inner.bound, outer.bound),
    };

    Ok(Some(Clamp {
        value: outer.value,
        minimum,
        maximum,
    }))
}

/// Select the value and returned bound from one comparison branch.
fn clamp_bound(
    module: &DirModule<'_>,
    condition: dir::LocalNodeId<dir::Expression>,
    result: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<ClampBound>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(condition)? else {
        return Ok(None);
    };
    if !matches!(
        operator,
        dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
    ) {
        return Ok(None);
    }

    // orient the comparison through the branch's returned bound
    let (value, bound, kind) = if module.is_same_computation(right.source.local_id, result)? {
        let kind = if matches!(
            operator,
            dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual
        ) {
            ClampBoundKind::Minimum
        } else {
            ClampBoundKind::Maximum
        };

        (left.source.local_id, right.source.local_id, kind)
    } else if module.is_same_computation(left.source.local_id, result)? {
        let kind = if matches!(
            operator,
            dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual
        ) {
            ClampBoundKind::Maximum
        } else {
            ClampBoundKind::Minimum
        };

        (right.source.local_id, left.source.local_id, kind)
    } else {
        return Ok(None);
    };

    let is_inclusive = matches!(
        operator,
        dir::BinaryOperator::LessThanOrEqual | dir::BinaryOperator::GreaterThanOrEqual
    );

    Ok(Some(ClampBound {
        value,
        bound,
        kind,
        is_inclusive,
    }))
}

/// Build the canonical clamp call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    clamp: &Clamp,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(clamp.value.into_any())?;
    let minimum_span = module.source_extent(clamp.minimum.into_any())?;
    let maximum_span = module.source_extent(clamp.maximum.into_any())?;
    if module.has_unretained_comment(span, &[value_span, minimum_span, maximum_span])? {
        return Ok(None);
    }

    // retain one evaluation of the value and each bound
    let value = module.expression_source(clamp.value, dir::OperatorPrecedence::Postfix)?;
    let minimum = module.source(minimum_span)?;
    let maximum = module.source(maximum_span)?;
    let replacement = format!("{value}.clamp({minimum}, {maximum})");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("call `.clamp()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace the canonical integer clamp ternary.
    #[test]
    fn test_replaces_clamp_ternary() {
        let session = TestSession::dir(
            &MANUAL_CLAMP,
            r#"
function bounded(value: int32, minimum: int32, maximum: int32): int32 {
    return value < minimum ? minimum : value > maximum ? maximum : value;
}
"#,
        );

        session.assert_suggestions(
            r#"
function bounded(value: int32, minimum: int32, maximum: int32): int32 {
    return value.clamp(minimum, maximum);
}
"#,
        );
    }

    /// Accept a branch that transforms the in-range value.
    #[test]
    fn test_accepts_other_ternary() {
        let session = TestSession::dir(
            &MANUAL_CLAMP,
            r#"
function bounded(value: int32, minimum: int32, maximum: int32): int32 {
    return value < minimum ? minimum : value > maximum ? maximum : value + 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept inclusive floating-point branches because signed zero can change.
    #[test]
    fn test_accepts_inclusive_float_clamp() {
        let session = TestSession::dir(
            &MANUAL_CLAMP,
            r#"
function bounded(value: float64, minimum: float64, maximum: float64): float64 {
    return value <= minimum ? minimum : value >= maximum ? maximum : value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
