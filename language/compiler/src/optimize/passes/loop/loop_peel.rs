use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree, LoopAnalysis};
use crate::optimize::common::{clone_loop_blocks, terminator_remap};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Peel a single iteration from loops guarded at the latch.
    ///
    /// The peeled iteration preserves the loop guard by redirecting the backedge
    /// to the original header. This is safe for do-while style loops where the
    /// first iteration always executes.
    ///
    /// ```mir
    /// function @before(v0: u32) -> u32 {
    /// block0(v0: u32):
    ///     v1 = iconst 0u32
    ///     jump block1(v1)
    /// block1(v2: u32):
    ///     v3 = iadd v2, v0
    ///     v4 = iconst 1u32
    ///     v5 = iadd v2, v4
    ///     v6 = icmp_ult v5, v0
    ///     branch v6, block1(v5), block2
    /// block2:
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: u32) -> u32 {
    /// block0(v0: u32):
    ///     v1 = iconst 0u32
    ///     jump block3(v1)
    /// block1(v2: u32):
    ///     v3 = iadd v2, v0
    ///     v4 = iconst 1u32
    ///     v5 = iadd v2, v4
    ///     v6 = icmp_ult v5, v0
    ///     branch v6, block1(v5), block2
    /// block2:
    ///     return v3
    /// block3(v7: u32):
    ///     v8 = iadd v7, v0
    ///     v9 = iconst 1u32
    ///     v10 = iadd v7, v9
    ///     v11 = icmp_ult v10, v0
    ///     branch v11, block1(v10), block2
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
        tree: &mut mir::NodeTree,
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
    tree: &mut mir::NodeTree,
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
            let mut block = tree.get(cloned_id).clone();
            terminator_remap(&mut block.terminator, &block_map, &value_map);
            tree.replace(cloned_id, block);
        }

        // redirect preheader to the peeled iteration
        let mut preheader_block = tree.get(preheader).clone();
        preheader_block.terminator = mir::Terminator::Jump {
            target: cloned_header,
            arguments: preheader_args,
        };
        tree.replace(preheader, preheader_block);

        // redirect cloned backedge to original header
        let mut cloned_latch_block = tree.get(cloned_latch).clone();
        if !redirect_backedge(&mut cloned_latch_block, cloned_header, lp.header) {
            continue;
        }
        tree.replace(cloned_latch, cloned_latch_block);

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
    tree: &mir::NodeTree,
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
    let arguments = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Redirect a cloned backedge to the original header.
fn redirect_backedge(
    block: &mut mir::Block,
    cloned_header: mir::LocalNodeId<mir::Block>,
    original_header: mir::LocalNodeId<mir::Block>,
) -> bool {
    // redirect the cloned backedge toward the original header
    match &block.terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let mut new_then = *then_target;
            let mut new_else = *else_target;

            // rewrite the backedge to the original header
            if *then_target == cloned_header {
                new_then = original_header;
            } else if *else_target == cloned_header {
                new_else = original_header;
            } else {
                return false;
            }

            block.terminator = mir::Terminator::Branch {
                condition: *condition,
                then_target: new_then,
                then_arguments: then_arguments.clone(),
                else_target: new_else,
                else_arguments: else_arguments.clone(),
            };
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let mut success_target = success.target;
            let mut failure_target = failure.target;

            // rewrite the backedge to the original header
            if success.target == cloned_header {
                success_target = original_header;
            } else if failure.target == cloned_header {
                failure_target = original_header;
            } else {
                return false;
            }

            block.terminator = mir::Terminator::Check {
                condition: *condition,
                constraint: constraint.clone(),
                success: mir::CheckTarget {
                    target: success_target,
                    arguments: success.arguments.clone(),
                },
                failure: mir::CheckTarget {
                    target: failure_target,
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
        let input = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 0u32
    jump block1(v1)
block1(v2: u32):
    v3 = iadd v2, v0
    v4 = iconst 1u32
    v5 = iadd v2, v4
    v6 = icmp_ult v5, v0
    branch v6, block1(v5), block2
block2:
    return v3
}"#;

        let expected = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 0u32
    jump block3(v1)
block1(v2: u32):
    v3 = iadd v2, v0
    v4 = iconst 1u32
    v5 = iadd v2, v4
    v6 = icmp_ult v5, v0
    branch v6, block1(v5), block2
block2:
    return v3
block3(v7: u32):
    v8 = iadd v7, v0
    v9 = iconst 1u32
    v10 = iadd v7, v9
    v11 = icmp_ult v10, v0
    branch v11, block1(v10), block2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(expected);
    }

    /// Header guarded loops are not peeled.
    #[test]
    fn test_loop_peel_skips_header_guard() {
        let input = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 1u32
    jump block1(v1)
block1(v3: u32):
    v4 = icmp_ult v3, v0
    branch v4, block2(v3), block3
block2(v5: u32):
    v6 = iadd v5, v2
    jump block1(v6)
block3:
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(input);
    }

    /// Multiple exits prevent peeling.
    #[test]
    fn test_loop_peel_skips_multiple_exits() {
        let input = r#"function @test(v0: u32, v1: bool) -> u32 {
block0(v0: u32, v1: bool):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v0
    branch v5, block2(v4), block4
block2(v6: u32):
    branch v1, block3(v6), block5
block3(v7: u32):
    v8 = iadd v7, v3
    jump block1(v8)
block4:
    return v4
block5:
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopPeel);
        test.assert_output(input);
    }
}
