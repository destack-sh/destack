use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, LoopAnalysis};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
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

impl FunctionPass for LoopSimplify {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (loops, cfg) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LoopAnalysis>().clone(),
                analyses.get::<ControlFlowGraph>().clone(),
            )
        };
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // run loop simplification
        let changed = run_loop_simplify(entry, function, tree, &loops, &cfg);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "LoopSimplify"
    }

    fn id(&self) -> &'static str {
        "loop-simplify"
    }
}

/// Core loop simplification logic. Returns true if changes were made.
fn run_loop_simplify(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    loops: &LoopAnalysis,
    cfg: &ControlFlowGraph,
) -> bool {
    // collect work items using loop indices (avoids cloning)
    let mut preheader_work: Vec<usize> = Vec::new();
    let mut latch_work: Vec<usize> = Vec::new();
    let mut exit_work: Vec<ExitWork> = Vec::new();

    for (loop_idx, lp) in loops.loops().iter().enumerate() {
        // check if preheader is needed
        if needs_preheader(lp, cfg, tree, entry) {
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

            if needs_dedicated_exit(exit_block, lp, cfg) {
                // collect exiting blocks that target this exit
                let exiting_to_exit: Vec<_> = lp
                    .exiting_blocks
                    .iter()
                    .filter(|&&eb| {
                        let block = tree.get(eb);
                        let terminator = tree.get(block.terminator);
                        terminator
                            .successors()
                            .iter()
                            .any(|target| target.block() == Some(exit_block))
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

    // snapshot loop data we need
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

    // nothing to do
    if preheader_data.is_empty() && latch_data.is_empty() && exit_work.is_empty() {
        return false;
    }

    function.recompute_next_value_id(tree);

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

    changed
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

/// Create fresh parameters matching an existing parameter list.
fn fresh_parameters_like(
    parameters: &[mir::Parameter],
    function: &mut mir::Function,
) -> Vec<mir::Parameter> {
    parameters
        .iter()
        .map(
            |parameter| match (parameter.value.value(), parameter.ty.ty()) {
                (Some(_), Some(ty)) => mir::Parameter {
                    value: function.next_typed_value(ty).into(),
                    ty: ty.into(),
                },
                _ => *parameter,
            },
        )
        .collect()
}

/// Check if a loop needs a preheader.
///
/// A loop needs a preheader if:
/// - The header is the function entry block
/// - Multiple blocks outside the loop jump to the header
/// - A single outside predecessor also branches elsewhere
fn needs_preheader(
    lp: &crate::common::mir::analysis::Loop,
    cfg: &ControlFlowGraph,
    tree: &mir::Tree,
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
        let pred_terminator = tree.get(pred_block.terminator);
        if pred_terminator.successors().len() > 1 {
            return true;
        }
    }

    false
}

/// Check if an exit block needs to be dedicated (only reachable from the loop).
fn needs_dedicated_exit(
    exit_block: mir::LocalNodeId<mir::Block>,
    lp: &crate::common::mir::analysis::Loop,
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
    tree: &mut mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    let header_block = tree.get(header);
    let header_params = &header_block.parameters;

    // create preheader with fresh parameters matching header's types
    let preheader_params = fresh_parameters_like(header_params, function);

    // preheader unconditionally jumps to header, forwarding its parameters
    let preheader_args: Vec<_> = preheader_params
        .iter()
        .map(|parameter| parameter.value)
        .collect();
    let preheader_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: header.into(),
            arguments: preheader_args,
        },
    });
    let preheader = mir::Block {
        name: None,
        parameters: preheader_params.clone(),
        instructions: vec![],
        terminator: preheader_terminator,
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
        let terminator = tree.get(block.terminator);
        if let Some(new_terminator) = redirect_terminator(terminator, header, preheader_id) {
            tree.replace(block.terminator, new_terminator);
            redirected = true;
        }
    }

    // if header was the entry, preheader becomes entry
    if header == entry {
        // keep function parameters aligned with the entry block
        function.parameters = preheader_params.clone();
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
        mir::Terminator::Jump { target } => {
            if target.block.block() == Some(old_target) {
                Some(mir::Terminator::Jump {
                    target: mir::BlockTarget {
                        block: new_target.into(),
                        arguments: target.arguments.clone(),
                    },
                })
            } else {
                None
            }
        }

        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let redirect_then = then_target.block.block() == Some(old_target);
            let redirect_else = else_target.block.block() == Some(old_target);

            if redirect_then || redirect_else {
                Some(mir::Terminator::Branch {
                    condition: *condition,
                    then_target: mir::BlockTarget {
                        block: if redirect_then {
                            new_target.into()
                        } else {
                            then_target.block
                        },
                        arguments: then_target.arguments.clone(),
                    },
                    else_target: mir::BlockTarget {
                        block: if redirect_else {
                            new_target.into()
                        } else {
                            else_target.block
                        },
                        arguments: else_target.arguments.clone(),
                    },
                })
            } else {
                None
            }
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let redirect_success = success.block == old_target.into();
            let redirect_failure = failure.block == old_target.into();

            if redirect_success || redirect_failure {
                Some(mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: mir::BlockTarget {
                        block: if redirect_success {
                            new_target.into()
                        } else {
                            success.block
                        },
                        arguments: success.arguments.clone(),
                    },
                    failure: mir::BlockTarget {
                        block: if redirect_failure {
                            new_target.into()
                        } else {
                            failure.block
                        },
                        arguments: failure.arguments.clone(),
                    },
                })
            } else {
                None
            }
        }

        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let redirect_default = default.block.block() == Some(old_target);
            let redirect_cases: Vec<bool> = cases
                .iter()
                .map(|case| case.target.block.block() == Some(old_target))
                .collect();
            let any_case_redirected = redirect_cases.iter().any(|&r| r);

            if redirect_default || any_case_redirected {
                let new_cases: Vec<_> = cases
                    .iter()
                    .zip(redirect_cases.iter())
                    .map(|(case, &redirect)| mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget {
                            block: if redirect {
                                new_target.into()
                            } else {
                                case.target.block
                            },
                            arguments: case.target.arguments.clone(),
                        },
                    })
                    .collect();

                Some(mir::Terminator::Switch {
                    value: *value,
                    default: mir::BlockTarget {
                        block: if redirect_default {
                            new_target.into()
                        } else {
                            default.block
                        },
                        arguments: default.arguments.clone(),
                    },
                    cases: new_cases,
                })
            } else {
                None
            }
        }

        mir::Terminator::Yield { value, resume } => {
            if resume.block.block() == Some(old_target) {
                Some(mir::Terminator::Yield {
                    value: *value,
                    resume: mir::BlockTarget {
                        block: new_target.into(),
                        arguments: resume.arguments.clone(),
                    },
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
    tree: &mut mir::Tree,
) -> bool {
    if latches.len() <= 1 {
        return false;
    }

    let header_block = tree.get(header);
    let header_params = &header_block.parameters;

    // create merged latch with fresh parameters matching header's types
    let latch_params = fresh_parameters_like(header_params, function);

    // merged latch jumps to header, forwarding its parameters
    let latch_args: Vec<_> = latch_params
        .iter()
        .map(|parameter| parameter.value)
        .collect();
    let latch_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: header.into(),
            arguments: latch_args,
        },
    });
    let new_latch = mir::Block {
        name: None,
        parameters: latch_params,
        instructions: vec![],
        terminator: latch_terminator,
    };
    let new_latch_id = tree.insert(new_latch);
    function.blocks.push(new_latch_id);

    // redirect all original latches to the new merged latch
    for &latch_id in latches {
        let latch_block = tree.get(latch_id);
        let latch_terminator = tree.get(latch_block.terminator);
        if let Some(new_terminator) = redirect_terminator(latch_terminator, header, new_latch_id) {
            tree.replace(latch_block.terminator, new_terminator);
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
    tree: &mut mir::Tree,
) -> bool {
    let exit_block_data = tree.get(exit_block);
    let exit_params = &exit_block_data.parameters;

    // create dedicated exit with fresh parameters matching exit block's types
    let dedicated_params = fresh_parameters_like(exit_params, function);

    // dedicated exit jumps to original exit, forwarding its parameters
    let dedicated_args: Vec<_> = dedicated_params
        .iter()
        .map(|parameter| parameter.value)
        .collect();
    let dedicated_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: exit_block.into(),
            arguments: dedicated_args,
        },
    });
    let dedicated_exit = mir::Block {
        name: None,
        parameters: dedicated_params,
        instructions: vec![],
        terminator: dedicated_terminator,
    };
    let dedicated_id = tree.insert(dedicated_exit);
    function.blocks.push(dedicated_id);

    // redirect exiting blocks to the dedicated exit
    let mut redirected = false;
    for &exiting_id in exiting_blocks {
        let exiting_block = tree.get(exiting_id);
        let exiting_terminator = tree.get(exiting_block.terminator);
        if let Some(new_terminator) =
            redirect_terminator(exiting_terminator, exit_block, dedicated_id)
        {
            tree.replace(exiting_block.terminator, new_terminator);
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
        // b0 and b1 both enter b2, so preheader needed
        let input = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b1, b2
b1:
    jump b2
b2:
    branch v1, b2, b3
b3:
    return
}"#;
        let expected = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b1, b4
b1:
    jump b4
b2:
    branch v1, b2, b3
b3:
    return
b4:
    jump b2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Loop already in simplified form is unchanged.
    #[test]
    fn test_preserve_already_canonical() {
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    jump b1
b1:
    branch v0, b2, b3
b2:
    jump b1
b3:
    return
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_unchanged(input);
    }

    /// Self-loop at entry block gets a preheader.
    #[test]
    fn test_insert_preheader_for_entry_loop() {
        // b0 is entry and has self-loop, needs preheader
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    branch v0, b0(v0), b1
b1:
    return
}"#;
        // preheader (block2) becomes new entry with fresh param v1
        let expected = r#"
function test(v0: boolean): void {
b0(v1: boolean):
    branch v1, b0(v1), b1
b1:
    return
b2(v0: boolean):
    jump b0(v0)
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);

        // verify entry changed to preheader
        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }

    /// Single predecessor with branch still gets preheader.
    #[test]
    fn test_insert_preheader_for_branch_predecessor() {
        // b0 branches to b1, not a dedicated entry
        let input = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b1, b2
b1:
    branch v1, b1, b2
b2:
    return
}"#;
        // b3 = preheader for b1's loop
        // b4 = dedicated exit (b2 has outside predecessor b0)
        let expected = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b3, b2
