use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, LoopAnalysis, OwnershipAnalysis, RangeAnalysis, ScalarEvolution, Scev,
    ValueRange,
};
use crate::optimize::common::{
    BlockParamForwarding, build_use_def_maps, clone_loop_blocks, terminator_remap,
    unsigned_int_width_for_value,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Version loops to specialize bounds checks with a preheader guard.
    ///
    /// When the loop guard bounds the iteration count, this pass emits a preheader comparison.
    /// This ensures the iteration count fits within the checked length.
    /// The fast version removes bounds checks inside the loop.
    ///
    /// ```mir
    /// function @before(v0: [u8; 8], v1: u32, v2: u32) -> void {
    /// block0(v0: [u8; 8], v1: u32, v2: u32):
    ///     v3 = iconst 0u32
    ///     v4 = iconst 1u32
    ///     jump block1(v3)
    /// block1(v5: u32):
    ///     v6 = icmp_ult v5, v2
    ///     branch v6, block2, block3
    /// block2:
    ///     v7 = icmp_ult v5, v1
    ///     check v7, bounds.unsigned v5, v1, v0, block4, block5
    /// block4:
    ///     v8 = element.addr v0, v5
    ///     v9 = iconst 1u8
    ///     store v8, v9
    ///     v10 = iadd v5, v4
    ///     jump block1(v10)
    /// block5:
    ///     unreachable
    /// block3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: [u8; 8], v1: u32, v2: u32) -> void {
    /// block0(v0: [u8; 8], v1: u32, v2: u32):
    ///     v3 = iconst 0u32
    ///     v4 = iconst 1u32
    ///     v11 = icmp_ule v2, v1
    ///     branch v11, block6(v3), block1(v3)
    /// block1(v5: u32):
    ///     v6 = icmp_ult v5, v2
    ///     branch v6, block2, block3
    /// block2:
    ///     v7 = icmp_ult v5, v1
    ///     check v7, bounds.unsigned v5, v1, v0, block4, block5
    /// block4:
    ///     v8 = element.addr v0, v5
    ///     v9 = iconst 1u8
    ///     store v8, v9
    ///     v10 = iadd v5, v4
    ///     jump block1(v10)
    /// block5:
    ///     unreachable
    /// block3:
    ///     return
    /// block6(v12: u32):
    ///     v13 = icmp_ult v12, v2
    ///     branch v13, block7, block3
    /// block7:
    ///     v14 = element.addr v0, v12
    ///     v15 = iconst 1u8
    ///     store v14, v15
    ///     v16 = iadd v12, v4
    ///     jump block6(v16)
    /// }
    /// ```
    #[pass(id = "loop-versioning")]
    pub LoopVersioning,
    "Version loops to specialize bounds checks"
}

impl FunctionPass for LoopVersioning {
    /// Run loop versioning on a function.
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

        let changed = run_loop_versioning(function, tree, ctx);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "LoopVersioning"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "loop-versioning"
    }
}

/// Captures the induction guard pattern for a loop.
#[derive(Debug, Clone)]
struct GuardInfo {
    /// The induction value used by the guard.
    induction: mir::Value,
    /// The bound value used by the guard.
    bound: mir::Value,
    /// Whether the guard is strict.
    is_strict: bool,
}

