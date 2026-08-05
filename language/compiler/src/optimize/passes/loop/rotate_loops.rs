use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, Mutation};

declare_pass! {
    /// Rotate loops to expose optimization opportunities.
    ///
    /// Loop rotation transforms a while-loop (test-at-top) into a do-while loop
    /// (test-at-bottom) guarded by an initial condition check:
    ///
    /// ```text
    /// Before (while):              After (do-while with guard):
    /// preheader -> header          preheader: if(cond) body else exit
    /// header: if(cond) body exit   body: ... -> latch
    /// body: ... -> header          latch: if(cond) body else exit
    /// ```
    ///
    /// Benefits:
    /// - Eliminates one branch per loop entry
    /// - Exposes the loop body to more optimizations
    /// - Enables better LICM (invariants can be hoisted above guard)
    /// - Simplifies loop analysis (single back edge to body, not header)
    ///
    /// Prerequisites:
    /// - Canonical loop form (preheader, single latch) from SimplifyLoops
    /// - Header must be a simple conditional (no instructions, just branch)
    #[pass(id = "rotate-loops")]
    pub RotateLoops,
    "Loop rotation"
}

impl FunctionPass for RotateLoops {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        _ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionAnalyses,
    ) -> Mutation {
        let tree = &mut optimized.tree;

        let entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get analyses
        let (loops, cfg, domtree) = {
            (
                analyses.loops(function, tree).clone(),
                analyses.control_flow(function, tree).clone(),
                analyses.dominators(function, tree).clone(),
            )
        };
        if loops.num_loops() == 0 {
            return Mutation::NONE;
        }

        let changed = run_rotate_loops(entry, function, tree, &loops, &cfg, &domtree);
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "RotateLoops"
    }

    fn id(&self) -> &'static str {
        "rotate-loops"
    }
}

/// Core loop rotation logic. Returns true if changes were made.
fn run_rotate_loops(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    loops: &LoopAnalysis,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> bool {
    // collect rotation candidates, innermost first
    let mut candidates: Vec<RotationCandidate> = Vec::new();

    for lp in loops.loops().iter().rev() {
        if let Some(candidate) = find_rotation_candidate(lp, cfg, domtree, tree, entry) {
            candidates.push(candidate);
        }
    }

    if candidates.is_empty() {
        return false;
    }

    let mut changed = false;
    for candidate in candidates {
        if rotate_loop(function, tree, &candidate) {
            changed = true;
        }
    }
    if changed {
        function.recompute_next_value_id(tree);
    }

    changed
}

/// Information needed to rotate a loop.
struct RotationCandidate {
    /// The loop header block.
    header: mir::LocalNodeId<mir::Block>,
    /// The preheader block (immediate dominator of header outside loop).
    preheader: mir::LocalNodeId<mir::Block>,
    /// The latch block (has the back edge).
    latch: mir::LocalNodeId<mir::Block>,
    /// The exit block (where we go when loop condition is false).
    exit_block: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to exit block from header.
    exit_arguments: Vec<mir::Value>,
    /// The body block (where we go when loop condition is true).
    body_block: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to body block from header.
    body_arguments: Vec<mir::Value>,
    /// The condition value used in the header's branch.
    branch_condition: mir::Value,
    /// True if then-branch goes to body (false means then-branch goes to exit).
    then_to_body: bool,
}

/// Check if a loop can be rotated and gather the necessary information.
fn find_rotation_candidate(
    lp: &Loop,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
    _entry: mir::LocalNodeId<mir::Block>,
) -> Option<RotationCandidate> {
    // need preheader (immediate dominator outside the loop)
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    // need exactly one latch
    let preds = cfg.predecessors(lp.header);
    let latch = preds
        .iter()
        .find(|&&p| lp.blocks.contains(&p) && p != preheader)?;

    // header must be simple conditional with no instructions
    let header_block = tree.get(lp.header);
    if !header_block.instructions.is_empty() {
        return None;
    }

    // header must end with conditional branch
    let header_terminator = tree.get(header_block.terminator);
    let (condition, then_target, then_args, else_target, else_args) = match header_terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => (
            *condition,
            then_target.block,
            tree.get_values(then_target.arguments).to_vec(),
            else_target.block,
            tree.get_values(else_target.arguments).to_vec(),
        ),
        _ => return None,
    };

    // one target inside loop (body), one outside (exit)
    let then_in_loop = lp.blocks.contains(&then_target);
    let else_in_loop = lp.blocks.contains(&else_target);

    let (body_block, body_arguments, exit_block, exit_arguments, then_to_body) =
        match (then_in_loop, else_in_loop) {
            (true, false) => (then_target, then_args, else_target, else_args, true),
            (false, true) => (else_target, else_args, then_target, then_args, false),
            _ => return None,
        };

    // skip single-block loops or body == header
    if body_block == lp.header || *latch == lp.header {
        return None;
    }

    // all header params must be passed to body, otherwise uses of header params
    // in the body would become invalid after rotation (body would no longer be
    // dominated by header)
    let header_params: Vec<_> = header_block
        .parameters
        .iter()
        .map(|param| param.value)
        .collect();
    let header_param_set: std::collections::HashSet<_> = header_params.iter().copied().collect();
    let passed_to_body: std::collections::HashSet<_> = body_arguments.iter().copied().collect();
    if !header_param_set.is_subset(&passed_to_body) {
        return None;
    }

    Some(RotationCandidate {
        header: lp.header,
        preheader,
        latch: *latch,
        exit_block,
        exit_arguments,
        body_block,
        body_arguments,
        branch_condition: condition,
        then_to_body,
    })
}