b1:
    branch v1, b1, b4
b2:
    return
b3:
    jump b1
b4:
    jump b2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// While-style loop with dedicated entry is unchanged.
    #[test]
    fn test_preserve_while_loop_with_preheader() {
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    jump b1
b1:
    branch v0, b2, b3
b2:
    jump b1
b3:
    return
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_unchanged(input);
    }

    /// Header parameters are preserved through preheader.
    #[test]
    fn test_preserve_header_parameters() {
        // block2 has parameter v3, preheader must forward it
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1(v1), b2(v1)
b1(v2: int32):
    jump b2(v2)
b2(v3: int32):
    branch v0, b2(v3), b3
b3:
    return v3
}"#;
        // preheader (block4) has parameter v4 and forwards to block2
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1(v1), b4(v1)
b1(v2: int32):
    jump b4(v2)
b2(v3: int32):
    branch v0, b2(v3), b3
b3:
    return v3
b4(v4: int32):
    jump b2(v4)
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Switch terminator entering loop gets redirected to preheader.
    #[test]
    fn test_switch_entry_to_loop() {
        // switch default and case 0 both go to block1
        let input = r#"
function test(v0: int32, v1: boolean): void {
b0(v0: int32, v1: boolean):
    switch v0, b1, 0 => b1, 1 => b2
b1:
    branch v1, b1, b2
b2:
    return
}"#;
        // block3 = preheader, block4 = dedicated exit
        let expected = r#"
function test(v0: int32, v1: boolean): void {
b0(v0: int32, v1: boolean):
    switch v0, b3, 0 => b3, 1 => b2
b1:
    branch v1, b1, b4
b2:
    return
b3:
    jump b1
b4:
    jump b2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Loop with no outside predecessors (infinite loop from entry) gets preheader.
    #[test]
    fn test_infinite_loop_from_entry() {
        // b0 is entry and only jumps to itself
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    jump b0(v0)
}"#;
        // preheader (block1) becomes entry with fresh param v1
        let expected = r#"
function test(v0: boolean): void {
b0(v1: boolean):
    jump b0(v1)
b1(v0: boolean):
    jump b0(v0)
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);

        // verify entry changed to preheader
        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        assert_ne!(function.entry.unwrap(), function.blocks[0]);
    }

    /// Multiple latches are merged into single latch.
    #[test]
    fn test_merge_multiple_latches() {
        // block2 and block3 both jump to block1 (two latches)
        let input = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1
b1:
    branch v0, b2, b3
b2:
    jump b1
b3:
    branch v1, b1, b4
b4:
    return
}"#;
        // merged latch (block5) inserted, block2 and block3 redirect to it
        let expected = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1
b1:
    branch v0, b2, b3
b2:
    jump b5
b3:
    branch v1, b5, b4
b4:
    return
b5:
    jump b1
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Multiple latches with different arguments to header are merged correctly.
    #[test]
    fn test_merge_latches_with_different_arguments() {
        // block2 jumps to block1(v5), block4 jumps to block1(v7)
        let input = r#"
function test(v0: boolean, v1: boolean): int32 {
b0(v0: boolean, v1: boolean):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    branch v0, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    jump b1(v5)
b3:
    branch v1, b4, b5
b4:
    v6: int32 = 10int32
    v7: int32 = int.add v3, v6
    jump b1(v7)
b5:
    return v3
}"#;
        // merged latch (block6) receives parameter v8 and forwards to header
        let expected = r#"
function test(v0: boolean, v1: boolean): int32 {
b0(v0: boolean, v1: boolean):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    branch v0, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    jump b6(v5)
b3:
    branch v1, b4, b5
b4:
    v6: int32 = 10int32
    v7: int32 = int.add v3, v6
    jump b6(v7)
b5:
    return v3
b6(v8: int32):
    jump b1(v8)
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Exit block with outside predecessor gets dedicated exit inserted.
    #[test]
    fn test_insert_dedicated_exit() {
        // b2 is exit block, reachable from both b1 (in loop) and b0 (outside)
        let input = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b1, b2
b1:
    branch v1, b1, b2
b2:
    return
}"#;
        // block3 = preheader for loop
        // block4 = dedicated exit for block2
        let expected = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b3, b2
b1:
    branch v1, b1, b4
b2:
    return
b3:
    jump b1
b4:
    jump b2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Exit block with parameters gets dedicated exit with forwarded params.
    #[test]
    fn test_dedicated_exit_with_parameters() {
        let input = r#"
function test(v0: boolean, v1: boolean): int32 {
b0(v0: boolean, v1: boolean):
    v2: int32 = 0int32
    branch v0, b1(v2), b2(v2)
b1(v3: int32):
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    branch v1, b1(v5), b2(v5)
b2(v6: int32):
    return v6
}"#;
        // block3 = preheader, block4 = dedicated exit
        let expected = r#"
function test(v0: boolean, v1: boolean): int32 {
b0(v0: boolean, v1: boolean):
    v2: int32 = 0int32
    branch v0, b3(v2), b2(v2)
b1(v3: int32):
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    branch v1, b1(v5), b4(v5)
b2(v6: int32):
    return v6
b3(v7: int32):
    jump b1(v7)
b4(v8: int32):
    jump b2(v8)
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Loop with dedicated exit already is unchanged.
    #[test]
    fn test_preserve_dedicated_exit() {
        // block3 is only reachable from block2 (in the loop)
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    jump b1
b1:
    branch v0, b2, b3
b2:
    jump b1
b3:
    return
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_unchanged(input);
    }

    /// Nested loops both get simplified.
    #[test]
    fn test_simplify_nested_loops() {
        // outer loop: header=block1, latches={block2, block3}
        // inner loop: header=block2, latch={block2} (self-loop)
        let input = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b1, b4
b1:
    branch v1, b2, b3
b2:
    branch v2, b2, b1
b3:
    jump b1
b4:
    return
}"#;
        // block5 = preheader for outer loop (block1, header ID comes first)
        // block6 = preheader for inner loop (block2)
        // block7 = merged latch for outer loop
        // Note: no dedicated exit needed (block4 has no preds from inside loop,
        // and block1 is a loop header so gets preheader instead)
        let expected = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b5, b4
b1:
    branch v1, b6, b3
b2:
    branch v2, b2, b7
b3:
    jump b7
b4:
    return
b5:
    jump b1
b6:
    jump b2
b7:
    jump b1
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Triple-nested loops all get preheaders.
    #[test]
    fn test_simplify_triple_nested_loops() {
        // outer: header=block1, inner: header=block2, innermost: header=block3
        let input = r#"
function test(v0: boolean, v1: boolean, v2: boolean, v3: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean, v3: boolean):
    branch v0, b1, b4
b1:
    branch v1, b2, b4
b2:
    branch v2, b3, b1
b3:
    branch v3, b3, b2
b4:
    return
}"#;
        // b5 = outer preheader
        // b6 = inner preheader
        // b7 = innermost preheader
        // b8 = dedicated exit for outer loop (b4 has outside pred b0)
        let expected = r#"
function test(v0: boolean, v1: boolean, v2: boolean, v3: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean, v3: boolean):
    branch v0, b5, b4
b1:
    branch v1, b6, b8
b2:
    branch v2, b7, b1
b3:
    branch v3, b3, b2
b4:
    return
b5:
    jump b1
b6:
    jump b2
b7:
    jump b3
b8:
    jump b4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Loop needing both preheader and latch merge.
    #[test]
    fn test_preheader_and_latch_merge_combined() {
        // b0 branches to loop (needs preheader)
        // b2 and b3 both back to header (needs latch merge)
        let input = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b1, b4
b1:
    branch v1, b2, b3
b2:
    jump b1
b3:
    branch v2, b1, b4
b4:
    return
}"#;
        // block5 = preheader, block6 = merged latch, block7 = dedicated exit
        let expected = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b5, b4
b1:
    branch v1, b2, b3
b2:
    jump b6
b3:
    branch v2, b6, b7
b4:
    return
b5:
    jump b1
b6:
    jump b1
b7:
    jump b4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Loop needing preheader, latch merge, and dedicated exit.
    #[test]
    fn test_all_canonicalizations_combined() {
        // needs preheader (b0 branches), latch merge (b2, b3),
        // and dedicated exit (b4 reachable from outside too)
        let input = r#"
function test(v0: boolean, v1: boolean, v2: boolean, v3: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean, v3: boolean):
    branch v0, b1, b4
b1:
    branch v1, b2, b3
b2:
    branch v2, b1, b4
b3:
    branch v3, b1, b4
b4:
    return
}"#;
        // block5 = preheader, block6 = merged latch, block7 = dedicated exit
        let expected = r#"
function test(v0: boolean, v1: boolean, v2: boolean, v3: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean, v3: boolean):
    branch v0, b5, b4
b1:
    branch v1, b2, b3
b2:
    branch v2, b6, b7
b3:
    branch v3, b6, b7
b4:
    return
b5:
    jump b1
b6:
    jump b1
b7:
    jump b4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_preserve_no_loops() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    return v1
b2:
    return v2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_unchanged(input);
    }

    /// Irreducible control flow is unchanged (not a natural loop).
    #[test]
    fn test_preserve_irreducible_cfg() {
        // two entry points to the "loop" region - not a natural loop
        // block1 and block2 can both be entered from outside and from each other
        let input = r#"
function test(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    branch v0, b1, b2
b1:
    branch v1, b2, b3
b2:
    branch v1, b1, b3
b3:
    return
}"#;
        // no natural loops detected, so unchanged
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_unchanged(input);
    }

    /// Multiple independent loops are all simplified.
    #[test]
    fn test_simplify_multiple_independent_loops() {
        // two separate loops with no nesting
        let input = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b1, b3
b1:
    branch v1, b1, b2
b2:
    branch v2, b3, b4
b3:
    branch v1, b3, b4
b4:
    return
}"#;
        // b5 = preheader for b1 (b0 branches)
        // b6 = preheader for b3 (b0 and b2 enter it)
        // b7 = dedicated exit for b3 (b4 has outside pred b2)
        // Note: b2 doesn't need dedicated exit (only pred is b1 which is in loop1)
        let expected = r#"
function test(v0: boolean, v1: boolean, v2: boolean): void {
b0(v0: boolean, v1: boolean, v2: boolean):
    branch v0, b5, b6
b1:
    branch v1, b1, b2
b2:
    branch v2, b6, b4
b3:
    branch v1, b3, b7
b4:
    return
b5:
    jump b1
b6:
    jump b3
b7:
    jump b4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.assert_output(expected);
    }
}
