use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, LoopAnalysis};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Canonicalize loops into a simplified form.
    ///
    /// This pass transforms loops to have:
    /// 1. A preheader: a single dedicated block that precedes the loop header
    /// 2. A single latch: one back edge to the header
    /// 3. Dedicated exit blocks: exit edges go to blocks only reachable from the loop
    ///
    /// These properties simplify subsequent loop transformations like LICM,
    /// loop rotation, and unrolling.
    ///
    /// A preheader is inserted when the loop header has:
    /// - Multiple predecessors from outside the loop
    /// - A single predecessor that also branches elsewhere (not a dedicated entry)
    /// - Is the function entry block
    ///
    /// Latches are merged when a loop has multiple back edges to its header.
    ///
    /// Exit blocks are split when they have predecessors from outside the loop.
    #[pass(id = "loop-simplify")]
    pub LoopSimplify,
    "Canonicalize loops (preheaders, single latch, dedicated exits)"
}

impl Pass for LoopSimplify {
    fn metadata(&self) -> &'static PassMetadata {
        LoopSimplify::metadata()
    }
}

impl FunctionPass for LoopSimplify {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, tree, context);
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }
        let cfg = context
            .analyses
            .get::<ControlFlowGraph>(function, tree, context);

        // collect work items using loop indices (avoids cloning)
        let mut preheader_work: Vec<usize> = Vec::new();
        let mut latch_work: Vec<usize> = Vec::new();
        let mut exit_work: Vec<ExitWork> = Vec::new();

        for (loop_idx, lp) in loops.loops().iter().enumerate() {
            // check if preheader is needed
            if needs_preheader(lp, &cfg, tree, entry) {
                preheader_work.push(loop_idx);
            }

            // check if latch merging is needed
            if !lp.has_single_latch() {
                latch_work.push(loop_idx);
            }

            // check if exit blocks need to be dedicated
            for &exit_block in &lp.exit_blocks {
                // skip exit blocks that are loop headers (they have their own preheaders)
                if loops.is_loop_header(exit_block) {
                    continue;
                }

                if needs_dedicated_exit(exit_block, lp, &cfg) {
                    // collect exiting blocks that target this exit
                    let exiting_to_exit: Vec<_> = lp
                        .exiting_blocks
                        .iter()
                        .filter(|&&eb| {
                            let block = tree.get(eb);
                            block.terminator.successors().contains(&exit_block)
                        })
                        .copied()
                        .collect();

                    if !exiting_to_exit.is_empty() {
                        exit_work.push(ExitWork {
                            exit_block,
                            exiting_blocks: exiting_to_exit,
                        });
                    }
                }
            }
        }

        // snapshot loop data we need before dropping analyses
        let preheader_data: Vec<_> = preheader_work
            .iter()
            .map(|&idx| {
                let lp = &loops.loops()[idx];
                PreheaderData {
                    header: lp.header,
                    loop_blocks: lp.blocks.iter().copied().collect(),
                }
            })
            .collect();

        let latch_data: Vec<_> = latch_work
            .iter()
            .map(|&idx| {
                let lp = &loops.loops()[idx];
                LatchData {
                    header: lp.header,
                    latches: lp.latches.clone(),
                }
            })
            .collect();

        // drop analyses before modifying
        drop(cfg);
        drop(loops);

        // nothing to do
        if preheader_data.is_empty() && latch_data.is_empty() && exit_work.is_empty() {
            return AnalysisPreservation::all();
        }

        let mut changed = false;

        // phase 1: insert preheaders
        for data in preheader_data {
            if insert_preheader(data.header, &data.loop_blocks, function, tree, entry) {
                changed = true;
            }
        }

        // phase 2: merge latches
        for data in latch_data {
            if merge_latches(data.header, &data.latches, function, tree) {
                changed = true;
            }
        }

        // phase 3: create dedicated exit blocks
        for work in exit_work {
            if insert_dedicated_exit(work.exit_block, &work.exiting_blocks, function, tree) {
                changed = true;
            }
        }

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Data for preheader insertion (snapshot of loop info).
struct PreheaderData {
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: Vec<mir::LocalNodeId<mir::Block>>,
}