/// Perform loop rotation.
///
/// Transform:
/// ```text
/// preheader: jump header(args)
/// header(params): branch cond => body(args) | exit(args)
/// body: ... -> latch
/// latch: ... jump header(args)
/// ```
///
/// Into:
/// ```text
/// preheader: branch cond => body(args) | exit(args)  [guard]
/// body: ... -> latch
/// latch: branch cond' => body(args) | exit(args)     [rotated condition]
/// (header is now dead and will be removed by DCE)
/// ```
fn rotate_loop(
    _function: &mut mir::Function,
    tree: &mut mir::Tree,
    candidate: &RotationCandidate,
) -> bool {
    let header_block = tree.get(candidate.header);
    let header_params: Vec<_> = header_block
        .parameters
        .iter()
        .map(|param| param.value)
        .collect();

    // get arguments passed to header from preheader and latch
    let preheader_block = tree.get(candidate.preheader);
    let preheader_current_terminator = tree.get(preheader_block.terminator);
    let preheader_args = match preheader_current_terminator {
        mir::Terminator::Jump { target } if target.block == candidate.header => {
            tree.get_values(target.arguments).to_vec()
        }
        _ => return false,
    };

    let latch_block = tree.get(candidate.latch);
    let latch_current_terminator = tree.get(latch_block.terminator);
    let latch_args = match latch_current_terminator {
        mir::Terminator::Jump { target } if target.block == candidate.header => {
            tree.get_values(target.arguments).to_vec()
        }
        _ => return false,
    };

    // build value mappings: header params -> actual args
    let preheader_value_map: HashMap<mir::Value, mir::Value> = header_params
        .iter()
        .zip(preheader_args.iter())
        .map(|(param, arg)| (*param, *arg))
        .collect();

    let latch_value_map: HashMap<mir::Value, mir::Value> = header_params
        .iter()
        .zip(latch_args.iter())
        .map(|(param, arg)| (*param, *arg))
        .collect();

    let remap = |v: mir::Value, map: &HashMap<mir::Value, mir::Value>| -> mir::Value {
        *map.get(&v).unwrap_or(&v)
    };

    let remap_args = |args: &[mir::Value],
                      map: &HashMap<mir::Value, mir::Value>|
     -> Vec<mir::Value> { args.iter().map(|v| remap(*v, map)).collect() };

    // update preheader: jump -> guard branch
    let remapped_body_args = remap_args(&candidate.body_arguments, &preheader_value_map);
    let remapped_exit_args = remap_args(&candidate.exit_arguments, &preheader_value_map);
    let preheader_condition = remap(candidate.branch_condition, &preheader_value_map);
    let preheader_body_args = tree.add_values(&remapped_body_args);
    let preheader_exit_args = tree.add_values(&remapped_exit_args);

    let preheader_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: preheader_condition,
            then_target: mir::BlockTarget::new(candidate.body_block, preheader_body_args),
            else_target: mir::BlockTarget::new(candidate.exit_block, preheader_exit_args),
        }
    } else {
        mir::Terminator::Branch {
            condition: preheader_condition,
            then_target: mir::BlockTarget::new(candidate.exit_block, preheader_exit_args),
            else_target: mir::BlockTarget::new(candidate.body_block, preheader_body_args),
        }
    };

    let preheader = tree.get(candidate.preheader).clone();
    tree.set(candidate.preheader, preheader);
    tree.set(
        tree.get(candidate.preheader).terminator,
        preheader_terminator,
    );

    // update latch: jump -> rotated branch
    let latch_body_args = remap_args(&candidate.body_arguments, &latch_value_map);
    let latch_exit_args = remap_args(&candidate.exit_arguments, &latch_value_map);
    let latch_condition = remap(candidate.branch_condition, &latch_value_map);
    let latch_body_args = tree.add_values(&latch_body_args);
    let latch_exit_args = tree.add_values(&latch_exit_args);

    let latch_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: latch_condition,
            then_target: mir::BlockTarget::new(candidate.body_block, latch_body_args),
            else_target: mir::BlockTarget::new(candidate.exit_block, latch_exit_args),
        }
    } else {
        mir::Terminator::Branch {
            condition: latch_condition,
            then_target: mir::BlockTarget::new(candidate.exit_block, latch_exit_args),
            else_target: mir::BlockTarget::new(candidate.body_block, latch_body_args),
        }
    };

    let latch = tree.get(candidate.latch).clone();
    tree.set(candidate.latch, latch);
    tree.set(tree.get(candidate.latch).terminator, latch_terminator);

    // header is now unreachable, SimplifyControlFlow will remove it

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::{SimplifyControlFlow, SimplifyLoops};

    /// Simple while loop is rotated to do-while with guard.
    #[test]
    fn test_basic_rotation() {
        let input = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    branch v0 => b2 | b3

b2:
    jump b1

b3:
    return
}
"#;
        // after rotation:
        // - preheader (b0) gets the guard branch
        // - latch (b2) gets the rotated branch
        // - header (b1) becomes dead and is removed by SimplifyControlFlow
        // - critical edges are split into jump blocks
        let expected = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => b2 | b1

