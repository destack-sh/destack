use std::collections::HashMap;

use destack_mir as mir;

use crate::ThreadedHandler;
use crate::memory::{ReferenceMeta, Value};

use super::dispatch;
use super::threaded::{
    ArgumentRange, CopyPair, CopyRange, INVALID_FUNCTION_INDEX, INVALID_VALUE_ID, SwitchCase,
    SwitchRange, ThreadedBlock, ThreadedFunction, ThreadedInstruction, ThreadedInstructionData,
    UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT, UNKNOWN_SLOT_COUNT, pack_optional_value,
};

/// Storage class for pointer-like values.
#[derive(Clone, Copy, Debug)]
enum PointerStorage {
    /// Managed heap reference.
    Managed,
    /// Raw heap pointer.
    Raw,
    /// Stack pointer.
    Stack,
    /// Global pointer.
    Global,
    /// Unknown pointer storage.
    Unknown,
}

/// Scalar and aggregate kinds used for typed dispatch selection.
#[derive(Clone, Copy, Debug)]
enum ValueKind {
    /// Void value.
    Void,
    /// Boolean value.
    Bool,
    /// Signed or unsigned integer with width.
    Int { width: u8, signed: bool },
    /// Floating point value with width.
    Float { width: u8 },
    /// Unicode character value.
    Char,
    /// Pointer-like value with pointee type.
    Pointer {
        pointee: mir::LocalNodeId<mir::Type>,
        storage: PointerStorage,
        reference: ReferenceMeta,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: mir::LocalNodeId<mir::Type> },
    /// Heap aggregate value with concrete type.
    Aggregate { ty: mir::LocalNodeId<mir::Type> },
    /// Managed array value with element type.
    Array {
        element: mir::LocalNodeId<mir::Type>,
        length: u64,
    },
    /// Unknown or unsupported type.
    Unknown,
}

/// Table mapping SSA value ids to their inferred kind.
struct ValueKinds {
    /// Kind for each SSA value id.
    kinds: Vec<Option<ValueKind>>,
}

impl ValueKinds {
    /// Create a new value kind table.
    fn new(value_count: usize) -> Self {
        Self {
            kinds: vec![None; value_count],
        }
    }

    /// Get the kind for a value.
    fn get(&self, value: mir::Value) -> Option<ValueKind> {
        self.kinds.get(value.0 as usize).and_then(|kind| *kind)
    }

    /// Set the kind for a value.
    fn set(&mut self, value: mir::Value, kind: ValueKind) {
        if let Some(slot) = self.kinds.get_mut(value.0 as usize) {
            *slot = Some(kind);
        }
    }
}

/// Pick a binary handler based on inferred operand kind.
fn select_binary_handler(
    value_kinds: &ValueKinds,
    left: mir::Value,
    operator: mir::BinaryOperator,
) -> ThreadedHandler {
    use mir::BinaryOperator::*;

    // resolve operand kind
    let kind = value_kinds.get(left);

    // try specialized integer handlers first (no operator dispatch overhead)
    if let Some(ValueKind::Int { signed, .. }) = kind
        && let Some(handler) = select_specialized_int_handler(operator, signed)
    {
        return handler;
    }

    // fall back to typed handlers
    match kind {
        Some(ValueKind::Int { signed: true, .. }) => dispatch::handle_binary_int,
        Some(ValueKind::Int { signed: false, .. }) => dispatch::handle_binary_uint,
        Some(ValueKind::Float { width: 32 }) => dispatch::handle_binary_float32,
        Some(ValueKind::Float { width: 64 }) => dispatch::handle_binary_float64,
        Some(ValueKind::Bool) if matches!(operator, And | Or | Xor) => dispatch::handle_binary_bool,
        _ => dispatch::handle_binary,
    }
}

/// Select a fully specialized integer handler if available.
fn select_specialized_int_handler(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<ThreadedHandler> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            // signed arithmetic
            Add => dispatch::handle_add_int,
            Subtract => dispatch::handle_sub_int,
            Multiply => dispatch::handle_mul_int,
            // signed bitwise
            And => dispatch::handle_and_int,
            Or => dispatch::handle_or_int,
            Xor => dispatch::handle_xor_int,
            ShiftLeft => dispatch::handle_shl_int,
            ArithmeticShiftRight => dispatch::handle_shr_int,
            // signed comparisons
            Equal => dispatch::handle_eq_int,
            NotEqual => dispatch::handle_ne_int,
            SignedLessThan => dispatch::handle_lt_int,
            SignedLessEqual => dispatch::handle_le_int,
            SignedGreaterThan => dispatch::handle_gt_int,
            SignedGreaterEqual => dispatch::handle_ge_int,
            _ => return None,
        }
    } else {
        match operator {
            // unsigned arithmetic
            Add => dispatch::handle_add_uint,
            Subtract => dispatch::handle_sub_uint,
            Multiply => dispatch::handle_mul_uint,
            // unsigned bitwise
            And => dispatch::handle_and_uint,
            Or => dispatch::handle_or_uint,
            Xor => dispatch::handle_xor_uint,
            ShiftLeft => dispatch::handle_shl_uint,
            LogicalShiftRight => dispatch::handle_shr_uint,
            // unsigned comparisons (eq/ne produce bool, not int/uint)
            Equal => dispatch::handle_eq_int,
            NotEqual => dispatch::handle_ne_int,
            UnsignedLessThan => dispatch::handle_lt_uint,
            UnsignedLessEqual => dispatch::handle_le_uint,
            UnsignedGreaterThan => dispatch::handle_gt_uint,
            UnsignedGreaterEqual => dispatch::handle_ge_uint,
            _ => return None,
        }
    })
}

/// Select a specialized const-right handler if available.
fn select_specialized_const_int_handler(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<ThreadedHandler> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            Add => dispatch::handle_add_const_int,
            Subtract => dispatch::handle_sub_const_int,
            Multiply => dispatch::handle_mul_const_int,
            Equal => dispatch::handle_eq_const_int,
            NotEqual => dispatch::handle_ne_const_int,
            SignedLessThan => dispatch::handle_lt_const_int,
            SignedLessEqual => dispatch::handle_le_const_int,
            SignedGreaterThan => dispatch::handle_gt_const_int,
            SignedGreaterEqual => dispatch::handle_ge_const_int,
            _ => return None,
        }
    } else {
        match operator {
            Add => dispatch::handle_add_const_uint,
            Subtract => dispatch::handle_sub_const_uint,
            Multiply => dispatch::handle_mul_const_uint,
            // eq/ne produce bool, can use signed version
            Equal => dispatch::handle_eq_const_int,
            NotEqual => dispatch::handle_ne_const_int,
            UnsignedLessThan => dispatch::handle_lt_const_uint,
            UnsignedLessEqual => dispatch::handle_le_const_uint,
            UnsignedGreaterThan => dispatch::handle_gt_const_uint,
            UnsignedGreaterEqual => dispatch::handle_ge_const_uint,
            _ => return None,
        }
    })
}