/// Data for latch merging (snapshot of loop info).
struct LatchData {
    header: mir::LocalNodeId<mir::Block>,
    latches: Vec<mir::LocalNodeId<mir::Block>>,
}

/// Work item for dedicated exit block insertion.
struct ExitWork {
    exit_block: mir::LocalNodeId<mir::Block>,
    exiting_blocks: Vec<mir::LocalNodeId<mir::Block>>,
}

/// Check if a loop needs a preheader.
///
/// A loop needs a preheader if:
/// - The header is the function entry block
/// - Multiple blocks outside the loop jump to the header
/// - A single outside predecessor also branches elsewhere
fn needs_preheader(
    lp: &crate::optimize::analyses::Loop,
    cfg: &ControlFlowGraph,
    tree: &mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // entry block always needs preheader if it's a loop header
    if lp.header == entry {
        return true;
    }

    // find outside predecessors using CFG (O(predecessors) not O(blocks))
    let preds = cfg.predecessors(lp.header);
    let outside_preds: Vec<_> = preds
        .iter()
        .filter(|&&pred| !lp.blocks.contains(&pred))
        .collect();

    // need preheader if multiple outside predecessors
    if outside_preds.len() > 1 {
        return true;
    }

    // need preheader if single predecessor has other successors (branch)
    if outside_preds.len() == 1 {
        let pred = *outside_preds[0];
        let pred_block = tree.get(pred);
        if pred_block.terminator.successors().len() > 1 {
            return true;
        }
    }

    false
}

/// Check if an exit block needs to be dedicated (only reachable from the loop).
fn needs_dedicated_exit(
    exit_block: mir::LocalNodeId<mir::Block>,
    lp: &crate::optimize::analyses::Loop,
    cfg: &ControlFlowGraph,
) -> bool {
    // check if any predecessor is from outside the loop
    let preds = cfg.predecessors(exit_block);
    preds.iter().any(|&pred| !lp.blocks.contains(&pred))
}

/// Insert a preheader block for a loop.
///
/// Creates a new block that:
/// 1. Has the same parameters as the header
/// 2. Receives all edges from outside the loop
/// 3. Unconditionally jumps to the header
///
/// Returns true if changes were made.
fn insert_preheader(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &[mir::LocalNodeId<mir::Block>],
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    let header_block = tree.get(header);
    let header_params = &header_block.parameters;

    // create preheader with fresh parameters matching header's types
    let preheader_params: Vec<mir::TypedValue> = header_params
        .iter()
        .map(|p| mir::TypedValue {
            value: function.next_value(),
            ty: p.ty,
        })
        .collect();

    // preheader unconditionally jumps to header, forwarding its parameters
    let preheader_args: Vec<mir::Value> = preheader_params.iter().map(|p| p.value).collect();
    let preheader = mir::Block {
        parameters: preheader_params,
        instructions: vec![],
        terminator: mir::Terminator::Jump {
            target: header,
            arguments: preheader_args,
        },
    };
    let preheader_id = tree.insert(preheader);
    function.blocks.push(preheader_id);

    // redirect all outside predecessors to preheader
    let mut redirected = false;
    for &block_id in &function.blocks {
        // skip preheader itself and blocks inside the loop
        if block_id == preheader_id || loop_blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        if let Some(new_terminator) = redirect_terminator(&block.terminator, header, preheader_id) {
            tree.get_mut(block_id).terminator = new_terminator;
            redirected = true;
        }
    }

    // if header was the entry, preheader becomes entry
    if header == entry {
        function.entry = Some(preheader_id);
        redirected = true;
    }

    redirected
}