b1:
    jump b3

b2:
    jump b2_1

b2_1:
    branch v0 => b5 | b4

b4:
    jump b3

b5:
    jump b2_1

b3:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&RotateLoops);
        test.run_pass(&SimplifyControlFlow); // clean up dead header
        test.assert_output(expected);
    }

    /// Loop with phi values and simple header (no instructions) is rotated.
    #[test]
    fn test_rotation_with_phi() {
        // header passes all its params to body, so rotation is valid
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.lt.s v2, v1
    jump b1(v2, v3)

b1(v4: int32, v5: boolean):
    branch v5 => b2(v4, v5) | b3(v4)

b2(v6: int32, v7: boolean):
    v8: int32 = 1
    v9: int32 = int.add v6, v8
    v10: boolean = int.lt.s v9, v1
    jump b1(v9, v10)

b3(v11: int32):
    return v11
}
"#;
        // after rotation:
        // - b0: guard branch using initial condition v3
        // - b2: latch branch using computed condition v10
        // - b1 becomes dead and is removed
        // - critical edges are split into jump blocks
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: boolean = int.lt.s v2, v1
    branch v3 => b2(v2, v3) | b1(v2)

b1(v14: int32):
    jump b6(v14)

b2(v12: int32, v13: boolean):
    jump b3(v12, v13)

b3(v6: int32, v7: boolean):
    v8: int32 = 1
    v9: int32 = int.add v6, v8
    v10: boolean = int.lt.s v9, v1
    branch v10 => b5(v9, v10) | b4(v9)

b4(v17: int32):
    jump b6(v17)

b5(v15: int32, v16: boolean):
    jump b3(v15, v16)

b6(v11: int32):
    return v11
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&RotateLoops);
        test.run_pass(&SimplifyControlFlow);
        test.assert_output(expected);
    }

    /// Loop with false-to-body (inverted condition) is rotated correctly.
    #[test]
    fn test_rotation_inverted_condition() {
        let input = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    branch v0 => b3 | b2

b2:
    jump b1

b3:
    return
}
"#;
        // condition false => body, condition true -> exit
        // critical edges are split after SimplifyControlFlow
        let expected = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => b2 | b1

