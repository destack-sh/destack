use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `.isFinite()` over equivalent infinity comparisons.
    pub MANUAL_IS_FINITE {
        id: "manual-is-finite",
        summary: "Prefer `.isFinite()` over equivalent infinity comparisons",
        explanation: r#"
Strict infinity bounds and an absolute-value comparison with positive infinity perform the same finite-value classification.
Instead, you SHOULD call `.isFinite()` on that value.

Paired `!=` checks are excluded because NaN satisfies both comparisons.
"#,
        example: {
            reported: r#"
function finite(value: float64): boolean {
    return value > -Infinity && value < Infinity;
}
"#,
            accepted: r#"
function finite(value: float64): boolean {
    return value.isFinite();
}
"#,
        },
        provenance: [Clippy("manual_is_finite")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report infinity comparisons equivalent to isFinite.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined float comparisons
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(value) = select_finite_value(module, expression)? else {
            continue;
        };
        if !matches!(
            module.primitive_type(value.into_any())?,
            Some(dir::PrimitiveType::Float(_))
        ) {
            continue;
        }

        // retain the canonical predicate implementation
        let is_finite = dir::LanguageItem::Float.member("isFinite");
        if module.is_within_language_member(expression.into_any(), is_finite)? {
            continue;
        }

        // replace the complete infinity comparison with the predicate call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("infinity comparison manually tests finiteness", span);
        if let Some(suggestion) = suggestion(module, lint, span, value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the tested value from one exact manual finiteness predicate.
fn select_finite_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    // recognize complementary strict infinity bounds
    if let Some((dir::BinaryOperator::And, [left, right])) = module.builtin_binary(expression)?
        && let Some((left_value, left_bound)) = select_infinity_bound(module, left.source.local_id)?
        && let Some((right_value, right_bound)) =
            select_infinity_bound(module, right.source.local_id)?
        && left_bound != right_bound
        && module.is_same_computation(left_value, right_value)?
    {
        return Ok(Some(left_value));
    }

    // recognize an absolute value strictly below positive infinity
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let absolute = match (
        operator,
        module.infinity(left.source.local_id)?,
        module.infinity(right.source.local_id)?,
    ) {
        (dir::BinaryOperator::LessThan, None, Some(infinity)) if infinity.is_sign_positive() => {
            left.source.local_id
        }
        (dir::BinaryOperator::GreaterThan, Some(infinity), None) if infinity.is_sign_positive() => {
            right.source.local_id
        }
        _ => return Ok(None),
    };
    let Some(call) = module.member_call(absolute) else {
        return Ok(None);
    };
    if module.language_member(absolute)? != Some(dir::LanguageItem::Float.member("abs"))
        || call.is_optional()
        || !call.generic_arguments.is_empty()
        || !call.arguments.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(call.receiver))
}

/// One side of a strict finite interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FiniteBound {
    /// The value is greater than negative infinity.
    Lower,
    /// The value is less than positive infinity.
    Upper,
}

/// Select the non-infinite float and interval side from one strict comparison.
fn select_infinity_bound(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, FiniteBound)>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let left = left.source.local_id;
    let right = right.source.local_id;

    // normalize each exact strict infinity bound
    let selected = match (operator, module.infinity(left)?, module.infinity(right)?) {
        (dir::BinaryOperator::GreaterThan, None, Some(value)) if value.is_sign_negative() => {
            Some((left, FiniteBound::Lower))
        }
        (dir::BinaryOperator::LessThan, Some(value), None) if value.is_sign_negative() => {
            Some((right, FiniteBound::Lower))
        }
        (dir::BinaryOperator::LessThan, None, Some(value)) if value.is_sign_positive() => {
            Some((left, FiniteBound::Upper))
        }
        (dir::BinaryOperator::GreaterThan, Some(value), None) if value.is_sign_positive() => {
            Some((right, FiniteBound::Upper))
        }
        _ => None,
    };

    Ok(selected)
}

/// Build the canonical finite predicate call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    span: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<tspp_source::DiagnosticSuggestion>, ProviderError> {
    let value_span = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(span, &[value_span])? {
        return Ok(None);
    }

    // retain the tested value with postfix-safe grouping
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(span, format!("{value}.isFinite()"));
    let suggestion = lint.suggestion("call `.isFinite()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace complementary strict infinity bounds.
    #[test]
    fn test_replaces_strict_infinity_bounds() {
        let session = TestSession::dir(
            &MANUAL_IS_FINITE,
            r#"
function finite(value: float64): boolean {
    return Number.NEGATIVE_INFINITY < value && Number.POSITIVE_INFINITY > value;
}
"#,
        );

        session.assert_suggestions(
            r#"
function finite(value: float64): boolean {
    return value.isFinite();
}
"#,
        );
    }

    /// Replace an absolute value below positive infinity.
    #[test]
    fn test_replaces_absolute_infinity_bound() {
        let session = TestSession::dir(
            &MANUAL_IS_FINITE,
            r#"
function finite(value: float64): boolean {
    return Infinity > value.abs();
}
"#,
        );

        session.assert_suggestions(
            r#"
function finite(value: float64): boolean {
    return value.isFinite();
}
"#,
        );
    }

    /// Preserve infinity inequalities because they accept NaN.
    #[test]
    fn test_accepts_infinity_inequalities() {
        let session = TestSession::dir(
            &MANUAL_IS_FINITE,
            r#"
function maybeFinite(value: float64): boolean {
    return value !== Infinity && value !== -Infinity;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept incomplete and inclusive infinity bounds.
    #[test]
    fn test_accepts_nonfinite_bounds() {
        let session = TestSession::dir(
            &MANUAL_IS_FINITE,
            r#"
function upper(value: float64): boolean {
    return value < Infinity;
}
function inclusive(value: float64): boolean {
    return value >= -Infinity && value <= Infinity;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