/// Redirect terminator edges from old_target to new_target.
///
/// Returns Some(new_terminator) if any edges were redirected, None otherwise.
fn redirect_terminator(
    terminator: &mir::Terminator,
    old_target: mir::LocalNodeId<mir::Block>,
    new_target: mir::LocalNodeId<mir::Block>,
) -> Option<mir::Terminator> {
    match terminator {
        mir::Terminator::Jump { target, arguments } => {
            if *target == old_target {
                Some(mir::Terminator::Jump {
                    target: new_target,
                    arguments: arguments.clone(),
                })
            } else {
                None
            }
        }

        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let redirect_then = *then_target == old_target;
            let redirect_else = *else_target == old_target;

            if redirect_then || redirect_else {
                Some(mir::Terminator::Branch {
                    condition: *condition,
                    then_target: if redirect_then {
                        new_target
                    } else {
                        *then_target
                    },
                    then_arguments: then_arguments.clone(),
                    else_target: if redirect_else {
                        new_target
                    } else {
                        *else_target
                    },
                    else_arguments: else_arguments.clone(),
                })
            } else {
                None
            }
        }

        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let redirect_default = *default == old_target;
            let redirect_cases: Vec<bool> = cases.iter().map(|c| c.target == old_target).collect();
            let any_case_redirected = redirect_cases.iter().any(|&r| r);

            if redirect_default || any_case_redirected {
                let new_cases: Vec<_> = cases
                    .iter()
                    .zip(redirect_cases.iter())
                    .map(|(case, &redirect)| mir::SwitchCase {
                        value: case.value,
                        target: if redirect { new_target } else { case.target },
                        arguments: case.arguments.clone(),
                    })
                    .collect();

                Some(mir::Terminator::Switch {
                    value: *value,
                    default: if redirect_default {
                        new_target
                    } else {
                        *default
                    },
                    default_arguments: default_arguments.clone(),
                    cases: new_cases,
                })
            } else {
                None
            }
        }

        _ => None,
    }
}

/// Merge multiple latch blocks into a single latch.
///
/// Creates a new latch block that:
/// 1. Has the same parameters as the header
/// 2. All original latches redirect to it
/// 3. Unconditionally jumps to the header
///
/// Returns true if changes were made.
fn merge_latches(
    header: mir::LocalNodeId<mir::Block>,
    latches: &[mir::LocalNodeId<mir::Block>],
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) -> bool {
    if latches.len() <= 1 {
        return false;
    }

    let header_block = tree.get(header);
    let header_params = &header_block.parameters;

    // create merged latch with fresh parameters matching header's types
    let latch_params: Vec<mir::TypedValue> = header_params
        .iter()
        .map(|p| mir::TypedValue {
            value: function.next_value(),
            ty: p.ty,
        })
        .collect();

    // merged latch jumps to header, forwarding its parameters
    let latch_args: Vec<mir::Value> = latch_params.iter().map(|p| p.value).collect();
    let new_latch = mir::Block {
        parameters: latch_params,
        instructions: vec![],
        terminator: mir::Terminator::Jump {
            target: header,
            arguments: latch_args,
        },
    };
    let new_latch_id = tree.insert(new_latch);
    function.blocks.push(new_latch_id);

    // redirect all original latches to the new merged latch
    for &latch_id in latches {
        let latch_block = tree.get(latch_id);
        if let Some(new_terminator) =
            redirect_terminator(&latch_block.terminator, header, new_latch_id)
        {
            tree.get_mut(latch_id).terminator = new_terminator;
        }
    }

    true
}

