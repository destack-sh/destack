use destack_core::FxIndexSet;

use crate::optimize::declare_pass;
use destack_mir as mir;
use destack_source::{ProvenanceId, ProvenanceJournal};

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    BlockParamForwarding, ControlTable, DefinitionTable, EvolutionTable, Mutation, RangeTable,
    Scev, UseTable, ValueRange, constant_for_value, constant_is_zero, instruction_has_side_effects,
    instruction_is_borrow_address, instruction_is_speculatable,
};

declare_pass! {
    /// Recognize loop idioms and replace them with memory intrinsics.
    ///
    /// ```mir
    /// function before(v0: ref<[uint8; 8], borrowed, mutable>, v1: uint32): void {
    /// b0(v0: ref<[uint8; 8], borrowed, mutable>, v1: uint32):
    ///     v2: uint32 = 0
    ///     v3: uint32 = 1
    ///     jump b1(v2)
    /// b1(v4: uint32):
    ///     v5: boolean = lt v4, v1
    ///     branch v5 => b2 | b3
    /// b2:
    ///     v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    ///     v7: uint8 = 0
    ///     store v6, v7
    ///     v8: uint32 = add v4, v3
    ///     jump b1(v8)
    /// b3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: ref<[uint8; 8], borrowed, mutable>, v1: uint32): void {
    /// b0(v0: ref<[uint8; 8], borrowed, mutable>, v1: uint32):
    ///     v2: uint32 = 0
    ///     v3: uint32 = 1
    ///     v9: uint8 = 0
    ///     v10: ref<uint8, borrowed, mutable> = element.address v0, v2
    ///     intrinsic.memory.raw.setBytes(v10, v9, v1)
    ///     jump b3
    /// b1(v4: uint32):
    ///     v5: boolean = lt v4, v1
    ///     branch v5 => b2 | b3
    /// b2:
    ///     v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    ///     v7: uint8 = 0
    ///     store v6, v7
    ///     v8: uint32 = add v4, v3
    ///     jump b1(v8)
    /// b3:
    ///     return
    /// }
    /// ```
    #[pass(id = "recognize-loop-idioms")]
    pub RecognizeLoopIdioms,
    "Recognize loop idioms (memset/memcpy)"
}

