use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ControlFlowGraph, DominatorTree, LoopAnalysis, OwnershipAnalysis, RangeAnalysis,
    ScalarEvolution, Scev, ValueRange,
};
use crate::optimize::common::{
    BlockParamForwarding, TypeKey, build_use_def_maps, build_value_definition_map,
    build_value_use_counts, constant_for_value, constant_is_zero, instruction_has_side_effects,
    instruction_is_speculatable, unsigned_int_width_for_value,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Recognize loop idioms and replace them with memory intrinsics.
    ///
    /// This pass recognizes simple byte memset loops and copy loops with a canonical induction
    /// variable.
    /// It requires a single store to `element.addr` and, for copies, a single load feeding that
    /// store.
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
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        let changed = run_loop_idiom(function, tree, ctx);
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
fn run_loop_idiom(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    let mut changed = false;
    loop {
        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let scev = analyses.get::<ScalarEvolution>().clone();
        let ranges = analyses.get::<RangeAnalysis>().clone();
        let ownership = analyses.get::<OwnershipAnalysis>().clone();
        let aa = analyses.get::<AliasAnalysis>().clone();
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // bail out when no loops are present
        if loops.num_loops() == 0 {
            break;
        }

        // build use def and definition maps
        let use_def = build_use_def_maps(function, tree);
        let value_definitions = build_value_definition_map(function, tree);
        let use_counts = build_value_use_counts(function, tree);

        // refresh value ids and track per iteration changes
        let mut changed_this_iteration = false;
        function.recompute_next_value_id(tree);

        // scan loops for idioms
        'scan_loops: for (loop_index, lp) in loops.loops().iter().enumerate() {
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

            // require a parameter free exit block
            let exit_block = lp.exit_blocks[0];
            if !tree.get(exit_block).parameters.is_empty() {
                continue;
            }

            let Some(start_value) =
                preheader_induction_start(guard.induction, header, &preheader_args, tree)
            else {
                continue;
            };

            let Some(bound_value) =
                preheader_value_for_param(guard.bound, header, &preheader_args, tree)
            else {
                continue;
            };

            // require consistent unsigned types for induction and bound
            let Some(induction_width) = unsigned_int_width_for_value(
                guard.induction,
                &ownership,
                ctx.type_context().pointer_width_bits,
                tree,
            ) else {
                continue;
            };
            let Some(bound_width) = unsigned_int_width_for_value(
                bound_value,
                &ownership,
                ctx.type_context().pointer_width_bits,
                tree,
            ) else {
                continue;
            };
            if induction_width != bound_width {
                continue;
            }
            let Some(start_width) = unsigned_int_width_for_value(
                start_value,
                &ownership,
                ctx.type_context().pointer_width_bits,
                tree,
            ) else {
                continue;
            };
            if start_width != induction_width {
                continue;
            }

            // require the guard bound to be loop invariant
            if !value_is_loop_invariant(bound_value, lp, &use_def, &forwarding) {
                continue;
            }
            if !value_is_loop_invariant(start_value, lp, &use_def, &forwarding) {
                continue;
            }

            // attempt to replace the loop with memset
            if let Some(pattern) =
                match_memset_pattern(lp, guard.induction, tree, &value_definitions)
            {
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

                // ensure the array element type is u8
                if !value_is_loop_invariant(pattern.array, lp, &use_def, &forwarding) {
                    continue;
                }

                if !array_is_u8(pattern.array, &ownership, tree) {
                    continue;
                }

                let should_guard = should_guard_copy_bounds(
                    start_value,
                    bound_value,
                    induction_width,
                    preheader,
                    &ranges,
                    &value_definitions,
                    tree,
                );

                let mem_block = if should_guard {
                    let mem_block = tree.insert(mir::Block {
                        parameters: Vec::new(),
                        instructions: Vec::new(),
                        terminator: mir::Terminator::Jump {
                            target: exit_block,
                            arguments: Vec::new(),
                        },
                    });
                    function.blocks.push(mem_block);
                    mem_block
                } else {
                    preheader
                };

                let mut mem_block_data = tree.get(mem_block).clone();
                let Some(length_value) = emit_copy_length(
                    bound_value,
                    start_value,
                    induction_width,
                    1,
                    should_guard,
                    &ranges,
                    preheader,
                    &value_definitions,
                    function,
                    tree,
                    &mut mem_block_data,
                ) else {
                    continue;
                };

                emit_memset(
                    pattern.array,
                    pattern.element_addr_type,
                    start_value,
                    value_const,
                    length_value,
                    function,
                    tree,
                    &mut mem_block_data,
                );
                mem_block_data.terminator = mir::Terminator::Jump {
                    target: exit_block,
                    arguments: Vec::new(),
                };
                tree.replace(mem_block, mem_block_data);

                // bypass the original loop body
                let mut preheader_block = tree.get(preheader).clone();
                if should_guard {
                    let guard_inst = insert_bound_guard(
                        start_value,
                        bound_value,
                        function,
                        tree,
                        &mut preheader_block,
                        mem_block,
                        exit_block,
                    );
                    if guard_inst.is_none() {
                        continue;
                    }
                } else {
                    preheader_block.terminator = mir::Terminator::Jump {
                        target: exit_block,
                        arguments: Vec::new(),
                    };
                }
                tree.replace(preheader, preheader_block);

                // preserve original loop blocks now unreachable
                changed = true;
                changed_this_iteration = true;
                break 'scan_loops;
            }

            // attempt to replace the loop with memcpy or memmove
            let Some(pattern) =
                match_memcpy_pattern(lp, guard.induction, tree, &value_definitions, &use_counts)
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

            // ensure the load executes on every iteration
            if !domtree.dominates(pattern.load_block, lp.latches[0]) {
                continue;
            }

            // require invariant arrays
            if !value_is_loop_invariant(pattern.dest_array, lp, &use_def, &forwarding)
                || !value_is_loop_invariant(pattern.src_array, lp, &use_def, &forwarding)
            {
                continue;
            }

            // resolve the element type for both arrays
            let Some(dest_element) = array_element_type(pattern.dest_array, &ownership, tree)
            else {
                continue;
            };
            let Some(src_element) = array_element_type(pattern.src_array, &ownership, tree) else {
                continue;
            };

            let dest_key = TypeKey::from_type(dest_element, tree);
            let src_key = TypeKey::from_type(src_element, tree);
            if dest_key != src_key {
                continue;
            }

            let Some(element_size) = dest_key.byte_size(ctx.type_context().pointer_width_bits)
            else {
                continue;
            };

            let Some(dest_space) = reference_address_space(pattern.dest_element_addr_type, tree)
            else {
                continue;
            };
            let Some(src_space) = reference_address_space(pattern.src_element_addr_type, tree)
            else {
                continue;
            };
            if dest_space != src_space {
                continue;
            }

            let should_guard = should_guard_copy_bounds(
                start_value,
                bound_value,
                induction_width,
                preheader,
                &ranges,
                &value_definitions,
                tree,
            );

            let mem_block = if should_guard {
                let mem_block = tree.insert(mir::Block {
                    parameters: Vec::new(),
                    instructions: Vec::new(),
                    terminator: mir::Terminator::Jump {
                        target: exit_block,
                        arguments: Vec::new(),
                    },
                });
                function.blocks.push(mem_block);
                mem_block
            } else {
                preheader
            };

            let mut mem_block_data = tree.get(mem_block).clone();
            let Some(length_value) = emit_copy_length(
                bound_value,
                start_value,
                induction_width,
                element_size,
                should_guard,
                &ranges,
                preheader,
                &value_definitions,
                function,
                tree,
                &mut mem_block_data,
            ) else {
                continue;
            };

            let use_memcpy =
                arrays_are_value_types(pattern.dest_array, pattern.src_array, &ownership, tree)
                    || aa.pointers_no_alias(pattern.store_pointer, pattern.load_pointer);
            let intrinsic = if use_memcpy {
                mir::Intrinsic::Memcpy
            } else {
                mir::Intrinsic::Memmove
            };

            emit_memcpy_or_memmove(
                intrinsic,
                pattern.dest_array,
                pattern.src_array,
                pattern.dest_element_addr_type,
                pattern.src_element_addr_type,
                start_value,
                length_value,
                function,
                tree,
                &mut mem_block_data,
            );
            mem_block_data.terminator = mir::Terminator::Jump {
                target: exit_block,
                arguments: Vec::new(),
            };
            tree.replace(mem_block, mem_block_data);

            let mut preheader_block = tree.get(preheader).clone();
            if should_guard {
                let guard_inst = insert_bound_guard(
                    start_value,
                    bound_value,
                    function,
                    tree,
                    &mut preheader_block,
                    mem_block,
                    exit_block,
                );
                if guard_inst.is_none() {
                    continue;
                }
            } else {
                preheader_block.terminator = mir::Terminator::Jump {
                    target: exit_block,
                    arguments: Vec::new(),
                };
            }
            tree.replace(preheader, preheader_block);

            changed = true;
            changed_this_iteration = true;
            break 'scan_loops;
        }

        if !changed_this_iteration {
            break;
        }
    }

    changed
}