/// Insert a dedicated exit block for a loop exit.
///
/// Creates a new block that:
/// 1. Has the same parameters as the original exit block
/// 2. Receives edges from exiting blocks within the loop
/// 3. Unconditionally jumps to the original exit block
///
/// This ensures the exit block is only reachable from the loop, which is
/// required for LCSSA form and simplifies loop transformations.
///
/// Returns true if changes were made.
fn insert_dedicated_exit(
    exit_block: mir::LocalNodeId<mir::Block>,
    exiting_blocks: &[mir::LocalNodeId<mir::Block>],
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) -> bool {
    let exit_block_data = tree.get(exit_block);
    let exit_params = &exit_block_data.parameters;

    // create dedicated exit with fresh parameters matching exit block's types
    let dedicated_params: Vec<mir::TypedValue> = exit_params
        .iter()
        .map(|p| mir::TypedValue {
            value: function.next_value(),
            ty: p.ty,
        })
        .collect();

    // dedicated exit jumps to original exit, forwarding its parameters
    let dedicated_args: Vec<mir::Value> = dedicated_params.iter().map(|p| p.value).collect();
    let dedicated_exit = mir::Block {
        parameters: dedicated_params,
        instructions: vec![],
        terminator: mir::Terminator::Jump {
            target: exit_block,
            arguments: dedicated_args,
        },
    };
    let dedicated_id = tree.insert(dedicated_exit);
    function.blocks.push(dedicated_id);

    // redirect exiting blocks to the dedicated exit
    let mut redirected = false;
    for &exiting_id in exiting_blocks {
        let exiting_block = tree.get(exiting_id);
        if let Some(new_terminator) =
            redirect_terminator(&exiting_block.terminator, exit_block, dedicated_id)
        {
            tree.get_mut(exiting_id).terminator = new_terminator;
            redirected = true;
        }
    }

    redirected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop with multiple entry edges gets a preheader inserted.
    #[test]
    fn test_insert_preheader_for_multiple_entries() {
        // block0 and block1 both enter block2, so preheader needed
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    jump block2
block2:
    branch v1, block2, block3
block3:
    return
}"#;
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block4
block1:
    jump block4
block2:
    branch v1, block2, block3
block3:
    return
block4:
    jump block2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop already in simplified form is unchanged.
    #[test]
    fn test_preserve_already_canonical() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Self-loop at entry block gets a preheader.
    #[test]
    fn test_insert_preheader_for_entry_loop() {
        // block0 is entry and has self-loop, needs preheader
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block0(v0), block1
block1:
    return
}"#;
        // preheader (block2) becomes new entry with fresh param v1
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block0(v0), block1
block1:
    return
block2(v1: bool):
    jump block0(v1)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);

        // verify entry changed to preheader
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }

    /// Single predecessor with branch still gets preheader.
    #[test]
    fn test_insert_preheader_for_branch_predecessor() {
        // block0 branches to block1, not a dedicated entry
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    branch v1, block1, block2
block2:
    return
}"#;
        // block3 = preheader for block1's loop
        // block4 = dedicated exit (block2 has outside predecessor block0)
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block3, block2
block1:
    branch v1, block1, block4
block2:
    return
block3:
    jump block1
block4:
    jump block2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// While-style loop with dedicated entry is unchanged.
    #[test]
    fn test_preserve_while_loop_with_preheader() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Header parameters are preserved through preheader.
    #[test]
    fn test_preserve_header_parameters() {
        // block2 has parameter v3, preheader must forward it
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block2(v1)
block1(v2: i32):
    jump block2(v2)
block2(v3: i32):
    branch v0, block2(v3), block3
block3:
    return v3
}"#;
        // preheader (block4) has parameter v4 and forwards to block2
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block4(v1)
block1(v2: i32):
    jump block4(v2)
block2(v3: i32):
    branch v0, block2(v3), block3
block3:
    return v3
block4(v4: i32):
    jump block2(v4)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Switch terminator entering loop gets redirected to preheader.
    #[test]
    fn test_switch_entry_to_loop() {
        // switch default and case 0 both go to block1
        let input = r#"function @test(v0: i32, v1: bool) -> void {
block0(v0: i32, v1: bool):
    switch v0, block1, 0 => block1, 1 => block2
block1:
    branch v1, block1, block2
block2:
    return
}"#;
        // block3 = preheader, block4 = dedicated exit
        let expected = r#"function @test(v0: i32, v1: bool) -> void {
block0(v0: i32, v1: bool):
    switch v0, block3, 0 => block3, 1 => block2
block1:
    branch v1, block1, block4
block2:
    return
block3:
    jump block1
