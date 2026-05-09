use std::collections::HashSet;

use crate::declare_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, DominatorTree, LoopAnalysis};
use crate::common::mir::{clone_loop_blocks, terminator_remap};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Peel a single iteration from loops guarded at the latch.
    ///
    /// The peeled iteration preserves the loop guard by redirecting the backedge
    /// to the original header. This is safe for do-while style loops where the
    /// first iteration always executes.
    ///
    /// ```mir
    /// function before(v0: uint32): uint32 {
    /// b0(v0: uint32):
    ///     v1 = 0uint32
    ///     jump b1(v1)
    /// b1(v2: uint32):
    ///     v3 = int.add v2, v0
    ///     v4 = 1uint32
    ///     v5 = int.add v2, v4
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b1(v5), b2
    /// b2:
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32): uint32 {
    /// b0(v0: uint32):
    ///     v1 = 0uint32
    ///     jump b3(v1)
    /// b1(v2: uint32):
    ///     v3 = int.add v2, v0
    ///     v4 = 1uint32
    ///     v5 = int.add v2, v4
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b1(v5), b2
    /// b2:
    ///     return v3
    /// b3(v7: uint32):
    ///     v8 = int.add v7, v0
    ///     v9 = 1uint32
    ///     v10 = int.add v7, v9
    ///     v11 = int.lt.u v10, v0
    ///     branch v11, b1(v10), b2
    /// }
    /// ```
    #[pass(id = "loop-peel")]
    pub LoopPeel,
    "Peel one iteration of latch-guarded loops"
}

impl FunctionPass for LoopPeel {
    /// Run loop peeling on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        let changed = run_loop_peel(function, tree, ctx);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "LoopPeel"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "loop-peel"
    }
}

/// Run loop peeling on a single function and report whether it changed.
fn run_loop_peel(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // gather analyses
    let analyses = ctx.function_analyses(function, tree);
    let loops = analyses.get::<LoopAnalysis>().clone();
    let cfg = analyses.get::<ControlFlowGraph>().clone();
    let domtree = analyses.get::<DominatorTree>().clone();

    // bail out when no loops are present
    if loops.num_loops() == 0 {
        return false;
    }

    // track whether we rewrote any loops
    let mut changed = false;
    function.recompute_next_value_id(tree);

    // peel each eligible loop once
    for lp in loops.loops() {
        // require a single latch and single exiting block
        if !lp.has_single_latch() || lp.exiting_blocks.len() != 1 {
            continue;
        }

        let latch = lp.latches[0];
        let exiting_block = lp.exiting_blocks[0];
        if latch != exiting_block {
            continue;
        }

        // locate the unique preheader
        let Some((preheader, preheader_args)) =
            find_preheader(lp.header, &lp.blocks, &cfg, &domtree, tree)
        else {
            continue;
        };

        // clone the loop once to form the peeled iteration
        let (block_map, value_map) = clone_loop_blocks(&lp.blocks, function, tree);

        // map header and latch to their cloned counterparts
        let cloned_header = block_map[&lp.header];
        let cloned_latch = block_map[&latch];

        // remap cloned terminators to cloned blocks
        for &cloned_id in block_map.values() {
            let block = tree.get(cloned_id).clone();
            let terminator_id = block.terminator;
            let mut terminator = tree.get(terminator_id).clone();
            terminator_remap(&mut terminator, &block_map, &value_map);
            tree.replace(cloned_id, block);
            tree.replace(terminator_id, terminator);
        }

        // redirect preheader to the peeled iteration
        let preheader_block = tree.get(preheader).clone();
        let preheader_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: cloned_header.into(),
                arguments: preheader_args.into_iter().map(Into::into).collect(),
            },
        };
        tree.replace(preheader, preheader_block);
        tree.replace(tree.get(preheader).terminator, preheader_terminator);

        // redirect cloned backedge to original header
        let cloned_latch_block = tree.get(cloned_latch).clone();
        let cloned_latch_terminator_id = cloned_latch_block.terminator;
        let mut cloned_latch_terminator = tree.get(cloned_latch_terminator_id).clone();
        if !redirect_backedge(&mut cloned_latch_terminator, cloned_header, lp.header) {
            continue;
        }
        tree.replace(cloned_latch, cloned_latch_block);
        tree.replace(cloned_latch_terminator_id, cloned_latch_terminator);

        // append cloned blocks to the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.blocks.push(block_id);
        }

        changed = true;
    }

    changed
}

