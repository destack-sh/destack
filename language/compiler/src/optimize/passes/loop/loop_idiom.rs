use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, LoopAnalysis, OwnershipAnalysis, ScalarEvolution, Scev,
};
use crate::optimize::common::{
    BlockParamForwarding, build_use_def_maps, build_value_definition_map,
    instruction_has_side_effects, instruction_is_speculatable, unsigned_int_width_for_value,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Recognize loop idioms and replace them with memory intrinsics.
    ///
    /// This pass recognizes simple byte memset loops with a canonical induction variable.
    /// It requires a single store to `element.addr`.
    ///
    /// ```mir
    /// function @before(v0: [u8; 8], v1: u32) -> void {
    /// block0(v0: [u8; 8], v1: u32):
    ///     v2 = iconst 0u32
    ///     v3 = iconst 1u32
    ///     jump block1(v2)
    /// block1(v4: u32):
    ///     v5 = icmp_ult v4, v1
    ///     branch v5, block2, block3
    /// block2:
    ///     v6 = element.addr v0, v4
    ///     v7 = iconst 0u8
    ///     store v6, v7
    ///     v8 = iadd v4, v3
    ///     jump block1(v8)
    /// block3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: [u8; 8], v1: u32) -> void {
    /// block0(v0: [u8; 8], v1: u32):
    ///     v2 = iconst 0u32
    ///     v3 = iconst 1u32
    ///     v9 = iconst 0u8
    ///     v10 = element.addr v0, v2
    ///     intrinsic.memset(v10, v9, v1)
    ///     jump block3
    /// block1(v4: u32):
    ///     v5 = icmp_ult v4, v1
    ///     branch v5, block2, block3
    /// block2:
    ///     v6 = element.addr v0, v4
    ///     v7 = iconst 0u8
    ///     store v6, v7
    ///     v8 = iadd v4, v3
    ///     jump block1(v8)
    /// block3:
    ///     return
    /// }
    /// ```
    #[pass(id = "loop-idiom")]
    pub LoopIdiomRecognize,
    "Recognize loop idioms (memset/memcpy)"
}

impl FunctionPass for LoopIdiomRecognize {
    /// Run loop idiom recognition on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        let changed = run_loop_idiom(function, tree);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "LoopIdiomRecognize"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "loop-idiom"
    }
}

/// Captures the induction guard pattern for a loop.
#[derive(Debug, Clone)]
struct GuardInfo {
    /// The induction value used by the guard.
    induction: mir::Value,
    /// The bound value used by the guard.
    bound: mir::Value,
}