impl FunctionPass for RecognizeLoopIdioms {
    /// Run loop idiom recognition on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        provenance: &mut ProvenanceJournal<'_>,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        let changed =
            run_recognize_loop_idioms(function, tree, accesses, ctx, analyses, provenance);
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// The induction guard pattern of one loop.
#[derive(Debug, Clone)]
struct LoopGuard {
    /// The comparison instruction used by the guard.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// The induction value used by the guard.
    induction: mir::Value,
    /// The bound value used by the guard.
    bound: mir::Value,
}

/// Run loop idiom recognition on a single function and report whether it changed.
fn run_recognize_loop_idioms(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    ctx: &PipelineContext<'_>,
    analyses: &mut mir::FunctionCache,
    provenance: &mut ProvenanceJournal<'_>,
) -> bool {
    let mut changed = false;
    loop {
        // gather analyses
        let loops = analyses.loops(function, tree).clone();
        let cfg = analyses.control(function, tree).clone();
        let domtree = analyses.dominator(function, tree).clone();
        let scev = analyses.evolution(function, tree).clone();
        let ranges = analyses.range(function, tree).clone();
        let aliases = analyses.alias(function, tree).clone();
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // bail out when no loops are present
        if loops.num_loops() == 0 {
            break;
        }

        // snapshot value definitions and uses
        let definitions = DefinitionTable::build(function, tree);
        let uses = UseTable::build(function, tree);

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
            let Some(guard) = guard_from_header(header, &lp.blocks, function, tree, &definitions)
            else {
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

            // resolve the induction start and the guard bound in the preheader
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
            let Some(induction_width) = function.unsigned_int_width(
                guard.induction,
                ctx.target_layout().pointer_bits(),
                tree,
            ) else {
                continue;
            };
            let Some(bound_width) =
                function.unsigned_int_width(bound_value, ctx.target_layout().pointer_bits(), tree)
            else {
                continue;
            };
            if induction_width != bound_width {
                continue;
            }
            let Some(start_width) =
                function.unsigned_int_width(start_value, ctx.target_layout().pointer_bits(), tree)
            else {
                continue;
            };
            if start_width != induction_width {
                continue;
            }

            // require the guard bound to be loop invariant
            if !value_is_loop_invariant(bound_value, lp, &definitions, &forwarding) {
                continue;
            }
            if !value_is_loop_invariant(start_value, lp, &definitions, &forwarding) {
                continue;
            }

            // attempt to replace the loop with memset
            if let Some(pattern) =
                match_memset_pattern(lp, guard.induction, function, tree, accesses, &definitions)
            {
                // require the store to sit in this loop itself
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

                // require a loop invariant array
                if !value_is_loop_invariant(pattern.array, lp, &definitions, &forwarding) {
                    continue;
                }

                // require a uint8 element type
                if !array_is_uint8(pattern.array, function, tree) {
                    continue;
                }
                let sources = [
                    tree.provenance(pattern.store),
                    tree.provenance(guard.instruction),
                ];

                // decide whether the copy needs a bounds guard
                let should_guard = should_guard_copy_bounds(
                    start_value,
                    bound_value,
                    induction_width,
                    preheader,
                    &ranges,
                    &definitions,
                    tree,
                );

                // materialize the byte length ahead of the intrinsic
                let mut instructions = if should_guard {
                    Vec::new()
                } else {
                    tree.get(preheader).instructions.clone()
                };
                let Some(length_value) = emit_copy_length(
                    bound_value,
                    start_value,
                    induction_width,
                    1,
                    should_guard,
                    &ranges,
                    preheader,
                    &definitions,
                    function,
                    tree,
                    &mut instructions,
                    &sources,
                    provenance,
                ) else {
                    continue;
                };

                // emit the memset
                emit_memset(
                    pattern.array,
                    pattern.element_addr_type,
                    start_value,
                    value_const,
                    length_value,
                    function,
                    tree,
                    &mut instructions,
                    &sources,
                    provenance,
                );

                // bypass the original loop body
                if should_guard {
                    let memory_block =
                        insert_memory_block(exit_block, instructions, tree, &sources, provenance);
                    function.add_block(memory_block, tree);
                    insert_bound_guard(
                        start_value,
                        bound_value,
                        function,
                        tree,
                        preheader,
                        memory_block,
                        exit_block,
                        &sources,
                        provenance,
                    );
                } else {
                    function.replace_block_instructions(preheader, instructions, tree, provenance);
                    let preheader_terminator = mir::Terminator::Jump {
                        target: mir::BlockTarget::new(exit_block, mir::ValueSlice::default()),
                    };
                    let terminator = tree.get(preheader).terminator;
                    tree.rewrite(terminator, preheader_terminator, provenance);
                }

                // record the rewrite and rescan from the top
                changed = true;
                changed_this_iteration = true;
                break 'scan_loops;
            }

            // attempt to replace the loop with memcpy or memmove
            let Some(pattern) = match_memcpy_pattern(
                lp,
                guard.induction,
                function,
                tree,
                accesses,
                &definitions,
                &uses,
            ) else {
                continue;
            };

            // require the store to sit in this loop itself
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
            if !value_is_loop_invariant(pattern.dest_array, lp, &definitions, &forwarding)
                || !value_is_loop_invariant(pattern.src_array, lp, &definitions, &forwarding)
            {
                continue;
            }

            // resolve the element type for both arrays
            let Some(dest_element) = array_element_type(pattern.dest_array, function, tree) else {
                continue;
            };
            let Some(src_element) = array_element_type(pattern.src_array, function, tree) else {
                continue;
            };

            if dest_element != src_element {
                continue;
            }

            // require a known element size
            let element = tree.ty(dest_element);
            let Some(element_size) = element.byte_size(tree, ctx.target_layout().pointer_bits())
            else {
                continue;
            };

            // require both arrays to live in the same space
            let Some(dest_space) = reference_space(pattern.dest_element_addr_type, tree) else {
                continue;
            };
            let Some(src_space) = reference_space(pattern.src_element_addr_type, tree) else {
                continue;
            };
            if dest_space != src_space {
                continue;
            }
            let sources = [
                tree.provenance(pattern.store),
                tree.provenance(pattern.load),
                tree.provenance(guard.instruction),
            ];

            // decide whether the copy needs a bounds guard
            let should_guard = should_guard_copy_bounds(
                start_value,
                bound_value,
                induction_width,
                preheader,
                &ranges,
                &definitions,
                tree,
            );

            // materialize the byte length ahead of the intrinsic
            let mut instructions = if should_guard {
                Vec::new()
            } else {
                tree.get(preheader).instructions.clone()
            };
            let Some(length_value) = emit_copy_length(
                bound_value,
                start_value,
                induction_width,
                element_size,
                should_guard,
                &ranges,
                preheader,
                &definitions,
                function,
                tree,
                &mut instructions,
                &sources,
                provenance,
            ) else {
                continue;
            };

            // copy when the ranges cannot overlap, otherwise move
            let use_memcpy =
                arrays_are_value_types(pattern.dest_array, pattern.src_array, function, tree)
                    || aliases.addresses_no_alias(pattern.store_pointer, pattern.load_pointer);
            let intrinsic = if use_memcpy {
                mir::Intrinsic::Memcpy
            } else {
                mir::Intrinsic::Memmove
            };

            // emit the intrinsic
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
                &mut instructions,
                &sources,
                provenance,
            );

            // bypass the original loop body
            if should_guard {
                let memory_block =
                    insert_memory_block(exit_block, instructions, tree, &sources, provenance);
                function.add_block(memory_block, tree);
                insert_bound_guard(
                    start_value,
                    bound_value,
                    function,
                    tree,
                    preheader,
                    memory_block,
                    exit_block,
                    &sources,
                    provenance,
                );
            } else {
                function.replace_block_instructions(preheader, instructions, tree, provenance);
                let preheader_terminator = mir::Terminator::Jump {
                    target: mir::BlockTarget::new(exit_block, mir::ValueSlice::default()),
                };
                let terminator = tree.get(preheader).terminator;
                tree.rewrite(terminator, preheader_terminator, provenance);
            }

            // record the rewrite and rescan from the top
            changed = true;
            changed_this_iteration = true;
            break 'scan_loops;
        }

        // stop once a full scan finds no further idiom
        if !changed_this_iteration {
            break;
        }
    }

    changed
}