/// Pick a unary handler based on inferred operand kind.
fn select_unary_handler(
    value_kinds: &ValueKinds,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> ThreadedHandler {
    // resolve operand kind
    let kind = value_kinds.get(argument);

    // select handler by kind
    match (kind, operator) {
        (Some(ValueKind::Int { signed: true, .. }), _) => dispatch::handle_unary_int,
        (Some(ValueKind::Int { signed: false, .. }), _) => dispatch::handle_unary_uint,
        (Some(ValueKind::Float { width: 32 }), mir::UnaryOperator::FloatNegate) => {
            dispatch::handle_unary_float32
        }
        (Some(ValueKind::Float { width: 64 }), mir::UnaryOperator::FloatNegate) => {
            dispatch::handle_unary_float64
        }
        (Some(ValueKind::Bool), mir::UnaryOperator::Not) => dispatch::handle_unary_bool,
        _ => dispatch::handle_unary,
    }
}

/// Pick a load handler based on inferred pointer storage.
fn select_load_handler(value_kinds: &ValueKinds, pointer: mir::Value) -> ThreadedHandler {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_load_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_load_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_load_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_load_global,
        _ => dispatch::handle_load,
    }
}

/// Pick a store handler based on inferred pointer storage.
fn select_store_handler(value_kinds: &ValueKinds, pointer: mir::Value) -> ThreadedHandler {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_store_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_store_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_store_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_store_global,
        _ => dispatch::handle_store,
    }
}

/// Pick a field get handler based on field count (inline optimization).
fn select_field_get_handler(field_count: u32, index: u32) -> ThreadedHandler {
    // small aggregates (≤2 fields) use inline slot storage
    // only use fast path if index is known to be valid
    if field_count > 0 && field_count <= 2 && index < field_count {
        dispatch::handle_field_get_inline
    } else {
        dispatch::handle_field_get
    }
}

/// Pick a field address handler based on inferred aggregate storage.
fn select_field_addr_handler(value_kinds: &ValueKinds, aggregate: mir::Value) -> ThreadedHandler {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => dispatch::handle_field_addr_aggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_field_addr_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_field_addr_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_field_addr_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_field_addr_global,
        _ => dispatch::handle_field_addr,
    }
}

/// Pick an element address handler based on inferred array storage.
fn select_element_addr_handler(value_kinds: &ValueKinds, array: mir::Value) -> ThreadedHandler {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            dispatch::handle_element_addr_aggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_element_addr_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_element_addr_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_element_addr_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_element_addr_global,
        _ => dispatch::handle_element_addr,
    }
}

/// Pick a field load handler based on inferred aggregate storage.
fn select_field_load_handler(value_kinds: &ValueKinds, aggregate: mir::Value) -> ThreadedHandler {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => dispatch::handle_field_load_aggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_field_load_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_field_load_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_field_load_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_field_load_global,
        _ => dispatch::handle_field_load,
    }
}

/// Pick a field store handler based on inferred aggregate storage.
fn select_field_store_handler(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
    field_count: u32,
    index: u32,
) -> ThreadedHandler {
    // small managed aggregates (≤2 fields) use inline slot storage
    if field_count > 0 && field_count <= 2 && index < field_count {
        if let Some(ValueKind::Aggregate { .. }) = value_kinds.get(aggregate) {
            return dispatch::handle_field_store_inline;
        }
    }

    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => dispatch::handle_field_store_aggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_field_store_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_field_store_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_field_store_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_field_store_global,
        _ => dispatch::handle_field_store,
    }
}

/// Pick an element load handler based on inferred array storage.
fn select_element_load_handler(value_kinds: &ValueKinds, array: mir::Value) -> ThreadedHandler {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            dispatch::handle_element_load_aggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_element_load_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_element_load_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_element_load_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_element_load_global,
        _ => dispatch::handle_element_load,
    }
}

/// Pick an element store handler based on inferred array storage.
fn select_element_store_handler(value_kinds: &ValueKinds, array: mir::Value) -> ThreadedHandler {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            dispatch::handle_element_store_aggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => dispatch::handle_element_store_managed,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => dispatch::handle_element_store_raw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => dispatch::handle_element_store_stack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => dispatch::handle_element_store_global,
        _ => dispatch::handle_element_store,
    }
}

/// Pick a branch handler based on inferred condition kind.
fn select_branch_handler(value_kinds: &ValueKinds, condition: mir::Value) -> ThreadedHandler {
    // resolve condition kind
    match value_kinds.get(condition) {
        Some(ValueKind::Bool) => dispatch::handle_branch_bool,
        _ => dispatch::handle_branch,
    }
}

/// Pick a switch handler based on inferred value kind.
fn select_switch_handler(value_kinds: &ValueKinds, value: mir::Value) -> ThreadedHandler {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => dispatch::handle_switch_int,
        _ => dispatch::handle_switch,
    }
}