/// Run loop versioning on a single function and report whether it changed.
fn run_loop_versioning(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // gather analyses
    let analyses = ctx.function_analyses(function, tree);
    let loops = analyses.get::<LoopAnalysis>().clone();
    let cfg = analyses.get::<ControlFlowGraph>().clone();
    let scev = analyses.get::<ScalarEvolution>().clone();
    let ownership = analyses.get::<OwnershipAnalysis>().clone();
    let ranges = analyses.get::<RangeAnalysis>().clone();
    let forwarding = BlockParamForwarding::build(function, tree, &cfg);

    // bail out when no loops are present
    if loops.num_loops() == 0 {
        return false;
    }

    // track whether we rewrote any loops
    let use_def = build_use_def_maps(function, tree);
    let mut changed = false;
    function.recompute_next_value_id(tree);

    // version each eligible loop
    for (loop_index, lp) in loops.loops().iter().enumerate() {
        // require a single latch
        if !lp.has_single_latch() {
            continue;
        }

        // require the header to be an exiting block
        let header = lp.header;
        if !lp.exiting_blocks.contains(&header) {
            continue;
        }

        // locate the loop preheader
        let Some((preheader, preheader_args)) = find_preheader(header, &lp.blocks, &cfg, tree)
        else {
            continue;
        };

        // extract the loop guard from the header
        let Some(guard) = guard_from_header(header, &lp.blocks, tree, &use_def) else {
            continue;
        };

        // require a simple induction pattern
        if !guard_is_simple(&guard, loop_index, &scev) {
            continue;
        }

        // find a matching bounds check inside the loop
        let Some((length, collection)) = bounds_check_in_loop(lp, guard.induction, tree) else {
            continue;
        };

        // require loop invariant bounds
        let resolved_length = forwarding.resolve(length);
        let resolved_bound = forwarding.resolve(guard.bound);

        if !value_is_loop_invariant(resolved_length, lp, &use_def, &forwarding)
            || !value_is_loop_invariant(resolved_bound, lp, &use_def, &forwarding)
        {
            continue;
        }

        // require consistent unsigned integer types
        let Some(bound_width) = unsigned_int_width_for_value(
            resolved_bound,
            &ownership,
            ctx.type_context().pointer_width_bits,
            tree,
        ) else {
            continue;
        };
        let Some(length_width) = unsigned_int_width_for_value(
            resolved_length,
            &ownership,
            ctx.type_context().pointer_width_bits,
            tree,
        ) else {
            continue;
        };
        let Some(induction_width) = unsigned_int_width_for_value(
            guard.induction,
            &ownership,
            ctx.type_context().pointer_width_bits,
            tree,
        ) else {
            continue;
        };
        if bound_width != length_width || bound_width != induction_width {
            continue;
        }

        // compute the bound used by the preheader guard
        let Some((guard_instructions, guard_bound)) = preheader_guard_bound(
            preheader,
            &guard,
            resolved_bound,
            bound_width,
            function,
            tree,
            &ranges,
        ) else {
            continue;
        };

        // insert the preheader guard for the fast path
        let Some(fast_guard) = insert_preheader_guard(
            preheader,
            guard_bound,
            resolved_length,
            function,
            tree,
            &preheader_args,
            header,
        ) else {
            continue;
        };

        // clone the loop body for the fast path
        let (block_map, value_map) = clone_loop_blocks(&lp.blocks, function, tree);

        // remember the cloned header for the fast path
        let fast_header = block_map[&header];

        // remap cloned terminators to cloned blocks
        for &cloned_id in block_map.values() {
            let mut block = tree.get(cloned_id).clone();
            terminator_remap(&mut block.terminator, &block_map, &value_map);
            tree.replace(cloned_id, block);
        }

        // remove bounds checks in the cloned loop
        let cloned_induction = *value_map.get(&guard.induction).unwrap_or(&guard.induction);
        let cloned_length = *value_map.get(&length).unwrap_or(&length);
        let cloned_collection = *value_map.get(&collection).unwrap_or(&collection);
        strip_bounds_checks(
            &block_map,
            cloned_induction,
            cloned_length,
            cloned_collection,
            tree,
        );

        // update the preheader to emit guard instructions and branch between fast and slow loops
        let mut preheader_block = tree.get(preheader).clone();
        preheader_block.instructions.extend(guard_instructions);
        preheader_block.instructions.push(fast_guard);
        preheader_block.terminator = mir::Terminator::Branch {
            condition: tree.get(fast_guard).destination().unwrap(),
            then_target: fast_header,
            then_arguments: preheader_args.clone(),
            else_target: header,
            else_arguments: preheader_args.clone(),
        };
        tree.replace(preheader, preheader_block);

        // append cloned blocks
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

    // confirm the predecessor jumps directly to the header
    let preheader = outside_preds.pop()?;
    let preheader_block = tree.get(preheader);
    let arguments = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Extract a loop guard from the header terminator.
fn guard_from_header(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::NodeTree,
    use_def: &crate::optimize::common::UseDefMaps,
) -> Option<GuardInfo> {
    // read the header terminator
    let header_block = tree.get(header);
    let mir::Terminator::Branch {
        condition,
        then_target,
        else_target: _,
        ..
    } = &header_block.terminator
    else {
        return None;
    };

    // require the true edge to stay inside the loop
    if !loop_blocks.contains(then_target) {
        return None;
    }

    // locate the guard instruction that produces the condition
    let definition = use_def.def_block.get(condition)?;
    let block = tree.get(*definition);
    let inst_id = block
        .instructions
        .iter()
        .find(|&&inst_id| tree.get(inst_id).destination() == Some(*condition))?;
    let inst = tree.get(*inst_id);

    let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = inst
    else {
        return None;
    };

    // accept unsigned comparisons that can be normalized to induction < bound
    let (induction, bound, is_strict) = match operator {
        mir::BinaryOperator::UnsignedLessThan => (*left, *right, true),
        mir::BinaryOperator::UnsignedLessEqual => (*left, *right, false),
        mir::BinaryOperator::UnsignedGreaterThan => (*right, *left, true),
        mir::BinaryOperator::UnsignedGreaterEqual => (*right, *left, false),
        _ => return None,
    };

    // require the induction variable to be a header parameter
    let header_params: Vec<_> = header_block.parameters.iter().map(|p| p.value).collect();
    if !header_params.contains(&induction) {
        return None;
    }

    Some(GuardInfo {
        induction,
        bound,
        is_strict,
    })
}

/// Check whether the guard describes a simple induction pattern.
fn guard_is_simple(guard: &GuardInfo, loop_index: usize, scev: &ScalarEvolution) -> bool {
    // require a simple add recurrence for the induction variable
    let Some(Scev::AddRec { start, step, .. }) =
        scev.scev_for_value_in_loop(loop_index, guard.induction)
    else {
        return false;
    };

    // require a constant non zero stride
    let Scev::Constant(mir::Constant::UInt { value: _start, .. }) = &**start else {
        return false;
    };
    let Scev::Constant(mir::Constant::UInt { value: step, .. }) = &**step else {
        return false;
    };

    *step != 0
}

/// Find a matching bounds check for the induction variable.
fn bounds_check_in_loop(
    lp: &crate::optimize::analyses::Loop,
    induction: mir::Value,
    tree: &mir::NodeTree,
) -> Option<(mir::Value, mir::Value)> {
    // scan loop blocks for a matching bounds check
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        let mir::Terminator::Check { constraint, .. } = &block.terminator else {
            continue;
        };

        let mir::CheckConstraint::Bounds {
            index,
            length,
            collection,
            is_signed,
        } = constraint
        else {
            continue;
        };

        // ignore signed checks
        if *is_signed {
            continue;
        }

        // ignore mismatched indices
        if *index != induction {
            continue;
        }

        return Some((*length, *collection));
    }

    None
}

/// Check whether a value is loop invariant.
fn value_is_loop_invariant(
    value: mir::Value,
    lp: &crate::optimize::analyses::Loop,
    use_def: &crate::optimize::common::UseDefMaps,
    forwarding: &BlockParamForwarding,
) -> bool {
    // resolve forwarded block parameters
    let value = forwarding.resolve(value);

    // values without a definition block are treated as invariant
    let Some(def_block) = use_def.def_block.get(&value).copied() else {
        return true;
    };

    // definitions outside the loop are invariant
    !lp.blocks.contains(&def_block)
}

/// Insert a preheader guard for the fast path.
fn insert_preheader_guard(
    preheader: mir::LocalNodeId<mir::Block>,
    bound: mir::Value,
    length: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    preheader_args: &[mir::Value],
    header: mir::LocalNodeId<mir::Block>,
) -> Option<mir::LocalNodeId<mir::Instruction>> {
    // verify the preheader still jumps to the header
    let preheader_block = tree.get(preheader);
    let target_args = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == header => arguments.clone(),
        _ => return None,
    };

    // reject mismatched argument lists
    if target_args != preheader_args {
        return None;
    }

    // build the guard instruction
    let destination = function.next_value();
    let guard = mir::Instruction::Binary {
        destination,
        operator: mir::BinaryOperator::UnsignedLessEqual,
        left: bound,
        right: length,
    };
    let guard_id = tree.insert(guard);

    Some(guard_id)
}