#[derive(Debug, Clone)]
struct MemsetPattern {
    /// The array being filled.
    array: mir::Value,
    /// The element address result type.
    element_addr_type: mir::LocalNodeId<mir::Type>,
    /// The constant value written by the store, when available.
    value_constant: Option<mir::Constant>,
    /// The block containing the store.
    store_block: mir::LocalNodeId<mir::Block>,
}

#[derive(Debug, Clone)]
struct MemcpyPattern {
    /// Destination array value.
    dest_array: mir::Value,
    /// Source array value.
    src_array: mir::Value,
    /// The destination element address result type.
    dest_element_addr_type: mir::LocalNodeId<mir::Type>,
    /// The source element address result type.
    src_element_addr_type: mir::LocalNodeId<mir::Type>,
    /// The block containing the store.
    store_block: mir::LocalNodeId<mir::Block>,
    /// The block containing the load.
    load_block: mir::LocalNodeId<mir::Block>,
    /// The store pointer value.
    store_pointer: mir::Value,
    /// The load pointer value.
    load_pointer: mir::Value,
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

    // require a single store and resolve its address
    let store_ptr = store_ptr?;
    let (array, index, element_addr_type) =
        element_addr_for_pointer(store_ptr, tree, value_definitions)?;
    if index != induction {
        return None;
    }

