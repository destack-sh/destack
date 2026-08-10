use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer range membership over equivalent bound comparisons.
    pub MANUAL_RANGE_CONTAINS {
        id: "manual-range-contains",
        summary: "Prefer range membership over equivalent bound comparisons",
        explanation: r#"
Separate lower-bound and upper-bound comparisons obscure a range membership test.
Instead, you SHOULD call `.contains()` on the corresponding range.

Out-of-range disjunctions over floating-point values are excluded because negating `.contains()` changes NaN behavior.
"#,
        example: {
            reported: r#"
function isByte(value: int32): boolean {
    return value >= 0 && value <= 255;
}
"#,
            accepted: r#"
function isByte(value: int32): boolean {
    return (0..=255).contains(value);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One comparison against a lower or upper bound.
#[derive(Clone, Copy)]
struct BoundComparison {
    /// The repeated value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The interval bound.
    bound: dir::LocalNodeId<dir::Expression>,
    /// The operator with the repeated value on its left.
    operator: dir::BinaryOperator,
}

/// One ordered binary comparison.
#[derive(Clone, Copy)]
struct Comparison {
    /// The comparison operands.
    operands: [dir::LocalNodeId<dir::Expression>; 2],
    /// The comparison operator.
    operator: dir::BinaryOperator,
}

impl Comparison {
    /// Place the selected value operand on the left.
    fn orient(self, is_value_right: bool) -> Option<BoundComparison> {
        let [left, right] = self.operands;
        let comparison = if is_value_right {
            BoundComparison {
                value: right,
                bound: left,
                operator: self.operator.swapped()?,
            }
        } else {
            BoundComparison {
                value: left,
                bound: right,
                operator: self.operator,
            }
        };

        Some(comparison)
    }
}

/// One manual range membership expression.
#[derive(Clone, Copy)]
struct RangeMembership {
    /// The repeated value.
    value: dir::LocalNodeId<dir::Expression>,
    /// The included lower bound.
    start: dir::LocalNodeId<dir::Expression>,
    /// The upper bound.
    end: dir::LocalNodeId<dir::Expression>,
    /// Whether the upper bound is included.
    end_kind: dir::RangeEnd,
    /// Whether the range membership is negated.
    is_negated: bool,
}

/// Report comparison pairs that test membership in one range.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin short-circuit conjunctions and disjunctions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((logical, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(logical, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            continue;
        }
        let Some(membership) =
            range_membership(module, logical, left.source.local_id, right.source.local_id)?
        else {
            continue;
        };

        // replace the comparisons with one range membership call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("comparisons manually test range membership", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &membership)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select a canonical range membership test from one logical pair.
fn range_membership(
    module: &DirModule<'_>,
    logical: dir::BinaryOperator,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<RangeMembership>, ProviderError> {
    let Some([left, right]) = align_comparisons(module, left, right)? else {
        return Ok(None);
    };

    // select the represented range and whether its membership is negated
    use dir::BinaryOperator::{
        And, GreaterThan, GreaterThanOrEqual, LessThan, LessThanOrEqual, Or,
    };
    use dir::RangeEnd::{Inclusive, Open};
    let (start, end, end_kind, is_negated) = match (logical, left.operator, right.operator) {
        (And, GreaterThanOrEqual, LessThan) => (left, right, Open, false),
        (And, LessThan, GreaterThanOrEqual) => (right, left, Open, false),
        (And, GreaterThanOrEqual, LessThanOrEqual) => (left, right, Inclusive, false),
        (And, LessThanOrEqual, GreaterThanOrEqual) => (right, left, Inclusive, false),
        (Or, LessThan, GreaterThanOrEqual) => (left, right, Open, true),
        (Or, GreaterThanOrEqual, LessThan) => (right, left, Open, true),
        (Or, LessThan, GreaterThan) => (left, right, Inclusive, true),
        (Or, GreaterThan, LessThan) => (right, left, Inclusive, true),
        _ => return Ok(None),
    };

    // preserve NaN behavior in out-of-range floating-point disjunctions
    let value_domain = module.node_type(start.value.into_any())?.scalar_domain();
    if is_negated && value_domain == Some(dir::ScalarDomain::Float) {
        return Ok(None);
    }

    // require compatible, effect-free retained expressions
    let start_domain = module.node_type(start.bound.into_any())?.scalar_domain();
    let end_domain = module.node_type(end.bound.into_any())?.scalar_domain();
    if start_domain != end_domain
        || !module.is_repeatable_expression(start.value)?
        || !module.is_repeatable_expression(start.bound)?
        || !module.is_repeatable_expression(end.bound)?
    {
        return Ok(None);
    }

    Ok(Some(RangeMembership {
        value: start.value,
        start: start.bound,
        end: end.bound,
        end_kind,
        is_negated,
    }))
}

/// Align two comparisons around their repeated operand.
fn align_comparisons(
    module: &DirModule<'_>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<[BoundComparison; 2]>, ProviderError> {
    let Some(left) = comparison(module, left)? else {
        return Ok(None);
    };
    let Some(right) = comparison(module, right)? else {
        return Ok(None);
    };

    // select the operand position repeated by both comparisons
    let [left_first, left_second] = left.operands;
    let [right_first, right_second] = right.operands;
    let orientation = if module.is_repeated_expression(left_first, right_first)? {
        (false, false)
    } else if module.is_repeated_expression(left_first, right_second)? {
        (false, true)
    } else if module.is_repeated_expression(left_second, right_first)? {
        (true, false)
    } else if module.is_repeated_expression(left_second, right_second)? {
        (true, true)
    } else {
        return Ok(None);
    };
    let (Some(left), Some(right)) = (left.orient(orientation.0), right.orient(orientation.1))
    else {
        return Ok(None);
    };

    Ok(Some([left, right]))
}

/// Select one builtin or canonical bigint ordering comparison.
fn comparison(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<Comparison>, ProviderError> {
    // recognize compiler-defined scalar comparisons
    if let Some((operator, operands)) = module.builtin_binary(expression)?
        && matches!(
            operator,
            dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        )
    {
        let operands = [operands[0].source.local_id, operands[1].source.local_id];

        return Ok(Some(Comparison { operands, operator }));
    }

    // recognize the standard-library bigint comparison
    let Some((operator, operands)) = module.integral_binary(expression)? else {
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

    Ok(Some(Comparison { operands, operator }))
}

/// Build the corresponding range membership call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    membership: &RangeMembership,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(membership.value.into_any())?;
    let start_span = module.source_extent(membership.start.into_any())?;
    let end_span = module.source_extent(membership.end.into_any())?;
    if module.has_unretained_comment(span, &[value_span, start_span, end_span])? {
        return Ok(None);
    }

    // retain one evaluation of the value and both bounds
    let value = module.source(value_span)?;
    let start = module.operand_source(membership.start, dir::OperatorPrecedence::Range)?;
    let end = module.operand_source(membership.end, dir::OperatorPrecedence::Range)?;
    let range_operator = match membership.end_kind {
        dir::RangeEnd::Open => "..",
        dir::RangeEnd::Inclusive => "..=",
    };
    let negation = if membership.is_negated { "!" } else { "" };
    let replacement = format!("{negation}({start}{range_operator}{end}).contains({value})");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("call `.contains()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace each orientation of half-open and inclusive membership comparisons.
    #[test]
    fn test_replaces_in_range_comparisons() {
        let session = TestSession::dir(
            &MANUAL_RANGE_CONTAINS,
            r#"
function contains(value: int32): boolean {
    const first = value >= 0 && value < 10;
    const second = value <= 10 && 0 <= value;
    const third = 10 > value && value >= 0;
    const fourth = 10 >= value && 0 <= value;

    return first || second || third || fourth;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(value: int32): boolean {
    const first = (0..10).contains(value);
    const second = (0..=10).contains(value);
    const third = (0..10).contains(value);
    const fourth = (0..=10).contains(value);

    return first || second || third || fourth;
}
"#,
        );
    }

    /// Replace complements of half-open and inclusive integer ranges.
    #[test]
    fn test_replaces_out_of_range_comparisons() {
        let session = TestSession::dir(
            &MANUAL_RANGE_CONTAINS,
            r#"
function outside(value: int32): boolean {
    const first = value < 0 || value >= 10;
    const second = value > 10 || 0 > value;

    return first || second;
}
"#,
        );

        session.assert_suggestions(
            r#"
function outside(value: int32): boolean {
    const first = !(0..10).contains(value);
    const second = !(0..=10).contains(value);

    return first || second;
}
"#,
        );
    }

    /// Replace floating-point membership while preserving NaN-sensitive disjunctions.
    #[test]
    fn test_preserves_float_nan_behavior() {
        let session = TestSession::dir(
            &MANUAL_RANGE_CONTAINS,
            r#"
function contains(value: float64): boolean {
    const inside = value >= 0.0 && value <= 1.0;
    const outside = value < 0.0 || value > 1.0;

    return inside || outside;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(value: float64): boolean {
    const inside = (0.0..=1.0).contains(value);
    const outside = value < 0.0 || value > 1.0;

    return inside || outside;
}
"#,
        );
    }

    /// Replace comparisons using canonical standard-library bigint operators.
    #[test]
    fn test_replaces_bigint_comparisons() {
        let session = TestSession::dir(
            &MANUAL_RANGE_CONTAINS,
            r#"
function contains(value: bigint): boolean {
    return value >= 0n && value < 10n;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(value: bigint): boolean {
    return (0n..10n).contains(value);
}
"#,
        );
    }

    /// Accept strict lower bounds, unrelated values, effects, and custom comparisons.
    #[test]
    fn test_accepts_other_comparisons() {
        let session = TestSession::dir(
            &MANUAL_RANGE_CONTAINS,
            r#"
struct Count {}

extension of Count implements Compare<Count> {
    equal(other: &readonly Count): boolean {
        return true;
    }

    partialCompare(other: &readonly Count): Ordering | null {
        return Ordering.Equal;
    }

    compare(other: &readonly Count): Ordering {
        return Ordering.Equal;
    }
}

declare function next(): int32;

function accepted(value: int32, other: int32, first: Count, second: Count): boolean {
    const strictStart = value > 0 && value < 10;
    const unrelated = value >= 0 && other < 10;
    const effects = next() >= 0 && next() < 10;
    const custom = first >= second && first <= second;

    return strictStart || unrelated || effects || custom;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
