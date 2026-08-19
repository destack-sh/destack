use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, IntegerStep, Lint, LintOutput, LintResult};

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

    // inspect canonical three-part loops
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::For {
            initialization: Some(initialization),
            condition: Some(condition),
            increment: Some(increment),
            body,
            ..
        } = node
        else {
            continue;
        };
        let Some((dir::Mutability::Mutable, declarator)) =
            module.binding_declarator(*initialization)
        else {
            continue;
        };
        let Some(initializer) = declarator.value else {
            continue;
        };
        if module.integral_constant(initializer)? != Some(0) {
            continue;
        }
        let counter = module.declaration_symbol(declarator.pattern)?;
        let Some(IntegerStep::Increment(increment_target)) = module.integer_update(*increment)?
        else {
            continue;
        };
        if module.selected_symbol(increment_target)? != Some(counter) {
            continue;
        }

        // require an exclusive upper bound of the exact array length
        let Some((operator, [left, right])) = module.builtin_binary(*condition)? else {
            continue;
        };
        let length = if operator == dir::BinaryOperator::LessThan
            && module.selected_symbol(left.source.local_id)? == Some(counter)
        {
            right.source.local_id
        } else if operator == dir::BinaryOperator::GreaterThan
            && module.selected_symbol(right.source.local_id)? == Some(counter)
        {
            left.source.local_id
        } else {
            continue;
        };
        let dir::Expression::Member {
            left: array,
            is_optional: false,
            ..
        } = view.get(length)
        else {
            continue;
        };
        if !matches!(
            module.language_member(length)?,
            Some(member)
                if member == dir::LanguageItem::Array.member("length")
                    || member == dir::LanguageItem::Slice.member("length")
        ) {
            continue;
        }

        // require one plain assignment to the corresponding array element
        let Some(action) = view.get(*body).only_expression() else {
            continue;
        };
        let Some(assignment) = module.place_assignment(action) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }
        let dir::Expression::Index {
            left: target_array,
            index: Some(index),
            is_optional: false,
            ..
        } = view.get(assignment.target)
        else {
            continue;
        };
        if module.selected_symbol(*index)? != Some(counter)
            || !module.is_same_computation(*array, *target_array)?
            || !is_invariant_fill_value(module, assignment.value, *body, counter, &occurrences)?
        {
            continue;
        }

        // replace the loop with fill
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("index loop assigns one value to every element", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *array, assignment.value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one expression has one value throughout the fill loop.
fn is_invariant_fill_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Block>,
    counter: dir::GlobalSymbolId,
    occurrences: &[dir::BindingOccurrence],
) -> Result<bool, ProviderError> {
    if !module.is_repeatable_expression(expression)? {
        return Ok(false);
    }
    let view = module.view();

    // collect bindings read while evaluating the assigned value
    let reads = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.uses.contains(dir::BindingUse::READ)
                && view.is_inside(occurrence.node, expression.into_any())
        })
        .map(|occurrence| occurrence.symbol)
        .collect::<Vec<_>>();
    if reads.contains(&counter) {
        return Ok(false);
    }

    // reject bindings changed by the loop body
    let changes_read = occurrences.iter().any(|occurrence| {
        let is_mutation = occurrence.uses.may_mutate();
        let is_within_loop = view.is_inside(occurrence.node, body.into_any());

        is_mutation && is_within_loop && reads.contains(&occurrence.symbol)
    });

    Ok(!changes_read)
}

/// Build one fill call from a complete index assignment loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    array: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let array_extent = module.source_extent(array.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[array_extent, value_extent])? {
        return Ok(None);
    }

    // preserve the authored array and repeated value expressions
    let array = module.expression_source(array, dir::OperatorPrecedence::Postfix)?;
    let value = module.source(value_extent)?;
    let replacement = format!("{array}.fill({value});");
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
 ──▶ main.ds:2:5
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
--- a/main.ds
+++ b/main.ds

    1│ function clear(values: int32[]): void {
-   2│     for (let index: isize = 0; index < values.length; index++) {
-   3│         values[index] = 0;
-   4│     }
+   2│     values.fill(0);
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
function clear(values: &exclusive [int32]): void {
    for (let index: isize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: &exclusive [int32]): void {
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