/// One loop body recognized as a memset.
#[derive(Debug, Clone)]
struct MemsetPattern {
    /// The store replaced by the intrinsic.
    store: mir::LocalNodeId<mir::Instruction>,
    /// The array being filled.
    array: mir::Value,
    /// The element address result type.
    element_addr_type: mir::TypeId,
    /// The constant value written by the store, when available.
    value_constant: Option<mir::Constant>,
    /// The block containing the store.
    store_block: mir::LocalNodeId<mir::Block>,
}

/// One loop body recognized as a memcpy or memmove.
#[derive(Debug, Clone)]
struct MemcpyPattern {
    /// The store replaced by the intrinsic.
    store: mir::LocalNodeId<mir::Instruction>,
    /// The load replaced by the intrinsic.
    load: mir::LocalNodeId<mir::Instruction>,
    /// Destination array value.
    dest_array: mir::Value,
    /// Source array value.
    src_array: mir::Value,
    /// The destination element address result type.
    dest_element_addr_type: mir::TypeId,
    /// The source element address result type.
    src_element_addr_type: mir::TypeId,
    /// The block containing the store.
    store_block: mir::LocalNodeId<mir::Block>,
    /// The block containing the load.
    load_block: mir::LocalNodeId<mir::Block>,
    /// The store reference value.
    store_pointer: mir::Value,
    /// The load reference value.
    load_pointer: mir::Value,
}

/// Match a loop body against a memset idiom.
fn match_memset_pattern(
    lp: &mir::Loop,
    induction: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    definitions: &DefinitionTable,
) -> Option<MemsetPattern> {
    // scan loop blocks for a single store with a speculatable body
    let mut store_reference = None;
    let mut store_value = None;
    let mut store_block = None;
    let mut store_instruction = None;

    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // require simple loop terminators
        match terminator {
            mir::Terminator::Jump { .. } | mir::Terminator::Branch { .. } => {}
            _ => return None,
        }

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            // reject volatile or ordered memory accesses
            if accesses.requires_exact_position(inst_id, tree) {
                return None;
            }

            if instruction_has_side_effects(inst) {
                if let mir::Instruction::Store { pointer, value } = inst {
                    if store_reference.is_some() {
                        return None;
                    }
                    store_reference = Some(*pointer);
                    store_value = Some(*value);
                    store_block = Some(block_id);
                    store_instruction = Some(inst_id);
                    continue;
                }

                return None;
            }

            if !instruction_is_speculatable(inst, function, tree)
                && !instruction_is_borrow_address(inst)
            {
                return None;
            }
        }
    }

    // require a single store and resolve its address
    let store_reference = store_reference?;
    let (array, index, element_addr_type) =
        element_addr_for_pointer(store_reference, tree, definitions)?;
    if index != induction {
        return None;
    }

    // extract a constant fill value when possible
    let value = store_value?;
    let value_const = constant_for_value(value, definitions, tree);
    Some(MemsetPattern {
        store: store_instruction?,
        array,
        element_addr_type,
        value_constant: value_const,
        store_block: store_block?,
    })
}

/// Match a loop body against a memcpy or memmove idiom.
fn match_memcpy_pattern(
    lp: &mir::Loop,
    induction: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    definitions: &DefinitionTable,
    uses: &UseTable,
) -> Option<MemcpyPattern> {
    // scan loop blocks for a single load and store
    let mut store_reference = None;
    let mut store_value = None;
    let mut store_block = None;
    let mut store_instruction = None;
    let mut load_reference = None;
    let mut load_value = None;
    let mut load_block = None;
    let mut load_instruction = None;

    for &block_id in &lp.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // require simple loop terminators
        match terminator {
            mir::Terminator::Jump { .. } | mir::Terminator::Branch { .. } => {}
            _ => return None,
        }

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            // reject volatile or ordered memory accesses
            if accesses.requires_exact_position(inst_id, tree) {
                return None;
            }

            match inst {
                mir::Instruction::Store { pointer, value } => {
                    if store_reference.is_some() {
                        return None;
                    }
                    store_reference = Some(*pointer);
                    store_value = Some(*value);
                    store_block = Some(block_id);
                    store_instruction = Some(inst_id);
                    continue;
                }
                mir::Instruction::Load {
                    destination,
                    pointer,
                    ..
                } => {
                    if load_reference.is_some() {
                        return None;
                    }
                    load_reference = Some(*pointer);
                    load_value = Some(*destination);
                    load_block = Some(block_id);
                    load_instruction = Some(inst_id);
                    continue;
                }
                _ => {}
            }

            if instruction_has_side_effects(inst) {
                return None;
            }
            if !instruction_is_speculatable(inst, function, tree)
                && !instruction_is_borrow_address(inst)
            {
                return None;
            }
        }
    }

    // require a single load and store
    let store_reference = store_reference?;
    let store_value = store_value?;
    let load_reference = load_reference?;
    let load_value = load_value?;

    // require the load value to feed the store
    if store_value != load_value {
        return None;
    }

    // require the load value to be used only by the store
    if uses.count(load_value) != 1 {
        return None;
    }

    // resolve both addresses back to element.address
    let (dest_array, dest_index, dest_element_addr_type) =
        element_addr_for_pointer(store_reference, tree, definitions)?;
    let (src_array, src_index, src_element_addr_type) =
        element_addr_for_pointer(load_reference, tree, definitions)?;

    if dest_index != induction || src_index != induction {
        return None;
    }

    Some(MemcpyPattern {
        store: store_instruction?,
        load: load_instruction?,
        dest_array,
        src_array,
        dest_element_addr_type,
        src_element_addr_type,
        store_block: store_block?,
        load_block: load_block?,
        store_pointer: store_reference,
        load_pointer: load_reference,
    })
}