/// Run loop idiom recognition on a single function and report whether it changed.
fn run_loop_idiom(function: &mut mir::Function, tree: &mut mir::NodeTree) -> bool {
    // gather analyses
    let analyses = FunctionAnalyses::new(function, tree);
    let loops = analyses.get::<LoopAnalysis>().clone();
    let cfg = analyses.get::<ControlFlowGraph>().clone();
    let domtree = analyses.get::<DominatorTree>().clone();
    let scev = analyses.get::<ScalarEvolution>().clone();
    let ownership = analyses.get::<OwnershipAnalysis>().clone();
    let forwarding = BlockParamForwarding::build(function, tree, &cfg);

    // bail out when no loops are present
    if loops.num_loops() == 0 {
        return false;
    }

    // build use def and definition maps
    let use_def = build_use_def_maps(function, tree);
    let value_definitions = build_value_definition_map(function, tree);

    // refresh value ids and track changes
    let mut changed = false;
    function.recompute_next_value_id(tree);

    // scan loops for idioms
    for (loop_index, lp) in loops.loops().iter().enumerate() {
        // require a single latch and exit
        if !lp.has_single_latch() || lp.exit_blocks.len() != 1 || lp.exiting_blocks.len() != 1 {
            continue;
        }

        // require a distinct header and latch
        let header = lp.header;
        if header == lp.latches[0] {
            continue;
        }

        // locate the loop preheader
        let Some((preheader, _)) = find_preheader(header, &lp.blocks, &cfg, tree) else {
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

        // require a parameter free exit block
        let exit_block = lp.exit_blocks[0];
        if !tree.get(exit_block).parameters.is_empty() {
            continue;
        }

        // require consistent unsigned types for induction and bound
        let Some(induction_width) = unsigned_int_width_for_value(guard.induction, &ownership, tree)
        else {
            continue;
        };
        let Some(bound_width) = unsigned_int_width_for_value(guard.bound, &ownership, tree) else {
            continue;
        };
        if induction_width != bound_width {
            continue;
        }

        // require the guard bound to be loop invariant
        if !value_is_loop_invariant(guard.bound, lp, &use_def, &forwarding) {
            continue;
        }

        // match a memset style store in the loop body
        let Some(pattern) = match_memset_pattern(lp, guard.induction, tree, &value_definitions)
        else {
            continue;
        };

        // require the store to be in this loop, not a nested one
        let Some(inner_loop) = loops.innermost_loop(pattern.store_block) else {
            continue;
        };
        if inner_loop.header != lp.header {
            continue;
        }

        // ensure the store executes on every iteration
        if !domtree.dominates(pattern.store_block, lp.latches[0]) {
            continue;
        }

        // require a constant fill value
        let Some(value_const) = pattern.value_constant else {
            continue;
        };

        // compute a zero index value
        let Some(zero_const) = const_zero_for_index(guard.induction, &ownership, tree) else {
            continue;
        };

        // ensure the array element type is u8
        if !value_is_loop_invariant(pattern.array, lp, &use_def, &forwarding) {
            continue;
        }

        if !array_is_u8(pattern.array, &ownership, tree) {
            continue;
        }

        // insert memset into the preheader
        let mut preheader_block = tree.get(preheader).clone();

        // emit the fill value constant
        let value_inst = tree.insert(mir::Instruction::Const {
            destination: function.next_value(),
            value: value_const,
        });
        preheader_block.instructions.push(value_inst);

        // emit a zero index constant
        let zero_inst = tree.insert(mir::Instruction::Const {
            destination: function.next_value(),
            value: zero_const,
        });
        preheader_block.instructions.push(zero_inst);

        // compute the base pointer for the memset
        let ptr_inst = tree.insert(mir::Instruction::ElementAddr {
            destination: function.next_value(),
            array: pattern.array,
            index: tree.get(zero_inst).destination().unwrap(),
        });
        preheader_block.instructions.push(ptr_inst);

        // emit the memset intrinsic call
        let args = tree.add_arguments(&[
            tree.get(ptr_inst).destination().unwrap(),
            tree.get(value_inst).destination().unwrap(),
            guard.bound,
        ]);
        let memset_inst = tree.insert(mir::Instruction::Intrinsic {
            destination: None,
            intrinsic: mir::Intrinsic::Memset,
            arguments: args,
            ordering: None,
        });
        preheader_block.instructions.push(memset_inst);

        // bypass the original loop body
        preheader_block.terminator = mir::Terminator::Jump {
            target: exit_block,
            arguments: Vec::new(),
        };

        tree.replace(preheader, preheader_block);

        // preserve original loop blocks (now unreachable)
        changed = true;
    }

    changed
}

#[derive(Debug, Clone)]
struct MemsetPattern {
    /// The array being filled.
    array: mir::Value,
    /// The constant value written by the store, when available.
    value_constant: Option<mir::Constant>,
    /// The block containing the store.
    store_block: mir::LocalNodeId<mir::Block>,
}

/// Match a loop body against a memset idiom.
fn match_memset_pattern(
    lp: &crate::optimize::analyses::Loop,
    induction: mir::Value,
    tree: &mir::NodeTree,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<MemsetPattern> {
    // scan loop blocks for a single store with a speculatable body
    let mut store_ptr = None;
    let mut store_value = None;
    let mut store_block = None;

    for &block_id in &lp.blocks {
        let block = tree.get(block_id);

        // require simple loop terminators
        match &block.terminator {
            mir::Terminator::Jump { .. } | mir::Terminator::Branch { .. } => {}
            _ => return None,
        }

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);
            if instruction_has_side_effects(inst) {
                if let mir::Instruction::Store { pointer, value } = inst {
                    if store_ptr.is_some() {
                        return None;
                    }
                    store_ptr = Some(*pointer);
                    store_value = Some(*value);
                    store_block = Some(block_id);
                    continue;
                }

                return None;
            }

            if !instruction_is_speculatable(inst) {
                return None;
            }
        }
    }

    // resolve the stored address back to an element.addr
    let store_ptr = store_ptr?;
    let (array, index) = element_addr_for_pointer(store_ptr, tree, value_definitions)?;
    if index != induction {
        return None;
    }

    // extract a constant fill value when possible
    let value = store_value?;
    let value_const = constant_for_value(value, tree, value_definitions);
    Some(MemsetPattern {
        array,
        value_constant: value_const,
        store_block: store_block?,
    })
}

