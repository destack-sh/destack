use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree, LoopAnalysis, MemorySSA};
use crate::optimize::common::{
    LoopEffectPolicy, build_value_definition_blocks, collect_loop_effects, loop_guard_branch,
    loop_preheader, value_available_in_block,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Interchange perfectly nested read-only loops.
    ///
    /// This pass swaps the order of two perfectly nested loops when the body
    /// contains only read effects and the induction starts are loop invariant.
    ///
    /// ```mir
    /// function @before(v0: u32) -> i32 {
    /// block0(v0: u32):
    ///     v1 = iconst 0u32
    ///     v2 = iconst 4u32
    ///     v3 = iconst 1u32
    ///     v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    ///     jump block1(v1)
    /// block1(v5: u32):
    ///     v6 = icmp_ult v5, v2
    ///     branch v6, block2(v1), block6
    /// block2(v7: u32):
    ///     v8 = icmp_ult v7, v2
    ///     branch v8, block3(v7), block4
    /// block3(v9: u32):
    ///     v10 = load v4 -> i32
    ///     v11 = iadd v9, v3
    ///     jump block2(v11)
    /// block4:
    ///     v12 = iadd v5, v3
    ///     jump block1(v12)
    /// block6:
    ///     return v10
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: u32) -> i32 {
    /// block0(v0: u32):
    ///     v1 = iconst 0u32
    ///     v2 = iconst 4u32
    ///     v3 = iconst 1u32
    ///     v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    ///     jump block2(v1)
    /// block1(v5: u32):
    ///     v6 = icmp_ult v5, v2
    ///     branch v6, block4, block3
    /// block2(v7: u32):
    ///     v8 = icmp_ult v7, v2
    ///     branch v8, block1(v1), block6
    /// block3(v9: u32):
    ///     v10 = load v4 -> i32
    ///     v11 = iadd v9, v3
    ///     jump block1(v11)
    /// block4:
    ///     v12 = iadd v5, v3
    ///     jump block2(v12)
    /// block6:
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
        tree: &mut mir::NodeTree,
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
    tree: &mut mir::NodeTree,
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
        .map(|param| param.value)
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
    outer: &crate::optimize::analyses::Loop,
    inner: &crate::optimize::analyses::Loop,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::NodeTree,
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
    if !matches!(
        tree.get(outer_latch).terminator,
        mir::Terminator::Jump { target, .. } if target == outer.header
    ) {
        return None;
    }

    if !matches!(
        tree.get(inner_latch).terminator,
        mir::Terminator::Jump { target, .. } if target == inner.header
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
    outer: &crate::optimize::analyses::Loop,
    inner: &crate::optimize::analyses::Loop,
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
    outer: &crate::optimize::analyses::Loop,
    inner: &crate::optimize::analyses::Loop,
    tree: &mir::NodeTree,
    memory_ssa: &MemorySSA,
) -> bool {
    // require read only effects for each loop
    collect_loop_effects(&outer.blocks, tree, memory_ssa, LoopEffectPolicy::ReadOnly).is_some()
        && collect_loop_effects(&inner.blocks, tree, memory_ssa, LoopEffectPolicy::ReadOnly)
            .is_some()
}

/// Apply loop interchange to a candidate.
fn apply_interchange(tree: &mut mir::NodeTree, candidate: &InterchangeCandidate) -> bool {
    // update the outer preheader to jump to the inner header
    let mut preheader_block = tree.get(candidate.outer_preheader).clone();
    preheader_block.terminator = mir::Terminator::Jump {
        target: candidate.inner_header,
        arguments: candidate.inner_header_args.clone(),
    };
    tree.replace(candidate.outer_preheader, preheader_block);

    // update the inner header to branch to the outer header
    let mut inner_header_block = tree.get(candidate.inner_header).clone();
    let mir::Terminator::Branch {
        condition,
        then_target,
        then_arguments: _,
        else_target: _,
        else_arguments: _,
    } = &inner_header_block.terminator
    else {
        return false;
    };

    // rewrite the inner header terminator
    let in_loop_is_then = *then_target == candidate.inner_latch;
    inner_header_block.terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: candidate.outer_header,
            then_arguments: candidate.outer_preheader_args.clone(),
            else_target: candidate.outer_exit,
            else_arguments: Vec::new(),
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: candidate.outer_exit,
            then_arguments: Vec::new(),
            else_target: candidate.outer_header,
            else_arguments: candidate.outer_preheader_args.clone(),
        }
    };
    tree.replace(candidate.inner_header, inner_header_block);

    // update the outer header to exit to the inner latch
    let mut outer_header_block = tree.get(candidate.outer_header).clone();
    let mir::Terminator::Branch {
        condition,
        then_target,
        then_arguments: _,
        else_target: _,
        else_arguments: _,
    } = &outer_header_block.terminator
    else {
        return false;
    };

    // rewrite the outer header terminator
    let in_loop_is_then = *then_target == candidate.inner_header;
    outer_header_block.terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: candidate.outer_latch,
            then_arguments: Vec::new(),
            else_target: candidate.inner_latch,
            else_arguments: Vec::new(),
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: candidate.inner_latch,
            then_arguments: Vec::new(),
            else_target: candidate.outer_latch,
            else_arguments: Vec::new(),
        }
    };
    tree.replace(candidate.outer_header, outer_header_block);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Perfectly nested read only loops are interchanged.
    #[test]
    fn test_loop_interchange_swaps_nested_loop() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v12 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v10 = iadd v7, v3
    jump block2(v10)
block4:
    v11 = iadd v5, v3
    jump block1(v11)