/// Find an element address instruction for the given pointer.
fn element_addr_for_pointer(
    pointer: mir::Value,
    tree: &mir::Tree,
    definitions: &DefinitionTable,
) -> Option<(mir::Value, mir::Value, mir::TypeId)> {
    // find the instruction that defines the pointer
    let inst_id = definitions.instruction(pointer)?;

    // require a direct element address computation
    match tree.get(inst_id) {
        mir::Instruction::ElementAddr {
            base,
            index,
            result_type,
            ..
        } => Some((*base, *index, *result_type)),
        _ => None,
    }
}

/// Check whether the array element type is uint8.
fn array_is_uint8(array: mir::Value, function: &mir::Function, tree: &mir::Tree) -> bool {
    // resolve the array element type
    let Some(element) = array_element_type(array, function, tree) else {
        return false;
    };
    let element_ty = tree.ty(element);
    matches!(
        element_ty,
        mir::Type::Int {
            width: 8,
            is_signed: false
        }
    )
}

/// Return the element type for an array or array reference value.
fn array_element_type(
    array: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
) -> Option<mir::TypeId> {
    // resolve the fixed array type
    let ty_id = function.expect_value_type(array);
    let ty = tree.ty(ty_id);

    match ty {
        mir::Type::FixedArray { element, .. } => Some(*element),
        mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
            match tree.ty(*pointee) {
                mir::Type::FixedArray { element, .. } => Some(*element),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Return true when both arrays are value types.
fn arrays_are_value_types(
    dest_array: mir::Value,
    src_array: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    if dest_array == src_array {
        return false;
    }

    let dest_ty = function.expect_value_type(dest_array);
    let src_ty = function.expect_value_type(src_array);

    matches!(tree.ty(dest_ty), mir::Type::FixedArray { .. })
        && matches!(tree.ty(src_ty), mir::Type::FixedArray { .. })
}

/// Return the space for a reference type.
fn reference_space(ty_id: mir::TypeId, tree: &mir::Tree) -> Option<mir::Space> {
    match tree.ty(ty_id) {
        mir::Type::Reference { storage, .. } => storage.heap_space(),
        _ => None,
    }
}

/// Decide whether a bounds guard is needed before emitting a copy intrinsic.
fn should_guard_copy_bounds(
    start: mir::Value,
    bound: mir::Value,
    bound_width: u16,
    preheader: mir::LocalNodeId<mir::Block>,
    ranges: &RangeTable,
    definitions: &DefinitionTable,
    tree: &mir::Tree,
) -> bool {
    // skip guards when the start is definitely zero
    let start_const = constant_for_value(start, definitions, tree);
    if constant_is_zero(start_const.as_ref()) {
        return false;
    }

    // require ranges that prove start is always below the bound
    let Some((_start_min, start_max)) =
        unsigned_bounds_for_value(start, bound_width, ranges, preheader, definitions, tree)
    else {
        return true;
    };
    let Some((bound_min, _bound_max)) =
        unsigned_bounds_for_value(bound, bound_width, ranges, preheader, definitions, tree)
    else {
        return true;
    };

    // guard when start could exceed the bound
    start_max > bound_min
}

/// Insert the memory intrinsic block.
fn insert_memory_block(
    exit: mir::LocalNodeId<mir::Block>,
    instructions: Vec<mir::LocalNodeId<mir::Instruction>>,
    tree: &mut mir::Tree,
    sources: &[ProvenanceId],
    provenance: &mut ProvenanceJournal<'_>,
) -> mir::LocalNodeId<mir::Block> {
    let [terminator_provenance, block_provenance] = provenance.generate_many(sources);
    let terminator = tree.insert(
        mir::Terminator::Jump {
            target: mir::BlockTarget::new(exit, mir::ValueSlice::default()),
        },
        terminator_provenance,
    );

    let mut block = mir::Block::new(terminator);
    block.instructions = instructions;

    tree.insert(block, block_provenance)
}

/// Insert a bounds guard branching to the fast path or exit.
fn insert_bound_guard(
    start: mir::Value,
    bound: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    preheader: mir::LocalNodeId<mir::Block>,
    success_target: mir::LocalNodeId<mir::Block>,
    failure_target: mir::LocalNodeId<mir::Block>,
    sources: &[ProvenanceId],
    provenance: &mut ProvenanceJournal<'_>,
) {
    // emit the bound comparison
    let bool_type = tree.boolean_type();
    let guard_value = function.next_typed_value(bool_type);
    let guard_provenance = provenance.generate(sources);
    let guard_inst = tree.insert(
        mir::Instruction::Binary {
            destination: guard_value,
            operator: mir::BinaryOperator::LessEqual,
            left: start,
            right: bound,
        },
        guard_provenance,
    );
    let mut instructions = tree.get(preheader).instructions.clone();
    instructions.push(guard_inst);
    function.replace_block_instructions(preheader, instructions, tree, provenance);

    // branch on the guard to the fast path or exit
    let guard_terminator = mir::Terminator::Branch {
        condition: guard_value,
        then_target: mir::BlockTarget::new(success_target, mir::ValueSlice::default()),
        else_target: mir::BlockTarget::new(failure_target, mir::ValueSlice::default()),
    };
    let terminator = tree.get(preheader).terminator;
    let inputs = [tree.provenance(terminator), guard_provenance];
    tree.replace(terminator, guard_terminator, provenance.generate(&inputs));
}

/// Emit an intrinsic.memory.raw.setBytes for the loop idiom.
fn emit_memset(
    array: mir::Value,
    element_addr_type: mir::TypeId,
    start: mir::Value,
    fill_value: mir::Constant,
    length: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    sources: &[ProvenanceId],
    provenance: &mut ProvenanceJournal<'_>,
) {
    // materialize the fill constant
    let element_type = match tree.ty(element_addr_type) {
        mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => *pointee,
        _ => unreachable!("element address must produce a reference or pointer"),
    };
    let fill = function.next_typed_value(element_type);
    let fill_inst = tree.insert(
        mir::Instruction::Const {
            destination: fill,
            value: fill_value,
        },
        provenance.fuse(sources),
    );
    instructions.push(fill_inst);

    // compute the base reference for the memset
    let pointer = function.next_typed_value(element_addr_type);
    let ptr_inst = tree.insert(
        mir::Instruction::ElementAddr {
            destination: pointer,
            base: array,
            index: start,
            result_type: element_addr_type,
        },
        provenance.fuse(sources),
    );
    instructions.push(ptr_inst);

    // emit the memset call
    let args = tree.add_values(&[pointer, fill, length]);
    let mem_inst = tree.insert(
        mir::Instruction::Intrinsic {
            destination: None,
            intrinsic: mir::Intrinsic::Memset,
            arguments: args,
        },
        provenance.fuse(sources),
    );
    instructions.push(mem_inst);
}

/// Emit memcpy or memmove for the loop idiom.
fn emit_memcpy_or_memmove(
    intrinsic: mir::Intrinsic,
    dest_array: mir::Value,
    src_array: mir::Value,
    dest_element_addr_type: mir::TypeId,
    src_element_addr_type: mir::TypeId,
    start: mir::Value,
    length: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    sources: &[ProvenanceId],
    provenance: &mut ProvenanceJournal<'_>,
) {
    // compute the destination base reference
    let destination_pointer = function.next_typed_value(dest_element_addr_type);
    let dest_ptr_inst = tree.insert(
        mir::Instruction::ElementAddr {
            destination: destination_pointer,
            base: dest_array,
            index: start,
            result_type: dest_element_addr_type,
        },
        provenance.fuse(sources),
    );
    instructions.push(dest_ptr_inst);

    // compute the source base reference
    let source_pointer = function.next_typed_value(src_element_addr_type);
    let src_ptr_inst = tree.insert(
        mir::Instruction::ElementAddr {
            destination: source_pointer,
            base: src_array,
            index: start,
            result_type: src_element_addr_type,
        },
        provenance.fuse(sources),
    );
    instructions.push(src_ptr_inst);

    // emit the intrinsic call
    let args = tree.add_values(&[destination_pointer, source_pointer, length]);
    let mem_inst = tree.insert(
        mir::Instruction::Intrinsic {
            destination: None,
            intrinsic,
            arguments: args,
        },
        provenance.fuse(sources),
    );
    instructions.push(mem_inst);
}

/// Return unsigned integer bounds for a value when available.
fn unsigned_bounds_for_value(
    value: mir::Value,
    bound_width: u16,
    ranges: &RangeTable,
    preheader: mir::LocalNodeId<mir::Block>,
    definitions: &DefinitionTable,
    tree: &mir::Tree,
) -> Option<(i128, i128)> {
    // use range information when available
    if let Some(ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    }) = ranges.exit(preheader).get(value)
    {
        if !*is_signed && *width == bound_width && *min >= 0 {
            return Some((*min, *max));
        }

        return None;
    }

    // fall back to a constant value range
    let constant = constant_for_value(value, definitions, tree)?;
    match constant {
        mir::Constant::UInt { value, width } => {
            if width == bound_width {
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
            if width != bound_width || value < 0 {
                return None;
            }

            Some((value, value))
        }
        _ => None,
    }
}

/// Emit a byte length value for memset, memcpy, or memmove.
fn emit_copy_length(
    bound: mir::Value,
    start: mir::Value,
    bound_width: u16,
    element_size: u64,
    guarded: bool,
    ranges: &RangeTable,
    preheader: mir::LocalNodeId<mir::Block>,
    definitions: &DefinitionTable,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    sources: &[ProvenanceId],
    provenance: &mut ProvenanceJournal<'_>,
) -> Option<mir::Value> {
    // skip length materialization when byte sized with zero start
    let start_const = constant_for_value(start, definitions, tree);
    let mut start_is_zero = constant_is_zero(start_const.as_ref());
    if element_size == 1 && start_is_zero {
        return Some(bound);
    }

    // collect range bounds when needed for safety
    let mut bounds = None;
    if element_size > 1 || !guarded {
        let bound_range =
            unsigned_bounds_for_value(bound, bound_width, ranges, preheader, definitions, tree)?;
        let start_range =
            unsigned_bounds_for_value(start, bound_width, ranges, preheader, definitions, tree)?;
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
        let destination = function.next_typed_value_like(bound);
        let subtract_inst = tree.insert(
            mir::Instruction::Binary {
                destination,
                operator: mir::BinaryOperator::Subtract,
                left: bound,
                right: start,
            },
            provenance.fuse(sources),
        );
        instructions.push(subtract_inst);
        destination
    };

    // return the base length directly for byte sized elements
    if element_size == 1 {
        return Some(length_base);
    }

    // materialize the element size constant
    let size_const = mir::Constant::UInt {
        value: u128::from(element_size),
        width: bound_width,
    };
    let size_value = function.next_typed_value_like(bound);
    let size_inst = tree.insert(
        mir::Instruction::Const {
            destination: size_value,
            value: size_const,
        },
        provenance.fuse(sources),
    );
    instructions.push(size_inst);

    // multiply by element size
    let length_value = function.next_typed_value_like(bound);
    let length_inst = tree.insert(
        mir::Instruction::Binary {
            destination: length_value,
            operator: mir::BinaryOperator::Multiply,
            left: length_base,
            right: size_value,
        },
        provenance.fuse(sources),
    );
    instructions.push(length_inst);

    Some(length_value)
}

/// Check whether a value is loop invariant.
fn value_is_loop_invariant(
    value: mir::Value,
    lp: &mir::Loop,
    definitions: &DefinitionTable,
    forwarding: &BlockParamForwarding,
) -> bool {
    // resolve forwarded parameters
    let value = forwarding.resolve(value);

    // treat values without a definition block as invariant
    let Some(def_block) = definitions.block(value) else {
        return true;
    };

    // treat definitions outside the loop as invariant
    !lp.blocks.contains(&def_block)
}

/// Find the loop preheader and its header arguments.
fn find_preheader(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &FxIndexSet<mir::LocalNodeId<mir::Block>>,
    cfg: &ControlTable,
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

/// Map a header parameter to its preheader argument, if applicable.
fn preheader_value_for_param(
    value: mir::Value,
    header: mir::LocalNodeId<mir::Block>,
    preheader_args: &[mir::Value],
    tree: &mir::Tree,
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
    tree: &mir::Tree,
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
    loop_blocks: &FxIndexSet<mir::LocalNodeId<mir::Block>>,
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &DefinitionTable,
) -> Option<LoopGuard> {
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
    if !loop_blocks.contains(&then_target.block) {
        return None;
    }

    // locate the guard instruction that produces the condition
    let instruction = definitions.instruction(*condition)?;
    let inst = tree.get(instruction);
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
    let operand_type = function.expect_value_type(*left);
    let is_unsigned = tree.ty(operand_type).integer_signedness() == Some(false);
    if *operator != mir::BinaryOperator::LessThan || !is_unsigned {
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

    Some(LoopGuard {
        instruction,
        induction: *left,
        bound: *right,
    })
}

/// Check whether the guard describes a simple induction pattern.
fn guard_is_simple(guard: &LoopGuard, loop_index: usize, scev: &EvolutionTable) -> bool {
    // require a simple add recurrence for the induction variable
    let Some(Scev::AddRec {
        start,
        step,
        loop_header,
    }) = scev.value_scev(loop_index, guard.induction)
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

    /// Memset loops are lowered to intrinsic.memory.raw.setBytes.
    #[test]
    fn test_recognize_loop_idioms_memset() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    v9: uint8 = 0
    v10: ref<uint8, borrowed, mutable> = element.address v0, v2
    intrinsic.memory.raw.setBytes(v10, v9, v1)
    jump b3

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Memset loops with a separate latch are lowered.
    #[test]
    fn test_recognize_loop_idioms_memset_multi_block() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b4

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    jump b3(v4)

b3(v8: uint32):
    v9: uint32 = add v8, v3
    jump b1(v9)

b4:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    v10: uint8 = 0
    v11: ref<uint8, borrowed, mutable> = element.address v0, v2
    intrinsic.memory.raw.setBytes(v11, v10, v1)
    jump b4

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b4

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    jump b3(v4)

b3(v8: uint32):
    v9: uint32 = add v8, v3
    jump b1(v9)

b4:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Volatile stores keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_volatile_store() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let function = test.optimized.tree.get(function_id);

        let mut store_id = None;
        let mut store_reference = None;
        for block_id in function.blocks() {
            let block = test.optimized.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Store { pointer, .. } =
                    test.optimized.tree.get(*instruction_id)
                {
                    store_id = Some(*instruction_id);
                    store_reference = Some(*pointer);
                    break;
                }
            }
            if store_id.is_some() {
                break;
            }
        }

        let store_id = store_id.expect("missing store instruction");
        let store_reference = store_reference.expect("missing store pointer");
        test.insert_pointer_access_with_options(
            store_id,
            mir::MemoryOperation::Write,
            store_reference,
            None,
            true,
            None,
        );

        test.run_pass(&RecognizeLoopIdioms);
        test.assert_unchanged(input);
    }

    /// Memcpy loops are lowered to intrinsic.memory.raw.copyBytes.
    #[test]
    fn test_recognize_loop_idioms_memcpy() {
        let input = r#"
function test(v0: [uint8; 8], v1: [uint8; 8], v2: uint32): void {
entry(v0: [uint8; 8], v1: [uint8; 8], v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v2
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    v8: ref<uint8, borrowed, mutable> = element.address v1, v5
    v9: uint8 = load v8
    store v7, v9
    v10: uint32 = add v5, v4
    jump b1(v10)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: [uint8; 8], v2: uint32): void {
entry(v0: [uint8; 8], v1: [uint8; 8], v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    v11: ref<uint8, borrowed, mutable> = element.address v0, v3
    v12: ref<uint8, borrowed, mutable> = element.address v1, v3
    intrinsic.memory.raw.copyBytes(v11, v12, v2)
    jump b3

b1(v5: uint32):
    v6: boolean = lt v5, v2
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    v8: ref<uint8, borrowed, mutable> = element.address v1, v5
    v9: uint8 = load v8
    store v7, v9
    v10: uint32 = add v5, v4
    jump b1(v10)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Overlapping copy loops use memmove.
    #[test]
    fn test_recognize_loop_idioms_memmove_aliasing() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = load v6
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    v9: ref<uint8, borrowed, mutable> = element.address v0, v2
    v10: ref<uint8, borrowed, mutable> = element.address v0, v2
    intrinsic.memory.raw.moveBytes(v9, v10, v1)
    jump b3

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = load v6
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Memcpy uses byte length when element size exceeds one byte.
    #[test]
    fn test_recognize_loop_idioms_memcpy_multiplies_length() {
        let input = r#"
function test(v0: [uint32; 8], v1: [uint32; 8]): void {
entry(v0: [uint32; 8], v1: [uint32; 8]):
    v2: uint32 = 0
    v3: uint32 = 1
    v4: uint32 = 4
    jump b1(v2)

b1(v5: uint32):
    v6: boolean = lt v5, v4
    branch v6 => b2 | b3

b2:
    v7: ref<uint32, borrowed, mutable> = element.address v0, v5
    v8: ref<uint32, borrowed, mutable> = element.address v1, v5
    v9: uint32 = load v8
    store v7, v9
    v10: uint32 = add v5, v3
    jump b1(v10)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint32; 8], v1: [uint32; 8]): void {
entry(v0: [uint32; 8], v1: [uint32; 8]):
    v2: uint32 = 0
    v3: uint32 = 1
    v4: uint32 = 4
    v11: uint32 = 4
    v12: uint32 = mul v4, v11
    v13: ref<uint32, borrowed, mutable> = element.address v0, v2
    v14: ref<uint32, borrowed, mutable> = element.address v1, v2
    intrinsic.memory.raw.copyBytes(v13, v14, v12)
    jump b3

b1(v5: uint32):
    v6: boolean = lt v5, v4
    branch v6 => b2 | b3

b2:
    v7: ref<uint32, borrowed, mutable> = element.address v0, v5
    v8: ref<uint32, borrowed, mutable> = element.address v1, v5
    v9: uint32 = load v8
    store v7, v9
    v10: uint32 = add v5, v3
    jump b1(v10)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Non zero starts scale byte length for wider elements.
    #[test]
    fn test_recognize_loop_idioms_memcpy_non_zero_start_multiplies_length() {
        let input = r#"
function test(v0: [uint32; 8], v1: [uint32; 8]): void {
entry(v0: [uint32; 8], v1: [uint32; 8]):
    v2: uint32 = 2
    v3: uint32 = 1
    v4: uint32 = 8
    jump b1(v2)

b1(v5: uint32):
    v6: boolean = lt v5, v4
    branch v6 => b2 | b3

b2:
    v7: ref<uint32, borrowed, mutable> = element.address v0, v5
    v8: ref<uint32, borrowed, mutable> = element.address v1, v5
    v9: uint32 = load v8
    store v7, v9
    v10: uint32 = add v5, v3
    jump b1(v10)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint32; 8], v1: [uint32; 8]): void {
entry(v0: [uint32; 8], v1: [uint32; 8]):
    v2: uint32 = 2
    v3: uint32 = 1
    v4: uint32 = 8
    v11: uint32 = sub v4, v2
    v12: uint32 = 4
    v13: uint32 = mul v11, v12
    v14: ref<uint32, borrowed, mutable> = element.address v0, v2
    v15: ref<uint32, borrowed, mutable> = element.address v1, v2
    intrinsic.memory.raw.copyBytes(v14, v15, v13)
    jump b3

b1(v5: uint32):
    v6: boolean = lt v5, v4
    branch v6 => b2 | b3

b2:
    v7: ref<uint32, borrowed, mutable> = element.address v0, v5
    v8: ref<uint32, borrowed, mutable> = element.address v1, v5
    v9: uint32 = load v8
    store v7, v9
    v10: uint32 = add v5, v3
    jump b1(v10)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Loops with non zero starts insert a guard.
    #[test]
    fn test_recognize_loop_idioms_guards_non_zero_start() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 1
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 1
    v3: uint32 = 1
    v12: boolean = le v2, v1
    branch v12 => b4 | b3

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return

b4:
    v9: uint32 = sub v1, v2
    v10: uint8 = 0
    v11: ref<uint8, borrowed, mutable> = element.address v0, v2
    intrinsic.memory.raw.setBytes(v11, v10, v9)
    jump b3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Non zero starts guard memcpy lowering.
    #[test]
    fn test_recognize_loop_idioms_memcpy_guards_non_zero_start() {
        let input = r#"
function test(v0: [uint8; 8], v1: [uint8; 8], v2: uint32, v3: uint32): void {
entry(v0: [uint8; 8], v1: [uint8; 8], v2: uint32, v3: uint32):
    v4: uint32 = 1
    jump b1(v2)

b1(v5: uint32):
    v6: boolean = lt v5, v3
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    v8: ref<uint8, borrowed, mutable> = element.address v1, v5
    v9: uint8 = load v8
    store v7, v9
    v10: uint32 = add v5, v4
    jump b1(v10)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: [uint8; 8], v2: uint32, v3: uint32): void {
entry(v0: [uint8; 8], v1: [uint8; 8], v2: uint32, v3: uint32):
    v4: uint32 = 1
    v14: boolean = le v2, v3
    branch v14 => b4 | b3

b1(v5: uint32):
    v6: boolean = lt v5, v3
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    v8: ref<uint8, borrowed, mutable> = element.address v1, v5
    v9: uint8 = load v8
    store v7, v9
    v10: uint32 = add v5, v4
    jump b1(v10)

b3:
    return

b4:
    v11: uint32 = sub v3, v2
    v12: ref<uint8, borrowed, mutable> = element.address v0, v2
    v13: ref<uint8, borrowed, mutable> = element.address v1, v2
    intrinsic.memory.raw.copyBytes(v12, v13, v11)
    jump b3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Non zero starts guard memmove lowering.
    #[test]
    fn test_recognize_loop_idioms_memmove_guards_non_zero_start() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = load v6
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: [uint8; 8], v1: uint32, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint32, v2: uint32):
    v3: uint32 = 1
    v12: boolean = le v2, v1
    branch v12 => b4 | b3

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = load v6
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return

b4:
    v9: uint32 = sub v1, v2
    v10: ref<uint8, borrowed, mutable> = element.address v0, v2
    v11: ref<uint8, borrowed, mutable> = element.address v0, v2
    intrinsic.memory.raw.moveBytes(v10, v11, v9)
    jump b3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(expected);
    }

    /// Loops with a non unit stride keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_non_unit_stride() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 2
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with a conditional store keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_conditional_store() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32, v2: boolean): void {
entry(v0: [uint8; 8], v1: uint32, v2: boolean):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v1
    branch v6 => b2(v5) | b5

b2(v7: uint32):
    branch v2 => b3(v7) | b4(v7)

b3(v8: uint32):
    v9: ref<uint8, borrowed, mutable> = element.address v0, v8
    v10: uint8 = 0
    store v9, v10
    jump b4(v8)

b4(v11: uint32):
    v12: uint32 = add v11, v4
    jump b1(v12)

b5:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with a nested store keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_nested_store() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    v4: uint32 = 2
    jump b1(v2)

b1(v5: uint32):
    v6: boolean = lt v5, v1
    branch v6 => b2 | b6

b2:
    v7: uint32 = 0
    jump b3(v7)

b3(v8: uint32):
    v9: ref<uint8, borrowed, mutable> = element.address v0, v5
    v10: uint8 = 0
    store v9, v10
    v11: boolean = lt v8, v4
    branch v11 => b4(v8) | b5

b4(v12: uint32):
    v13: uint32 = add v12, v3
    jump b3(v13)

b5:
    v14: uint32 = add v5, v3
    jump b1(v14)

b6:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with a varying array keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_variant_array() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2, v0)

b1(v4: uint32, v5: [uint8; 8]):
    v6: boolean = lt v4, v1
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v5, v4
    v8: uint8 = 0
    store v7, v8
    v9: [uint8; 8] = field.set v5, 0, v8
    v10: uint32 = add v4, v3
    jump b1(v10, v9)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with a non constant store keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_non_constant_store() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint8, v2: uint32): void {
entry(v0: [uint8; 8], v1: uint8, v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v2
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    store v7, v1
    v8: uint32 = add v5, v4
    jump b1(v8)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with side effects keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_side_effects() {
        let input = r#"
function test(v0: [uint8; 8], v1: uint32): void {
entry(v0: [uint8; 8], v1: uint32):
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v1
    branch v5 => b2 | b3

b2:
    call touch(v4): (uint32) => void
    v6: ref<uint8, borrowed, mutable> = element.address v0, v4
    v7: uint8 = 0
    store v6, v7
    v8: uint32 = add v4, v3
    jump b1(v8)

b3:
    return
}

function touch(v0: uint32): void {
entry(v0: uint32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }

    /// Loops with multiple stores keep the original loop.
    #[test]
    fn test_recognize_loop_idioms_skips_multiple_stores() {
        let input = r#"
function test(v0: [uint8; 8], v1: [uint8; 8], v2: uint32): void {
entry(v0: [uint8; 8], v1: [uint8; 8], v2: uint32):
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v2
    branch v6 => b2 | b3

b2:
    v7: ref<uint8, borrowed, mutable> = element.address v0, v5
    v8: uint8 = 0
    store v7, v8
    v9: ref<uint8, borrowed, mutable> = element.address v1, v5
    store v9, v8
    v10: uint32 = add v5, v4
    jump b1(v10)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&RecognizeLoopIdioms);
        test.assert_output(input);
    }
}