/// Find the loop preheader and its header arguments.
fn find_preheader(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // collect outside predecessors
    let mut outside_preds: Vec<_> = cfg
        .predecessors(header)
        .iter()
        .copied()
        .filter(|pred| !loop_blocks.contains(pred))
        .collect();

    // require a single outside predecessor
    if outside_preds.len() != 1 {
        return None;
    }
    let preheader = outside_preds.pop()?;

    // ensure the preheader dominates the header
    if !domtree.dominates(preheader, header) {
        return None;
    }

    // require a direct jump to the header
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);
    let arguments = match preheader_terminator {
        mir::Terminator::Jump { target } if target.block.block()? == header => target
            .arguments
            .iter()
            .copied()
            .map(|argument| argument.value())
            .collect::<Option<_>>()?,
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Redirect a cloned backedge to the original header.
fn redirect_backedge(
    terminator: &mut mir::Terminator,
    cloned_header: mir::LocalNodeId<mir::Block>,
    original_header: mir::LocalNodeId<mir::Block>,
) -> bool {
    // redirect the cloned backedge toward the original header
    match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let mut new_then = then_target.block;
            let mut new_else = else_target.block;

            // rewrite the backedge to the original header
            if then_target.block == cloned_header.into() {
                new_then = original_header.into();
            } else if else_target.block == cloned_header.into() {
                new_else = original_header.into();
            } else {
                return false;
            }

            *terminator = mir::Terminator::Branch {
                condition: *condition,
                then_target: mir::BlockTarget {
                    block: new_then,
                    arguments: then_target.arguments.clone(),
                },
                else_target: mir::BlockTarget {
                    block: new_else,
                    arguments: else_target.arguments.clone(),
                },
            };
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let mut success_target = success.block;
            let mut failure_target = failure.block;

            // rewrite the backedge to the original header
            if success.block == cloned_header.into() {
                success_target = original_header.into();
            } else if failure.block == cloned_header.into() {
                failure_target = original_header.into();
            } else {
                return false;
            }

            *terminator = mir::Terminator::Check {
                constraint: constraint.clone(),
                success: mir::BlockTarget {
                    block: success_target,
                    arguments: success.arguments.clone(),
                },
                failure: mir::BlockTarget {
                    block: failure_target,
                    arguments: failure.arguments.clone(),
                },
            };
        }
        _ => return false,
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Latch guarded loops are peeled once.
    #[test]
    fn test_loop_peel_single_iteration() {
        let input = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    jump b1(v1)
b1(v2: uint32):
    v3: uint32 = int.add v2, v0
    v4: uint32 = 1uint32
    v5: uint32 = int.add v2, v4
    v6: boolean = int.lt.u v5, v0
    branch v6, b1(v5), b2
b2:
    return v3
}"#;

        let expected = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    jump b3(v1)
b1(v2: uint32):
    v3: uint32 = int.add v2, v0
    v4: uint32 = 1uint32
    v5: uint32 = int.add v2, v4
    v6: boolean = int.lt.u v5, v0
    branch v6, b1(v5), b2
b2:
    return v3
b3(v7: uint32):
    v8: uint32 = int.add v7, v0
    v9: uint32 = 1uint32
    v10: uint32 = int.add v7, v9
    v11: boolean = int.lt.u v10, v0
    branch v11, b1(v10), b2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(expected);
    }

    /// Header guarded loops are not peeled.
    #[test]
    fn test_loop_peel_skips_header_guard() {
        let input = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    jump b1(v1)
b1(v3: uint32):
    v4: boolean = int.lt.u v3, v0
    branch v4, b2(v3), b3
b2(v5: uint32):
    v6: uint32 = int.add v5, v2
    jump b1(v6)
b3:
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(input);
    }

    /// Multiple exits prevent peeling.
    #[test]
    fn test_loop_peel_skips_multiple_exits() {
        let input = r#"
function test(v0: uint32, v1: boolean): uint32 {
b0(v0: uint32, v1: boolean):
    v2: uint32 = 0uint32
    v3: uint32 = 1uint32
    jump b1(v2)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v0
    branch v5, b2(v4), b4
b2(v6: uint32):
    branch v1, b3(v6), b5
b3(v7: uint32):
    v8: uint32 = int.add v7, v3
    jump b1(v8)
b4:
    return v4
b5:
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(input);
    }
}
