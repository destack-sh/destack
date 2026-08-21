use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer bulk initialization over repeatedly pushing one invariant value.
    pub SAME_ITEM_PUSH {
        id: "same-item-push",
        summary: "Prefer bulk initialization over repeatedly pushing one invariant value",
        explanation: r#"
A counted loop that only pushes one invariant value repeatedly grows and initializes a collection element by element.
Instead, you SHOULD use bulk repeated initialization or resize the collection once.
"#,
        example: {
            reported: r#"
function append(values: int32[], count: isize): void {
    for (const _ of 0..count) {
        values.push(0);
    }
}
"#,
            accepted: r#"
function append(values: int32[], count: isize): void {
    if (count > 0) {
        values.resize(values.length + count, 0);
    }
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report counted loops whose only action pushes one invariant value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect finite counted loops with one direct action
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.counted_iteration(expression)? else {
            continue;
        };
        // skip bounds known to execute fewer than twice
        if let Some(end) = module.integral_constant(iteration.end)? {
            let distance = i128::from(end) - i128::from(iteration.start);
            let count = match iteration.end_kind {
                dir::RangeEnd::Open => distance.max(0),
                dir::RangeEnd::Inclusive => (distance + 1).max(0),
            };
            if count <= 1 {
                continue;
            }
        }
        let Some(action) = view.get(iteration.body).only_expression() else {
            continue;
        };
        let Some(push) = module.member_call(action) else {
            continue;
        };
        if push.is_optional()
            || !push.generic_arguments.is_empty()
            || module.language_member(action)? != Some(dir::LanguageItem::Array.member("push"))
        {
            continue;
        }
        let [argument] = push.arguments else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };

        // require a duplicable receiver and loop-invariant pushed value
        let excluded = iteration.binding.into_iter().collect::<Vec<_>>();
        if !module.is_duplicable_expression(push.receiver)?
            || !module.is_invariant_expression(
                *value,
                iteration.body.into_any(),
                &excluded,
                &occurrences,
            )?
        {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("counted loop repeatedly pushes one value", span)
            .help("use repeated bulk initialization or resize the collection once");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report repeated constant pushes over a finite range.
    #[test]
    fn test_reports_repeated_constant() {
        let session = TestSession::dir(
            &SAME_ITEM_PUSH,
            r#"
function append(values: int32[], count: isize): void {
    for (const _ of 0..count) {
        values.push(0);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[same-item-push]: counted loop repeatedly pushes one value
 ──▶ main.ds:2:5
  │
1 │ function append(values: int32[], count: isize): void {
2 │     for (const _ of 0..count) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         values.push(0);
  │         ^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │

 = help: use repeated bulk initialization or resize the collection once
"#,
        );
    }

    /// Accept a pushed value derived from the loop counter.
    #[test]
    fn test_accepts_counter_value() {
        let session = TestSession::dir(
            &SAME_ITEM_PUSH,
            r#"
function append(values: isize[], count: isize): void {
    for (const index of 0..count) {
        values.push(index);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a range that executes at most once.
    #[test]
    fn test_accepts_single_push() {
        let session = TestSession::dir(
            &SAME_ITEM_PUSH,
            r#"
function append(values: int32[]): void {
    for (const _ of 0..1) {
        values.push(0);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop that mutates a value before pushing it.
    #[test]
    fn test_accepts_changed_value() {
        let session = TestSession::dir(
            &SAME_ITEM_PUSH,
            r#"
function append(values: int32[], count: isize): void {
    let value: int32 = 0;
    for (const _ of 0..count) {
        value++;
        values.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
