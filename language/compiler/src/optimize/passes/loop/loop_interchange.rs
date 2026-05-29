use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, DominatorTree, LoopAnalysis, MemorySSA};
use crate::common::mir::{
    LoopEffectPolicy, build_value_definition_blocks, collect_loop_effects, loop_guard_branch,
    loop_preheader, value_available_in_block,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Interchange perfectly nested read-only loops.
    ///
    /// This pass swaps the order of two perfectly nested loops when the body
    /// contains only read effects and the induction starts are loop invariant.
    ///
    /// ```mir
    /// function before(v0: uint32): int32 {
    /// b0(v0: uint32):
    ///     v1 = 0uint32
    ///     v2 = 4uint32
    ///     v3 = 1uint32
    ///     v4 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     jump b1(v1)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v2
    ///     branch v6, b2(v1), b6
    /// b2(v7: uint32):
    ///     v8 = int.lt.u v7, v2
    ///     branch v8, b3(v7), b4
    /// b3(v9: uint32):
    ///     v10 = load v4 -> int32
    ///     v11 = int.add v9, v3
    ///     jump b2(v11)
    /// b4:
    ///     v12 = int.add v5, v3
    ///     jump b1(v12)
    /// b6:
    ///     return v10
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32): int32 {
    /// b0(v0: uint32):
    ///     v1 = 0uint32
    ///     v2 = 4uint32
    ///     v3 = 1uint32
    ///     v4 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     jump b2(v1)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v2
    ///     branch v6, b4, b3
    /// b2(v7: uint32):
    ///     v8 = int.lt.u v7, v2
    ///     branch v8, b1(v1), b6
    /// b3(v9: uint32):
    ///     v10 = load v4 -> int32
    ///     v11 = int.add v9, v3
    ///     jump b1(v11)
    /// b4:
    ///     v12 = int.add v5, v3
    ///     jump b2(v12)
    /// b6:
    ///     return v10
    /// }
    /// ```
    #[pass(id = "loop-interchange")]
    pub LoopInterchange,
    "Interchange perfectly nested read only loops"
}

impl FunctionPass for LoopInterchange {
    /// Run loop interchange on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let memory_ssa = analyses.get::<MemorySSA>();
        // run loop interchange
        let changed =
            run_loop_interchange(function, tree, &loops, &cfg, &domtree, memory_ssa.as_ref());

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopInterchange"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "loop-interchange"
    }
}

/// Interchange candidate data.
struct InterchangeCandidate {
    /// Outer loop header.
    outer_header: mir::LocalNodeId<mir::Block>,
    /// Outer loop latch.
    outer_latch: mir::LocalNodeId<mir::Block>,
    /// Inner loop header.
    inner_header: mir::LocalNodeId<mir::Block>,
    /// Inner loop latch.
    inner_latch: mir::LocalNodeId<mir::Block>,
    /// Outer loop exit block.
    outer_exit: mir::LocalNodeId<mir::Block>,
    /// Outer preheader block.
    outer_preheader: mir::LocalNodeId<mir::Block>,
    /// Outer header arguments from preheader.
    outer_preheader_args: Vec<mir::Value>,
    /// Inner header arguments from outer header.
    inner_header_args: Vec<mir::Value>,
}

/// Run loop interchange and return true when changes are made.
fn run_loop_interchange(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    loops: &LoopAnalysis,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    memory_ssa: &MemorySSA,
) -> bool {
    // build definition info
    let def_blocks = build_value_definition_blocks(function, tree);
    let function_params: HashSet<_> = function
        .parameters
        .iter()
        .filter_map(|param| param.value.value())
        .collect();

    // scan nested loops
    let candidate = loops.loops().iter().find_map(|inner| {
        let parent_index = inner.parent?;
        let outer = &loops.loops()[parent_index];

        build_interchange_candidate(
            outer,
            inner,
            cfg,
            domtree,
            tree,
            memory_ssa,
            &def_blocks,
            &function_params,
        )
    });
    let Some(candidate) = candidate else {
        return false;
    };

    // apply interchange
    apply_interchange(tree, &candidate)
}

