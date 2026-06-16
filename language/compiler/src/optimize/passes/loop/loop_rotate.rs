use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
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
    /// - Canonical loop form (preheader, single latch) from LoopSimplify
    /// - Header must be a simple conditional (no instructions, just branch)
    #[pass(id = "loop-rotate")]
    pub LoopRotate,
    "Loop rotation"
}

impl FunctionPass for LoopRotate {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get analyses
        let (loops, cfg, domtree) = {
            (
                analyses.get::<LoopAnalysis>(function, tree).clone(),
                analyses.get::<ControlFlowGraph>(function, tree).clone(),
                analyses.get::<DominatorTree>(function, tree).clone(),
            )
        };
        if loops.num_loops() == 0 {
            return Mutation::NONE;
        }

        let changed = run_loop_rotate(entry, function, tree, &loops, &cfg, &domtree);
        if changed {
            Mutation::CONTROL_FLOW | Mutation::VALUES
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "LoopRotate"
    }

    fn id(&self) -> &'static str {
        "loop-rotate"
    }
}

/// Core loop rotation logic. Returns true if changes were made.
fn run_loop_rotate(
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
            condition.value()?,
            then_target.block.block()?,
            then_target
                .arguments
                .iter()
                .copied()
                .map(|argument| argument.value())
                .collect::<Option<Vec<_>>>()?,
            else_target.block.block()?,
            else_target
                .arguments
                .iter()
                .copied()
                .map(|argument| argument.value())
                .collect::<Option<Vec<_>>>()?,
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
        .map(|param| param.value.value())
        .collect::<Option<_>>()?;
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
    tree: &mut mir::Tree,
    candidate: &RotationCandidate,
) -> bool {
    let header_block = tree.get(candidate.header).clone();
    let Some(header_params): Option<Vec<_>> = header_block
        .parameters
        .iter()
        .map(|param| param.value.value())
        .collect()
    else {
        return false;
    };

    // get arguments passed to header from preheader and latch
    let preheader_block = tree.get(candidate.preheader);
    let preheader_current_terminator = tree.get(preheader_block.terminator);
    let preheader_args = match preheader_current_terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(candidate.header) => {
            let Some(arguments) = target
                .arguments
                .iter()
                .copied()
                .map(|argument| argument.value())
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };

            arguments
        }
        _ => return false,
    };

    let latch_block = tree.get(candidate.latch);
    let latch_current_terminator = tree.get(latch_block.terminator);
    let latch_args = match latch_current_terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(candidate.header) => {
            let Some(arguments) = target
                .arguments
                .iter()
                .copied()
                .map(|argument| argument.value())
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };

            arguments
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

    let preheader_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: preheader_condition.into(),
            then_target: mir::BlockTarget::new(
                candidate.body_block.into(),
                remapped_body_args.into_iter().map(Into::into).collect(),
            ),
            else_target: mir::BlockTarget::new(
                candidate.exit_block.into(),
                remapped_exit_args.into_iter().map(Into::into).collect(),
            ),
        }
    } else {
        mir::Terminator::Branch {
            condition: preheader_condition.into(),
            then_target: mir::BlockTarget::new(
                candidate.exit_block.into(),
                remapped_exit_args.into_iter().map(Into::into).collect(),
            ),
            else_target: mir::BlockTarget::new(
                candidate.body_block.into(),
                remapped_body_args.into_iter().map(Into::into).collect(),
            ),
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

    let latch_terminator = if candidate.then_to_body {
        mir::Terminator::Branch {
            condition: latch_condition.into(),
            then_target: mir::BlockTarget::new(
                candidate.body_block.into(),
                latch_body_args.into_iter().map(Into::into).collect(),
            ),
            else_target: mir::BlockTarget::new(
                candidate.exit_block.into(),
                latch_exit_args.into_iter().map(Into::into).collect(),
            ),
        }
    } else {
        mir::Terminator::Branch {
            condition: latch_condition.into(),
            then_target: mir::BlockTarget::new(
                candidate.exit_block.into(),
                latch_exit_args.into_iter().map(Into::into).collect(),
            ),
            else_target: mir::BlockTarget::new(
                candidate.body_block.into(),
                latch_body_args.into_iter().map(Into::into).collect(),
            ),
        }
    };

    let latch = tree.get(candidate.latch).clone();
    tree.set(candidate.latch, latch);
    tree.set(tree.get(candidate.latch).terminator, latch_terminator);

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
        let input = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    jump block1()

block1:
    branch value0, block2(), block3()

block2:
    jump block1()

block3:
    return
}
"#;
        // after rotation:
        // - preheader (b0) gets the guard branch
        // - latch (b2) gets the rotated branch
        // - header (b1) becomes dead and is removed by SimplifyCfg
        // - critical edges are split into jump blocks
        let expected = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    branch value0, block2(), block1()

block1:
    jump block3()

block2:
    jump block2_1()

block2_1:
    branch value0, block5(), block4()

block4:
    jump block3()

block5:
    jump block2_1()

block3:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopRotate);
        test.run_pass(&SimplifyCfg); // clean up dead header
        test.assert_output(expected);
    }

    /// Loop with phi values and simple header (no instructions) is rotated.
    #[test]
    fn test_rotation_with_phi() {
        // header passes all its params to body, so rotation is valid
        let input = r#"
function test(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = 0int32
    value3: boolean = int.lt.s value2, value1
    jump block1(value2, value3)

block1(value4: int32, value5: boolean):
    branch value5, block2(value4, value5), block3(value4)

block2(value6: int32, value7: boolean):
    value8: int32 = 1int32
    value9: int32 = int.add value6, value8
    value10: boolean = int.lt.s value9, value1
    jump block1(value9, value10)

block3(value11: int32):
    return value11
}
"#;
        // after rotation:
        // - b0: guard branch using initial condition v3
        // - b2: latch branch using computed condition v10
        // - b1 becomes dead and is removed
        // - critical edges are split into jump blocks
        let expected = r#"
function test(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = 0int32
    value3: boolean = int.lt.s value2, value1
    branch value3, block2(value2, value3), block1(value2)

block1(value14: int32):
    jump block3(value14)

block2(value12: int32, value13: boolean):
    jump block2_1(value12, value13)

block2_1(value6: int32, value7: boolean):
    value8: int32 = 1int32
    value9: int32 = int.add value6, value8
    value10: boolean = int.lt.s value9, value1
    branch value10, block5(value9, value10), block4(value9)

block4(value17: int32):
    jump block3(value17)

block5(value15: int32, value16: boolean):
    jump block2_1(value15, value16)

block3(value11: int32):
    return value11
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopRotate);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Loop with false-to-body (inverted condition) is rotated correctly.
    #[test]
    fn test_rotation_inverted_condition() {
        let input = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    jump block1()

block1:
    branch value0, block3(), block2()

block2:
    jump block1()

block3:
    return
}
"#;
        // condition false -> body, condition true -> exit
        // critical edges are split after SimplifyCfg
        let expected = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    branch value0, block2(), block1()

block1:
    jump block2_1()

block2:
    jump block3()

block2_1:
    branch value0, block5(), block4()

block4:
    jump block2_1()

block5:
    jump block3()

block3:
    return
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopRotate);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Loop with instructions in header is not rotated.
    #[test]
    fn test_no_rotate_header_with_instructions() {
        let input = r#"
function test(value0: boolean, value1: int32): int32 {
entry0(value0: boolean, value1: int32):
    jump block1()

block1:
    value2: int32 = 1int32
    value3: int32 = int.add value1, value2
    branch value0, block2(), block3()

block2:
    jump block1()

block3:
    return value3
}
"#;
        // header has instructions, don't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopRotate);
        test.assert_output(&before);
    }

    /// Single-block loop is not rotated.
    #[test]
    fn test_no_rotate_single_block() {
        let input = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    jump block1()

block1:
    branch value0, block1(), block2()

block2:
    return
}
"#;
        // latch == header, don't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopRotate);
        test.assert_output(&before);
    }

    /// Loop with unconditional header terminator is not rotated.
    #[test]
    fn test_no_rotate_unconditional_header() {
        let input = r#"
function test(value0: boolean): void {
entry0(value0: boolean):
    jump block1()

block1:
    jump block2()

block2:
    branch value0, block1(), block3()

block3:
    return
}
"#;
        // header ends with jump, not branch - can't rotate
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopRotate);
        test.assert_output(&before);
    }

    /// Loop where header params aren't all passed to body is not rotated.
    #[test]
    fn test_no_rotate_missing_param_passthrough() {
        // header has param v2, but body doesn't receive it (body uses v2 directly)
        // rotation would make v2 undefined in body, so we skip
        let input = r#"
function test(value0: boolean, value1: int32): int32 {
entry0(value0: boolean, value1: int32):
    jump block1(value1)

block1(value2: int32):
    branch value0, block2(), block3()

block2:
    value3: int32 = 1int32
    value4: int32 = int.add value2, value3
    jump block1(value4)

block3:
    return value2
}
"#;
        // header has param v2, body doesn't receive v2 as argument, skip rotation
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&LoopRotate);
        test.assert_output(&before);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_no_loops() {
        let input = r#"
function test(value0: int32): int32 {
entry0(value0: int32):
    value1: int32 = 1int32
    value2: int32 = int.add value0, value1
    return value2
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopRotate);
        test.assert_unchanged(input);
    }

    /// Loop with multiple phi values and simple header is rotated.
    /// Body block must have parameters matching what the header would pass.
    #[test]
    fn test_rotation_multiple_phis() {
        // condition is passed as a block parameter to enable rotation.
        // body block (block2) has same parameters as header (block1).
        let input = r#"
function test(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = 0int32
    value3: int32 = 1int32
    value4: boolean = int.lt.s value2, value0
    jump block1(value2, value3, value4)

block1(value5: int32, value6: int32, value7: boolean):
    branch value7, block2(value5, value6, value7), block3(value5, value6)

block2(value8: int32, value9: int32, value10: boolean):
    value11: int32 = int.add value8, value9
    value12: int32 = int.add value9, value3
    value13: boolean = int.lt.s value11, value0
    jump block1(value11, value12, value13)

block3(value14: int32, value15: int32):
    value16: int32 = int.add value14, value15
    return value16
}
"#;
        // after rotation:
        // - b0: guard branch using initial condition v4
        // - b2: latch branch using computed condition v13
        // - b1 becomes dead and is removed
        // - critical edges are split into jump blocks
        let expected = r#"
function test(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = 0int32
    value3: int32 = 1int32
    value4: boolean = int.lt.s value2, value0
    branch value4, block2(value2, value3, value4), block1(value2, value3)

block1(value20: int32, value21: int32):
    jump block3(value20, value21)

block2(value17: int32, value18: int32, value19: boolean):
    jump block2_1(value17, value18, value19)

block2_1(value8: int32, value9: int32, value10: boolean):
    value11: int32 = int.add value8, value9
    value12: int32 = int.add value9, value3
    value13: boolean = int.lt.s value11, value0
    branch value13, block5(value11, value12, value13), block4(value11, value12)

block4(value25: int32, value26: int32):
    jump block3(value25, value26)

block5(value22: int32, value23: int32, value24: boolean):
    jump block2_1(value22, value23, value24)

block3(value14: int32, value15: int32):
    value16: int32 = int.add value14, value15
    return value16
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopRotate);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }
}