/// Extract a constant value from the given operand.
fn constant_for_value(
    value: mir::Value,
    tree: &mir::NodeTree,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<mir::Constant> {
    // find the instruction that defines the value
    let inst_id = *value_definitions.get(&value)?;

    // accept only constant definitions
    match tree.get(inst_id) {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Find an element address instruction for the given pointer.
fn element_addr_for_pointer(
    pointer: mir::Value,
    tree: &mir::NodeTree,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<(mir::Value, mir::Value)> {
    // find the instruction that defines the pointer
    let inst_id = *value_definitions.get(&pointer)?;

    // require a direct element address computation
    match tree.get(inst_id) {
        mir::Instruction::ElementAddr { array, index, .. } => Some((*array, *index)),
        _ => None,
    }
}

/// Return a zero constant for the given index type.
fn const_zero_for_index(
    index: mir::Value,
    ownership: &OwnershipAnalysis,
    tree: &mir::NodeTree,
) -> Option<mir::Constant> {
    // select a zero constant that matches the index type
    let ty_id = ownership.value_type(index)?;
    let ty = tree.get(ty_id);
    match ty {
        mir::Type::Int { width, signed } => {
            if *signed {
                Some(mir::Constant::Int {
                    value: 0,
                    width: *width as u8,
                    is_signed: true,
                })
            } else {
                Some(mir::Constant::UInt {
                    value: 0,
                    width: *width as u8,
                })
            }
        }
        _ => None,
    }
}

/// Check whether the array element type is u8.
fn array_is_u8(array: mir::Value, ownership: &OwnershipAnalysis, tree: &mir::NodeTree) -> bool {
    // resolve the array element type
    let Some(ty_id) = ownership.value_type(array) else {
        return false;
    };
    let ty = tree.get(ty_id);
    let element = match ty {
        mir::Type::Array { element, .. } => *element,
        mir::Type::Reference { pointee, .. } => *pointee,
        _ => return false,
    };
    let element_ty = tree.get(element);
    matches!(
        element_ty,
        mir::Type::Int {
            width: 8,
            signed: false
        }
    )
}

/// Check whether a value is loop invariant.
fn value_is_loop_invariant(
    value: mir::Value,
    lp: &crate::optimize::analyses::Loop,
    use_def: &crate::optimize::common::UseDefMaps,
    forwarding: &BlockParamForwarding,
) -> bool {
    // resolve forwarded parameters
    let value = forwarding.resolve(value);

    // values without a definition block are treated as invariant
    let Some(def_block) = use_def.def_block.get(&value).copied() else {
        return true;
    };

    // definitions outside the loop are invariant
    !lp.blocks.contains(&def_block)
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
    let header_block = tree.get(header);
    let mir::Terminator::Branch {
        condition,
        then_target,
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
    let def_block = use_def.def_block.get(condition)?;
    let block = tree.get(*def_block);
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

    // accept only unsigned less than guards
    if *operator != mir::BinaryOperator::UnsignedLessThan {
        return None;
    }

    // require the induction variable to be a header parameter
    let header_params: Vec<_> = header_block
        .parameters
        .iter()
        .map(|param| param.value)
        .collect();
    if !header_params.contains(left) {
        return None;
    }

    Some(GuardInfo {
        induction: *left,
        bound: *right,
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

    // require a unit stride starting at zero
    let Scev::Constant(mir::Constant::UInt { value: start, .. }) = &**start else {
        return false;
    };
    let Scev::Constant(mir::Constant::UInt { value: step, .. }) = &**step else {
        return false;
    };

    *start == 0 && *step == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Memset loops are lowered to intrinsic.memset.
    #[test]
    fn test_loop_idiom_memset() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    v9 = iconst 0u8
    v10 = iconst 0u32
    v11 = element.addr v0, v10
    intrinsic.memset(v11, v9, v1)
    jump block3
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Memset loops with a separate latch are lowered.
    #[test]
    fn test_loop_idiom_memset_multi_block() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block4
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    jump block3(v4)
block3(v8: u32):
    v9 = iadd v8, v3
    jump block1(v9)
block4:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    v10 = iconst 0u8
    v11 = iconst 0u32
    v12 = element.addr v0, v11
    intrinsic.memset(v12, v10, v1)
    jump block4
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block4
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    jump block3(v4)
block3(v8: u32):
    v9 = iadd v8, v3
    jump block1(v9)
block4:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Loops with non zero starts are not lowered.
    #[test]
    fn test_loop_idiom_skips_non_zero_start() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 1u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with non unit stride are not lowered.
    #[test]
    fn test_loop_idiom_skips_non_unit_stride() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 2u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with conditional stores are not lowered.
    #[test]
    fn test_loop_idiom_skips_conditional_store() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: bool) -> void {
block0(v0: [u8; 8], v1: u32, v2: bool):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v1
    branch v6, block2(v5), block5
block2(v7: u32):
    branch v2, block3(v7), block4(v7)
block3(v8: u32):
    v9 = element.addr v0, v8
    v10 = iconst 0u8
    store v9, v10
    jump block4(v8)
block4(v11: u32):
    v12 = iadd v11, v4
    jump block1(v12)
block5:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with nested stores are not lowered.
    #[test]
    fn test_loop_idiom_skips_nested_store() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    v4 = iconst 2u32
    jump block1(v2)
block1(v5: u32):
    v6 = icmp_ult v5, v1
    branch v6, block2, block6
block2:
    v7 = iconst 0u32
    jump block3(v7)
block3(v8: u32):
    v9 = element.addr v0, v5
    v10 = iconst 0u8
    store v9, v10
    v11 = icmp_ult v8, v4
    branch v11, block4(v8), block5
block4(v12: u32):
    v13 = iadd v12, v3
    jump block3(v13)
block5:
    v14 = iadd v5, v3
    jump block1(v14)
block6:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with variant arrays are not lowered.
    #[test]
    fn test_loop_idiom_skips_variant_array() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2, v0)
block1(v4: u32, v5: [u8; 8]):
    v6 = icmp_ult v4, v1
    branch v6, block2, block3
block2:
    v7 = element.addr v5, v4
    v8 = iconst 0u8
    store v7, v8
    v9 = element.set v5, v4, v8
    v10 = iadd v4, v3
    jump block1(v10, v9)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with non constant stores are left unchanged.
    #[test]
    fn test_loop_idiom_skips_non_constant_store() {
        let input = r#"function @test(v0: [u8; 8], v1: u8, v2: u32) -> void {
block0(v0: [u8; 8], v1: u8, v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5
    store v7, v1
    v8 = iadd v5, v4
    jump block1(v8)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with side effects are not lowered.
    #[test]
    fn test_loop_idiom_skips_side_effects() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    call @touch(v4)
    v6 = element.addr v0, v4
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}
function @touch(v0: u32) -> void {
block0(v0: u32):
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }

    /// Loops with multiple stores are not lowered.
    #[test]
    fn test_loop_idiom_skips_multiple_stores() {
        let input = r#"function @test(v0: [u8; 8], v1: [u8; 8], v2: u32) -> void {
block0(v0: [u8; 8], v1: [u8; 8], v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5
    v8 = iconst 0u8
    store v7, v8
    v9 = element.addr v1, v5
    store v9, v8
    v10 = iadd v5, v4
    jump block1(v10)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(input);
    }
}