/// Compute the bound value to use for the preheader guard.
fn preheader_guard_bound(
    preheader: mir::LocalNodeId<mir::Block>,
    guard: &GuardInfo,
    bound: mir::Value,
    width: u16,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ranges: &RangeAnalysis,
) -> Option<(Vec<mir::LocalNodeId<mir::Instruction>>, mir::Value)> {
    // use the existing bound for strict guards
    if guard.is_strict {
        return Some((Vec::new(), bound));
    }

    // look up the range for the guard bound
    let range = ranges.exit(preheader).get(bound)?;
    let ValueRange::Integer {
        max,
        width: range_width,
        is_signed,
        ..
    } = range
    else {
        return None;
    };

    // require an unsigned range with a matching width
    if *is_signed || *range_width as u16 != width {
        return None;
    }

    // reject invalid widths
    if width == 0 || width > 127 {
        return None;
    }

    // require space for a non overflowing plus one
    let max_value = (1i128 << u32::from(width)) - 1;
    if *max >= max_value {
        return None;
    }

    // build a constant one value for the add
    let one_value = function.next_value();
    let one_inst = tree.insert(mir::Instruction::Const {
        destination: one_value,
        value: mir::Constant::UInt {
            value: 1,
            width: width as u8,
        },
    });

    // build the incremented bound
    let add_value = function.next_value();
    let add_inst = tree.insert(mir::Instruction::Binary {
        destination: add_value,
        operator: mir::BinaryOperator::Add,
        left: bound,
        right: one_value,
    });

    Some((vec![one_inst, add_inst], add_value))
}