block4:
    jump block2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop with no outside predecessors (infinite loop from entry) gets preheader.
    #[test]
    fn test_infinite_loop_from_entry() {
        // block0 is entry and only jumps to itself
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block0(v0)
}"#;
        // preheader (block1) becomes entry with fresh param v1
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block0(v0)
block1(v1: bool):
    jump block0(v1)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);

        // verify entry changed to preheader
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }

    /// Multiple latches are merged into single latch.
    #[test]
    fn test_merge_multiple_latches() {
        // block2 and block3 both jump to block1 (two latches)
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    branch v1, block1, block4
block4:
    return
}"#;
        // merged latch (block5) inserted, block2 and block3 redirect to it
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block5
block3:
    branch v1, block5, block4
block4:
    return
block5:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Multiple latches with different arguments to header are merged correctly.
    #[test]
    fn test_merge_latches_with_different_arguments() {
        // block2 jumps to block1(v5), block4 jumps to block1(v7)
        let input = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    branch v0, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    jump block1(v5)
block3:
    branch v1, block4, block5
block4:
    v6 = iconst 10i32
    v7 = iadd v3, v6
    jump block1(v7)
block5:
    return v3
}"#;
        // merged latch (block6) receives parameter v8 and forwards to header
        let expected = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    branch v0, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v3, v4
    jump block6(v5)
block3:
    branch v1, block4, block5
block4:
    v6 = iconst 10i32
    v7 = iadd v3, v6
    jump block6(v7)
block5:
    return v3
block6(v8: i32):
    jump block1(v8)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Exit block with outside predecessor gets dedicated exit inserted.
    #[test]
    fn test_insert_dedicated_exit() {
        // block2 is exit block, reachable from both block1 (in loop) and block0 (outside)
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    branch v1, block1, block2
block2:
    return
}"#;
        // block3 = preheader for loop
        // block4 = dedicated exit for block2
        let expected = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block3, block2
block1:
    branch v1, block1, block4
block2:
    return
block3:
    jump block1