/// Convert a MIR function to threaded form for fast execution.
pub(super) fn thread_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
    function_indices: &[u32],
) -> Option<ThreadedFunction> {
    // load function
    let func = tree.get(func_id);

    // imported functions can't be threaded
    if func.is_import() {
        return None;
    }

    // read entry block
    let entry_block = func.entry?;

    // prepare block index mapping
    let mut block_index_map: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut mir_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // seed traversal queue
    let mut queue = vec![entry_block];
    let mut visited = std::collections::HashSet::new();

    while let Some(block_id) = queue.pop() {
        // skip visited blocks
        if visited.contains(&block_id) {
            continue;
        }

        // record block index
        visited.insert(block_id);
        let idx = mir_blocks.len();
        block_index_map.insert(block_id, idx);
        mir_blocks.push(block_id);

        // enqueue successor blocks
        let block = tree.get(block_id);
        match &block.terminator {
            mir::Terminator::Jump { target, .. } => {
                queue.push(*target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                queue.push(*then_target);
                queue.push(*else_target);
            }
            mir::Terminator::Switch { cases, default, .. } => {
                for case in cases {
                    queue.push(case.target);
                }
                queue.push(*default);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::Yield { .. } => {}
        }
    }

    // compute value kinds for typed dispatch
    let value_count = compute_value_count_from_mir(tree, func, &mir_blocks);
    let value_kinds = build_value_kinds(tree, func, &mir_blocks, value_count);
    let value_uses = compute_value_use_counts(tree, &mir_blocks, value_count);

    // validate threaded indices
    debug_assert!(
        mir_blocks.len() <= u32::MAX as usize,
        "too many blocks for threaded indices"
    );

    // collect block parameters
    let mut block_parameters = Vec::with_capacity(mir_blocks.len());
    for block_id in &mir_blocks {
        let block = tree.get(*block_id);
        let params: Vec<mir::Value> = block.parameters.iter().map(|p| p.value).collect();
        block_parameters.push(params);
    }

    // allocate pools
    let mut argument_pool = Vec::new();
    let mut switch_case_pool = Vec::new();
    let mut copy_pool = Vec::new();

    // gather function parameters
    let parameter_values: Vec<mir::Value> = func.parameters.iter().map(|p| p.value).collect();
    let parameters = push_argument_range(&mut argument_pool, &parameter_values);

    // allocate threaded blocks
    let mut threaded_blocks = Vec::with_capacity(mir_blocks.len());

    // thread each mir block
    for mir_block_id in &mir_blocks {
        let block = tree.get(*mir_block_id);
        let threaded = thread_block(
            tree,
            *mir_block_id,
            block,
            &block_index_map,
            &block_parameters,
            function_indices,
            &value_kinds,
            &value_uses,
            &mut argument_pool,
            &mut switch_case_pool,
            &mut copy_pool,
        );
        threaded_blocks.push(threaded);
    }

    // compute storage sizes
    let local_count = func.locals.len();

    // assemble threaded function
    Some(ThreadedFunction {
        parameters,
        entry: block_index_map[&entry_block] as u32,
        blocks: threaded_blocks,
        argument_pool,
        switch_case_pool,
        copy_pool,
        value_count,
        local_count,
    })
}

/// Convert a MIR block to threaded form.
#[allow(clippy::too_many_arguments)]
fn thread_block(
    tree: &mir::NodeTree,
    mir_block: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    function_indices: &[u32],
    value_kinds: &ValueKinds,
    value_uses: &[u32],
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
) -> ThreadedBlock {
    // resolve block parameter values
    let block_index = block_index_map[&mir_block];
    let parameter_values = block_parameters
        .get(block_index)
        .map(|params| params.as_slice())
        .unwrap_or_default();

    // preallocate instruction list
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);

    // convert regular instructions
    let mut inst_index = 0usize;
    while inst_index < block.instructions.len() {
        // load current instruction
        let inst_id = block.instructions[inst_index];
        let inst = tree.get(inst_id);

        // attempt addr + load/store fusion
        if let Some((threaded, skip)) = try_fuse_addr_access(
            tree,
            inst,
            block.instructions.get(inst_index + 1).copied(),
            value_kinds,
            value_uses,
        ) {
            instructions.push(threaded);
            inst_index += skip;
            continue;
        }

        // attempt const + binary fusion
        if let Some((threaded, skip)) = try_fuse_const_binary(
            tree,
            inst,
            block.instructions.get(inst_index + 1).copied(),
            value_kinds,
            value_uses,
        ) {
            instructions.push(threaded);
            inst_index += skip;
            continue;
        }

        // fall back to standard threading
        let threaded = thread_instruction(
            tree,
            inst,
            value_kinds,
            argument_pool,
            copy_pool,
            function_indices,
        );
        instructions.push(threaded);
        inst_index += 1;
    }

    // try to fuse compare + branch
    if let Some(fused) = try_fuse_compare_branch(
        tree,
        block,
        &mut instructions,
        block_index_map,
        block_parameters,
        value_uses,
        copy_pool,
    ) {
        instructions.push(fused);
    } else {
        // append threaded terminator
        let terminator = thread_terminator(
            &block.terminator,
            block_index_map,
            block_parameters,
            value_kinds,
            switch_case_pool,
            copy_pool,
        );
        instructions.push(terminator);
    }

    // gather block parameters
    let parameters = push_argument_range(argument_pool, parameter_values);

    // compute original MIR instruction count (instructions + terminator)
    let mir_instruction_count = (block.instructions.len() + 1) as u32;

    // assemble block
    ThreadedBlock {
        mir_block,
        parameters,
        instructions,
        mir_instruction_count,
    }
}

/// Try to fuse addr + load/store into a single threaded instruction.
fn try_fuse_addr_access(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
) -> Option<(ThreadedInstruction, usize)> {
    // bail if there is no next instruction
    let next_inst_id = next_inst_id?;

    // check value usage count for the addr result
    let can_fuse =
        |value: mir::Value| -> bool { value_uses.get(value.0 as usize).copied().unwrap_or(0) == 1 };

    // load next instruction for pattern matching
    let next_inst = tree.get(next_inst_id);

    match inst {
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
        } => {
            if !can_fuse(*destination) {
                return None;
            }

            let field_count = value_kinds
                .get(*aggregate)
                .and_then(|kind| field_count_from_kind(tree, kind))
                .unwrap_or(UNKNOWN_FIELD_COUNT);

            match next_inst {
                mir::Instruction::Load {
                    destination: load_dest,
                    pointer,
                } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: select_field_load_handler(value_kinds, *aggregate),
                        data: ThreadedInstructionData::FieldLoad {
                            dest: *load_dest,
                            aggregate: *aggregate,
                            index: *index,
                            field_count,
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: select_field_store_handler(
                            value_kinds,
                            *aggregate,
                            field_count,
                            *index,
                        ),
                        data: ThreadedInstructionData::FieldStore {
                            aggregate: *aggregate,
                            index: *index,
                            value: *value,
                            reference: reference_meta_for_value(value_kinds, *destination),
                            field_count,
                        },
                    },
                    2,
                )),
                _ => None,
            }
        }
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
        } => {
            if !can_fuse(*destination) {
                return None;
            }

            let array_length = value_kinds
                .get(*array)
                .and_then(|kind| array_length_from_kind(tree, kind))
                .unwrap_or(UNKNOWN_ARRAY_LENGTH);

            match next_inst {
                mir::Instruction::Load {
                    destination: load_dest,
                    pointer,
                } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: select_element_load_handler(value_kinds, *array),
                        data: ThreadedInstructionData::ElementLoad {
                            dest: *load_dest,
                            array: *array,
                            index: *index,
                            array_length,
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: select_element_store_handler(value_kinds, *array),
                        data: ThreadedInstructionData::ElementStore {
                            array: *array,
                            index: *index,
                            value: *value,
                            reference: reference_meta_for_value(value_kinds, *destination),
                            array_length,
                        },
                    },
                    2,
                )),
                _ => None,
            }
        }
        mir::Instruction::GlobalAddr {
            destination,
            global,
        } => {
            if !can_fuse(*destination) {
                return None;
            }

            let reference = reference_meta_for_value(value_kinds, *destination);

            match next_inst {
                mir::Instruction::Load {
                    destination: load_dest,
                    pointer,
                } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: dispatch::handle_global_load,
                        data: ThreadedInstructionData::GlobalLoad {
                            dest: *load_dest,
                            global: global.id,
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    ThreadedInstruction {
                        handler: dispatch::handle_global_store,
                        data: ThreadedInstructionData::GlobalStore {
                            global: global.id,
                            value: *value,
                            reference,
                        },
                    },
                    2,
                )),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Try to fuse const + binary into a single instruction with embedded constant.
fn try_fuse_const_binary(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
) -> Option<(ThreadedInstruction, usize)> {
    // bail if there is no next instruction
    let next_inst_id = next_inst_id?;

    // check if the value is only used once
    let can_fuse =
        |value: mir::Value| -> bool { value_uses.get(value.0 as usize).copied().unwrap_or(0) == 1 };

    // we need a Const instruction
    let mir::Instruction::Const { destination, value } = inst else {
        return None;
    };

    // check if the const value is only used once
    if !can_fuse(*destination) {
        return None;
    }

    // load next instruction and check if it's a binary using our constant
    let next_inst = tree.get(next_inst_id);
    let mir::Instruction::Binary {
        destination: bin_dest,
        operator,
        left,
        right,
    } = next_inst
    else {
        return None;
    };

    // we can fuse if the constant is the right operand
    if right != destination {
        return None;
    }

    // skip comparisons: they may be fused with branches, which expect both
    // operands to be materialized values
    if operator.is_comparison() {
        return None;
    }

    // convert constant to runtime Value
    let const_value = Value::from(value);

    // check if left operand is integer for specialized handler
    let kind = value_kinds.get(*left);
    if let Some(ValueKind::Int { signed, .. }) = kind {
        // try to get a specialized const handler
        if let Some(handler) = select_specialized_const_int_handler(*operator, signed) {
            return Some((
                ThreadedInstruction {
                    handler,
                    data: ThreadedInstructionData::BinaryConstRightSpecialized {
                        dest: *bin_dest,
                        left: *left,
                        right_const: const_value,
                    },
                },
                2,
            ));
        }
    }

    // fall back to generic binary with const right
    Some((
        ThreadedInstruction {
            handler: dispatch::handle_binary_const_right,
            data: ThreadedInstructionData::BinaryConstRight {
                dest: *bin_dest,
                op: *operator,
                left: *left,
                right_const: const_value,
            },
        },
        2,
    ))
}

/// Try to fuse compare + branch into a single instruction.
///
/// If the terminator is a Branch whose condition comes from an icmp in the same block
/// with only one use, we can fuse them into a CompareAndBranch.
#[allow(clippy::too_many_arguments)]
fn try_fuse_compare_branch(
    tree: &mir::NodeTree,
    block: &mir::Block,
    instructions: &mut Vec<ThreadedInstruction>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    value_uses: &[u32],
    copy_pool: &mut Vec<CopyPair>,
) -> Option<ThreadedInstruction> {
    // only fuse Branch terminators
    let mir::Terminator::Branch {
        condition,
        then_target,
        then_arguments,
        else_target,
        else_arguments,
    } = &block.terminator
    else {
        return None;
    };

    // the condition must have exactly one use (this branch)
    if value_uses.get(condition.0 as usize).copied().unwrap_or(0) != 1 {
        return None;
    }

    // the last instruction in the block must produce the condition
    let last_inst_id = block.instructions.last()?;
    let last_inst = tree.get(*last_inst_id);

    // must be a binary comparison instruction
    let mir::Instruction::Binary {
        destination,
        operator,
        left,
        right,
    } = last_inst
    else {
        return None;
    };

    // operator must be a comparison
    if !operator.is_comparison() {
        return None;
    }

    // destination must match the branch condition
    if *destination != *condition {
        return None;
    }

    // remove the compare instruction (it's now fused)
    instructions.pop();

    // resolve branch target parameters
    let then_index = block_index_map[then_target];
    let else_index = block_index_map[else_target];
    let then_parameters = block_parameters
        .get(then_index)
        .map(|params| params.as_slice())
        .unwrap_or_default();
    let else_parameters = block_parameters
        .get(else_index)
        .map(|params| params.as_slice())
        .unwrap_or_default();
    let then_copies = push_copy_range(copy_pool, then_parameters, then_arguments);
    let else_copies = push_copy_range(copy_pool, else_parameters, else_arguments);

    // select specialized handler based on operator
    let handler = select_compare_branch_handler(*operator);

    // emit fused compare-and-branch
    Some(ThreadedInstruction {
        handler,
        data: ThreadedInstructionData::CompareAndBranch {
            left: *left,
            right: *right,
            operator: *operator,
            then_target: then_index as u32,
            then_copies,
            else_target: else_index as u32,
            else_copies,
        },
    })
}

/// Pick a compare-and-branch handler based on operator type.
fn select_compare_branch_handler(operator: mir::BinaryOperator) -> ThreadedHandler {
    match operator {
        // signed integer comparisons (most common in loops)
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual => dispatch::handle_compare_and_branch_int,
        // unsigned integer comparisons
        mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual => dispatch::handle_compare_and_branch_uint,
        // float comparisons
        mir::BinaryOperator::FloatEqual
        | mir::BinaryOperator::FloatNotEqual
        | mir::BinaryOperator::FloatLessThan
        | mir::BinaryOperator::FloatLessEqual
        | mir::BinaryOperator::FloatGreaterThan
        | mir::BinaryOperator::FloatGreaterEqual => dispatch::handle_compare_and_branch_float,
        // fallback for non-comparison operators (should not happen)
        _ => dispatch::handle_compare_and_branch,
    }
}

/// Convert a MIR instruction to threaded form.
fn thread_instruction(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
    argument_pool: &mut Vec<mir::Value>,
    copy_pool: &mut Vec<CopyPair>,
    function_indices: &[u32],
) -> ThreadedInstruction {
    // map instruction opcode to threaded form
    match inst {
        mir::Instruction::Const { destination, value } => ThreadedInstruction {
            handler: dispatch::handle_const,
            data: ThreadedInstructionData::Const {
                dest: *destination,
                value: Value::from(value),
            },
        },

        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => {
            // check if we can use a specialized handler
            let kind = value_kinds.get(*left);
            if let Some(ValueKind::Int { signed, .. }) = kind
                && let Some(handler) = select_specialized_int_handler(*operator, signed)
            {
                return ThreadedInstruction {
                    handler,
                    data: ThreadedInstructionData::BinarySpecialized {
                        dest: *destination,
                        left: *left,
                        right: *right,
                    },
                };
            }

            // fall back to generic binary instruction
            ThreadedInstruction {
                handler: select_binary_handler(value_kinds, *left, *operator),
                data: ThreadedInstructionData::Binary {
                    dest: *destination,
                    op: *operator,
                    left: *left,
                    right: *right,
                },
            }
        }

        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => ThreadedInstruction {
            handler: select_unary_handler(value_kinds, *argument, *operator),
            data: ThreadedInstructionData::Unary {
                dest: *destination,
                op: *operator,
                arg: *argument,
            },
        },

        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => ThreadedInstruction {
            handler: dispatch::handle_cast,
            data: ThreadedInstructionData::Cast {
                dest: *destination,
                op: *operator,
                arg: *argument,
                to_type: to_type.id,
            },
        },

        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => {
            // resolve call arguments and copy plan
            let args = tree.get_arguments(*arguments);
            let args_range = push_argument_range(argument_pool, args);
            let callee = tree.get(*function);
            let copies = push_copy_range_from_params(copy_pool, &callee.parameters, args);
            let callee_index = lookup_function_index(function_indices, *function)
                .unwrap_or(INVALID_FUNCTION_INDEX);

            // assemble threaded call
            ThreadedInstruction {
                handler: dispatch::handle_call,
                data: ThreadedInstructionData::Call {
                    dest: pack_optional_value(*destination),
                    function: function.id,
                    callee_index,
                    arguments: args_range,
                    copies,
                },
            }
        }

        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            ThreadedInstruction {
                handler: dispatch::handle_call_indirect,
                data: ThreadedInstructionData::CallIndirect {
                    dest: pack_optional_value(*destination),
                    callee: *callee,
                    arguments: args,
                },
            }
        }

        mir::Instruction::LocalGet { destination, local } => ThreadedInstruction {
            handler: dispatch::handle_local_get,
            data: ThreadedInstructionData::LocalGet {
                dest: *destination,
                local: local.id,
            },
        },

        mir::Instruction::LocalSet { local, value } => ThreadedInstruction {
            handler: dispatch::handle_local_set,
            data: ThreadedInstructionData::LocalSet {
                local: local.id,
                value: *value,
            },
        },

        mir::Instruction::GlobalAddr {
            destination,
            global,
        } => ThreadedInstruction {
            handler: dispatch::handle_global_addr,
            data: ThreadedInstructionData::GlobalAddr {
                dest: *destination,
                global: global.id,
                reference: reference_meta_for_value(value_kinds, *destination),
            },
        },

        mir::Instruction::GlobalConst {
            destination,
            global,
        } => ThreadedInstruction {
            handler: dispatch::handle_global_const,
            data: ThreadedInstructionData::GlobalConst {
                dest: *destination,
                global: global.id,
            },
        },

        mir::Instruction::Load {
            destination,
            pointer,
        } => ThreadedInstruction {
            handler: select_load_handler(value_kinds, *pointer),
            data: ThreadedInstructionData::Load {
                dest: *destination,
                pointer: *pointer,
            },
        },

        mir::Instruction::Store { pointer, value } => ThreadedInstruction {
            handler: select_store_handler(value_kinds, *pointer),
            data: ThreadedInstructionData::Store {
                pointer: *pointer,
                value: *value,
                reference: reference_meta_for_value(value_kinds, *pointer),
            },
        },

        mir::Instruction::Drop { .. } => ThreadedInstruction {
            handler: dispatch::handle_unsupported,
            data: ThreadedInstructionData::Unsupported { name: "drop" },
        },

        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => {
            let field_count = value_kinds
                .get(*aggregate)
                .and_then(|kind| field_count_from_kind(tree, kind))
                .unwrap_or(UNKNOWN_FIELD_COUNT);
            ThreadedInstruction {
                handler: select_field_get_handler(field_count, *index),
                data: ThreadedInstructionData::FieldGet {
                    dest: *destination,
                    aggregate: *aggregate,
                    index: *index,
                    field_count,
                },
            }
        }

        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
        } => ThreadedInstruction {
            handler: select_field_addr_handler(value_kinds, *aggregate),
            data: ThreadedInstructionData::FieldAddr {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                reference: reference_meta_for_value(value_kinds, *destination),
                field_count: value_kinds
                    .get(*aggregate)
                    .and_then(|kind| field_count_from_kind(tree, kind))
                    .unwrap_or(UNKNOWN_FIELD_COUNT),
            },
        },

        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => ThreadedInstruction {
            handler: dispatch::handle_field_set,
            data: ThreadedInstructionData::FieldSet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => ThreadedInstruction {
            handler: dispatch::handle_element_get,
            data: ThreadedInstructionData::ElementGet {
                dest: *destination,
                array: *array,
                index: *index,
            },
        },

        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
        } => ThreadedInstruction {
            handler: select_element_addr_handler(value_kinds, *array),
            data: ThreadedInstructionData::ElementAddr {
                dest: *destination,
                array: *array,
                index: *index,
                reference: reference_meta_for_value(value_kinds, *destination),
                array_length: value_kinds
                    .get(*array)
                    .and_then(|kind| array_length_from_kind(tree, kind))
                    .unwrap_or(UNKNOWN_ARRAY_LENGTH),
            },
        },

        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => ThreadedInstruction {
            handler: dispatch::handle_element_set,
            data: ThreadedInstructionData::ElementSet {
                dest: *destination,
                array: *array,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ManagedAlloc {
            destination,
            layout,
        } => ThreadedInstruction {
            handler: dispatch::handle_managed_alloc,
            data: ThreadedInstructionData::ManagedAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                slot_count: slot_count_from_type(tree, *layout).unwrap_or(UNKNOWN_SLOT_COUNT),
            },
        },

        mir::Instruction::ManagedAllocArray {
            destination,
            length,
            ..
        } => ThreadedInstruction {
            handler: dispatch::handle_managed_alloc_array,
            data: ThreadedInstructionData::ManagedAllocArray {
                dest: *destination,
                length: *length,
                reference: ReferenceMeta::new(
                    mir::ReferenceKind::Managed,
                    mir::Mutability::Mutable,
                    false,
                ),
            },
        },

        mir::Instruction::RawAlloc {
            destination,
            layout,
        } => ThreadedInstruction {
            handler: dispatch::handle_raw_alloc,
            data: ThreadedInstructionData::RawAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                slot_count: slot_count_from_type(tree, *layout).unwrap_or(UNKNOWN_SLOT_COUNT),
            },
        },

        mir::Instruction::RawFree { pointer } => ThreadedInstruction {
            handler: dispatch::handle_raw_free,
            data: ThreadedInstructionData::RawFree { pointer: *pointer },
        },

        mir::Instruction::StackAlloc {
            destination,
            layout,
        } => ThreadedInstruction {
            handler: dispatch::handle_stack_alloc,
            data: ThreadedInstructionData::StackAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                slot_count: slot_count_from_type(tree, *layout).unwrap_or(UNKNOWN_SLOT_COUNT),
            },
        },

        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            ThreadedInstruction {
                handler: dispatch::handle_intrinsic,
                data: ThreadedInstructionData::Intrinsic {
                    dest: pack_optional_value(*destination),
                    intrinsic: *intrinsic,
                    arguments: args,
                    ordering: *ordering,
                },
            }
        }
    }
}

