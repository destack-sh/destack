use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    BlockParamForwarding, ControlFlowGraph, LoopAnalysis, Mutation, RangeAnalysis, ScalarEvolution,
    Scev, UseDefMaps, ValueRange, ValueTypes, build_use_def_maps, clone_loop_blocks,
    terminator_remap,
};

declare_pass! {
    /// Version loops to specialize bounds checks with a preheader guard.
    ///
    /// When the loop guard bounds the iteration count, this pass emits a preheader comparison.
    /// This ensures the iteration count fits within the checked length.
    /// The fast version removes bounds checks inside the loop.
    ///
    /// ```mir
    /// function before(v0: [uint8; 8], v1: uint32, v2: uint32): void {
    /// b0(v0: [uint8; 8], v1: uint32, v2: uint32):
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v2
    ///     branch v6, b2, b3
    /// b2:
    ///     v7 = int.lt.u v5, v1
    ///     check bounds.u v5, v1, v0 => b4, b5
    /// b4:
    ///     v8 = element.address v0, v5
    ///     v9 = 1uint8
    ///     store v8, v9
    ///     v10 = int.add v5, v4
    ///     jump b1(v10)
    /// b5:
    ///     unreachable
    /// b3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: [uint8; 8], v1: uint32, v2: uint32): void {
    /// b0(v0: [uint8; 8], v1: uint32, v2: uint32):
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     v11 = int.le.u v2, v1
    ///     branch v11, b6(v3), b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v2
    ///     branch v6, b2, b3
    /// b2:
    ///     v7 = int.lt.u v5, v1
    ///     check bounds.u v5, v1, v0 => b4, b5
    /// b4:
    ///     v8 = element.address v0, v5
    ///     v9 = 1uint8
    ///     store v8, v9
    ///     v10 = int.add v5, v4
    ///     jump b1(v10)
    /// b5:
    ///     unreachable
    /// b3:
    ///     return
    /// b6(v12: uint32):
    ///     v13 = int.lt.u v12, v2
    ///     branch v13, b7, b3
    /// b7:
    ///     v14 = element.address v0, v12
    ///     v15 = 1uint8
    ///     store v14, v15
    ///     v16 = int.add v12, v4
    ///     jump b6(v16)
    /// }
    /// ```
    #[pass(id = "version-loops")]
    pub VersionLoops,
    "Version loops to specialize bounds checks"
}

