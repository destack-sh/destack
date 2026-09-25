use destack_dir as dir;
use destack_mir as mir;

use crate::rules::declare_lint;
use crate::{Lint, LintOutput, LintResult, MirModule};

declare_lint! {
    /// Disallow clones proven unnecessary by ownership and liveness.
    pub REDUNDANT_CLONE {
        id: "redundant-clone",
        summary: "Disallow clones proven unnecessary by ownership and liveness",
        explanation: r#"
Cloning an owned value at its final use duplicates the value before the original is discarded.
Instead, you SHOULD move the original value when it is not used afterward.
"#,
        example: {
            reported: r#"
import { rc } from "destack:memory";

function retain(value: rc.Rc<int32>): rc.Rc<int32> {
    return value.clone();
}
"#,
            accepted: r#"
import { rc } from "destack:memory";

function retain(value: rc.Rc<int32>): rc.Rc<int32> {
    return value;
}
"#,
        },
        provenance: [Clippy("redundant_clone")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Report canonical clone calls whose source can be moved instead.
fn check(module: &mut MirModule<'_>, lint: &Lint) -> LintResult {
    let tree = &module.lowered.tree;
    let mut output = LintOutput::default();

    // inspect every defined function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if !function.is_defined() {
            continue;
        }

        // analyse structural overlap and ownership at each clone
        let constants = module.analyses.constant(function_id, tree);
        let liveness = module.analyses.liveness(function_id, tree);
        let moves = module.analyses.moves(function_id, tree);
        let places = module.analyses.place(function_id, tree);
        let origin = module.analyses.origin(function_id, tree);
        let loans = origin.loans();

        // inspect reachable instructions with their forward analysis states
        for &block_id in function.blocks() {
            let Some(mut state) = origin.entry(block_id).cloned() else {
                continue;
            };
            let block = tree.get(block_id);
            let mut live = liveness.cursor(tree, block_id);

            for &instruction_id in &block.instructions {
                // select the receiver of each canonical clone call
                let receiver = module
                    .language_call(instruction_id, dir::LanguageItem::Clone.member("clone"))?
                    .and_then(|call| tree.get_values(call.arguments).first().copied());

                // report move-only sources that are movable and dead afterward
                let is_copy = receiver.is_some_and(|receiver| {
                    function
                        .value_type(receiver)
                        .and_then(|ty| tree.get(ty).pointee_type())
                        .is_some_and(|pointee| mir::is_copy(tree, pointee, &function.generics))
                });
                if let Some(receiver) = receiver
                    && !is_copy
                {
                    let receiver_origin = state.value(receiver);
                    if let [loan_id] = receiver_origin.loans() {
                        let loan = loans.get(*loan_id);

                        if loan.parents().is_empty()
                            && loan.place().is_some_and(|place| {
                                is_redundant_clone(
                                    place,
                                    *loan_id,
                                    instruction_id.into_any(),
                                    &state,
                                    &live,
                                    |left, right| {
                                        left.may_overlap(
                                            right,
                                            &constants,
                                            &places,
                                            function_id,
                                            tree,
                                        )
                                    },
                                    &moves,
                                    &places,
                                    loans,
                                )
                            })
                        {
                            let diagnostic = lint
                                .diagnostic(
                                    "owned value is cloned at its final use",
                                    module.anchor(instruction_id.into_any())?,
                                )
                                .help("move the original value instead");
                            output.report(diagnostic);
                        }
                    }
                }

                // advance origin and liveness together
                let instruction = tree.get(instruction_id);
                let cx = mir::OriginContext::new(function_id, tree, &places, loans);
                state.advance(&cx, instruction_id);
                live.advance(instruction, tree);
            }
        }
    }

    Ok(output)
}

/// Return whether one cloned place can be moved at the current operation.
fn is_redundant_clone(
    place: &mir::Place,
    receiver_loan: mir::LoanId,
    at: mir::LocalNodeIdAny,
    state: &mir::OriginState,
    live: &mir::LivenessCursor<'_>,
    may_overlap: impl FnMut(&mir::Place, &mir::Place) -> bool,
    moves: &mir::MoveTable,
    places: &mir::PlaceTable,
    loans: &mir::LoanTable,
) -> bool {
    let Some(path) = moves.place(place) else {
        return false;
    };

    // require the complete storage root to be dead after this call
    let root = moves.get(moves.root(path)).place.origin;
    if live.find_representation(root, places).is_some() {
        return false;
    }

    // reject moves blocked by any other live loan
    let mut active = state.active_loans(
        |value| live.contains_value(value),
        |place| live.find_representation(place.origin, places).is_some(),
    );
    active.retain(|loan| *loan != receiver_loan);

    loans
        .blocking_change(place, at, &active, places, may_overlap)
        .is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a clone at the final use of an owned value.
    #[test]
    fn test_reports_clone_at_final_use() {
        let session = TestSession::dir(
            &REDUNDANT_CLONE,
            r#"
import { rc } from "destack:memory";

function retain(value: rc.Rc<int32>): rc.Rc<int32> {
    return value.clone();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[redundant-clone]: owned value is cloned at its final use
 ──▶ main.ds:4:12
  │
2 │
3 │ function retain(value: rc.Rc<int32>): rc.Rc<int32> {
4 │     return value.clone();
  │            ^^^^^^^^^^^^^
5 │ }
  │

 = help: move the original value instead
"#,
        );
    }

    /// Accept a clone whose source remains live.
    #[test]
    fn test_accepts_clone_before_later_use() {
        let session = TestSession::dir(
            &REDUNDANT_CLONE,
            r#"
import { rc } from "destack:memory";

function retain(value: rc.Rc<int32>): (rc.Rc<int32>, rc.Rc<int32>) {
    const cloned = value.clone();

    return (cloned, value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a clone while a borrow of the source remains live.
    #[test]
    fn test_accepts_clone_while_source_is_borrowed() {
        let session = TestSession::dir(
            &REDUNDANT_CLONE,
            r#"
import { rc } from "destack:memory";

function retain(value: rc.Rc<int32>): (rc.Rc<int32>, usize) {
    const borrowed: &immutable rc.Rc<int32> = &immutable value;
    const cloned = value.clone();

    return (cloned, borrowed.strongCount());
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept clone through a borrowed value.
    #[test]
    fn test_accepts_borrowed_value() {
        let session = TestSession::dir(
            &REDUNDANT_CLONE,
            r#"
import { rc } from "destack:memory";

function retain(value: &immutable rc.Rc<int32>): rc.Rc<int32> {
    return value.clone();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave Copy clones to the more specific rule.
    #[test]
    fn test_accepts_copy_value() {
        let session = TestSession::dir(
            &REDUNDANT_CLONE,
            r#"
function duplicate(value: int32): int32 {
    return value.clone();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