block5:
    return v12
}"#;

        let expected = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v12 = iconst 0i32
    jump block2(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block4, block3
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block1(v1), block5
block3:
    v9 = load v4 -> i32
    v10 = iadd v7, v3
    jump block2(v10)
block4:
    v11 = iadd v5, v3
    jump block1(v11)
block5:
    return v12
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(expected);
    }

    /// Loops with stores are not interchanged.
    #[test]
    fn test_loop_interchange_skips_writes() {
        let input = r#"function @test(v0: u32) -> void {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3(v7), block4
block3(v9: u32):
    store v4, v9
    v10 = iadd v9, v3
    jump block2(v10)
block4:
    v11 = iadd v5, v3
    jump block1(v11)
block5:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Inner exits that do not target the outer latch prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_exit_mismatch() {
        let input = r#"function @test(v0: u32) -> void {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block6
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v10 = iadd v7, v3
    jump block2(v10)
block4:
    jump block5
block5:
    v11 = iadd v5, v3
    jump block1(v11)
block6:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Inner header values unavailable at the outer preheader prevent interchange.
    #[test]
    fn test_loop_interchange_skips_unavailable_inner_args() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v12 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    v7 = iadd v5, v3
    branch v6, block2(v1, v7), block5
block2(v8: u32, v9: u32):
    v10 = icmp_ult v8, v2
    branch v10, block3, block4
block3:
    v11 = load v4 -> i32
    v13 = iadd v8, v3
    jump block2(v13, v9)
block4:
    v14 = iadd v5, v3
    jump block1(v14)
block5:
    return v12
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Non jump inner latches prevent interchange.
    #[test]
    fn test_loop_interchange_skips_non_jump_inner_latch() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v12 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v10 = iadd v7, v3
    v11 = icmp_ult v7, v2
    branch v11, block2(v10), block4
block4:
    v13 = iadd v5, v3
    jump block1(v13)
block5:
    return v12
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Inner latch parameters prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_latch_parameters() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v12 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4(v7)
block3:
    v9 = load v4 -> i32
    v10 = iadd v7, v3
    jump block2(v10)
block4(v11: u32):
    v13 = iadd v5, v3
    jump block1(v13)
block5:
    return v12
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Missing outer preheaders prevent interchange.
    #[test]
    fn test_loop_interchange_skips_missing_preheader() {
        let input = r#"function @test(v0: bool, v1: u32) -> i32 {
block0(v0: bool, v1: u32):
    v2 = iconst 0u32
    v3 = iconst 4u32
    v4 = iconst 1u32
    v5 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v6 = iconst 0i32
    branch v0, block1(v2), block2(v2)
block2(v7: u32):
    jump block1(v7)
block1(v8: u32):
    v9 = icmp_ult v8, v3
    branch v9, block3(v2), block6
block3(v10: u32):
    v11 = icmp_ult v10, v3
    branch v11, block4, block5
block4:
    v12 = load v5 -> i32
    v13 = iadd v10, v4
    jump block3(v13)
block5:
    v14 = iadd v8, v4
    jump block1(v14)
block6:
    return v6
}"#;

        let expected = r#"function @test(v0: bool, v1: u32) -> i32 {
block0(v0: bool, v1: u32):
    v2 = iconst 0u32
    v3 = iconst 4u32
    v4 = iconst 1u32
    v5 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v6 = iconst 0i32
    branch v0, block2(v2), block1(v2)
block1(v7: u32):
    jump block2(v7)
block2(v8: u32):
    v9 = icmp_ult v8, v3
    branch v9, block3(v2), block6
block3(v10: u32):
    v11 = icmp_ult v10, v3
    branch v11, block4, block5
block4:
    v12 = load v5 -> i32
    v13 = iadd v10, v4
    jump block3(v13)
block5:
    v14 = iadd v8, v4
    jump block2(v14)
block6:
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(expected);
    }

    /// Non perfect nesting prevents interchange.
    #[test]
    fn test_loop_interchange_skips_non_perfect_nesting() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v10 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block6
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v11 = iadd v7, v3
    jump block2(v11)
block4:
    v12 = iadd v5, v3
    jump block5(v12)
block5(v13: u32):
    jump block1(v13)
block6:
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(input);
    }

    /// Inner exit arguments prevent interchange.
    #[test]
    fn test_loop_interchange_skips_inner_exit_arguments() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v10 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block6
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4(v7)
block3:
    v9 = load v4 -> i32
    v11 = iadd v7, v3
    jump block2(v11)
block4(v12: u32):
    v13 = iadd v5, v3
    jump block1(v13)
block6:
    return v10
}"#;

        let expected = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v10 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4(v7)
block3:
    v9 = load v4 -> i32
    v11 = iadd v7, v3
    jump block2(v11)
block4(v12: u32):
    v13 = iadd v5, v3
    jump block1(v13)
block5:
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(expected);
    }

    /// Outer exit arguments prevent interchange.
    #[test]
    fn test_loop_interchange_skips_outer_exit_arguments() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v10 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block6(v5)
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v11 = iadd v7, v3
    jump block2(v11)
block4:
    v12 = iadd v5, v3
    jump block1(v12)
block6(v13: u32):
    return v10
}"#;

        let expected = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 0u32
    v2 = iconst 4u32
    v3 = iconst 1u32
    v4 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v10 = iconst 0i32
    jump block1(v1)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2(v1), block5(v5)
block2(v7: u32):
    v8 = icmp_ult v7, v2
    branch v8, block3, block4
block3:
    v9 = load v4 -> i32
    v11 = iadd v7, v3
    jump block2(v11)
block4:
    v12 = iadd v5, v3
    jump block1(v12)
block5(v13: u32):
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopInterchange);
        program.assert_output(expected);
    }
}