    // extract a constant fill value when possible
    let value = store_value?;
    let value_const = constant_for_value(value, value_definitions, tree);
    Some(MemsetPattern {
        array,
        element_addr_type,
        value_constant: value_const,
        store_block: store_block?,
    })
}

/// Match a loop body against a memcpy or memmove idiom.
fn match_memcpy_pattern(
    lp: &crate::optimize::analyses::Loop,
    induction: mir::Value,
    tree: &mir::NodeTree,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    use_counts: &HashMap<mir::Value, usize>,
) -> Option<MemcpyPattern> {
    // scan loop blocks for a single load and store
    let mut store_ptr = None;
    let mut store_value = None;
    let mut store_block = None;
    let mut load_ptr = None;
    let mut load_value = None;
    let mut load_block = None;

    for &block_id in &lp.blocks {
        let block = tree.get(block_id);

        // require simple loop terminators
        match &block.terminator {
            mir::Terminator::Jump { .. } | mir::Terminator::Branch { .. } => {}
            _ => return None,
        }

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);
            match inst {
                mir::Instruction::Store { pointer, value } => {
                    if store_ptr.is_some() {
                        return None;
                    }
                    store_ptr = Some(*pointer);
                    store_value = Some(*value);
                    store_block = Some(block_id);
                    continue;
                }
                mir::Instruction::Load {
                    destination,
                    pointer,
                    ..
                } => {
                    if load_ptr.is_some() {
                        return None;
                    }
                    load_ptr = Some(*pointer);
                    load_value = Some(*destination);
                    load_block = Some(block_id);
                    continue;
                }
                _ => {}
            }

            if instruction_has_side_effects(inst) {
                return None;
            }
            if !instruction_is_speculatable(inst) {
                return None;
            }
        }
    }

    // require a single load and store
    let store_ptr = store_ptr?;
    let store_value = store_value?;
    let load_ptr = load_ptr?;
    let load_value = load_value?;

    // require the load value to feed the store
    if store_value != load_value {
        return None;
    }

    // require the load value to be used only by the store
    if use_counts.get(&load_value).copied().unwrap_or(0) != 1 {
        return None;
    }

    // resolve both addresses back to element.addr
    let (dest_array, dest_index, dest_element_addr_type) =
        element_addr_for_pointer(store_ptr, tree, value_definitions)?;
    let (src_array, src_index, src_element_addr_type) =
        element_addr_for_pointer(load_ptr, tree, value_definitions)?;

    if dest_index != induction || src_index != induction {
        return None;
    }

    Some(MemcpyPattern {
        dest_array,
        src_array,
        dest_element_addr_type,
        src_element_addr_type,
        store_block: store_block?,
        load_block: load_block?,
        store_pointer: store_ptr,
        load_pointer: load_ptr,
    })
}

