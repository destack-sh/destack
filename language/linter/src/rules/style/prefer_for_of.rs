use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer for-of when a loop only indexes one array.
    pub PREFER_FOR_OF {
        id: "prefer-for-of",
        summary: "Prefer for-of when a loop only indexes one array",
        explanation: r#"
A complete index loop whose counter only selects array elements maintains an index it never uses directly.
Instead, you SHOULD iterate over the array values with a for-of loop.
"#,
        example: {
            reported: r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push(values[index]);
    }
}
"#,
            accepted: r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-for-of")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report complete array index loops that never use their index directly.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let accesses = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect zero based loops spanning one complete array
    for expression in module.view().iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.counted_iteration(expression)? else {
            continue;
        };
        let Some(index) = iteration.binding else {
            continue;
        };
        if iteration.start != 0 || iteration.end_kind != dir::RangeEnd::Open {
            continue;
        }
        let Some(array) = module.length_receiver(iteration.end)? else {
            continue;
        };
        if module.language_member(iteration.end)? != Some(dir::LanguageItem::Array.member("length"))
            || !module
                .binding_uses_within(index, iteration.end.into_any(), &occurrences)
                .is_empty()
        {
            continue;
        }

        // preserve one stable array value throughout iteration
        let Some(array_access) = module.access_resolution(array) else {
            continue;
        };
        // require every body use to be a direct index into the bounded array
        let Some(reads) = index_reads(module, index, array, iteration.body, &occurrences)? else {
            continue;
        };

        // preserve one stable array value throughout iteration, the element reads aside
        if module.takes_mutable_within(
            array_access.path(),
            iteration.body.into_any(),
            &accesses,
            &reads,
        ) {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint.diagnostic("loop index only selects array elements", span);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the array operands read through one index binding used only to index that array
/// within a loop body, none when the binding is used otherwise.
fn index_reads(
    module: &DirModule<'_>,
    index: dir::GlobalSymbolId,
    array: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Block>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<Vec<dir::LocalNodeIdAny>>, ProviderError> {
    let view = module.view();
    let uses = occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.symbol == index && view.is_inside(occurrence.node, body.into_any())
        })
        .collect::<Vec<_>>();
    if uses.is_empty()
        || uses
            .iter()
            .any(|occurrence| occurrence.uses != dir::BindingUse::READ)
    {
        return Ok(None);
    }

    // require each reference to be the complete index operand, keeping the array it reads
    let mut reads = Vec::new();
    for reference in module.binding_references(index) {
        if !view.is_inside(reference.into_any(), body.into_any()) {
            continue;
        }
        let Some(parent) = view.get_parent_for(reference) else {
            return Ok(None);
        };
        let Ok(access) = parent.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };
        let dir::Expression::Index {
            left,
            index: Some(selected),
            is_optional: false,
            ..
        } = view.get(access)
        else {
            return Ok(None);
        };
        if *selected != reference || !module.is_same_computation(*left, array)? {
            return Ok(None);
        }
        reads.push(left.into_any());
    }

    Ok(Some(reads))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a complete loop that only reads corresponding array elements.
    #[test]
    fn test_reports_array_index_loop() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push(values[index]);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of]: loop index only selects array elements
 ──▶ main.tspp:2:5
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     for (let index: isize = 0; index < values.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         output.push(values[index]);
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Accept a loop that also observes its index.
    #[test]
    fn test_accepts_observed_index() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], indexes: isize[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        indexes.push(index);
        output.push(values[index]);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop that indexes another array.
    #[test]
    fn test_accepts_other_array() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], other: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push(other[index]);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a partial array traversal.
    #[test]
    fn test_accepts_partial_iteration() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 1; index < values.length; index++) {
        output.push(values[index]);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept iteration that changes the bounded array.
    #[test]
    fn test_accepts_array_mutation() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push(values[index]);
        values.push(0);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an upper bound that depends on the loop index.
    #[test]
    fn test_accepts_counter_dependent_bound() {
        let session = TestSession::dir(
            &PREFER_FOR_OF,
            r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length + index; index++) {
        output.push(values[index]);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