impl FunctionPass for VersionLoops {
    /// Run loop versioning on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        let changed = run_version_loops(function, tree, memory, ctx, analyses);
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "VersionLoops"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "version-loops"
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
fn run_version_loops(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    ctx: &PipelineContext<'_>,
    analyses: &mir::FunctionAnalysisCache,
) -> bool {
    // gather analyses
    let loops = analyses.get::<LoopAnalysis>(function, tree).clone();
    let cfg = analyses.get::<ControlFlowGraph>(function, tree).clone();
    let scev = analyses.get::<ScalarEvolution>(function, tree).clone();
    let ranges = analyses.get::<RangeAnalysis>(function, tree).clone();
    let forwarding = BlockParamForwarding::build(function, tree, &cfg);

    // bail out when no loops are present
    if loops.num_loops() == 0 {
        return false;
    }

    // track whether we rewrote any loops
    let use_def = build_use_def_maps(function, tree);
    let value_types = analyses.get::<ValueTypes>(function, tree);
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
        let Some(bound_width) = value_types.unsigned_int_width(
            resolved_bound,
            ctx.target_layout().pointer_bits(),
            tree,
        ) else {
            continue;
        };
        let Some(length_width) = value_types.unsigned_int_width(
            resolved_length,
            ctx.target_layout().pointer_bits(),
            tree,
        ) else {
            continue;
        };
        let Some(induction_width) = value_types.unsigned_int_width(
            guard.induction,
            ctx.target_layout().pointer_bits(),
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
        let (block_map, value_map) = clone_loop_blocks(&lp.blocks, function, tree, memory);

        // remember the cloned header for the fast path
        let fast_header = block_map[&header];

        // remap cloned terminators to cloned blocks
        for &cloned_id in block_map.values() {
            let block = tree.get(cloned_id);
            let terminator_id = block.terminator;
            let mut terminator = tree.get(terminator_id).clone();
            terminator_remap(tree, &mut terminator, &block_map, &value_map);
            tree.set(terminator_id, terminator);
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
        let preheader_block = tree.get(preheader).clone();
        let mut preheader_instructions = preheader_block.instructions.clone();
        preheader_instructions.extend(guard_instructions);
        preheader_instructions.push(fast_guard);
        function.replace_block_instructions(preheader, preheader_instructions, tree);
        let Some(condition) = tree.get(fast_guard).destination() else {
            continue;
        };
        let preheader_args = tree.add_values(&preheader_args);
        let preheader_terminator = mir::Terminator::Branch {
            condition,
            then_target: mir::BlockTarget::new(fast_header, preheader_args),
            else_target: mir::BlockTarget::new(header, preheader_args),
        };
        tree.set(preheader_block.terminator, preheader_terminator);

        // append cloned blocks
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.add_block(block_id, tree);
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

    // confirm the predecessor jumps directly to the header
    let preheader = outside_preds.pop()?;
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);
    let arguments = match preheader_terminator {
        mir::Terminator::Jump { target } if target.block == header => {
            tree.get_values(target.arguments).to_vec()
        }
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Extract a loop guard from the header terminator.
fn guard_from_header(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::Tree,
    use_def: &UseDefMaps,
) -> Option<GuardInfo> {
    // read the header terminator
    let header_block = tree.get(header);
    let header_terminator = tree.get(header_block.terminator);
    let mir::Terminator::Branch {
        condition,
        then_target,
        ..
    } = header_terminator
    else {
        return None;
    };

    // require the true edge to stay inside the loop
    let then_target = then_target.block;
    if !loop_blocks.contains(&then_target) {
        return None;
    }

    // locate the guard instruction that produces the condition
    let definition = use_def.def_block.get(condition)?;
    let block = tree.get(*definition);
    let inst_id = block
        .instructions
        .iter()
        .find(|&&inst_id| tree.get(inst_id).destination() == (*condition).into())?;
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
        mir::BinaryOperator::UnsignedLessThan => (left, right, true),
        mir::BinaryOperator::UnsignedLessEqual => (left, right, false),
        mir::BinaryOperator::UnsignedGreaterThan => (right, left, true),
        mir::BinaryOperator::UnsignedGreaterEqual => (right, left, false),
        _ => return None,
    };

    // require the induction variable to be a header parameter
    let header_params: Vec<_> = header_block
        .parameters
        .iter()
        .map(|parameter| parameter.value)
        .collect();
    if !header_params.contains(induction) {
        return None;
    }

    Some(GuardInfo {
        induction: *induction,
        bound: *bound,
        is_strict,
    })
}

/// Check whether the guard describes a simple induction pattern.
fn guard_is_simple(guard: &GuardInfo, loop_index: usize, scev: &ScalarEvolution) -> bool {
    // require a simple add recurrence for the induction variable
    let Some(Scev::AddRec { start, step, .. }) = scev.value_scev(loop_index, guard.induction)
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
    lp: &mir::Loop,
    induction: mir::Value,
    tree: &mir::Tree,
) -> Option<(mir::Value, mir::Value)> {
    // scan loop blocks for a matching bounds check
    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let mir::Terminator::Check { constraint, .. } = terminator else {
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
    lp: &mir::Loop,
    use_def: &UseDefMaps,
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
    tree: &mut mir::Tree,
    preheader_args: &[mir::Value],
    header: mir::LocalNodeId<mir::Block>,
) -> Option<mir::LocalNodeId<mir::Instruction>> {
    // verify the preheader still jumps to the header
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);
    let target_args = match preheader_terminator {
        mir::Terminator::Jump { target } if target.block == header => {
            tree.get_values(target.arguments)
        }
        _ => return None,
    };

    // reject mismatched argument lists
    if target_args != preheader_args {
        return None;
    }

    // build the guard instruction
    let bool_type = tree.boolean_type();
    let destination = function.next_typed_value(bool_type);
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
    tree: &mut mir::Tree,
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
    if *is_signed || *range_width != width {
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
    let one_value = function.next_typed_value_like(bound);
    let one_inst = tree.insert(mir::Instruction::Const {
        destination: one_value,
        value: mir::Constant::UInt { value: 1, width },
    });

    // build the incremented bound
    let add_value = function.next_typed_value_like(bound);
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
    tree: &mut mir::Tree,
) {
    // strip matching bounds checks in cloned blocks
    for &cloned_id in block_map.values() {
        let block = tree.get(cloned_id).clone();
        let terminator = tree.get(block.terminator).clone();
        let mir::Terminator::Check {
            constraint,
            success,
            ..
        } = &terminator
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
        let new_terminator = mir::Terminator::Jump {
            target: success.clone(),
        };
        tree.set(block.terminator, new_terminator);
        tree.set(cloned_id, block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop versioning inserts a fast path for bounds checks.
    #[test]
    fn test_version_loops_bounds_guard() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    v11: boolean = int.le.u v2, v1
    branch v11, b6(v3), b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return

b6(v12: uint32):
    v13: boolean = int.lt.u v12, v2
    branch v13, b7, b5

b7:
    v14: boolean = int.lt.u v12, v1
    jump b8

b8:
    v15: ref<uint8, borrowed, mutable> = element.address v0, v12
    v16: uint8 = 1
    store v15, v16
    v17: uint32 = int.add v12, v4
    jump b6(v17)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(expected);
    }

    /// Loop versioning handles non zero induction starts.
    #[test]
    fn test_version_loops_non_zero_start() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 2
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 2
    v4: uint32 = 1
    v11: boolean = int.le.u v2, v1
    branch v11, b6(v3), b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return

b6(v12: uint32):
    v13: boolean = int.lt.u v12, v2
    branch v13, b7, b5

b7:
    v14: boolean = int.lt.u v12, v1
    jump b8

b8:
    v15: ref<uint8, borrowed, mutable> = element.address v0, v12
    v16: uint8 = 1
    store v15, v16
    v17: uint32 = int.add v12, v4
    jump b6(v17)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(expected);
    }

    /// Signed bounds checks are not versioned.
    #[test]
    fn test_version_loops_skips_signed_bounds() {
        let input = r#"
function test(v0: [int32; 8], v1: int32, v2: int32): void {
entry(v0: [int32; 8], v1: int32, v2: int32):
    v3: int32 = 0
    v4: int32 = 1
    jump b1(v3)

b1(v5: int32):
    v6: boolean = int.lt.s v5, v2
    branch v6, b2, b3

b2:
    v7: boolean = int.lt.s v5, v1
    check bounds.s v5, v1, v0 => b4, b5

b3:
    return

b4:
    v8: ref<int32, borrowed, mutable> = element.address v0, v5
    v9: int32 = 1
    store v8, v9
    v10: int32 = int.add v5, v4
    jump b1(v10)

b5:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(input);
    }

    /// Mismatched integer types prevent versioning.
    #[test]
    fn test_version_loops_skips_type_mismatch() {
        let input = r#"
function test(v0: [uint8; 8], v1: int32, v2: uint32): void {
entry(v0: [uint8; 8], v1: int32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b3

b2:
    v7: boolean = int.lt.u v5, v2
    check bounds.u v5, v1, v0 => b4, b5

b3:
    return

b4:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b5:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(input);
    }

    /// Non unit strides are still versioned.
    #[test]
    fn test_version_loops_handles_non_unit_stride() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 2
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 2
    v11: boolean = int.le.u v2, v1
    branch v11, b6(v3), b1(v3)

b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return

b6(v12: uint32):
    v13: boolean = int.lt.u v12, v2
    branch v13, b7, b5

b7:
    v14: boolean = int.lt.u v12, v1
    jump b8

b8:
    v15: ref<uint8, borrowed, mutable> = element.address v0, v12
    v16: uint8 = 1
    store v15, v16
    v17: uint32 = int.add v12, v4
    jump b6(v17)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(expected);
    }

    /// Non strict loop guards are not versioned.
    #[test]
    fn test_version_loops_skips_non_strict_guard() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = int.le.u v5, v2
    branch v6, b2, b5

b2:
    v7: boolean = int.lt.u v5, v1
    check bounds.u v5, v1, v0 => b3, b4

b3:
    v8: ref<uint8, borrowed, mutable> = element.address v0, v5
    v9: uint8 = 1
    store v8, v9
    v10: uint32 = int.add v5, v4
    jump b1(v10)

b4:
    unreachable

b5:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&VersionLoops);
        test.assert_output(input);
    }
}