/// Remove bounds checks from cloned loop blocks.
fn strip_bounds_checks(
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    induction: mir::Value,
    length: mir::Value,
    collection: mir::Value,
    tree: &mut mir::NodeTree,
) {
    // strip matching bounds checks in cloned blocks
    for &cloned_id in block_map.values() {
        let mut block = tree.get(cloned_id).clone();
        let mir::Terminator::Check {
            constraint,
            success,
            failure: _,
            ..
        } = &block.terminator
        else {
            continue;
        };

        // require a matching bounds constraint
        let mir::CheckConstraint::Bounds {
            index,
            length: bound,
            collection: col,
            is_signed,
        } = constraint
        else {
            continue;
        };

        // ignore non matching checks
        if *is_signed || *index != induction || *bound != length || *col != collection {
            continue;
        }

        // replace the check with the success edge
        block.terminator = mir::Terminator::Jump {
            target: success.target,
            arguments: success.arguments.clone(),
        };
        tree.replace(cloned_id, block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop versioning inserts a fast path for bounds checks.
    #[test]
    fn test_loop_versioning_bounds_guard() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block4, block5
block4:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block5:
    unreachable
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    v11 = icmp_ule v2, v1
    branch v11, block6(v3), block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block5
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block3, block4
block3:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block4:
    unreachable
block5:
    return
block6(v12: u32):
    v13 = icmp_ult v12, v2
    branch v13, block7, block5
block7:
    v14 = icmp_ult v12, v1
    jump block8
block8:
    v15 = element.addr v0, v12 -> ref<borrowed u8>
    v16 = iconst 1u8
    store v15, v16
    v17 = iadd v12, v4
    jump block6(v17)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(expected);
    }

    /// Loop versioning handles non zero induction starts.
    #[test]
    fn test_loop_versioning_non_zero_start() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 2u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block4, block5
block4:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block5:
    unreachable
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 2u32
    v4 = iconst 1u32
    v11 = icmp_ule v2, v1
    branch v11, block6(v3), block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block5
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block3, block4
block3:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block4:
    unreachable
block5:
    return
block6(v12: u32):
    v13 = icmp_ult v12, v2
    branch v13, block7, block5
block7:
    v14 = icmp_ult v12, v1
    jump block8
block8:
    v15 = element.addr v0, v12 -> ref<borrowed u8>
    v16 = iconst 1u8
    store v15, v16
    v17 = iadd v12, v4
    jump block6(v17)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(expected);
    }

    /// Signed bounds checks are not versioned.
    #[test]
    fn test_loop_versioning_skips_signed_bounds() {
        let input = r#"function @test(v0: [i32; 8], v1: i32, v2: i32) -> void {
block0(v0: [i32; 8], v1: i32, v2: i32):
    v3 = iconst 0i32
    v4 = iconst 1i32
    jump block1(v3)
block1(v5: i32):
    v6 = icmp_slt v5, v2
    branch v6, block2, block3
block2:
    v7 = icmp_slt v5, v1
    check v7, bounds.signed v5, v1, v0, block4, block5
block3:
    return
block4:
    v8 = element.addr v0, v5 -> ref<borrowed i32>
    v9 = iconst 1i32
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block5:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(input);
    }

    /// Mismatched integer types prevent versioning.
    #[test]
    fn test_loop_versioning_skips_type_mismatch() {
        let input = r#"function @test(v0: [u8; 8], v1: i32, v2: u32) -> void {
block0(v0: [u8; 8], v1: i32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = icmp_ult v5, v2
    check v7, bounds.unsigned v5, v1, v0, block4, block5
block3:
    return
block4:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block5:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(input);
    }

    /// Non unit strides are still versioned.
    #[test]
    fn test_loop_versioning_handles_non_unit_stride() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 2u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block4, block5
block4:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block5:
    unreachable
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 2u32
    v11 = icmp_ule v2, v1
    branch v11, block6(v3), block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block5
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block3, block4
block3:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block4:
    unreachable
block5:
    return
block6(v12: u32):
    v13 = icmp_ult v12, v2
    branch v13, block7, block5
block7:
    v14 = icmp_ult v12, v1
    jump block8
block8:
    v15 = element.addr v0, v12 -> ref<borrowed u8>
    v16 = iconst 1u8
    store v15, v16
    v17 = iadd v12, v4
    jump block6(v17)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(expected);
    }

    /// Non strict loop guards are not versioned.
    #[test]
    fn test_loop_versioning_skips_non_strict_guard() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ule v5, v2
    branch v6, block2, block5
block2:
    v7 = icmp_ult v5, v1
    check v7, bounds.unsigned v5, v1, v0, block3, block4
block3:
    v8 = element.addr v0, v5 -> ref<borrowed u8>
    v9 = iconst 1u8
    store v8, v9
    v10 = iadd v5, v4
    jump block1(v10)
block4:
    unreachable
block5:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopVersioning);
        program.assert_output(input);
    }
}
