use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `.isInfinite()` over equivalent infinity comparisons.
    pub MANUAL_IS_INFINITE {
        id: "manual-is-infinite",
        summary: "Prefer `.isInfinite()` over equivalent infinity comparisons",
        explanation: r#"
Comparing one floating-point value with both positive and negative infinity performs the same classification as `isInfinite`.
Instead, you SHOULD call `.isInfinite()` on that value.
"#,
        example: {
            reported: r#"
function infinite(value: float64): boolean {
    return value === Infinity || value === -Infinity;
}
"#,
            accepted: r#"
function infinite(value: float64): boolean {
    return value.isInfinite();
}
"#,
        },
        provenance: [Clippy("manual_is_infinite")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report paired equality checks against positive and negative infinity.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect compiler-defined logical disjunctions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((dir::BinaryOperator::Or, [left, right])) = module.builtin_binary(expression)?
        else {
            continue;
        };

        // require complementary equality checks over one duplicable float
        let Some((left_value, left_infinity)) =
            select_infinity_equality(module, left.source.local_id)?
        else {
            continue;
        };
        let Some((right_value, right_infinity)) =
            select_infinity_equality(module, right.source.local_id)?
        else {
            continue;
        };
        let is_float = matches!(
            module.primitive_type(left_value.into_any())?,
            Some(dir::PrimitiveType::Float(_))
        );
        if left_infinity.is_sign_negative() == right_infinity.is_sign_negative()
            || !module.is_same_computation(left_value, right_value)?
            || !is_float
        {
            continue;
        }

        // retain the canonical predicate implementation
        let is_infinite = dir::LanguageItem::Float.member("isInfinite");
        if module.is_within_language_member(expression.into_any(), is_infinite)? {
            continue;
        }

        // replace the complete comparison pair with the predicate call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("infinity is tested with two comparisons", span);
        if let Some(suggestion) = suggestion(module, lint, span, left_value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the non-infinite float from one builtin equality comparison.
fn select_infinity_equality(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, f64)>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    if !matches!(
        operator,
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
    ) {
        return Ok(None);
    }

    // normalize the infinite operand to the right
    let left = left.source.local_id;
    let right = right.source.local_id;
    if let Some(infinity) = module.infinity(right)?
        && module.infinity(left)?.is_none()
    {
        return Ok(Some((left, infinity)));
    }
    if let Some(infinity) = module.infinity(left)?
        && module.infinity(right)?.is_none()
    {
        return Ok(Some((right, infinity)));
    }

    Ok(None)
}

/// Build the canonical infinity predicate call.
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

    // retain the value with postfix-safe grouping
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(span, format!("{value}.isInfinite()"));
    let suggestion = lint.suggestion("call `.isInfinite()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace complementary infinity equality checks.
    #[test]
    fn test_replaces_infinity_equalities() {
        let session = TestSession::dir(
            &MANUAL_IS_INFINITE,
            r#"
function infinite(value: float64): boolean {
    return Number.NEGATIVE_INFINITY === value || value == Number.POSITIVE_INFINITY;
}
"#,
        );

        session.assert_suggestions(
            r#"
function infinite(value: float64): boolean {
    return value.isInfinite();
}
"#,
        );
    }

    /// Accept one-sided infinity checks and repeated signs.
    #[test]
    fn test_accepts_incomplete_infinity_tests() {
        let session = TestSession::dir(
            &MANUAL_IS_INFINITE,
            r#"
function positive(value: float64): boolean {
    return value === Infinity;
}
function repeated(value: float64): boolean {
    return value === Infinity || value === Number.POSITIVE_INFINITY;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept equality selected through a user-defined protocol.
    #[test]
    fn test_accepts_overloaded_equality() {
        let session = TestSession::dir(
            &MANUAL_IS_INFINITE,
            r#"
import { PartialEqual } from "tspp:ops";

struct Measure {}
extension of Measure implements PartialEqual<float64> {
    equal(&readonly this, other: &readonly float64): boolean {
        return false;
    }
}

declare const value: Measure;
const infinite = value == Infinity || value == -Infinity;
"#,
        );

        session.assert_no_diagnostics();
    }
}
