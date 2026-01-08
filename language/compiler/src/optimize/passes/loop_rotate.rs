use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree, Loop, LoopAnalysis};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

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
    /// - Canonical loop form (preheader, single latch) from LoopSimplify
    /// - Header must be a simple conditional (no instructions, just branch)
    #[pass(id = "loop-rotate")]
    pub LoopRotate,
    "Loop rotation"
}

impl Pass for LoopRotate {
    fn metadata(&self) -> &'static PassMetadata {
        LoopRotate::metadata()
    }
}

impl FunctionPass for LoopRotate {
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

        let loops = context.analyses.get::<LoopAnalysis>(function, tree);
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }
        let cfg = context.analyses.get::<ControlFlowGraph>(function, tree);
        let domtree = context.analyses.get::<DominatorTree>(function, tree);

        // collect rotation candidates, innermost first
        let mut candidates: Vec<RotationCandidate> = Vec::new();

        for lp in loops.loops().iter().rev() {
            if let Some(candidate) = find_rotation_candidate(lp, &cfg, &domtree, tree, entry) {
                candidates.push(candidate);
            }
        }

        drop(domtree);
        drop(cfg);
        drop(loops);

        if candidates.is_empty() {
            return AnalysisPreservation::all();
        }

        let mut changed = false;
        for candidate in candidates {
            if rotate_loop(function, tree, &candidate) {
                changed = true;
            }
        }

        if changed {
            function.recompute_next_value_id(tree);
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }
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
    condition: mir::Value,
    /// True if then-branch goes to body (false means then-branch goes to exit).
    then_to_body: bool,
}

/// Check if a loop can be rotated and gather the necessary information.
fn find_rotation_candidate(
    lp: &Loop,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::NodeTree,
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
    let (condition, then_target, then_args, else_target, else_args) = match &header_block.terminator
    {
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => (
            *condition,
            *then_target,
            then_arguments.clone(),
            *else_target,
            else_arguments.clone(),
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
    let header_params: std::collections::HashSet<_> =
        header_block.parameters.iter().map(|p| p.value).collect();
    let passed_to_body: std::collections::HashSet<_> = body_arguments.iter().copied().collect();
    if !header_params.is_subset(&passed_to_body) {
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
        condition,
        then_to_body,
    })
}

/// Perform loop rotation.
///
/// Transform:
/// ```text
/// preheader: jump header(args)
/// header(params): branch cond, body(args), exit(args)
/// body: ... -> latch
/// latch: ... jump header(args)
/// ```
///
/// Into:
/// ```text
/// preheader: branch cond, body(args), exit(args)  [guard]
/// body: ... -> latch
/// latch: branch cond', body(args), exit(args)     [rotated condition]
/// (header is now dead and will be removed by DCE)
/// ```
fn rotate_loop(
    _function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &RotationCandidate,
) -> bool {
    let header_block = tree.get(candidate.header).clone();
    let header_params = header_block.parameters.clone();

    // get arguments passed to header from preheader and latch
    let preheader_block = tree.get(candidate.preheader);
    let preheader_args = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == candidate.header => {
            arguments.clone()
        }
        _ => return false,
    };

    let latch_block = tree.get(candidate.latch);
    let latch_args = match &latch_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == candidate.header => {
            arguments.clone()
        }
        _ => return false,
    };

    // build value mappings: header params -> actual args
    let preheader_value_map: HashMap<mir::Value, mir::Value> = header_params
        .iter()
        .zip(preheader_args.iter())
        .map(|(p, a)| (p.value, *a))
        .collect();

    let latch_value_map: HashMap<mir::Value, mir::Value> = header_params
        .iter()
        .zip(latch_args.iter())
        .map(|(p, a)| (p.value, *a))
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
    let preheader_condition = remap(candidate.condition, &preheader_value_map);

    let preheader_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: preheader_condition,
            then_target: candidate.body_block,
            then_arguments: remapped_body_args,
            else_target: candidate.exit_block,
            else_arguments: remapped_exit_args,
        }
    } else {
        mir::Terminator::Branch {
            condition: preheader_condition,
            then_target: candidate.exit_block,
            then_arguments: remapped_exit_args,
            else_target: candidate.body_block,
            else_arguments: remapped_body_args,
        }
    };

    let mut preheader = tree.get(candidate.preheader).clone();
    preheader.terminator = preheader_terminator;
    tree.replace(candidate.preheader, preheader);

    // update latch: jump -> rotated branch
    let latch_body_args = remap_args(&candidate.body_arguments, &latch_value_map);
    let latch_exit_args = remap_args(&candidate.exit_arguments, &latch_value_map);
    let latch_condition = remap(candidate.condition, &latch_value_map);

    let latch_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: latch_condition,
            then_target: candidate.body_block,
            then_arguments: latch_body_args,
            else_target: candidate.exit_block,
            else_arguments: latch_exit_args,
        }
    } else {
        mir::Terminator::Branch {
            condition: latch_condition,
            then_target: candidate.exit_block,
            then_arguments: latch_exit_args,
            else_target: candidate.body_block,
            else_arguments: latch_body_args,
        }
    };

    let mut latch = tree.get(candidate.latch).clone();
    latch.terminator = latch_terminator;
    tree.replace(candidate.latch, latch);

    // header is now unreachable, SimplifyCfg will remove it

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::{LoopSimplify, SimplifyCfg};

    /// Simple while loop is rotated to do-while with guard.
    #[test]
    fn test_basic_rotation() {
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
        // after rotation:
        // - preheader (block0) gets the guard branch
        // - latch (block2) gets the rotated branch
        // - header (block1) becomes dead and is removed by SimplifyCfg
        // block numbers get renumbered: block2->block1, block3->block2
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    branch v0, block1, block2
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopRotate);
        program.run_pass(&SimplifyCfg); // clean up dead header
        program.assert_output(expected);
    }

    /// Loop with phi values and simple header (no instructions) is rotated.
    #[test]
    fn test_rotation_with_phi() {
        // header passes all its params to body, so rotation is valid
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_slt v2, v1
    jump block1(v2, v3)
block1(v4: i32, v5: bool):
    branch v5, block2(v4, v5), block3(v4)
block2(v6: i32, v7: bool):
    v8 = iconst 1i32
    v9 = iadd v6, v8
    v10 = icmp_slt v9, v1
    jump block1(v9, v10)
block3(v11: i32):
    return v11
}"#;
        // after rotation:
        // - block0: guard branch using initial condition v3
        // - block2: latch branch using computed condition v10
        // - block1 becomes dead and is removed
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_slt v2, v1
    branch v3, block1(v2, v3), block2(v2)