/// Build a loop interchange candidate.
#[allow(clippy::too_many_arguments)]
fn build_interchange_candidate(
    outer: &crate::common::mir::analysis::Loop,
    inner: &crate::common::mir::analysis::Loop,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    function_params: &HashSet<mir::Value>,
) -> Option<InterchangeCandidate> {
    // require canonical loops
    if !outer.has_single_latch() || !inner.has_single_latch() {
        return None;
    }

    // require single exit loops
    if outer.exit_blocks.len() != 1 || inner.exit_blocks.len() != 1 {
        return None;
    }

    // resolve latch blocks
    let outer_latch = *outer.latches.first()?;
    let inner_latch = *inner.latches.first()?;

    // require perfect nesting
    if !is_perfectly_nested(outer, inner, cfg) {
        return None;
    }

    // require outer preheader
    let (outer_preheader, outer_preheader_args) =
        loop_preheader(outer.header, &outer.blocks, cfg, domtree, tree)?;
    if outer_preheader_args.len() != tree.get(outer.header).parameters.len() {
        return None;
    }

    // resolve header branches
    let outer_guard = loop_guard_branch(outer.header, inner.header, &outer.blocks, tree)?;
    let inner_guard = loop_guard_branch(inner.header, inner_latch, &inner.blocks, tree)?;

    // require empty exit arguments and params
    if !outer_guard.exit_arguments.is_empty() || !inner_guard.exit_arguments.is_empty() {
        return None;
    }

    // require empty in loop arguments
    if !inner_guard.in_loop_arguments.is_empty() {
        return None;
    }

    // require empty exit parameters
    if !tree.get(outer_guard.exit_block).parameters.is_empty() {
        return None;
    }
    if !tree.get(inner_guard.exit_block).parameters.is_empty() {
        return None;
    }

    // require the inner exit to target the outer latch
    if inner_guard.exit_block != outer_latch {
        return None;
    }

    // require empty latch parameters
    if !tree.get(inner_latch).parameters.is_empty() || !tree.get(outer_latch).parameters.is_empty()
    {
        return None;
    }

    // require latch jump terminators
    let outer_latch_terminator = tree.get(tree.get(outer_latch).terminator);
    if !matches!(
        outer_latch_terminator,
        mir::Terminator::Jump { target } if target.block.block() == Some(outer.header)
    ) {
        return None;
    }

    let inner_latch_terminator = tree.get(tree.get(inner_latch).terminator);
    if !matches!(
        inner_latch_terminator,
        mir::Terminator::Jump { target } if target.block.block() == Some(inner.header)
    ) {
        return None;
    }

    // resolve arguments passed from outer header to inner header
    let inner_header_args = outer_guard.in_loop_arguments.clone();
    if inner_header_args.len() != tree.get(inner.header).parameters.len() {
        return None;
    }

    // ensure inner header args are available at the outer preheader
    for value in &inner_header_args {
        if !value_available_in_block(
            *value,
            outer_preheader,
            def_blocks,
            function_params,
            domtree,
        ) {
            return None;
        }
    }

    // ensure outer preheader args are available at the inner header
    for value in &outer_preheader_args {
        if !value_available_in_block(*value, inner.header, def_blocks, function_params, domtree) {
            return None;
        }
    }

    // reject non speculatable instructions
    if !loops_are_read_only(outer, inner, tree, memory_ssa) {
        return None;
    }

    Some(InterchangeCandidate {
        outer_header: outer.header,
        outer_latch,
        inner_header: inner.header,
        inner_latch,
        outer_exit: outer_guard.exit_block,
        outer_preheader,
        outer_preheader_args,
        inner_header_args,
    })
}

/// Check whether the inner loop is perfectly nested.
fn is_perfectly_nested(
    outer: &crate::common::mir::analysis::Loop,
    inner: &crate::common::mir::analysis::Loop,
    cfg: &ControlFlowGraph,
) -> bool {
    // locate the inner preheader
    let mut inner_preheader = None;
    for &pred in cfg.predecessors(inner.header) {
        if outer.blocks.contains(&pred) && !inner.blocks.contains(&pred) {
            inner_preheader = Some(pred);
        }
    }

    // collect blocks that are outside the inner loop
    let mut extras: HashSet<_> = outer
        .blocks
        .iter()
        .copied()
        .filter(|block| !inner.blocks.contains(block))
        .collect();

    // drop the inner preheader and outer control blocks
    if let Some(preheader) = inner_preheader {
        extras.remove(&preheader);
    }

    extras.remove(&outer.header);
    if let Some(latch) = outer.latches.first() {
        extras.remove(latch);
    }

    // require no extra blocks
    extras.is_empty()
}

