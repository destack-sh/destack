use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer fill over an index loop assigning one repeated value.
    pub MANUAL_FILL {
        id: "manual-fill",
        summary: "Prefer fill over an index loop assigning one repeated value",
        explanation: r#"
An index loop from zero to an array or slice length that assigns one repeated value spells the fill operation manually.
Instead, you SHOULD call `fill` with that value.
"#,
        example: {
            reported: r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
            accepted: r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        },
        provenance: [Clippy("manual_slice_fill")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report complete index loops assigning one invariant value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect zero-based counted loops spanning one complete collection
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.counted_iteration(expression)? else {
            continue;
        };
        let Some(counter) = iteration.binding else {
            continue;
        };
        if iteration.start != 0 || iteration.end_kind != dir::RangeEnd::Open {
            continue;
        }
        let Some(collection) = module.length_receiver(iteration.end)? else {
            continue;
        };
        if !matches!(
            module.language_member(iteration.end)?,
            Some(member)
                if member == dir::LanguageItem::Array.member("length")
                    || member == dir::LanguageItem::Slice.member("length")
        ) {
            continue;
        }

        // require one plain assignment to the corresponding array element
        let Some(action) = view.get(iteration.body).only_expression() else {
            continue;
        };
        let Some(assignment) = module.place_assignment(action) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }
        let dir::Expression::Index {
            left: target,
            index: Some(index),
            is_optional: false,
            ..
        } = view.get(assignment.target)
        else {
            continue;
        };
        if module.selected_symbol(*index)? != Some(counter)
            || !module.is_same_computation(collection, *target)?
            || !module.is_invariant_expression(
                assignment.value,
                iteration.body.into_any(),
                &[counter],
                &occurrences,
            )?
        {
            continue;
        }

        // replace the loop with fill
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("index loop assigns one value to every element", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, collection, assignment.value)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one fill call from a complete index assignment loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    collection: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let collection_extent = module.source_extent(collection.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[collection_extent, value_extent])? {
        return Ok(None);
    }

    // preserve the authored collection and repeated value expressions
    let collection = module.expression_source(collection, dir::OperatorPrecedence::Postfix)?;
    let value = module.source(value_extent)?;
    let replacement = format!("{collection}.fill({value});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("call fill directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a complete zero-to-length Array assignment loop.
    #[test]
    fn test_replaces_index_fill_loop() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-fill]: index loop assigns one value to every element
 ──▶ main.tspp:2:5
  │
1 │ function clear(values: int32[]): void {
2 │     for (let index: isize = 0; index < values.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         values[index] = 0;
  │         ^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │

 = suggestion: call fill directly (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function clear(values: int32[]): void {
-   2│     for (let index: isize = 0; index < values.length; index++) {
-   3│         values[index] = 0;
-   4│     }
+   2│     values.fill(0);
    5│ }
"#,
        );
        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete fill over an authored index range.
    #[test]
    fn test_replaces_range_fill_loop() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (const index of 0..values.length) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete fill loop whose counter uses additive assignment.
    #[test]
    fn test_replaces_additive_counter() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index += 1) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete fill loop whose counter uses an explicit sum assignment.
    #[test]
    fn test_replaces_assigned_counter() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index = index + 1) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete fill loop with its bound written before the counter.
    #[test]
    fn test_replaces_reversed_bound() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; values.length > index; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete zero-to-length slice assignment loop.
    #[test]
    fn test_replaces_slice_fill_loop() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: &[int32]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: &[int32]): void {
    values.fill(0);
}
"#,
        );
    }

    /// Replace a complete fill loop with a negative constant value.
    #[test]
    fn test_replaces_negative_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = -1;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: int32[]): void {
    values.fill(-1);
}
"#,
        );
    }

    /// Accept a loop that assigns a value derived from its counter.
    #[test]
    fn test_accepts_counter_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function indices(values: isize[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = index;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with a nonzero lower bound.
    #[test]
    fn test_accepts_partial_range() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clearTail(values: int32[]): void {
    for (let index: isize = 1; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an effectful value evaluated once per iteration.
    #[test]
    fn test_accepts_effectful_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function fillRandom(values: float64[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = Math.random();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a value that can trap when an empty array skips the loop body.
    #[test]
    fn test_accepts_trapping_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function divide(values: int32[], divisor: int32): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = 1 / divisor;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a stored value whose evaluation can trap.
    #[test]
    fn test_accepts_trapping_access() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function copyFirst(values: int32[], source: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = source[0];
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