block1(v6: i32, v7: bool):
    v8 = iconst 1i32
    v9 = iadd v6, v8
    v10 = icmp_slt v9, v1
    branch v10, block1(v9, v10), block2(v9)
block2(v11: i32):
    return v11
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopRotate);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Loop with false-to-body (inverted condition) is rotated correctly.
    #[test]
    fn test_rotation_inverted_condition() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block3, block2
block2:
    jump block1
block3:
    return
}"#;
        // condition false -> body, condition true -> exit
        // block numbers get renumbered after SimplifyCfg: block2->block1, block3->block2
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block2, block1
block1:
    branch v0, block2, block1
block2:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopRotate);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Loop with instructions in header is not rotated.
    #[test]
    fn test_no_rotate_header_with_instructions() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1
block1:
    v2 = iconst 1i32
    v3 = iadd v1, v2
    branch v0, block2, block3
block2:
    jump block1
block3:
    return v3
}"#;
        // header has instructions, don't rotate
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopRotate);
        program.assert_output(&before);
    }

    /// Single-block loop is not rotated.
    #[test]
    fn test_no_rotate_single_block() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    branch v0, block1, block2
block2:
    return
}"#;
        // latch == header, don't rotate
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopRotate);
        program.assert_output(&before);
    }

    /// Loop with unconditional header terminator is not rotated.
    #[test]
    fn test_no_rotate_unconditional_header() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    jump block2
block2:
    branch v0, block1, block3
block3:
    return
}"#;
        // header ends with jump, not branch - can't rotate
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopRotate);
        program.assert_output(&before);
    }

    /// Loop where header params aren't all passed to body is not rotated.
    #[test]
    fn test_no_rotate_missing_param_passthrough() {
        // header has param v2, but body doesn't receive it (body uses v2 directly)
        // rotation would make v2 undefined in body, so we skip
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1(v1)
block1(v2: i32):
    branch v0, block2, block3
block2:
    v3 = iconst 1i32
    v4 = iadd v2, v3
    jump block1(v4)
block3:
    return v2
}"#;
        // header has param v2, body doesn't receive v2 as argument, skip rotation
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&LoopRotate);
        program.assert_output(&before);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_no_loops() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iadd v0, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopRotate);
        program.assert_unchanged(input);
    }

    /// Loop with multiple phi values and simple header is rotated.
    /// Body block must have parameters matching what the header would pass.
    #[test]
    fn test_rotation_multiple_phis() {
        // condition is passed as a block parameter to enable rotation.
        // body block (block2) has same parameters as header (block1).
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = iconst 1i32
    v4 = icmp_slt v2, v0
    jump block1(v2, v3, v4)
block1(v5: i32, v6: i32, v7: bool):
    branch v7, block2(v5, v6, v7), block3(v5, v6)
block2(v8: i32, v9: i32, v10: bool):
    v11 = iadd v8, v9
    v12 = iadd v9, v3
    v13 = icmp_slt v11, v0
    jump block1(v11, v12, v13)
block3(v14: i32, v15: i32):
    v16 = iadd v14, v15
    return v16
}"#;
        // after rotation:
        // - block0: guard branch using initial condition v4
        // - block2: latch branch using computed condition v13
        // - block1 becomes dead and is removed
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = iconst 1i32
    v4 = icmp_slt v2, v0
    branch v4, block1(v2, v3, v4), block2(v2, v3)
block1(v8: i32, v9: i32, v10: bool):
    v11 = iadd v8, v9
    v12 = iadd v9, v3
    v13 = icmp_slt v11, v0
    branch v13, block1(v11, v12, v13), block2(v11, v12)
block2(v14: i32, v15: i32):
    v16 = iadd v14, v15
    return v16
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopRotate);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }
}
