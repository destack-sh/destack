use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, IntegerStep, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer exclusive ranges over inclusive ranges offset by one.
    pub PREFER_EXCLUSIVE_RANGE {
        id: "prefer-exclusive-range",
        summary: "Prefer exclusive ranges over inclusive ranges offset by one",
        explanation: r#"
An inclusive range ending one below its upper bound expresses exclusion through extra arithmetic.
Instead, you SHOULD write the upper bound directly with `..`.

The rewritten range has type `Range<T>` rather than `RangeInclusive<T>`.
The rewritten range does not preserve an overflow trap when the upper bound is the minimum integer.
"#,
        example: {
            reported: r#"
declare const start: int32;
declare const end: int32;

const before = start..=end - 1;
"#,
            accepted: r#"
declare const start: int32;
declare const end: int32;

const before = start..end;
"#,
        },
        provenance: [Clippy("range_minus_one")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report inclusive integer ranges whose upper bound is decremented by one.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect inclusive ranges with an authored upper bound
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::RangeExpression {
            start,
            end: Some(end),
            end_kind: dir::RangeEnd::Inclusive,
        } = node
        else {
            continue;
        };
        let Some(IntegerStep::Decrement(end)) = module.integer_step(*end)? else {
            continue;
        };

        // replace the decremented inclusive bound with an exclusive bound
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("inclusive range decrements its upper bound", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *start, end)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the corresponding exclusive range.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    start: Option<dir::LocalNodeId<dir::Expression>>,
    end: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let start_span = start
        .map(|start| module.source_extent(start.into_any()))
        .transpose()?;
    let end_span = module.source_extent(end.into_any())?;
    let has_unretained_comment = if let Some(start_span) = start_span {
        module.has_unretained_comment(span, &[start_span, end_span])?
    } else {
        module.has_unretained_comment(span, &[end_span])?
    };
    if has_unretained_comment {
        return Ok(None);
    }

    // retain the authored bounds around the exclusive range operator
    let start = start
        .map(|start| module.expression_source(start, dir::OperatorPrecedence::Range))
        .transpose()?
        .unwrap_or_default();
    let end = module.expression_source(end, dir::OperatorPrecedence::Range)?;
    let replacement = format!("{start}..{end}");
    let replacement = if module.source_parentheses(expression.into_any()).is_some() {
        format!("({replacement})")
    } else {
        replacement
    };
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("use an exclusive range", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace bounded and startless inclusive ranges with decremented ends.
    #[test]
    fn test_replaces_decremented_inclusive_bounds() {
        let session = TestSession::dir(
            &PREFER_EXCLUSIVE_RANGE,
            r#"
declare const start: int32;
declare const end: int32;

const bounded = start..=end - 1;
const startless = ..=-1 + end;
const bigints = 0n..=100n - 1n;
"#,
        );

        session.assert_suggestions(
            r#"
declare const start: int32;
declare const end: int32;

const bounded = start..end;
const startless = ..end;
const bigints = 0n..100n;
"#,
        );
    }

    /// Accept exclusive, floating-point, non-unit, and user-defined bounds.
    #[test]
    fn test_accepts_other_ranges() {
        let session = TestSession::dir(
            &PREFER_EXCLUSIVE_RANGE,
            r#"
struct Count {}

extension of Count implements Subtract<Count> {
    type Output = Count;

    subtract(other: Count): Count {
        return other;
    }
}

function accepted(start: int32, end: int32, floating: float64, first: Count, second: Count): void {
    const exclusive = start..end - 1;
    const otherOffset = start..=end - 2;
    const floats = floating..=floating - 1;
    const custom = first..=second - second;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
