use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow mutating range bounds during iteration over that range.
    pub NO_MUTATED_RANGE_BOUND {
        id: "no-mutated-range-bound",
        summary: "Disallow mutating range bounds during iteration over that range",
        explanation: r#"
A range captures its bounds before iteration, so mutating their source storage cannot change the active traversal.
Instead, you SHOULD leave captured bounds unchanged or use a conditional loop when each iteration must observe an updated bound.
"#,
        example: {
            reported: r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        end -= 1;
        value;
    }
}
"#,
            accepted: r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        value;
    }
}
"#,
        },
        provenance: [Clippy("mut_range_bound")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report writes to storage captured by an actively iterated range.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect authored range iteration
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.for_of(expression) else {
            continue;
        };
        let callable = module.enclosing_callable_body(expression.into_any());
        let dir::Expression::RangeExpression { start, end, .. } = view.get(iteration.iterator)
        else {
            continue;
        };

        // accept bound mutations inside a loop that never runs another iteration
        if module.flows.is_single_pass(expression.into_any()) {
            continue;
        }

        // collect the most specific storage read by either range bound
        let bound_accesses = collect_bound_accesses(module, &occurrences, *start, *end);

        // report mutations of captured bounds inside the loop body
        for occurrence in &occurrences {
            if !occurrence.uses.may_mutate()
                || !view.is_inside(occurrence.node, iteration.body.into_any())
                || module.enclosing_callable_body(occurrence.node) != callable
            {
                continue;
            }
            if !bound_accesses
                .iter()
                .any(|bound| bound.starts_with(&occurrence.path))
            {
                continue;
            }

            let span = mutation_span(module, occurrence)?;
            let diagnostic =
                lint.diagnostic("range bound is mutated after the range captures it", span);
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Collect the most specific stable accesses read by two range bounds.
fn collect_bound_accesses(
    module: &DirModule<'_>,
    occurrences: &[dir::AccessOccurrence],
    start: Option<dir::LocalNodeId<dir::Expression>>,
    end: Option<dir::LocalNodeId<dir::Expression>>,
) -> Vec<dir::AccessPath> {
    let view = module.view();
    let mut accesses = Vec::<dir::AccessPath>::new();

    // retain leaf accesses rather than their receiver prefixes
    for occurrence in occurrences {
        let is_bound = start.is_some_and(|bound| view.is_inside(occurrence.node, bound.into_any()))
            || end.is_some_and(|bound| view.is_inside(occurrence.node, bound.into_any()));
        if !is_bound
            || !occurrence.uses.contains(dir::BindingUse::READ)
            || accesses
                .iter()
                .any(|selected| selected.starts_with(&occurrence.path))
        {
            continue;
        }

        accesses.retain(|selected| !occurrence.path.starts_with(selected));
        accesses.push(occurrence.path.clone());
    }

    accesses
}

/// Return the authored operation span responsible for one mutation.
fn mutation_span(
    module: &DirModule<'_>,
    occurrence: &dir::AccessOccurrence,
) -> Result<Span, ProviderError> {
    let view = module.view();

    // anchor mutable access at its explicit borrow operation
    if occurrence.uses.contains(dir::BindingUse::MUTATE)
        && let Some(expression) = view.ancestor::<dir::Expression>(occurrence.node)
        && matches!(view.get(expression), dir::Expression::BorrowOf { .. })
    {
        return module.source_extent(expression.into_any());
    }

    module.main_span(occurrence.node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report mutation of a captured range end.
    #[test]
    fn test_reports_mutated_end() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        end -= 1;
        value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-mutated-range-bound]: range bound is mutated after the range captures it
 ──▶ main.tspp:4:9
  │
2 │     let end = limit;
3 │     for (const value of 0..end) {
4 │         end -= 1;
  │         ^^^
5 │         value;
6 │     }
  │
"#,
        );
    }

    /// Report mutation of the exact field captured as a range bound.
    #[test]
    fn test_reports_mutated_bound_field() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(state: { end: int32; count: int32 }): void {
    for (const value of 0..state.end) {
        state.end -= 1;
        value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-mutated-range-bound]: range bound is mutated after the range captures it
 ──▶ main.tspp:3:15
  │
1 │ function visit(state: { end: int32; count: int32 }): void {
2 │     for (const value of 0..state.end) {
3 │         state.end -= 1;
  │               ^^^
4 │         value;
5 │     }
  │
"#,
        );
    }

    /// Report mutation through an exclusive borrow of a range bound.
    #[test]
    fn test_reports_borrowed_bound() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
declare function reset(value: &int32): void;

function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        reset(&end);
        value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-mutated-range-bound]: range bound is mutated after the range captures it
 ──▶ main.tspp:6:15
  │
4 │     let end = limit;
5 │     for (const value of 0..end) {
6 │         reset(&end);
  │               ^^^^
7 │         value;
8 │     }
  │
"#,
        );
    }

    /// Accept mutation of storage not used by the iterated range.
    #[test]
    fn test_accepts_unrelated_mutation() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(limit: int32): void {
    let count = 0;
    for (const value of 0..limit) {
        count += 1;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept mutation of a field distinct from the captured bound.
    #[test]
    fn test_accepts_distinct_field_mutation() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(state: { end: int32; count: int32 }): void {
    for (const value of 0..state.end) {
        state.count += 1;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a mutation authored in a nested callable that the loop does not execute.
    #[test]
    fn test_accepts_nested_callable_mutation() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function visit(limit: int32): void {
    let end = limit;
    for (const value of 0..end) {
        const change = (): void => {
            end -= 1;
        };
        change;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a bound mutation when control leaves the loop before another iteration.
    #[test]
    fn test_accepts_mutation_before_loop_exit() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
function consume(limit: int32): int32 {
    let end = limit;
    for (const value of 0..end) {
        end -= value;
        break;
    }
    return end;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a range whose bounds are constants.
    #[test]
    fn test_accepts_constant_bounds() {
        let session = TestSession::dir(
            &NO_MUTATED_RANGE_BOUND,
            r#"
for (const value of 0..10) {
    value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
