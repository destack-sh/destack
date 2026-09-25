use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer tuple assignment for swaps.
    pub PREFER_TUPLE_SWAP {
        id: "prefer-tuple-swap",
        summary: "Prefer tuple assignment for swaps",
        explanation: r#"
Swapping two places through a temporary spreads one parallel assignment across three statements.
Instead, you SHOULD assign the reversed tuple directly to both places.
"#,
        example: {
            reported: r#"
function swap(left: int32, right: int32): (int32, int32) {
    let first = left;
    let second = right;
    const temporary = first;
    first = second;
    second = temporary;

    return (first, second);
}
"#,
            accepted: r#"
function swap(left: int32, right: int32): (int32, int32) {
    let first = left;
    let second = right;
    (first, second) = (second, first);

    return (first, second);
}
"#,
        },
        provenance: [Clippy("manual_swap")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One three-statement swap through a temporary binding.
#[derive(Debug, Clone, Copy)]
struct ManualSwap {
    /// The first place.
    left: dir::LocalNodeId<dir::Expression>,
    /// The second place.
    right: dir::LocalNodeId<dir::Expression>,
}

/// Report adjacent three-statement swaps through temporary bindings.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect adjacent statement triples within every block
    for (_, block) in module.view().iter_nodes::<dir::Block>() {
        let expressions = block.iter_expressions().collect::<Vec<_>>();
        for statements in expressions.windows(3) {
            let Some(swap) = manual_swap(module, statements, &occurrences)? else {
                continue;
            };

            // replace the temporary sequence with parallel tuple assignment
            let first = module.statement_span(statements[0])?;
            let last = module.statement_span(statements[2])?;
            let span = first.merge(last);
            let mut diagnostic = lint.diagnostic("places are swapped through a temporary", span);
            if let Some(suggestion) = suggestion(module, lint, span, swap)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Select one direct swap through an otherwise unused temporary.
fn manual_swap(
    module: &DirModule<'_>,
    statements: &[dir::LocalNodeId<dir::Expression>],
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<ManualSwap>, ProviderError> {
    let [declaration, left_statement, right_statement] = statements else {
        return Ok(None);
    };
    let Some((_, declarator)) = module.binding_declarator(*declaration) else {
        return Ok(None);
    };
    let Some(left) = declarator.value else {
        return Ok(None);
    };
    let Some(assign_left) = module.place_assignment(*left_statement) else {
        return Ok(None);
    };
    let Some(assign_right) = module.place_assignment(*right_statement) else {
        return Ok(None);
    };
    if assign_left.operator != dir::AssignOperator::Assign
        || assign_right.operator != dir::AssignOperator::Assign
        || !module.is_discarded_expression(*left_statement)
        || !module.is_discarded_expression(*right_statement)
    {
        return Ok(None);
    }

    // require left = right followed by right = temporary
    let temporary = module.declaration_symbol(declarator.pattern)?;
    if !module.is_same_computation(left, assign_left.target)?
        || !module.is_same_computation(assign_left.value, assign_right.target)?
        || module.selected_symbol(assign_right.value)? != Some(temporary)
        || !module.is_duplicable_expression(left)?
        || !module.is_duplicable_expression(assign_left.value)?
    {
        return Ok(None);
    }
    let uses =
        module.binding_uses_outside(temporary, &[assign_right.value.into_any()], occurrences);
    if !uses.is_empty() {
        return Ok(None);
    }

    Ok(Some(ManualSwap {
        left,
        right: assign_left.value,
    }))
}

/// Replace one manual swap with parallel tuple assignment.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    swap: ManualSwap,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let left = module.source_extent(swap.left.into_any())?;
    let right = module.source_extent(swap.right.into_any())?;
    if module.has_unretained_comment(extent, &[left, right])? {
        return Ok(None);
    }

    // retain both authored places in reversed tuple order
    let left = module.source(left)?;
    let right = module.source(right)?;
    let replacement = format!("({left}, {right}) = ({right}, {left});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("swap both places with tuple assignment", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a direct scalar swap through a temporary.
    #[test]
    fn test_replaces_manual_swap() {
        let session = TestSession::dir(
            &PREFER_TUPLE_SWAP,
            r#"
function swap(left: int32, right: int32): (int32, int32) {
    let first = left;
    let second = right;
    const temporary = first;
    first = second;
    second = temporary;

    return (first, second);
}
"#,
        );

        session.assert_suggestions(
            r#"
function swap(left: int32, right: int32): (int32, int32) {
    let first = left;
    let second = right;
    (first, second) = (second, first);

    return (first, second);
}
"#,
        );
    }

    /// Accept a temporary used after the assignments.
    #[test]
    fn test_accepts_retained_temporary() {
        let session = TestSession::dir(
            &PREFER_TUPLE_SWAP,
            r#"
function swap(left: int32, right: int32): int32 {
    let first = left;
    let second = right;
    const temporary = first;
    first = second;
    second = temporary;

    return temporary;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept assignments that do not exchange the same places.
    #[test]
    fn test_accepts_distinct_assignments() {
        let session = TestSession::dir(
            &PREFER_TUPLE_SWAP,
            r#"
function assign(left: int32, right: int32): (int32, int32) {
    let first = left;
    let second = right;
    const temporary = first;
    first = second;
    second = 0;

    return (first, temporary);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