/// Build the value kind table for typed dispatch.
fn build_value_kinds(
    tree: &mir::NodeTree,
    func: &mir::Function,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_count: usize,
) -> ValueKinds {
    // allocate value kinds
    let mut value_kinds = ValueKinds::new(value_count);

    // seed function parameter kinds
    for param in &func.parameters {
        value_kinds.set(param.value, kind_from_type(tree, param.ty));
    }

    // seed block parameter kinds
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        for param in &block.parameters {
            value_kinds.set(param.value, kind_from_type(tree, param.ty));
        }
    }

    // iteratively infer instruction results
    let mut changed = true;
    while changed {
        // reset iteration flag
        changed = false;

        // scan instructions for new kinds
        for block_id in mir_blocks {
            let block = tree.get(*block_id);
            // scan block instructions
            for inst_id in &block.instructions {
                let inst = tree.get(*inst_id);
                let Some(dest) = inst.destination() else {
                    continue;
                };
                if value_kinds.get(dest).is_some() {
                    continue;
                }
                let Some(kind) = infer_instruction_kind(tree, inst, &value_kinds) else {
                    continue;
                };
                value_kinds.set(dest, kind);
                changed = true;
            }
        }
    }

    // return inferred kinds
    value_kinds
}

/// Compute SSA value use counts across the function.
fn compute_value_use_counts(
    tree: &mir::NodeTree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_count: usize,
) -> Vec<u32> {
    // allocate use counters
    let mut uses = vec![0u32; value_count];

    // record a single use safely
    let mut record_use = |value: mir::Value| {
        if let Some(slot) = uses.get_mut(value.0 as usize) {
            *slot = slot.saturating_add(1);
        }
    };

    // scan instructions and terminators for value uses
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        for inst_id in &block.instructions {
            let inst = tree.get(*inst_id);
            for value in inst.uses() {
                record_use(value);
            }
            if let Some(args) = inst.argument_slice() {
                for arg in tree.get_arguments(args) {
                    record_use(*arg);
                }
            }
        }
        for value in block.terminator.uses() {
            record_use(value);
        }
    }

    // return the use table
    uses
}