b1:
    jump b2_1

b2:
    jump b3

b2_1:
    branch v0 => b5 | b4

b4:
    jump b2_1

b5:
    jump b3

b3:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&RotateLoops);
        test.run_pass(&SimplifyControlFlow);
        test.assert_output(expected);
    }

    /// Loop with instructions in header is not rotated.
    #[test]
    fn test_no_rotate_header_with_instructions() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    jump b1

b1:
    v2: int32 = 1
    v3: int32 = int.add v1, v2
    branch v0 => b2 | b3

b2:
    jump b1

b3:
    return v3
}
"#;
        // header has instructions, don't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&RotateLoops);
        test.assert_output(&before);
    }

    /// Single-block loop is not rotated.
    #[test]
    fn test_no_rotate_single_block() {
        let input = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    branch v0 => b1 | b2

b2:
    return
}
"#;
        // latch == header, don't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&RotateLoops);
        test.assert_output(&before);
    }

    /// Loop with unconditional header terminator is not rotated.
    #[test]
    fn test_no_rotate_unconditional_header() {
        let input = r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    jump b2

b2:
    branch v0 => b1 | b3

b3:
    return
}
"#;
        // header ends with jump, not branch - can't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&RotateLoops);
        test.assert_output(&before);
    }

    /// Loop where header params aren't all passed to body is not rotated.
    #[test]
    fn test_no_rotate_missing_param_passthrough() {
        // header has param v2, but body doesn't receive it (body uses v2 directly)
        // rotation would make v2 undefined in body, so we skip
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    jump b1(v1)

b1(v2: int32):
    branch v0 => b2 | b3

b2:
    v3: int32 = 1
    v4: int32 = int.add v2, v3
    jump b1(v4)

b3:
    return v2
}
"#;
        // header has param v2, body doesn't receive v2 as argument, skip rotation
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        let before = test.format();
        test.run_pass(&RotateLoops);
        test.assert_output(&before);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_no_loops() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = int.add v0, v1
    return v2
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&RotateLoops);
        test.assert_unchanged(input);
    }

    /// Loop with multiple phi values and simple header is rotated.
    /// Body block must have parameters matching what the header would pass.
    #[test]
    fn test_rotation_multiple_phis() {
        // condition is passed as a block parameter to enable rotation.
        // body block (block2) has same parameters as header (block1).
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: int32 = 1
    v4: boolean = int.lt.s v2, v0
    jump b1(v2, v3, v4)

b1(v5: int32, v6: int32, v7: boolean):
    branch v7 => b2(v5, v6, v7) | b3(v5, v6)

b2(v8: int32, v9: int32, v10: boolean):
    v11: int32 = int.add v8, v9
    v12: int32 = int.add v9, v3
    v13: boolean = int.lt.s v11, v0
    jump b1(v11, v12, v13)

b3(v14: int32, v15: int32):
    v16: int32 = int.add v14, v15
    return v16
}
"#;
        // after rotation:
        // - b0: guard branch using initial condition v4
        // - b2: latch branch using computed condition v13
        // - b1 becomes dead and is removed
        // - critical edges are split into jump blocks
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: int32 = 1
    v4: boolean = int.lt.s v2, v0
    branch v4 => b2(v2, v3, v4) | b1(v2, v3)

b1(v20: int32, v21: int32):
    jump b6(v20, v21)

b2(v17: int32, v18: int32, v19: boolean):
    jump b3(v17, v18, v19)

b3(v8: int32, v9: int32, v10: boolean):
    v11: int32 = int.add v8, v9
    v12: int32 = int.add v9, v3
    v13: boolean = int.lt.s v11, v0
    branch v13 => b5(v11, v12, v13) | b4(v11, v12)

b4(v25: int32, v26: int32):
    jump b6(v25, v26)

b5(v22: int32, v23: int32, v24: boolean):
    jump b3(v22, v23, v24)

b6(v14: int32, v15: int32):
    v16: int32 = int.add v14, v15
    return v16
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyLoops);
        test.run_pass(&RotateLoops);
        test.run_pass(&SimplifyControlFlow);
        test.assert_output(expected);
    }
}
