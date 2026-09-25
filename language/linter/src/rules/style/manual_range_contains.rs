use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{ComparisonRange, DirModule, Lint, LintOutput, LintResult};

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
function isByte(value: int64): boolean {
    return value >= 0 && value <= 255;
}
"#,
            accepted: r#"
function isByte(value: int64): boolean {
    return (0..=255).contains(value);
}
"#,
        },
        provenance: [Clippy("manual_range_contains")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report comparison pairs that test membership in one range.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin short-circuit conjunctions and disjunctions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(range) = module.comparison_range(expression)? else {
            continue;
        };

        // replace the comparisons with one range membership call
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("comparisons manually test range membership", span);
        if let Some(fix) = fix(module, lint, expression, &range)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the corresponding range membership call.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    range: &ComparisonRange,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(range.value.into_any())?;
    let start_span = module.source_extent(range.start.into_any())?;
    let end_span = module.source_extent(range.end.into_any())?;
    if module.has_unretained_comment(span, &[value_span, start_span, end_span])? {
        return Ok(None);
    }

    // retain one evaluation of the value and both bounds
    let value = module.source(value_span)?;
    let start = module.expression_source(range.start, dir::OperatorPrecedence::Range)?;
    let end = module.expression_source(range.end, dir::OperatorPrecedence::Range)?;
    let range_operator = match range.end_kind {
        dir::RangeEnd::Open => "..",
        dir::RangeEnd::Inclusive => "..=",
    };
    let negation = if range.is_negated { "!" } else { "" };
    let replacement = format!("{negation}({start}{range_operator}{end}).contains({value})");
    let patch = Patch::replace(span, replacement);
    let fix = lint.fix("call `.contains()`", patch)?;

    Ok(Some(fix))
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
function contains(value: int64): boolean {
    const first = value >= 0 && value < 10;
    const second = value <= 10 && 0 <= value;
    const third = 10 > value && value >= 0;
    const fourth = 10 >= value && 0 <= value;

    return first || second || third || fourth;
}
"#,
        );

        session.assert_fixes(
            r#"
function contains(value: int64): boolean {
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
function outside(value: int64): boolean {
    const first = value < 0 || value >= 10;
    const second = value > 10 || 0 > value;

    return first || second;
}
"#,
        );

        session.assert_fixes(
            r#"
function outside(value: int64): boolean {
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

        session.assert_fixes(
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

        session.assert_fixes(
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

function accepted(value: int64, other: int32, first: Count, second: Count): boolean {
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