/// Check whether loops are read only.
fn loops_are_read_only(
    outer: &crate::common::mir::analysis::Loop,
    inner: &crate::common::mir::analysis::Loop,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> bool {
    // require read only effects for each loop
    collect_loop_effects(&outer.blocks, tree, memory_ssa, LoopEffectPolicy::ReadOnly).is_some()
        && collect_loop_effects(&inner.blocks, tree, memory_ssa, LoopEffectPolicy::ReadOnly)
            .is_some()
}

/// Apply loop interchange to a candidate.
fn apply_interchange(tree: &mut mir::Tree, candidate: &InterchangeCandidate) -> bool {
    // update the outer preheader to jump to the inner header
    let preheader_block = tree.get(candidate.outer_preheader).clone();
    let preheader_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: candidate.inner_header.into(),
            arguments: candidate
                .inner_header_args
                .iter()
                .copied()
                .map(Into::into)
                .collect(),
        },
    };
    tree.replace(candidate.outer_preheader, preheader_block);
    tree.replace(
        tree.get(candidate.outer_preheader).terminator,
        preheader_terminator,
    );

    // update the inner header to branch to the outer header
    let inner_header_block = tree.get(candidate.inner_header).clone();
    let inner_header_terminator = tree.get(inner_header_block.terminator).clone();
    let mir::Terminator::Branch {
        condition,
        then_target,
        ..
    } = &inner_header_terminator
    else {
        return false;
    };

    // rewrite the inner header terminator
    let in_loop_is_then = then_target.block.block() == Some(candidate.inner_latch);
    let new_inner_terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: mir::BlockTarget {
                block: candidate.outer_header.into(),
                arguments: candidate
                    .outer_preheader_args
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
            },
            else_target: mir::BlockTarget {
                block: candidate.outer_exit.into(),
                arguments: Vec::new(),
            },
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: mir::BlockTarget {
                block: candidate.outer_exit.into(),
                arguments: Vec::new(),
            },
            else_target: mir::BlockTarget {
                block: candidate.outer_header.into(),
                arguments: candidate
                    .outer_preheader_args
                    .iter()
                    .copied()
                    .map(Into::into)
                    .collect(),
            },
        }
    };
    tree.replace(candidate.inner_header, inner_header_block);
    tree.replace(
        tree.get(candidate.inner_header).terminator,
        new_inner_terminator,
    );

    // update the outer header to exit to the inner latch
    let outer_header_block = tree.get(candidate.outer_header).clone();
    let outer_header_terminator = tree.get(outer_header_block.terminator).clone();
    let mir::Terminator::Branch {
        condition,
        then_target,
        ..
    } = &outer_header_terminator
    else {
        return false;
    };

    // rewrite the outer header terminator
    let in_loop_is_then = then_target.block.block() == Some(candidate.inner_header);
    let new_outer_terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: mir::BlockTarget {
                block: candidate.outer_latch.into(),
                arguments: Vec::new(),
            },
            else_target: mir::BlockTarget {
                block: candidate.inner_latch.into(),
                arguments: Vec::new(),
            },
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: mir::BlockTarget {
                block: candidate.inner_latch.into(),
                arguments: Vec::new(),
            },
            else_target: mir::BlockTarget {
                block: candidate.outer_latch.into(),
                arguments: Vec::new(),
            },
        }
    };
    tree.replace(candidate.outer_header, outer_header_block);
    tree.replace(
        tree.get(candidate.outer_header).terminator,
        new_outer_terminator,
    );

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Perfectly nested read only loops are interchanged.
    #[test]
    fn test_loop_interchange_swaps_nested_loop() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4:
    v12: uint32 = int.add v6, v3
    jump b1(v12)
b5:
    return v5
}"#;

        let expected = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b2(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b4, b3
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b1(v1), b5
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4:
    v12: uint32 = int.add v6, v3
    jump b1(v12)
b5:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(expected);
    }

    /// Loops with stores are not interchanged.
    #[test]
    fn test_loop_interchange_skips_writes() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2(v1), b5
b2(v7: uint32):
    v8: boolean = int.lt.u v7, v2
    branch v8, b3(v7), b4
b3(v9: uint32):
    store v4, v9
    v10: uint32 = int.add v9, v3
    jump b2(v10)
b4:
    v11: uint32 = int.add v5, v3
    jump b1(v11)
b5:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Inner exits that do not target the outer latch prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_exit_mismatch() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2(v1), b6
b2(v7: uint32):
    v8: boolean = int.lt.u v7, v2
    branch v8, b3, b4