/// Get reference metadata for a value when available.
fn reference_meta_for_value(value_kinds: &ValueKinds, value: mir::Value) -> ReferenceMeta {
    match value_kinds.get(value) {
        Some(ValueKind::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Infer the value kind for a MIR instruction.
fn infer_instruction_kind(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
) -> Option<ValueKind> {
    // resolve instruction kind
    match inst {
        mir::Instruction::Const { value, .. } => Some(kind_from_constant(value)),
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            // comparisons always produce bool
            if operator.is_comparison() {
                return Some(ValueKind::Bool);
            }

            // resolve operand kind
            let lhs_kind = value_kinds.get(*left);
            let rhs_kind = value_kinds.get(*right);
            let operand_kind = lhs_kind.or(rhs_kind);

            // select result kind by operand and operator
            match (operator.is_float(), operand_kind) {
                (true, Some(ValueKind::Float { width })) => Some(ValueKind::Float { width }),
                (false, Some(ValueKind::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueKind::Bool)
                }
                (false, Some(ValueKind::Int { width, signed })) => {
                    Some(ValueKind::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_kinds.get(*argument),
        mir::Instruction::Cast { to_type, .. } => Some(kind_from_type(tree, *to_type)),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            destination.as_ref()?;
            let func = tree.get(*function);
            Some(kind_from_type(tree, func.return_type))
        }
        mir::Instruction::CallIndirect {
            destination,
            callee,
            ..
        } => {
            destination.as_ref()?;
            let callee_kind = value_kinds.get(*callee)?;
            match callee_kind {
                ValueKind::FunctionPointer { result } => Some(kind_from_type(tree, result)),
                _ => None,
            }
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(*local);
            Some(kind_from_type(tree, local.ty))
        }
        mir::Instruction::GlobalAddr { global, .. } => {
            let global = tree.get(*global);
            Some(ValueKind::Pointer {
                pointee: global.ty,
                storage: PointerStorage::Global,
                reference: ReferenceMeta::new(mir::ReferenceKind::Raw, global.mutability, false),
            })
        }
        mir::Instruction::GlobalConst { global, .. } => {
            let global = tree.get(*global);
            Some(kind_from_type(tree, global.ty))
        }
        mir::Instruction::Load { pointer, .. } => {
            let pointer_kind = value_kinds.get(*pointer)?;
            kind_from_pointer(tree, pointer_kind)
        }
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kinds.get(*aggregate)?;
            kind_from_field(tree, aggregate_kind, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kinds.get(*aggregate)?;
            let field_type_id = field_type_id_from_kind(tree, aggregate_kind, *index)?;
            let (storage, reference) = pointer_metadata_from_aggregate(aggregate_kind);
            Some(ValueKind::Pointer {
                pointee: field_type_id,
                storage,
                reference,
            })
        }
        mir::Instruction::FieldSet { aggregate, .. } => value_kinds.get(*aggregate),
        mir::Instruction::ElementGet { array, .. } => {
            let array_kind = value_kinds.get(*array)?;
            kind_from_element(tree, array_kind)
        }
        mir::Instruction::ElementAddr { array, .. } => {
            let array_kind = value_kinds.get(*array)?;
            let element_type_id = element_type_id_from_kind(tree, array_kind)?;
            let (storage, reference) = pointer_metadata_from_array(array_kind);
            Some(ValueKind::Pointer {
                pointee: element_type_id,
                storage,
                reference,
            })
        }
        mir::Instruction::ElementSet { array, .. } => value_kinds.get(*array),
        mir::Instruction::ManagedAlloc { layout, .. } => Some(ValueKind::Pointer {
            pointee: *layout,
            storage: PointerStorage::Managed,
            reference: ReferenceMeta::new(
                mir::ReferenceKind::Managed,
                mir::Mutability::Mutable,
                false,
            ),
        }),
        mir::Instruction::ManagedAllocArray { element, .. } => Some(ValueKind::Pointer {
            pointee: *element,
            storage: PointerStorage::Managed,
            reference: ReferenceMeta::new(
                mir::ReferenceKind::Managed,
                mir::Mutability::Mutable,
                false,
            ),
        }),
        mir::Instruction::RawAlloc { layout, .. } => Some(ValueKind::Pointer {
            pointee: *layout,
            storage: PointerStorage::Raw,
            reference: ReferenceMeta::new(mir::ReferenceKind::Raw, mir::Mutability::Mutable, false),
        }),
        mir::Instruction::StackAlloc { layout, .. } => Some(ValueKind::Pointer {
            pointee: *layout,
            storage: PointerStorage::Stack,
            reference: ReferenceMeta::new(mir::ReferenceKind::Raw, mir::Mutability::Mutable, false),
        }),
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_kind(tree, *intrinsic, *arguments, value_kinds),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::RawFree { .. } => None,
    }
}

/// Infer the value kind for an intrinsic call.
fn infer_intrinsic_kind(
    tree: &mir::NodeTree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_kinds: &ValueKinds,
) -> Option<ValueKind> {
    // load argument values
    let args = tree.get_arguments(arguments);

    // resolve result kind from intrinsic metadata
    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Bool => Some(ValueKind::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueKind::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueKind::Int {
            width: 64,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueKind::Int {
            width: 64,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let arg = args.get(index as usize)?;
            value_kinds.get(*arg)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let arg = args.get(index as usize)?;
            let pointer_kind = value_kinds.get(*arg)?;
            kind_from_pointer(tree, pointer_kind)
        }
        mir::IntrinsicResultType::CheckedArithmetic => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::TypeDescriptor => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::Explicit => Some(ValueKind::Unknown),
    }
}

/// Get the kind for a MIR type.
fn kind_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> ValueKind {
    // map mir type to value kind
    match tree.get(ty) {
        mir::Type::Void => ValueKind::Void,
        mir::Type::Boolean => ValueKind::Bool,
        mir::Type::Int { width, signed } => ValueKind::Int {
            width: *width as u8,
            signed: *signed,
        },
        mir::Type::Float { width } => ValueKind::Float {
            width: *width as u8,
        },
        mir::Type::Reference {
            kind,
            mutability,
            pointee,
            is_nullable,
        } => ValueKind::Pointer {
            pointee: *pointee,
            storage: pointer_storage_from_reference_kind(*kind),
            reference: ReferenceMeta::new(*kind, *mutability, *is_nullable),
        },
        mir::Type::FunctionPointer { result, .. } => ValueKind::FunctionPointer { result: *result },
        mir::Type::Array { element, length } => ValueKind::Array {
            element: *element,
            length: *length,
        },
        mir::Type::Tuple { .. } | mir::Type::Struct { .. } => ValueKind::Aggregate { ty },
    }
}

/// Map a reference kind to a pointer storage class.
fn pointer_storage_from_reference_kind(kind: mir::ReferenceKind) -> PointerStorage {
    match kind {
        mir::ReferenceKind::Managed => PointerStorage::Managed,
        mir::ReferenceKind::Owned | mir::ReferenceKind::Raw => PointerStorage::Raw,
        mir::ReferenceKind::Borrowed => PointerStorage::Unknown,
    }
}

/// Resolve pointer metadata for aggregate values or pointers.
fn pointer_metadata_from_aggregate(kind: ValueKind) -> (PointerStorage, ReferenceMeta) {
    match kind {
        ValueKind::Pointer {
            storage, reference, ..
        } => (storage, reference),
        ValueKind::Aggregate { .. } => (
            PointerStorage::Managed,
            ReferenceMeta::new(mir::ReferenceKind::Managed, mir::Mutability::Mutable, false),
        ),
        _ => (PointerStorage::Unknown, ReferenceMeta::NONE),
    }
}

/// Resolve pointer metadata for array values or pointers.
fn pointer_metadata_from_array(kind: ValueKind) -> (PointerStorage, ReferenceMeta) {
    match kind {
        ValueKind::Pointer {
            storage, reference, ..
        } => (storage, reference),
        ValueKind::Array { .. } | ValueKind::Aggregate { .. } => (
            PointerStorage::Managed,
            ReferenceMeta::new(mir::ReferenceKind::Managed, mir::Mutability::Mutable, false),
        ),
        _ => (PointerStorage::Unknown, ReferenceMeta::NONE),
    }
}

/// Get the kind for a constant value.
fn kind_from_constant(constant: &mir::Constant) -> ValueKind {
    // map constant to value kind
    match constant {
        mir::Constant::Boolean { .. } => ValueKind::Bool,
        mir::Constant::Int {
            width, is_signed, ..
        } => ValueKind::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ValueKind::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { width, .. } => ValueKind::Float { width: *width },
        mir::Constant::String { .. } => ValueKind::Unknown,
        mir::Constant::Char { .. } => ValueKind::Char,
    }
}

/// Resolve the pointee kind from a pointer-like value.
fn kind_from_pointer(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    // resolve pointee kind when available
    match kind {
        ValueKind::Pointer { pointee, .. } => Some(kind_from_type(tree, pointee)),
        _ => None,
    }
}

/// Resolve the field kind for an aggregate value.
fn kind_from_field(tree: &mir::NodeTree, kind: ValueKind, index: u32) -> Option<ValueKind> {
    let ValueKind::Aggregate { ty } = kind else {
        return None;
    };

    // resolve field type from aggregate layout
    match tree.get(ty) {
        mir::Type::Struct { fields } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(kind_from_type(tree, field.ty))
        }
        mir::Type::Tuple { elements } => {
            let field = elements.get(index as usize)?;
            Some(kind_from_type(tree, *field))
        }
        _ => None,
    }
}

/// Resolve the field type id for an aggregate or pointer kind.
fn field_type_id_from_kind(
    tree: &mir::NodeTree,
    kind: ValueKind,
    index: u32,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // unwrap pointer kinds to their pointee
    let type_id = match kind {
        ValueKind::Aggregate { ty } => ty,
        ValueKind::Pointer { pointee, .. } => pointee,
        _ => return None,
    };

    // resolve field type from aggregate layout
    match tree.get(type_id) {
        mir::Type::Struct { fields } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(field.ty)
        }
        mir::Type::Tuple { elements } => elements.get(index as usize).copied(),
        _ => None,
    }
}

/// Resolve the element kind for an array value.
fn kind_from_element(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    // resolve element kind for array layouts
    match kind {
        ValueKind::Array { element, .. } => Some(kind_from_type(tree, element)),
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(kind_from_type(tree, *element)),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element type id for an array or pointer kind.
fn element_type_id_from_kind(
    tree: &mir::NodeTree,
    kind: ValueKind,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // unwrap pointer kinds to their pointee
    match kind {
        ValueKind::Array { element, .. } => Some(element),
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(*element),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { element, .. } => Some(*element),
            _ => Some(pointee),
        },
        _ => None,
    }
}

/// Resolve the slot count for a concrete type layout.
fn slot_count_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> Option<u32> {
    // map types to slot counts
    match tree.get(ty) {
        mir::Type::Struct { fields } => u32::try_from(fields.len()).ok(),
        mir::Type::Tuple { elements } => u32::try_from(elements.len()).ok(),
        mir::Type::Array { length, .. } => u32::try_from(*length).ok(),
        mir::Type::Void => Some(1),
        mir::Type::Boolean
        | mir::Type::Int { .. }
        | mir::Type::Float { .. }
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. } => Some(1),
    }
}

/// Resolve the field count for a struct or tuple kind.
fn field_count_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u32> {
    match kind {
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Struct { fields } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct { fields } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element length for an array kind.
fn array_length_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u64> {
    match kind {
        ValueKind::Array { length, .. } => {
            if length == UNKNOWN_ARRAY_LENGTH {
                None
            } else {
                Some(length)
            }
        }
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        _ => None,
    }
}

/// Compute the number of SSA values required by a function.
fn compute_value_count_from_mir(
    tree: &mir::NodeTree,
    func: &mir::Function,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
) -> usize {
    // start with no max value id
    let mut max_value: Option<u32> = None;

    // scan function parameters
    for param in &func.parameters {
        update_max_value(&mut max_value, param.value);
    }

    // scan blocks and instructions
    for block_id in mir_blocks {
        let block = tree.get(*block_id);

        // scan block parameters
        for param in &block.parameters {
            update_max_value(&mut max_value, param.value);
        }

        // scan block instructions
        for inst_id in &block.instructions {
            let inst = tree.get(*inst_id);

            // scan instruction destination
            if let Some(dest) = inst.destination() {
                update_max_value(&mut max_value, dest);
            }

            // scan inline instruction uses
            for value in inst.uses() {
                update_max_value(&mut max_value, value);
            }

            // scan externalized argument lists
            match inst {
                mir::Instruction::Call { arguments, .. }
                | mir::Instruction::CallIndirect { arguments, .. }
                | mir::Instruction::Intrinsic { arguments, .. } => {
                    for value in tree.get_arguments(*arguments) {
                        update_max_value(&mut max_value, *value);
                    }
                }
                _ => {}
            }
        }

        // scan terminator uses
        for value in block.terminator.uses() {
            update_max_value(&mut max_value, value);
        }
    }

    max_value.map(|id| id as usize + 1).unwrap_or(0)
}

/// Update the tracked maximum SSA value id.
fn update_max_value(max_value: &mut Option<u32>, value: mir::Value) {
    // grab the raw value id
    let id = value.0;

    // update max tracking
    match max_value {
        Some(current) => {
            if id > *current {
                *current = id;
            }
        }
        None => {
            *max_value = Some(id);
        }
    }
}

/// Append arguments to the pool and return their range.
fn push_argument_range(pool: &mut Vec<mir::Value>, arguments: &[mir::Value]) -> ArgumentRange {
    // fast path: no arguments
    if arguments.is_empty() {
        return ArgumentRange::empty();
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + arguments.len() <= u32::MAX as usize,
        "argument pool overflow"
    );

    // append arguments
    pool.extend_from_slice(arguments);

    // return range
    ArgumentRange {
        start: start as u32,
        len: arguments.len() as u32,
    }
}

/// Append parameter copies to the pool and return their range.
fn push_copy_range(
    pool: &mut Vec<CopyPair>,
    parameters: &[mir::Value],
    arguments: &[mir::Value],
) -> CopyRange {
    // fast path: no parameters
    if parameters.is_empty() {
        return CopyRange::empty();
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "copy pool overflow"
    );

    // append copy pairs
    for (index, param) in parameters.iter().enumerate() {
        let src = arguments
            .get(index)
            .map(|value| value.0)
            .unwrap_or(INVALID_VALUE_ID);
        pool.push(CopyPair { dest: param.0, src });
    }

    // return range
    CopyRange {
        start: start as u32,
        len: parameters.len() as u32,
    }
}

/// Append parameter copies to the pool and return their range.
fn push_copy_range_from_params(
    pool: &mut Vec<CopyPair>,
    parameters: &[mir::TypedValue],
    arguments: &[mir::Value],
) -> CopyRange {
    // fast path: no parameters
    if parameters.is_empty() {
        return CopyRange::empty();
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "copy pool overflow"
    );

    // append copy pairs
    for (index, param) in parameters.iter().enumerate() {
        let src = arguments
            .get(index)
            .map(|value| value.0)
            .unwrap_or(INVALID_VALUE_ID);
        pool.push(CopyPair {
            dest: param.value.0,
            src,
        });
    }

    // return range
    CopyRange {
        start: start as u32,
        len: parameters.len() as u32,
    }
}

/// Resolve a threaded function index for the given function id.
fn lookup_function_index(
    function_indices: &[u32],
    function: mir::LocalNodeId<mir::Function>,
) -> Option<u32> {
    // look up raw index
    let index = function_indices.get(function.id as usize).copied()?;

    // reject invalid entries
    if index == INVALID_FUNCTION_INDEX {
        return None;
    }

    // return valid index
    Some(index)
}

/// Append switch cases to the pool and return their range.
fn push_switch_case_range(
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
) -> SwitchRange {
    // fast path: no cases
    if cases.is_empty() {
        return SwitchRange::empty();
    }

    // compute range start
    let start = switch_case_pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + cases.len() <= u32::MAX as usize,
        "switch case pool overflow"
    );

    // append cases
    for case in cases {
        let target_index = block_index_map[&case.target];
        let target_parameters = block_parameters
            .get(target_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let copies = push_copy_range(copy_pool, target_parameters, &case.arguments);
        switch_case_pool.push(SwitchCase {
            value: case.value,
            target: target_index as u32,
            copies,
        });
    }

    // return range
    SwitchRange {
        start: start as u32,
        len: cases.len() as u32,
    }
}

/// Convert a MIR terminator to threaded form.
fn thread_terminator(
    term: &mir::Terminator,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    value_kinds: &ValueKinds,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
) -> ThreadedInstruction {
    // map terminator opcode to threaded form
    match term {
        mir::Terminator::Return { value } => ThreadedInstruction {
            handler: dispatch::handle_return,
            data: ThreadedInstructionData::Return {
                value: pack_optional_value(*value),
            },
        },

        mir::Terminator::Jump { target, arguments } => {
            // resolve target parameter copies
            let target_index = block_index_map[target];
            let target_parameters = block_parameters
                .get(target_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let copies = push_copy_range(copy_pool, target_parameters, arguments);

            // assemble threaded jump
            ThreadedInstruction {
                handler: dispatch::handle_jump,
                data: ThreadedInstructionData::Jump {
                    target: target_index as u32,
                    copies,
                },
            }
        }

        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            // resolve branch target parameters
            let then_index = block_index_map[then_target];
            let else_index = block_index_map[else_target];
            let then_parameters = block_parameters
                .get(then_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let else_parameters = block_parameters
                .get(else_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let then_copies = push_copy_range(copy_pool, then_parameters, then_arguments);
            let else_copies = push_copy_range(copy_pool, else_parameters, else_arguments);

            // assemble threaded branch
            ThreadedInstruction {
                handler: select_branch_handler(value_kinds, *condition),
                data: ThreadedInstructionData::Branch {
                    condition: *condition,
                    then_target: then_index as u32,
                    then_copies,
                    else_target: else_index as u32,
                    else_copies,
                },
            }
        }

        mir::Terminator::Switch {
            value,
            cases,
            default,
            default_arguments,
        } => {
            // resolve switch case copies
            let cases = push_switch_case_range(
                switch_case_pool,
                copy_pool,
                block_index_map,
                block_parameters,
                cases,
            );
            let default_index = block_index_map[default];
            let default_parameters = block_parameters
                .get(default_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let default_copies = push_copy_range(copy_pool, default_parameters, default_arguments);

            // assemble threaded switch
            ThreadedInstruction {
                handler: select_switch_handler(value_kinds, *value),
                data: ThreadedInstructionData::Switch {
                    value: *value,
                    cases,
                    default_target: default_index as u32,
                    default_copies,
                },
            }
        }

        mir::Terminator::Unreachable => ThreadedInstruction {
            handler: dispatch::handle_unreachable,
            data: ThreadedInstructionData::Unreachable,
        },

        mir::Terminator::Yield { .. } => ThreadedInstruction {
            handler: dispatch::handle_unsupported,
            data: ThreadedInstructionData::Unsupported { name: "yield" },
        },
    }
}