/// Find an element address instruction for the given pointer.
fn element_addr_for_pointer(
    pointer: mir::Value,
    tree: &mir::NodeTree,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<(mir::Value, mir::Value, mir::LocalNodeId<mir::Type>)> {
    // find the instruction that defines the pointer
    let inst_id = *value_definitions.get(&pointer)?;

    // require a direct element address computation
    match tree.get(inst_id) {
        mir::Instruction::ElementAddr {
            array,
            index,
            result_type,
            ..
        } => Some((*array, *index, *result_type)),
        _ => None,
    }
}

/// Check whether the array element type is u8.
fn array_is_u8(array: mir::Value, ownership: &OwnershipAnalysis, tree: &mir::NodeTree) -> bool {
    // resolve the array element type
    let Some(element) = array_element_type(array, ownership, tree) else {
        return false;
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

/// Return the element type for an array or array reference value.
fn array_element_type(
    array: mir::Value,
    ownership: &OwnershipAnalysis,
    tree: &mir::NodeTree,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // resolve the array type
    let ty_id = ownership.value_type(array)?;
    let ty = tree.get(ty_id);

    match ty {
        mir::Type::Array { element, .. } => Some(*element),
        mir::Type::Reference { pointee, .. } => match tree.get(*pointee) {
            mir::Type::Array { element, .. } => Some(*element),
            _ => None,
        },
        _ => None,
    }
}

/// Return true when both arrays are value types.
fn arrays_are_value_types(
    dest_array: mir::Value,
    src_array: mir::Value,
    ownership: &OwnershipAnalysis,
    tree: &mir::NodeTree,
) -> bool {
    if dest_array == src_array {
        return false;
    }

    let Some(dest_ty) = ownership.value_type(dest_array) else {
        return false;
    };
    let Some(src_ty) = ownership.value_type(src_array) else {
        return false;
    };

    matches!(tree.get(dest_ty), mir::Type::Array { .. })
        && matches!(tree.get(src_ty), mir::Type::Array { .. })
}

/// Return the address space for a reference type.
fn reference_address_space(
    ty_id: mir::LocalNodeId<mir::Type>,
    tree: &mir::NodeTree,
) -> Option<mir::AddressSpace> {
    match tree.get(ty_id) {
        mir::Type::Reference { address_space, .. } => Some(*address_space),
        _ => None,
    }
}

/// Decide whether a bounds guard is needed before emitting a copy intrinsic.
fn should_guard_copy_bounds(
    start: mir::Value,
    bound: mir::Value,
    bound_width: u16,
    preheader: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
) -> bool {
    // skip guards when the start is definitely zero
    let start_const = constant_for_value(start, value_definitions, tree);
    if constant_is_zero(start_const.as_ref()) {
        return false;
    }

    // require ranges that prove start is always below the bound
    let Some((_start_min, start_max)) = unsigned_bounds_for_value(
        start,
        bound_width,
        ranges,
        preheader,
        value_definitions,
        tree,
    ) else {
        return true;
    };
    let Some((bound_min, _bound_max)) = unsigned_bounds_for_value(
        bound,
        bound_width,
        ranges,
        preheader,
        value_definitions,
        tree,
    ) else {
        return true;
    };

    // guard when start could exceed the bound
    start_max > bound_min
}

/// Insert a bounds guard branching to the fast path or exit.
fn insert_bound_guard(
    start: mir::Value,
    bound: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    preheader_block: &mut mir::Block,
    success_target: mir::LocalNodeId<mir::Block>,
    failure_target: mir::LocalNodeId<mir::Block>,
) -> Option<mir::LocalNodeId<mir::Instruction>> {
    // emit the bound comparison
    let guard_inst = tree.insert(mir::Instruction::Binary {
        destination: function.next_value(),
        operator: mir::BinaryOperator::UnsignedLessEqual,
        left: start,
        right: bound,
    });
    preheader_block.instructions.push(guard_inst);

    // branch on the guard to the fast path or exit
    let guard_value = tree.get(guard_inst).destination()?;
    preheader_block.terminator = mir::Terminator::Branch {
        condition: guard_value,
        then_target: success_target,
        then_arguments: Vec::new(),
        else_target: failure_target,
        else_arguments: Vec::new(),
    };

    Some(guard_inst)
}

/// Emit an intrinsic.memset for the loop idiom.
// allow explicit context parameters for memset emission
#[allow(clippy::too_many_arguments)]
fn emit_memset(
    array: mir::Value,
    element_addr_type: mir::LocalNodeId<mir::Type>,
    start: mir::Value,
    fill_value: mir::Constant,
    length: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    block: &mut mir::Block,
) {
    // materialize the fill constant
    let fill_inst = tree.insert(mir::Instruction::Const {
        destination: function.next_value(),
        value: fill_value,
    });
    block.instructions.push(fill_inst);

    // compute the base pointer for the memset
    let ptr_inst = tree.insert(mir::Instruction::ElementAddr {
        destination: function.next_value(),
        array,
        index: start,
        result_type: element_addr_type,
    });
    block.instructions.push(ptr_inst);

    // emit the memset call
    let args = tree.add_arguments(&[
        tree.get(ptr_inst).destination().unwrap(),
        tree.get(fill_inst).destination().unwrap(),
        length,
    ]);
    let mem_inst = tree.insert(mir::Instruction::Intrinsic {
        destination: None,
        intrinsic: mir::Intrinsic::Memset,
        arguments: args,
        ordering: None,
    });
    block.instructions.push(mem_inst);
}

/// Emit memcpy or memmove for the loop idiom.
// allow explicit context parameters for copy emission
#[allow(clippy::too_many_arguments)]
fn emit_memcpy_or_memmove(
    intrinsic: mir::Intrinsic,
    dest_array: mir::Value,
    src_array: mir::Value,
    dest_element_addr_type: mir::LocalNodeId<mir::Type>,
    src_element_addr_type: mir::LocalNodeId<mir::Type>,
    start: mir::Value,
    length: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    block: &mut mir::Block,
) {
    // compute the destination base pointer
    let dest_ptr_inst = tree.insert(mir::Instruction::ElementAddr {
        destination: function.next_value(),
        array: dest_array,
        index: start,
        result_type: dest_element_addr_type,
    });
    block.instructions.push(dest_ptr_inst);

    // compute the source base pointer
    let src_ptr_inst = tree.insert(mir::Instruction::ElementAddr {
        destination: function.next_value(),
        array: src_array,
        index: start,
        result_type: src_element_addr_type,
    });
    block.instructions.push(src_ptr_inst);

    // emit the intrinsic call
    let args = tree.add_arguments(&[
        tree.get(dest_ptr_inst).destination().unwrap(),
        tree.get(src_ptr_inst).destination().unwrap(),
        length,
    ]);
    let mem_inst = tree.insert(mir::Instruction::Intrinsic {
        destination: None,
        intrinsic,
        arguments: args,
        ordering: None,
    });
    block.instructions.push(mem_inst);
}

/// Return unsigned integer bounds for a value when available.
fn unsigned_bounds_for_value(
    value: mir::Value,
    bound_width: u16,
    ranges: &RangeAnalysis,
    preheader: mir::LocalNodeId<mir::Block>,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
) -> Option<(i128, i128)> {
    // use range information when available
    if let Some(ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    }) = ranges.exit(preheader).get(value)
    {
        if !*is_signed && *width as u16 == bound_width && *min >= 0 {
            return Some((*min, *max));
        }

        return None;
    }

    // fall back to a constant value range
    let constant = constant_for_value(value, value_definitions, tree)?;
    match constant {
        mir::Constant::UInt { value, width } => {
            if width as u16 == bound_width {
                let value = value as i128;
                Some((value, value))
            } else {
                None
            }
        }
        mir::Constant::Int {
            value,
            width,
            is_signed: _,
        } => {
            if width as u16 != bound_width || value < 0 {
                return None;
            }

            let value = value as i128;
            Some((value, value))
        }
        _ => None,
    }
}

/// Emit a byte length value for memset, memcpy, or memmove.
// allow explicit context parameters for length materialization
#[allow(clippy::too_many_arguments)]
fn emit_copy_length(
    bound: mir::Value,
    start: mir::Value,
    bound_width: u16,
    element_size: u64,
    guarded: bool,
    ranges: &RangeAnalysis,
    preheader: mir::LocalNodeId<mir::Block>,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    block: &mut mir::Block,
) -> Option<mir::Value> {
    // skip length materialization when byte sized with zero start
    let start_const = constant_for_value(start, value_definitions, tree);
    let mut start_is_zero = constant_is_zero(start_const.as_ref());
    if element_size == 1 && start_is_zero {
        return Some(bound);
    }

    // collect range bounds when needed for safety
    let mut bounds = None;
    if element_size > 1 || !guarded {
        let bound_range = unsigned_bounds_for_value(
            bound,
            bound_width,
            ranges,
            preheader,
            value_definitions,
            tree,
        )?;
        let start_range = unsigned_bounds_for_value(
            start,
            bound_width,
            ranges,
            preheader,
            value_definitions,
            tree,
        )?;
        bounds = Some((bound_range, start_range));
    }

    // update zero start detection based on ranges
    if let Some((_bound, (start_min, start_max))) = bounds {
        start_is_zero = start_is_zero || (start_min == 0 && start_max == 0);
    }
    if element_size == 1 && start_is_zero {
        return Some(bound);
    }

    // require a range that keeps the arithmetic in bounds
    if let Some(((bound_min, bound_max), (start_min, start_max))) = bounds {
        if start_min > bound_max {
            return None;
        }
        if start_max < 0 || bound_min < 0 {
            return None;
        }

        let max_value = if bound_width == 128 {
            u128::MAX
        } else {
            (1u128 << bound_width) - 1
        };

        if u128::from(element_size) > max_value {
            return None;
        }

        let length_max = (bound_max as u128).saturating_sub(start_min as u128);
        if length_max > max_value / element_size.max(1) as u128 {
            return None;
        }
    } else if element_size > 1 {
        return None;
    }

    // materialize the base length
    let length_base = if start_is_zero {
        bound
    } else {
        let subtract_inst = tree.insert(mir::Instruction::Binary {
            destination: function.next_value(),
            operator: mir::BinaryOperator::Subtract,
            left: bound,
            right: start,
        });
        block.instructions.push(subtract_inst);
        tree.get(subtract_inst).destination().unwrap()
    };

    // byte size requires no further scaling
    if element_size == 1 {
        return Some(length_base);
    }

    // materialize the element size constant
    let size_const = mir::Constant::UInt {
        value: element_size,
        width: bound_width as u8,
    };
    let size_inst = tree.insert(mir::Instruction::Const {
        destination: function.next_value(),
        value: size_const,
    });
    block.instructions.push(size_inst);

    // multiply by element size
    let length_inst = tree.insert(mir::Instruction::Binary {
        destination: function.next_value(),
        operator: mir::BinaryOperator::Multiply,
        left: length_base,
        right: tree.get(size_inst).destination().unwrap(),
    });
    block.instructions.push(length_inst);

    Some(tree.get(length_inst).destination().unwrap())
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

/// Map a header parameter to its preheader argument, if applicable.
fn preheader_value_for_param(
    value: mir::Value,
    header: mir::LocalNodeId<mir::Block>,
    preheader_args: &[mir::Value],
    tree: &mir::NodeTree,
) -> Option<mir::Value> {
    // identify the header parameter position
    let header_block = tree.get(header);
    let position = header_block
        .parameters
        .iter()
        .position(|param| param.value == value);

    // return the mapped argument when the value is a header parameter
    match position {
        Some(index) => preheader_args.get(index).copied(),
        None => Some(value),
    }
}

/// Resolve the induction variable start from the preheader.
fn preheader_induction_start(
    induction: mir::Value,
    header: mir::LocalNodeId<mir::Block>,
    preheader_args: &[mir::Value],
    tree: &mir::NodeTree,
) -> Option<mir::Value> {
    // require the induction to be a header parameter
    let header_block = tree.get(header);
    let position = header_block
        .parameters
        .iter()
        .position(|param| param.value == induction)?;

    // return the preheader argument for the induction parameter
    preheader_args.get(position).copied()
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
    let Some(Scev::AddRec {
        start,
        step,
        loop_header,
    }) = scev.scev_for_value_in_loop(loop_index, guard.induction)
    else {
        return false;
    };

    // require a unit stride
    let step_is_one = matches!(
        &**step,
        Scev::Constant(mir::Constant::UInt { value: 1, .. })
            | Scev::Constant(mir::Constant::Int { value: 1, .. })
    );
    if !step_is_one {
        return false;
    }

    // allow any loop invariant start
    start.is_loop_invariant(*loop_header)
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
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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
    v10 = element.addr v0, v2 -> ref<borrowed u8>
    intrinsic.memset(v10, v9, v1)
    jump block3
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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
    v11 = element.addr v0, v2 -> ref<borrowed u8>
    intrinsic.memset(v11, v10, v1)
    jump block4
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block4
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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

    /// Memcpy loops are lowered to intrinsic.memcpy.
    #[test]
    fn test_loop_idiom_memcpy() {
        let input = r#"function @test(v0: [u8; 8], v1: [u8; 8], v2: u32) -> void {
block0(v0: [u8; 8], v1: [u8; 8], v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    jump block1(v3)
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u8>
    v8 = element.addr v1, v5 -> ref<borrowed u8>
    v9 = load v8 -> u8
    store v7, v9
    v10 = iadd v5, v4
    jump block1(v10)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: [u8; 8], v2: u32) -> void {
block0(v0: [u8; 8], v1: [u8; 8], v2: u32):
    v3 = iconst 0u32
    v4 = iconst 1u32
    v11 = element.addr v0, v3 -> ref<borrowed u8>
    v12 = element.addr v1, v3 -> ref<borrowed u8>
    intrinsic.memcpy(v11, v12, v2)
    jump block3
block1(v5: u32):
    v6 = icmp_ult v5, v2
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u8>
    v8 = element.addr v1, v5 -> ref<borrowed u8>
    v9 = load v8 -> u8
    store v7, v9
    v10 = iadd v5, v4
    jump block1(v10)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Overlapping copy loops use memmove.
    #[test]
    fn test_loop_idiom_memmove_aliasing() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 0u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = load v6 -> u8
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
    v9 = element.addr v0, v2 -> ref<borrowed u8>
    v10 = element.addr v0, v2 -> ref<borrowed u8>
    intrinsic.memmove(v9, v10, v1)
    jump block3
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = load v6 -> u8
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

    /// Memcpy uses byte length when element size exceeds one byte.
    #[test]
    fn test_loop_idiom_memcpy_multiplies_length() {
        let input = r#"function @test(v0: [u32; 8], v1: [u32; 8]) -> void {
block0(v0: [u32; 8], v1: [u32; 8]):
    v2 = iconst 0u32
    v3 = iconst 1u32
    v4 = iconst 4u32
    jump block1(v2)
block1(v5: u32):
    v6 = icmp_ult v5, v4
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u32>
    v8 = element.addr v1, v5 -> ref<borrowed u32>
    v9 = load v8 -> u32
    store v7, v9
    v10 = iadd v5, v3
    jump block1(v10)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u32; 8], v1: [u32; 8]) -> void {
block0(v0: [u32; 8], v1: [u32; 8]):
    v2 = iconst 0u32
    v3 = iconst 1u32
    v4 = iconst 4u32
    v11 = iconst 4u32
    v12 = imul v4, v11
    v13 = element.addr v0, v2 -> ref<borrowed u32>
    v14 = element.addr v1, v2 -> ref<borrowed u32>
    intrinsic.memcpy(v13, v14, v12)
    jump block3
block1(v5: u32):
    v6 = icmp_ult v5, v4
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u32>
    v8 = element.addr v1, v5 -> ref<borrowed u32>
    v9 = load v8 -> u32
    store v7, v9
    v10 = iadd v5, v3
    jump block1(v10)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Non zero starts scale byte length for wider elements.
    #[test]
    fn test_loop_idiom_memcpy_non_zero_start_multiplies_length() {
        let input = r#"function @test(v0: [u32; 8], v1: [u32; 8]) -> void {
block0(v0: [u32; 8], v1: [u32; 8]):
    v2 = iconst 2u32
    v3 = iconst 1u32
    v4 = iconst 8u32
    jump block1(v2)
block1(v5: u32):
    v6 = icmp_ult v5, v4
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u32>
    v8 = element.addr v1, v5 -> ref<borrowed u32>
    v9 = load v8 -> u32
    store v7, v9
    v10 = iadd v5, v3
    jump block1(v10)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u32; 8], v1: [u32; 8]) -> void {
block0(v0: [u32; 8], v1: [u32; 8]):
    v2 = iconst 2u32
    v3 = iconst 1u32
    v4 = iconst 8u32
    v11 = isub v4, v2
    v12 = iconst 4u32
    v13 = imul v11, v12
    v14 = element.addr v0, v2 -> ref<borrowed u32>
    v15 = element.addr v1, v2 -> ref<borrowed u32>
    intrinsic.memcpy(v14, v15, v13)
    jump block3
block1(v5: u32):
    v6 = icmp_ult v5, v4
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u32>
    v8 = element.addr v1, v5 -> ref<borrowed u32>
    v9 = load v8 -> u32
    store v7, v9
    v10 = iadd v5, v3
    jump block1(v10)
block3:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Loops with non zero starts insert a guard.
    #[test]
    fn test_loop_idiom_guards_non_zero_start() {
        let input = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 1u32
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32) -> void {
block0(v0: [u8; 8], v1: u32):
    v2 = iconst 1u32
    v3 = iconst 1u32
    v12 = icmp_ule v2, v1
    branch v12, block4, block3
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = iconst 0u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
block4:
    v9 = isub v1, v2
    v10 = iconst 0u8
    v11 = element.addr v0, v2 -> ref<borrowed u8>
    intrinsic.memset(v11, v10, v9)
    jump block3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Non zero starts guard memcpy lowering.
    #[test]
    fn test_loop_idiom_memcpy_guards_non_zero_start() {
        let input = r#"function @test(v0: [u8; 8], v1: [u8; 8], v2: u32, v3: u32) -> void {
block0(v0: [u8; 8], v1: [u8; 8], v2: u32, v3: u32):
    v4 = iconst 1u32
    jump block1(v2)
block1(v5: u32):
    v6 = icmp_ult v5, v3
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u8>
    v8 = element.addr v1, v5 -> ref<borrowed u8>
    v9 = load v8 -> u8
    store v7, v9
    v10 = iadd v5, v4
    jump block1(v10)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: [u8; 8], v2: u32, v3: u32) -> void {
block0(v0: [u8; 8], v1: [u8; 8], v2: u32, v3: u32):
    v4 = iconst 1u32
    v14 = icmp_ule v2, v3
    branch v14, block4, block3
block1(v5: u32):
    v6 = icmp_ult v5, v3
    branch v6, block2, block3
block2:
    v7 = element.addr v0, v5 -> ref<borrowed u8>
    v8 = element.addr v1, v5 -> ref<borrowed u8>
    v9 = load v8 -> u8
    store v7, v9
    v10 = iadd v5, v4
    jump block1(v10)
block3:
    return
block4:
    v11 = isub v3, v2
    v12 = element.addr v0, v2 -> ref<borrowed u8>
    v13 = element.addr v1, v2 -> ref<borrowed u8>
    intrinsic.memcpy(v12, v13, v11)
    jump block3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
    }

    /// Non zero starts guard memmove lowering.
    #[test]
    fn test_loop_idiom_memmove_guards_non_zero_start() {
        let input = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 1u32
    jump block1(v2)
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = load v6 -> u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
}"#;

        let expected = r#"function @test(v0: [u8; 8], v1: u32, v2: u32) -> void {
block0(v0: [u8; 8], v1: u32, v2: u32):
    v3 = iconst 1u32
    v12 = icmp_ule v2, v1
    branch v12, block4, block3
block1(v4: u32):
    v5 = icmp_ult v4, v1
    branch v5, block2, block3
block2:
    v6 = element.addr v0, v4 -> ref<borrowed u8>
    v7 = load v6 -> u8
    store v6, v7
    v8 = iadd v4, v3
    jump block1(v8)
block3:
    return
block4:
    v9 = isub v1, v2
    v10 = element.addr v0, v2 -> ref<borrowed u8>
    v11 = element.addr v0, v2 -> ref<borrowed u8>
    intrinsic.memmove(v10, v11, v9)
    jump block3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopIdiomRecognize);
        program.assert_output(expected);
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
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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
    v9 = element.addr v0, v8 -> ref<borrowed u8>
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
    v9 = element.addr v0, v5 -> ref<borrowed u8>
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
    v7 = element.addr v5, v4 -> ref<borrowed u8>
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
    v7 = element.addr v0, v5 -> ref<borrowed u8>
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
    call @touch(v4) -> fn(u32) -> void
    v6 = element.addr v0, v4 -> ref<borrowed u8>
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
    v7 = element.addr v0, v5 -> ref<borrowed u8>
    v8 = iconst 0u8
    store v7, v8
    v9 = element.addr v1, v5 -> ref<borrowed u8>
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