b3:
    v9: int32 = load v4
    v10: uint32 = int.add v7, v3
    jump b2(v10)
b4:
    jump b5
b5:
    v11: uint32 = int.add v5, v3
    jump b1(v11)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Inner header values unavailable at the outer preheader prevent interchange.
    #[test]
    fn test_loop_interchange_skips_unavailable_inner_args() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    v8: uint32 = int.add v6, v3
    branch v7, b2(v1, v8), b5
b2(v9: uint32, v10: uint32):
    v11: boolean = int.lt.u v9, v2
    branch v11, b3, b4
b3:
    v12: int32 = load v4
    v13: uint32 = int.add v9, v3
    jump b2(v13, v10)
b4:
    v14: uint32 = int.add v6, v3
    jump b1(v14)
b5:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Non jump inner latches prevent interchange.
    #[test]
    fn test_loop_interchange_skips_non_jump_inner_latch() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    v12: boolean = int.lt.u v8, v2
    branch v12, b2(v11), b4
b4:
    v13: uint32 = int.add v6, v3
    jump b1(v13)
b5:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Inner latch parameters prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_latch_parameters() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4(v8)
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4(v12: uint32):
    v13: uint32 = int.add v6, v3
    jump b1(v13)
b5:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Missing outer preheaders prevent interchange.
    #[test]
    fn test_loop_interchange_skips_missing_preheader() {
        let input = r#"
function test(v0: boolean, v1: uint32): int32 {
b0(v0: boolean, v1: uint32):
    v2: uint32 = 0uint32
    v3: uint32 = 4uint32
    v4: uint32 = 1uint32
    v5: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v6: int32 = 0int32
    branch v0, b2(v2), b1(v2)
b1(v7: uint32):
    jump b2(v7)
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v3
    branch v9, b3(v2), b6
b3(v10: uint32):
    v11: boolean = int.lt.u v10, v3
    branch v11, b4, b5
b4:
    v12: int32 = load v5
    v13: uint32 = int.add v10, v4
    jump b3(v13)
b5:
    v14: uint32 = int.add v8, v4
    jump b2(v14)
b6:
    return v6
}"#;

        let expected = r#"
function test(v0: boolean, v1: uint32): int32 {
b0(v0: boolean, v1: uint32):
    v2: uint32 = 0uint32
    v3: uint32 = 4uint32
    v4: uint32 = 1uint32
    v5: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v6: int32 = 0int32
    branch v0, b2(v2), b1(v2)
b1(v7: uint32):
    jump b2(v7)
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v3
    branch v9, b3(v2), b6
b3(v10: uint32):
    v11: boolean = int.lt.u v10, v3
    branch v11, b4, b5
b4:
    v12: int32 = load v5
    v13: uint32 = int.add v10, v4
    jump b3(v13)
b5:
    v14: uint32 = int.add v8, v4
    jump b2(v14)
b6:
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(expected);
    }

    /// Non perfect nesting prevents interchange.
    #[test]
    fn test_loop_interchange_skips_non_perfect_nesting() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b6
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4:
    v12: uint32 = int.add v6, v3
    jump b5(v12)
b5(v13: uint32):
    jump b1(v13)
b6:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(input);
    }

    /// Inner exit arguments prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_exit_arguments() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4(v8)
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4(v12: uint32):
    v13: uint32 = int.add v6, v3
    jump b1(v13)
b5:
    return v5
}"#;

        let expected = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4(v8)
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4(v12: uint32):
    v13: uint32 = int.add v6, v3
    jump b1(v13)
b5:
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(expected);
    }

    /// Outer exit arguments prevent interchange.
    #[test]
    fn test_loop_interchange_skips_outer_exit_arguments() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5(v6)
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4:
    v12: uint32 = int.add v6, v3
    jump b1(v12)
b5(v13: uint32):
    return v5
}"#;

        let expected = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 1uint32
    v4: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v5: int32 = 0int32
    jump b1(v1)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v2
    branch v7, b2(v1), b5(v6)
b2(v8: uint32):
    v9: boolean = int.lt.u v8, v2
    branch v9, b3, b4
b3:
    v10: int32 = load v4
    v11: uint32 = int.add v8, v3
    jump b2(v11)
b4:
    v12: uint32 = int.add v6, v3
    jump b1(v12)
b5(v13: uint32):
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopInterchange);
        test.assert_output(expected);
    }
}