block4:
    jump block2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Exit block with parameters gets dedicated exit with forwarded params.
    #[test]
    fn test_dedicated_exit_with_parameters() {
        let input = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    branch v0, block1(v2), block2(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v1, block1(v5), block2(v5)
block2(v6: i32):
    return v6
}"#;
        // block3 = preheader, block4 = dedicated exit
        let expected = r#"function @test(v0: bool, v1: bool) -> i32 {
block0(v0: bool, v1: bool):
    v2 = iconst 0i32
    branch v0, block3(v2), block2(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    branch v1, block1(v5), block4(v5)
block2(v6: i32):
    return v6
block3(v7: i32):
    jump block1(v7)
block4(v8: i32):
    jump block2(v8)
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop with dedicated exit already is unchanged.
    #[test]
    fn test_preserve_dedicated_exit() {
        // block3 is only reachable from block2 (in the loop)
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block2, block3
block2:
    jump block1
block3:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Nested loops both get simplified.
    #[test]
    fn test_simplify_nested_loops() {
        // outer loop: header=block1, latches={block2, block3}
        // inner loop: header=block2, latch={block2} (self-loop)
        let input = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block1, block4
block1:
    branch v1, block2, block3
block2:
    branch v2, block2, block1
block3:
    jump block1
block4:
    return
}"#;
        // block5 = preheader for outer loop (block1, header ID comes first)
        // block6 = preheader for inner loop (block2)
        // block7 = merged latch for outer loop
        // Note: no dedicated exit needed (block4 has no preds from inside loop,
        // and block1 is a loop header so gets preheader instead)
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block5, block4
block1:
    branch v1, block6, block3
block2:
    branch v2, block2, block7
block3:
    jump block7
block4:
    return
block5:
    jump block1
block6:
    jump block2
block7:
    jump block1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Triple-nested loops all get preheaders.
    #[test]
    fn test_simplify_triple_nested_loops() {
        // outer: header=block1, inner: header=block2, innermost: header=block3
        let input = r#"function @test(v0: bool, v1: bool, v2: bool, v3: bool) -> void {
block0(v0: bool, v1: bool, v2: bool, v3: bool):
    branch v0, block1, block4
block1:
    branch v1, block2, block4
block2:
    branch v2, block3, block1
block3:
    branch v3, block3, block2
block4:
    return
}"#;
        // block5 = outer preheader
        // block6 = inner preheader
        // block7 = innermost preheader
        // block8 = dedicated exit for outer loop (block4 has outside pred block0)
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool, v3: bool) -> void {
block0(v0: bool, v1: bool, v2: bool, v3: bool):
    branch v0, block5, block4
block1:
    branch v1, block6, block8
block2:
    branch v2, block7, block1
block3:
    branch v3, block3, block2
block4:
    return
block5:
    jump block1
block6:
    jump block2
block7:
    jump block3
block8:
    jump block4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop needing both preheader and latch merge.
    #[test]
    fn test_preheader_and_latch_merge_combined() {
        // block0 branches to loop (needs preheader)
        // block2 and block3 both back to header (needs latch merge)
        let input = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block1, block4
block1:
    branch v1, block2, block3
block2:
    jump block1
block3:
    branch v2, block1, block4
block4:
    return
}"#;
        // block5 = preheader, block6 = merged latch, block7 = dedicated exit
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block5, block4
block1:
    branch v1, block2, block3
block2:
    jump block6
block3:
    branch v2, block6, block7
block4:
    return
block5:
    jump block1
block6:
    jump block1
block7:
    jump block4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Loop needing preheader, latch merge, and dedicated exit.
    #[test]
    fn test_all_canonicalizations_combined() {
        // needs preheader (block0 branches), latch merge (block2, block3),
        // and dedicated exit (block4 reachable from outside too)
        let input = r#"function @test(v0: bool, v1: bool, v2: bool, v3: bool) -> void {
block0(v0: bool, v1: bool, v2: bool, v3: bool):
    branch v0, block1, block4
block1:
    branch v1, block2, block3
block2:
    branch v2, block1, block4
block3:
    branch v3, block1, block4
block4:
    return
}"#;
        // block5 = preheader, block6 = merged latch, block7 = dedicated exit
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool, v3: bool) -> void {
block0(v0: bool, v1: bool, v2: bool, v3: bool):
    branch v0, block5, block4
block1:
    branch v1, block2, block3
block2:
    branch v2, block6, block7
block3:
    branch v3, block6, block7
block4:
    return
block5:
    jump block1
block6:
    jump block1
block7:
    jump block4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_preserve_no_loops() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Irreducible control flow is unchanged (not a natural loop).
    #[test]
    fn test_preserve_irreducible_cfg() {
        // two entry points to the "loop" region - not a natural loop
        // block1 and block2 can both be entered from outside and from each other
        let input = r#"function @test(v0: bool, v1: bool) -> void {
block0(v0: bool, v1: bool):
    branch v0, block1, block2
block1:
    branch v1, block2, block3
block2:
    branch v1, block1, block3
block3:
    return
}"#;
        // no natural loops detected, so unchanged
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_unchanged(input);
    }

    /// Multiple independent loops are all simplified.
    #[test]
    fn test_simplify_multiple_independent_loops() {
        // two separate loops with no nesting
        let input = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block1, block3
block1:
    branch v1, block1, block2
block2:
    branch v2, block3, block4
block3:
    branch v1, block3, block4
block4:
    return
}"#;
        // block5 = preheader for block1 (block0 branches)
        // block6 = preheader for block3 (block0 and block2 enter it)
        // block7 = dedicated exit for block3 (block4 has outside pred block2)
        // Note: block2 doesn't need dedicated exit (only pred is block1 which is in loop1)
        let expected = r#"function @test(v0: bool, v1: bool, v2: bool) -> void {
block0(v0: bool, v1: bool, v2: bool):
    branch v0, block5, block6
block1:
    branch v1, block1, block2
block2:
    branch v2, block6, block4
block3:
    branch v1, block3, block7
block4:
    return
block5:
    jump block1
block6:
    jump block3
block7:
    jump block4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.assert_output(expected);
    }
}
