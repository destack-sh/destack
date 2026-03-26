use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use destack_heap::{
    ManagedReference, RawPointer, ReferenceMap, ReferenceMeta, SharedPointer, Value,
};

use super::value::{PointerStorage, ValueKind, kind_from_type, pointer_storage_from_reference};
use super::{
    ArgumentRange, Block, ConstValue, CopyPair, CopyRange, Function, INVALID_FUNCTION_INDEX,
    INVALID_VALUE_ID, Instruction, InstructionData, InstructionOperation, SwitchCase, SwitchRange,
    UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT, UNKNOWN_SLOT_COUNT, pack_optional_value,
};

// switch table density threshold
const SWITCH_TABLE_MIN_DENSITY: f64 = 0.5;
// cap the number of jump table entries
const SWITCH_TABLE_MAX_RANGE: usize = 2048;

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

/// Normalize value kinds for block parameters.
fn kind_for_block_param(kind: ValueKind) -> ValueKind {
    // keep the type but avoid picking a storage class too early
    match kind {
        ValueKind::Pointer {
            pointee, reference, ..
        } => ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Unknown,
            reference,
        },
        _ => kind,
    }
}

/// Merge pointer storage classes when propagating block parameter kinds.
fn merge_pointer_storage(existing: PointerStorage, incoming: PointerStorage) -> PointerStorage {
    // prefer known storage, fall back to unknown when mismatched
    match (existing, incoming) {
        (PointerStorage::Unknown, other) => other,
        (other, PointerStorage::Unknown) => other,
        (left, right) if left == right => left,
        _ => PointerStorage::Unknown,
    }
}

/// Merge block parameter kinds with incoming argument kinds.
fn merge_block_param_kind(existing: ValueKind, incoming: ValueKind) -> ValueKind {
    // specialize pointer storage when possible
    match (existing, incoming) {
        (
            ValueKind::Pointer {
                pointee,
                storage,
                reference,
            },
            ValueKind::Pointer {
                storage: incoming_storage,
                ..
            },
        ) => ValueKind::Pointer {
            pointee,
            storage: merge_pointer_storage(storage, incoming_storage),
            reference,
        },
        (ValueKind::Unknown, other) => other,
        (other, ValueKind::Unknown) => other,
        (other, _) => other,
    }
}

/// Resolve a MIR value type from the function type table.
fn value_type_for_value(
    value: mir::Value,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> mir::LocalNodeId<mir::Type> {
    value_types
        .get(value.0 as usize)
        .copied()
        .unwrap_or_else(|| panic!("missing type for {value:?}"))
}

/// Propagate value kinds into block parameters from control flow edges.
fn propagate_block_param_kinds(
    tree: &mir::NodeTree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_kinds: &mut ValueKinds,
) -> bool {
    // track whether any kind changed
    let mut changed = false;

    // scan terminators for argument forwarding
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        match &block.terminator {
            mir::Terminator::Jump { target, arguments } => {
                changed |= propagate_target_kinds(tree, value_kinds, *target, arguments);
            }
            mir::Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                changed |= propagate_target_kinds(tree, value_kinds, *then_target, then_arguments);
                changed |= propagate_target_kinds(tree, value_kinds, *else_target, else_arguments);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                changed |=
                    propagate_target_kinds(tree, value_kinds, success.target, &success.arguments);
                changed |=
                    propagate_target_kinds(tree, value_kinds, failure.target, &failure.arguments);
            }
            mir::Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                changed |= propagate_target_kinds(tree, value_kinds, *default, default_arguments);
                for case in cases {
                    changed |=
                        propagate_target_kinds(tree, value_kinds, case.target, &case.arguments);
                }
            }
            mir::Terminator::Yield {
                resume,
                resume_arguments,
                ..
            } => {
                changed |= propagate_target_kinds(tree, value_kinds, *resume, resume_arguments);
            }
            mir::Terminator::Call {
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            }
            | mir::Terminator::CallIndirect {
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            }
            | mir::Terminator::CallVirtual {
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            }
            | mir::Terminator::CallInterface {
                normal_target,
                normal_arguments,
                unwind_target,
                unwind_arguments,
                ..
            } => {
                changed |=
                    propagate_target_kinds(tree, value_kinds, *normal_target, normal_arguments);
                changed |=
                    propagate_target_kinds(tree, value_kinds, *unwind_target, unwind_arguments);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Throw { .. }
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallInterface { .. } => {}
        }
    }

    // return whether any changes were applied
    changed
}

/// Update target block parameter kinds from incoming argument kinds.
fn propagate_target_kinds(
    tree: &mir::NodeTree,
    value_kinds: &mut ValueKinds,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) -> bool {
    // track whether any kind changed
    let mut changed = false;
    let target_block = tree.get(target);

    // merge each parameter with its incoming argument
    for (param, arg) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(arg_kind) = value_kinds.get(*arg) else {
            continue;
        };
        let existing = value_kinds.get(param.value);
        let next_kind = match existing {
            Some(kind) => merge_block_param_kind(kind, arg_kind),
            None => arg_kind,
        };
        if existing != Some(next_kind) {
            value_kinds.set(param.value, next_kind);
            changed = true;
        }
    }

    // return whether any kinds changed
    changed
}

/// Pick a binary handler based on inferred operand kind.
fn select_binary_operation(
    value_kinds: &ValueKinds,
    left: mir::Value,
    operator: mir::BinaryOperator,
) -> InstructionOperation {
    use mir::BinaryOperator::*;

    // resolve operand kind
    let kind = value_kinds.get(left);

    // try specialized integer handlers first (no operator dispatch overhead)
    if let Some(ValueKind::Int { signed, .. }) = kind
        && let Some(handler) = select_specialized_int_operation(operator, signed)
    {
        return handler;
    }

    // fall back to typed handlers
    match kind {
        Some(ValueKind::Int { signed: true, .. }) => InstructionOperation::BinaryInt,
        Some(ValueKind::Int { signed: false, .. }) => InstructionOperation::BinaryUint,
        Some(ValueKind::Float { width: 32 }) => InstructionOperation::BinaryFloat32,
        Some(ValueKind::Float { width: 64 }) => InstructionOperation::BinaryFloat64,
        Some(ValueKind::Bool) if matches!(operator, And | Or | Xor) => {
            InstructionOperation::BinaryBool
        }
        _ => InstructionOperation::Binary,
    }
}

/// Select a fully specialized integer handler if available.
fn select_specialized_int_operation(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<InstructionOperation> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            // signed arithmetic
            Add => InstructionOperation::AddInt,
            Subtract => InstructionOperation::SubInt,
            Multiply => InstructionOperation::MulInt,
            // signed bitwise
            And => InstructionOperation::AndInt,
            Or => InstructionOperation::OrInt,
            Xor => InstructionOperation::XorInt,
            ShiftLeft => InstructionOperation::ShlInt,
            ArithmeticShiftRight => InstructionOperation::ShrInt,
            // signed comparisons
            Equal => InstructionOperation::EqInt,
            NotEqual => InstructionOperation::NeInt,
            SignedLessThan => InstructionOperation::LtInt,
            SignedLessEqual => InstructionOperation::LeInt,
            SignedGreaterThan => InstructionOperation::GtInt,
            SignedGreaterEqual => InstructionOperation::GeInt,
            _ => return None,
        }
    } else {
        match operator {
            // unsigned arithmetic
            Add => InstructionOperation::AddUint,
            Subtract => InstructionOperation::SubUint,
            Multiply => InstructionOperation::MulUint,
            // unsigned bitwise
            And => InstructionOperation::AndUint,
            Or => InstructionOperation::OrUint,
            Xor => InstructionOperation::XorUint,
            ShiftLeft => InstructionOperation::ShlUint,
            LogicalShiftRight => InstructionOperation::ShrUint,
            // unsigned comparisons (eq/ne produce bool, not int/uint)
            Equal => InstructionOperation::EqInt,
            NotEqual => InstructionOperation::NeInt,
            UnsignedLessThan => InstructionOperation::LtUint,
            UnsignedLessEqual => InstructionOperation::LeUint,
            UnsignedGreaterThan => InstructionOperation::GtUint,
            UnsignedGreaterEqual => InstructionOperation::GeUint,
            _ => return None,
        }
    })
}

/// Select a specialized const-right handler if available.
fn select_specialized_const_int_operation(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<InstructionOperation> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            Add => InstructionOperation::AddConstInt,
            Subtract => InstructionOperation::SubConstInt,
            Multiply => InstructionOperation::MulConstInt,
            Equal => InstructionOperation::EqConstInt,
            NotEqual => InstructionOperation::NeConstInt,
            SignedLessThan => InstructionOperation::LtConstInt,
            SignedLessEqual => InstructionOperation::LeConstInt,
            SignedGreaterThan => InstructionOperation::GtConstInt,
            SignedGreaterEqual => InstructionOperation::GeConstInt,
            _ => return None,
        }
    } else {
        match operator {
            Add => InstructionOperation::AddConstUint,
            Subtract => InstructionOperation::SubConstUint,
            Multiply => InstructionOperation::MulConstUint,
            // eq/ne produce bool, can use signed version
            Equal => InstructionOperation::EqConstInt,
            NotEqual => InstructionOperation::NeConstInt,
            UnsignedLessThan => InstructionOperation::LtConstUint,
            UnsignedLessEqual => InstructionOperation::LeConstUint,
            UnsignedGreaterThan => InstructionOperation::GtConstUint,
            UnsignedGreaterEqual => InstructionOperation::GeConstUint,
            _ => return None,
        }
    })
}

/// Pick a unary handler based on inferred operand kind.
fn select_unary_operation(
    value_kinds: &ValueKinds,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> InstructionOperation {
    // resolve operand kind
    let kind = value_kinds.get(argument);

    // select handler by kind
    match (kind, operator) {
        (Some(ValueKind::Int { signed: true, .. }), _) => InstructionOperation::UnaryInt,
        (Some(ValueKind::Int { signed: false, .. }), _) => InstructionOperation::UnaryUint,
        (Some(ValueKind::Float { width: 32 }), mir::UnaryOperator::FloatNegate) => {
            InstructionOperation::UnaryFloat32
        }
        (Some(ValueKind::Float { width: 64 }), mir::UnaryOperator::FloatNegate) => {
            InstructionOperation::UnaryFloat64
        }
        (Some(ValueKind::Bool), mir::UnaryOperator::Not) => InstructionOperation::UnaryBool,
        _ => InstructionOperation::Unary,
    }
}

/// Pick a load handler based on inferred pointer storage.
fn select_load_operation(value_kinds: &ValueKinds, pointer: mir::Value) -> InstructionOperation {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::LoadManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::LoadRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::LoadStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::LoadLocal,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::LoadGlobal,
        _ => InstructionOperation::Load,
    }
}

/// Pick a store handler based on inferred pointer storage.
fn select_store_operation(value_kinds: &ValueKinds, pointer: mir::Value) -> InstructionOperation {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::StoreManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::StoreRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::StoreStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::StoreLocal,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::StoreGlobal,
        _ => InstructionOperation::Store,
    }
}

/// Pick a field get handler based on field count (inline optimization).
fn select_field_get_operation(field_count: u32, index: u32) -> InstructionOperation {
    // small aggregates (≤2 fields) use inline slot storage
    // only use fast path if index is known to be valid
    if field_count > 0 && field_count <= 2 && index < field_count {
        InstructionOperation::FieldGetInline
    } else {
        InstructionOperation::FieldGet
    }
}

/// Pick a field address handler based on inferred aggregate storage.
fn select_field_addr_operation(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => InstructionOperation::FieldAddrAggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::FieldAddrManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::FieldAddrRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::FieldAddrStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::FieldAddr,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::FieldAddrGlobal,
        _ => InstructionOperation::FieldAddr,
    }
}

/// Pick an element address handler based on inferred array storage.
fn select_element_addr_operation(
    value_kinds: &ValueKinds,
    array: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            InstructionOperation::ElementAddrAggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::ElementAddrManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::ElementAddrRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::ElementAddrStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::ElementAddr,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::ElementAddrGlobal,
        _ => InstructionOperation::ElementAddr,
    }
}

/// Pick a field load handler based on inferred aggregate storage.
fn select_field_load_operation(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => InstructionOperation::FieldLoadAggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::FieldLoadManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::FieldLoadRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::FieldLoadStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::FieldLoad,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::FieldLoadGlobal,
        _ => InstructionOperation::FieldLoad,
    }
}

/// Pick a field store handler based on inferred aggregate storage.
fn select_field_store_operation(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
    field_count: u32,
    index: u32,
) -> InstructionOperation {
    // small managed aggregates (≤2 fields) use inline slot storage
    if field_count > 0
        && field_count <= 2
        && index < field_count
        && let Some(ValueKind::Aggregate { .. }) = value_kinds.get(aggregate)
    {
        return InstructionOperation::FieldStoreInline;
    }

    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => InstructionOperation::FieldStoreAggregate,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::FieldStoreManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::FieldStoreRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::FieldStoreStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::FieldStore,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::FieldStoreGlobal,
        _ => InstructionOperation::FieldStore,
    }
}

/// Pick an element load handler based on inferred array storage.
fn select_element_load_operation(
    value_kinds: &ValueKinds,
    array: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            InstructionOperation::ElementLoadAggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::ElementLoadManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::ElementLoadRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::ElementLoadStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::ElementLoad,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::ElementLoadGlobal,
        _ => InstructionOperation::ElementLoad,
    }
}

/// Pick an element store handler based on inferred array storage.
fn select_element_store_operation(
    value_kinds: &ValueKinds,
    array: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => {
            InstructionOperation::ElementStoreAggregate
        }
        Some(ValueKind::Pointer {
            storage: PointerStorage::Managed,
            ..
        }) => InstructionOperation::ElementStoreManaged,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Raw,
            ..
        }) => InstructionOperation::ElementStoreRaw,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Stack,
            ..
        }) => InstructionOperation::ElementStoreStack,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Local,
            ..
        }) => InstructionOperation::ElementStore,
        Some(ValueKind::Pointer {
            storage: PointerStorage::Global,
            ..
        }) => InstructionOperation::ElementStoreGlobal,
        _ => InstructionOperation::ElementStore,
    }
}

/// Pick a branch handler based on inferred condition kind.
fn select_branch_operation(
    value_kinds: &ValueKinds,
    condition: mir::Value,
) -> InstructionOperation {
    // resolve condition kind
    match value_kinds.get(condition) {
        Some(ValueKind::Bool) => InstructionOperation::BranchBool,
        _ => InstructionOperation::Branch,
    }
}

/// Pick a switch handler based on inferred value kind.
fn select_switch_operation(value_kinds: &ValueKinds, value: mir::Value) -> InstructionOperation {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => InstructionOperation::SwitchInt,
        _ => InstructionOperation::Switch,
    }
}

/// Pick a switch table handler based on inferred value kind.
fn select_switch_table_operation(
    value_kinds: &ValueKinds,
    value: mir::Value,
) -> InstructionOperation {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => InstructionOperation::SwitchTableInt,
        _ => InstructionOperation::SwitchTable,
    }
}

/// Lower a MIR function into the interpreter function form.
pub(super) fn lower_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
    frame_layout: destack_engine::FrameLayoutId,
    yield_resume_points: &HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
) -> Option<Function> {
    // load function
    let func = tree.get(func_id);

    // imported functions can't be lowered here
    if func.is_import() {
        return None;
    }

    // read entry block
    let entry_block = func.entry?;

    // reject exception edges until the interpreter supports them
    for block_id in &func.blocks {
        let block = tree.get(*block_id);
        if matches!(
            block.terminator,
            mir::Terminator::Call { .. }
                | mir::Terminator::CallIndirect { .. }
                | mir::Terminator::CallVirtual { .. }
                | mir::Terminator::CallInterface { .. }
        ) {
            // TODO #Incomplete: support exception edges in VM
            return None;
        }
    }

    // prepare block index mapping
    let mut block_index_map: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut mir_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // seed traversal queue
    let mut queue = vec![entry_block];
    let mut visited = HashSet::new();

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
            mir::Terminator::Check {
                success, failure, ..
            } => {
                queue.push(success.target);
                queue.push(failure.target);
            }
            mir::Terminator::Switch { cases, default, .. } => {
                for case in cases {
                    queue.push(case.target);
                }
                queue.push(*default);
            }
            mir::Terminator::Yield { resume, .. } => {
                queue.push(*resume);
            }
            mir::Terminator::Call {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                queue.push(*normal_target);
                queue.push(*unwind_target);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Throw { .. }
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallInterface { .. } => {}
        }
    }

    // compute value kinds for typed dispatch
    let value_count = compute_value_count_from_mir(tree, func, &mir_blocks);
    let value_kinds = build_value_kinds(tree, func, &mir_blocks, value_count);
    let value_uses = compute_value_use_counts(tree, &mir_blocks, value_count);

    // build local id to local index mapping
    debug_assert!(
        func.locals.len() <= u32::MAX as usize,
        "too many locals for lowered function indices"
    );
    let mut local_index_by_id = HashMap::with_capacity(func.locals.len());
    for (index, local) in func.locals.iter().enumerate() {
        local_index_by_id.insert(*local, index as u32);
    }

    // validate lowered function indices
    debug_assert!(
        mir_blocks.len() <= u32::MAX as usize,
        "too many blocks for lowered block indices"
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

    // resolve entry block index
    let entry_index = block_index_map[&entry_block] as u32;

    // allocate lowered blocks
    let mut blocks = Vec::with_capacity(mir_blocks.len());

    // thread each mir block
    for mir_block_id in &mir_blocks {
        let block = tree.get(*mir_block_id);
        let block = lower_block(
            tree,
            *mir_block_id,
            block,
            &block_index_map,
            &block_parameters,
            &local_index_by_id,
            func_id,
            entry_index,
            yield_resume_points,
            function_indices,
            &value_kinds,
            &value_uses,
            &func.value_types,
            &mut argument_pool,
            &mut switch_case_pool,
            &mut copy_pool,
        );
        blocks.push(block);
    }

    // compute storage sizes
    let local_count = func.locals.len();

    // assemble lowered function
    Some(Function {
        frame_layout,
        parameters,
        entry: entry_index,
        blocks: blocks,
        argument_pool,
        switch_case_pool,
        copy_pool,
        value_count,
        local_count,
    })
}

/// Convert a MIR block to lowered interpreter form.
#[allow(clippy::too_many_arguments)]
fn lower_block(
    tree: &mir::NodeTree,
    mir_block: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    local_index_by_id: &HashMap<mir::LocalNodeId<mir::Local>, u32>,
    current_function: mir::LocalNodeId<mir::Function>,
    entry_block: u32,
    yield_resume_points: &HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
    value_types: &[mir::LocalNodeId<mir::Type>],
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
) -> Block {
    // preallocate instruction list
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);
    let mut mir_instruction_offsets = Vec::with_capacity(block.instructions.len() + 2);
    mir_instruction_offsets.push(0);

    // convert regular instructions
    let mut inst_index = 0usize;
    while inst_index < block.instructions.len() {
        // load current instruction
        let inst_id = block.instructions[inst_index];
        let inst = tree.get(inst_id);

        // attempt addr + load/store fusion
        if let Some((instruction, skip)) = try_fuse_addr_access(
            tree,
            inst,
            block.instructions.get(inst_index + 1).copied(),
            value_kinds,
            value_uses,
        ) {
            instructions.push(instruction);
            inst_index += skip;
            mir_instruction_offsets.push(inst_index as u32);
            continue;
        }

        // attempt const + binary fusion
        if let Some((instruction, skip)) = try_fuse_const_binary(
            tree,
            inst,
            block.instructions.get(inst_index + 1).copied(),
            value_kinds,
            value_uses,
        ) {
            instructions.push(instruction);
            inst_index += skip;
            mir_instruction_offsets.push(inst_index as u32);
            continue;
        }

        // fall back to standard threading
        let instruction = lower_instruction(
            tree,
            inst,
            value_kinds,
            value_types,
            argument_pool,
            copy_pool,
            function_indices,
            local_index_by_id,
        );
        instructions.push(instruction);
        inst_index += 1;
        mir_instruction_offsets.push(inst_index as u32);
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
        // append lowered terminator
        let terminator = lower_terminator(
            tree,
            &block.terminator,
            block_index_map,
            block_parameters,
            current_function,
            entry_block,
            mir_block,
            yield_resume_points,
            function_indices,
            value_kinds,
            argument_pool,
            switch_case_pool,
            copy_pool,
        );
        instructions.push(terminator);
    }
    mir_instruction_offsets.push((block.instructions.len() + 1) as u32);

    // compute original MIR instruction count (instructions + terminator)
    let mir_instruction_count = (block.instructions.len() + 1) as u32;

    // assemble block
    Block {
        mir_block,
        instructions,
        mir_instruction_offsets,
        mir_instruction_count,
    }
}

/// Try to fuse addr + load/store into a single lowered instruction.
fn try_fuse_addr_access(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
) -> Option<(Instruction, usize)> {
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
            ..
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
                    ..
                } if pointer == destination => Some((
                    Instruction {
                        operation: select_field_load_operation(value_kinds, *aggregate),
                        data: InstructionData::FieldLoad {
                            dest: *load_dest,
                            aggregate: *aggregate,
                            index: *index,
                            field_count,
                            managed_pointee: managed_pointee_type_for_value_kind(
                                value_kinds,
                                *aggregate,
                            ),
                            raw_pointee: raw_pointee_type_for_value_kind(value_kinds, *aggregate),
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    Instruction {
                        operation: select_field_store_operation(
                            value_kinds,
                            *aggregate,
                            field_count,
                            *index,
                        ),
                        data: InstructionData::FieldStore {
                            aggregate: *aggregate,
                            index: *index,
                            value: *value,
                            reference: reference_meta_for_value(value_kinds, *destination),
                            field_count,
                            managed_pointee: managed_pointee_type_for_value_kind(
                                value_kinds,
                                *aggregate,
                            ),
                            raw_pointee: raw_pointee_type_for_value_kind(value_kinds, *aggregate),
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
            ..
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
                    ..
                } if pointer == destination => Some((
                    Instruction {
                        operation: select_element_load_operation(value_kinds, *array),
                        data: InstructionData::ElementLoad {
                            dest: *load_dest,
                            array: *array,
                            index: *index,
                            array_length,
                            managed_pointee: managed_pointee_type_for_value_kind(
                                value_kinds,
                                *array,
                            ),
                            raw_pointee: raw_pointee_type_for_value_kind(value_kinds, *array),
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    Instruction {
                        operation: select_element_store_operation(value_kinds, *array),
                        data: InstructionData::ElementStore {
                            array: *array,
                            index: *index,
                            value: *value,
                            reference: reference_meta_for_value(value_kinds, *destination),
                            array_length,
                            managed_pointee: managed_pointee_type_for_value_kind(
                                value_kinds,
                                *array,
                            ),
                            raw_pointee: raw_pointee_type_for_value_kind(value_kinds, *array),
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
            ..
        } => {
            if !can_fuse(*destination) {
                return None;
            }

            let reference = reference_meta_for_value(value_kinds, *destination);

            match next_inst {
                mir::Instruction::Load {
                    destination: load_dest,
                    pointer,
                    ..
                } if pointer == destination => Some((
                    Instruction {
                        operation: InstructionOperation::GlobalLoad,
                        data: InstructionData::GlobalLoad {
                            dest: *load_dest,
                            global: global.id,
                        },
                    },
                    2,
                )),
                mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                    Instruction {
                        operation: InstructionOperation::GlobalStore,
                        data: InstructionData::GlobalStore {
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
) -> Option<(Instruction, usize)> {
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
        if let Some(operation) = select_specialized_const_int_operation(*operator, signed) {
            return Some((
                Instruction {
                    operation,
                    data: InstructionData::BinaryConstRightSpecialized {
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
        Instruction {
            operation: InstructionOperation::BinaryConstRight,
            data: InstructionData::BinaryConstRight {
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
    instructions: &mut Vec<Instruction>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    value_uses: &[u32],
    copy_pool: &mut Vec<CopyPair>,
) -> Option<Instruction> {
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

    // resolve const operand from the previous instruction
    let mut const_value = None;
    let mut left_value = *left;
    let mut operator = *operator;
    let mut pop_const = false;
    if let Some(prev_inst_id) = block
        .instructions
        .get(block.instructions.len().saturating_sub(2))
    {
        let prev_inst = tree.get(*prev_inst_id);
        if let mir::Instruction::Const { destination, value } = prev_inst {
            let uses = value_uses.get(destination.0 as usize).copied().unwrap_or(0);
            if uses == 1 {
                if *destination == *right {
                    const_value = Some(Value::from(value));
                    pop_const = true;
                } else if *destination == *left {
                    const_value = Some(Value::from(value));
                    left_value = *right;
                    operator = swap_compare_operator(operator);
                    pop_const = true;
                }
            }
        }
    }

    // remove the compare instruction (it's now fused)
    instructions.pop();
    if pop_const {
        instructions.pop();
    }

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
    if let Some(right_const) = const_value {
        let operation = select_compare_branch_const_operation(operator);
        return Some(Instruction {
            operation,
            data: InstructionData::CompareAndBranchConst {
                left: left_value,
                right_const,
                operator,
                then_target: then_index as u32,
                then_copies,
                else_target: else_index as u32,
                else_copies,
            },
        });
    }

    let operation = select_compare_branch_operation(operator);
    Some(Instruction {
        operation,
        data: InstructionData::CompareAndBranch {
            left: *left,
            right: *right,
            operator,
            then_target: then_index as u32,
            then_copies,
            else_target: else_index as u32,
            else_copies,
        },
    })
}

/// Pick a compare-and-branch handler based on operator type.
fn select_compare_branch_operation(operator: mir::BinaryOperator) -> InstructionOperation {
    match operator {
        // signed integer comparisons (most common in loops)
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual => InstructionOperation::CompareAndBranchInt,
        // unsigned integer comparisons
        mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual => InstructionOperation::CompareAndBranchUint,
        // float comparisons
        mir::BinaryOperator::FloatEqual
        | mir::BinaryOperator::FloatNotEqual
        | mir::BinaryOperator::FloatLessThan
        | mir::BinaryOperator::FloatLessEqual
        | mir::BinaryOperator::FloatGreaterThan
        | mir::BinaryOperator::FloatGreaterEqual => InstructionOperation::CompareAndBranchFloat,
        // fallback for non-comparison operators (should not happen)
        _ => InstructionOperation::CompareAndBranch,
    }
}

/// Pick a compare-and-branch handler for constant right operands.
fn select_compare_branch_const_operation(operator: mir::BinaryOperator) -> InstructionOperation {
    match operator {
        // signed integer comparisons (most common in loops)
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual => InstructionOperation::CompareAndBranchConstInt,
        // unsigned integer comparisons
        mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual => {
            InstructionOperation::CompareAndBranchConstUint
        }
        // float comparisons
        mir::BinaryOperator::FloatEqual
        | mir::BinaryOperator::FloatNotEqual
        | mir::BinaryOperator::FloatLessThan
        | mir::BinaryOperator::FloatLessEqual
        | mir::BinaryOperator::FloatGreaterThan
        | mir::BinaryOperator::FloatGreaterEqual => {
            InstructionOperation::CompareAndBranchConstFloat
        }
        // fallback for non-comparison operators (should not happen)
        _ => InstructionOperation::CompareAndBranchConst,
    }
}

/// Swap comparison operator when the constant appears on the left.
fn swap_compare_operator(operator: mir::BinaryOperator) -> mir::BinaryOperator {
    match operator {
        mir::BinaryOperator::Equal => mir::BinaryOperator::Equal,
        mir::BinaryOperator::NotEqual => mir::BinaryOperator::NotEqual,
        mir::BinaryOperator::SignedLessThan => mir::BinaryOperator::SignedGreaterThan,
        mir::BinaryOperator::SignedLessEqual => mir::BinaryOperator::SignedGreaterEqual,
        mir::BinaryOperator::SignedGreaterThan => mir::BinaryOperator::SignedLessThan,
        mir::BinaryOperator::SignedGreaterEqual => mir::BinaryOperator::SignedLessEqual,
        mir::BinaryOperator::UnsignedLessThan => mir::BinaryOperator::UnsignedGreaterThan,
        mir::BinaryOperator::UnsignedLessEqual => mir::BinaryOperator::UnsignedGreaterEqual,
        mir::BinaryOperator::UnsignedGreaterThan => mir::BinaryOperator::UnsignedLessThan,
        mir::BinaryOperator::UnsignedGreaterEqual => mir::BinaryOperator::UnsignedLessEqual,
        mir::BinaryOperator::FloatLessThan => mir::BinaryOperator::FloatGreaterThan,
        mir::BinaryOperator::FloatLessEqual => mir::BinaryOperator::FloatGreaterEqual,
        mir::BinaryOperator::FloatGreaterThan => mir::BinaryOperator::FloatLessThan,
        mir::BinaryOperator::FloatGreaterEqual => mir::BinaryOperator::FloatLessEqual,
        mir::BinaryOperator::FloatEqual => mir::BinaryOperator::FloatEqual,
        mir::BinaryOperator::FloatNotEqual => mir::BinaryOperator::FloatNotEqual,
        _ => operator,
    }
}

/// Convert a MIR instruction to lowered interpreter form.
#[allow(clippy::too_many_arguments)]
fn lower_instruction(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
    value_types: &[mir::LocalNodeId<mir::Type>],
    argument_pool: &mut Vec<mir::Value>,
    copy_pool: &mut Vec<CopyPair>,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    local_index_by_id: &HashMap<mir::LocalNodeId<mir::Local>, u32>,
) -> Instruction {
    // map instruction opcode to lowered form
    match inst {
        mir::Instruction::Const { destination, value } => {
            let const_value = if matches!(value, mir::Constant::Null) {
                let reference =
                    reference_meta_for_type(tree, value_type_for_value(*destination, value_types));
                let value = match reference.kind() {
                    Some(mir::ReferenceKind::Managed) => {
                        Value::managed_reference_with_meta(ManagedReference::NULL, reference)
                    }
                    _ if matches!(
                        reference.address_space(),
                        destack_heap::ReferenceAddressSpace::Shared
                    ) =>
                    {
                        Value::shared_pointer_with_meta(SharedPointer::NULL, reference)
                    }
                    _ => Value::raw_pointer_with_meta(RawPointer::NULL, reference),
                };
                ConstValue::Value(value)
            } else {
                ConstValue::Value(Value::from(value))
            };
            Instruction {
                operation: InstructionOperation::Const,
                data: InstructionData::Const {
                    dest: *destination,
                    value: const_value,
                },
            }
        }

        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => {
            let left_type = value_type_for_value(*left, value_types);
            if matches!(
                tree.get(left_type),
                mir::Type::Vector { .. } | mir::Type::Tensor { .. }
            ) {
                let result_type = value_type_for_value(*destination, value_types);
                return Instruction {
                    operation: InstructionOperation::BinaryElementwise,
                    data: InstructionData::BinaryElementwise {
                        dest: *destination,
                        op: *operator,
                        left: *left,
                        right: *right,
                        result_type,
                    },
                };
            }

            // check if we can use a specialized handler
            let kind = value_kinds.get(*left);
            if let Some(ValueKind::Int { signed, .. }) = kind
                && let Some(operation) = select_specialized_int_operation(*operator, signed)
            {
                return Instruction {
                    operation,
                    data: InstructionData::BinarySpecialized {
                        dest: *destination,
                        left: *left,
                        right: *right,
                    },
                };
            }

            // fall back to generic binary instruction
            Instruction {
                operation: select_binary_operation(value_kinds, *left, *operator),
                data: InstructionData::Binary {
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
        } => {
            let argument_type = value_type_for_value(*argument, value_types);
            if matches!(
                tree.get(argument_type),
                mir::Type::Vector { .. } | mir::Type::Tensor { .. }
            ) {
                let result_type = value_type_for_value(*destination, value_types);
                return Instruction {
                    operation: InstructionOperation::UnaryElementwise,
                    data: InstructionData::UnaryElementwise {
                        dest: *destination,
                        op: *operator,
                        arg: *argument,
                        result_type,
                    },
                };
            }

            Instruction {
                operation: select_unary_operation(value_kinds, *argument, *operator),
                data: InstructionData::Unary {
                    dest: *destination,
                    op: *operator,
                    arg: *argument,
                },
            }
        }

        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => Instruction {
            operation: InstructionOperation::Cast,
            data: InstructionData::Cast {
                dest: *destination,
                op: *operator,
                arg: *argument,
                to_type: to_type.id,
            },
        },

        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => Instruction {
            operation: InstructionOperation::Select,
            data: InstructionData::Select {
                dest: *destination,
                condition: *condition,
                then_value: *then_value,
                else_value: *else_value,
            },
        },

        mir::Instruction::Call {
            destination,
            function,
            arguments,
            ..
        } => {
            // resolve call arguments and copy plan
            let args = tree.get_arguments(*arguments);
            let args_range = push_argument_range(argument_pool, args);
            let callee = tree.get(*function);
            let copies = push_copy_range_from_params(copy_pool, &callee.parameters, args);
            let callee_index = lookup_function_index(function_indices, *function)
                .unwrap_or(INVALID_FUNCTION_INDEX);

            // assemble lowered call
            Instruction {
                operation: InstructionOperation::Call,
                data: InstructionData::Call {
                    dest: pack_optional_value(*destination),
                    function: function.id,
                    callee_index,
                    arguments: args_range,
                    copies,
                },
            }
        }

        mir::Instruction::CallVirtual {
            destination,
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            let args_range = push_argument_range(argument_pool, args);
            Instruction {
                operation: InstructionOperation::CallVirtual,
                data: InstructionData::CallVirtual {
                    dest: pack_optional_value(*destination),
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value(tree, value_types, *receiver),
                    slot_id: slot_id.0,
                    arguments: args_range,
                },
            }
        }

        mir::Instruction::CallInterface {
            destination,
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            let args_range = push_argument_range(argument_pool, args);
            Instruction {
                operation: InstructionOperation::CallInterface,
                data: InstructionData::CallInterface {
                    dest: pack_optional_value(*destination),
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value(tree, value_types, *receiver),
                    slot_id: slot_id.0,
                    arguments: args_range,
                },
            }
        }

        mir::Instruction::CallIndirect {
            destination,
            callee,
            env,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            Instruction {
                operation: InstructionOperation::CallIndirect,
                data: InstructionData::CallIndirect {
                    dest: pack_optional_value(*destination),
                    callee: *callee,
                    env: *env,
                    arguments: args,
                    cached_function: Cell::new(None),
                    cached_index: Cell::new(None),
                },
            }
        }

        mir::Instruction::LocalGet { destination, local } => {
            let local_index = match local_index_by_id.get(local) {
                Some(index) => *index,
                None => panic!("missing local index for {local:?}"),
            };
            Instruction {
                operation: InstructionOperation::LocalGet,
                data: InstructionData::LocalGet {
                    dest: *destination,
                    local: local_index,
                },
            }
        }

        mir::Instruction::LocalAddr {
            destination, local, ..
        } => {
            let local_index = match local_index_by_id.get(local) {
                Some(index) => *index,
                None => panic!("missing local index for {local:?}"),
            };
            Instruction {
                operation: InstructionOperation::LocalAddr,
                data: InstructionData::LocalAddr {
                    dest: *destination,
                    local: local_index,
                    reference: reference_meta_for_value(value_kinds, *destination),
                },
            }
        }

        mir::Instruction::LocalSet { local, value } => {
            let local_index = match local_index_by_id.get(local) {
                Some(index) => *index,
                None => panic!("missing local index for {local:?}"),
            };
            Instruction {
                operation: InstructionOperation::LocalSet,
                data: InstructionData::LocalSet {
                    local: local_index,
                    value: *value,
                },
            }
        }

        mir::Instruction::GlobalAddr {
            destination,
            global,
            ..
        } => Instruction {
            operation: InstructionOperation::GlobalAddr,
            data: InstructionData::GlobalAddr {
                dest: *destination,
                global: global.id,
                reference: reference_meta_for_value(value_kinds, *destination),
            },
        },

        mir::Instruction::GlobalConst {
            destination,
            global,
        } => Instruction {
            operation: InstructionOperation::GlobalConst,
            data: InstructionData::GlobalConst {
                dest: *destination,
                global: global.id,
            },
        },

        mir::Instruction::FunctionAddr {
            destination,
            function,
        } => Instruction {
            operation: InstructionOperation::FunctionAddr,
            data: InstructionData::FunctionAddr {
                dest: *destination,
                function: function.id,
            },
        },
        mir::Instruction::FunctionEnv { destination } => Instruction {
            operation: InstructionOperation::FunctionEnv,
            data: InstructionData::FunctionEnv { dest: *destination },
        },
        mir::Instruction::Load {
            destination,
            pointer,
            ..
        } => Instruction {
            operation: select_load_operation(value_kinds, *pointer),
            data: InstructionData::Load {
                dest: *destination,
                pointer: *pointer,
                managed_pointee: managed_pointee_type_for_value(tree, value_types, *pointer),
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::Store { pointer, value } => Instruction {
            operation: select_store_operation(value_kinds, *pointer),
            data: InstructionData::Store {
                pointer: *pointer,
                value: *value,
                reference: reference_meta_for_value(value_kinds, *pointer),
                managed_pointee: managed_pointee_type_for_value(tree, value_types, *pointer),
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::RawDrop { value } => Instruction {
            operation: InstructionOperation::RawDrop,
            data: InstructionData::RawDrop { value: *value },
        },

        mir::Instruction::StackDrop { value } => Instruction {
            operation: InstructionOperation::StackDrop,
            data: InstructionData::StackDrop { value: *value },
        },

        mir::Instruction::Assume { condition: _ } => Instruction {
            operation: InstructionOperation::Assume,
            data: InstructionData::Assume,
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
            Instruction {
                operation: select_field_get_operation(field_count, *index),
                data: InstructionData::FieldGet {
                    dest: *destination,
                    aggregate: *aggregate,
                    index: *index,
                },
            }
        }

        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            ..
        } => Instruction {
            operation: select_field_addr_operation(value_kinds, *aggregate),
            data: InstructionData::FieldAddr {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                reference: reference_meta_for_value(value_kinds, *destination),
                field_count: value_kinds
                    .get(*aggregate)
                    .and_then(|kind| field_count_from_kind(tree, kind))
                    .unwrap_or(UNKNOWN_FIELD_COUNT),
                managed_pointee: managed_pointee_type_for_value(tree, value_types, *aggregate),
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *aggregate),
            },
        },

        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => Instruction {
            operation: InstructionOperation::FieldSet,
            data: InstructionData::FieldSet {
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
        } => Instruction {
            operation: InstructionOperation::ElementGet,
            data: InstructionData::ElementGet {
                dest: *destination,
                array: *array,
                index: *index,
            },
        },

        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            ..
        } => Instruction {
            operation: select_element_addr_operation(value_kinds, *array),
            data: InstructionData::ElementAddr {
                dest: *destination,
                array: *array,
                index: *index,
                reference: reference_meta_for_value(value_kinds, *destination),
                array_length: value_kinds
                    .get(*array)
                    .and_then(|kind| array_length_from_kind(tree, kind))
                    .unwrap_or(UNKNOWN_ARRAY_LENGTH),
                managed_pointee: managed_pointee_type_for_value(tree, value_types, *array),
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *array),
            },
        },

        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => Instruction {
            operation: InstructionOperation::ElementSet,
            data: InstructionData::ElementSet {
                dest: *destination,
                array: *array,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::Struct {
            destination,
            fields,
            ..
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*fields));
            Instruction {
                operation: InstructionOperation::Aggregate,
                data: InstructionData::Aggregate {
                    dest: *destination,
                    elements: args,
                },
            }
        }

        mir::Instruction::Tuple {
            destination,
            elements,
            ..
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*elements));
            Instruction {
                operation: InstructionOperation::Aggregate,
                data: InstructionData::Aggregate {
                    dest: *destination,
                    elements: args,
                },
            }
        }

        mir::Instruction::Array {
            destination,
            elements,
            ..
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*elements));
            Instruction {
                operation: InstructionOperation::Aggregate,
                data: InstructionData::Aggregate {
                    dest: *destination,
                    elements: args,
                },
            }
        }

        mir::Instruction::VectorSplat { destination, value } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let mir::Type::Vector { lanes, .. } = tree.get(dest_type) else {
                panic!("expected vector type for {destination:?}");
            };
            Instruction {
                operation: InstructionOperation::VectorSplat,
                data: InstructionData::VectorSplat {
                    dest: *destination,
                    value: *value,
                    lanes: *lanes,
                },
            }
        }

        mir::Instruction::VectorExtract {
            destination,
            vector,
            index,
        } => Instruction {
            operation: InstructionOperation::VectorExtract,
            data: InstructionData::VectorExtract {
                dest: *destination,
                vector: *vector,
                index: *index,
            },
        },

        mir::Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        } => Instruction {
            operation: InstructionOperation::VectorInsert,
            data: InstructionData::VectorInsert {
                dest: *destination,
                vector: *vector,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        } => Instruction {
            operation: InstructionOperation::VectorShuffle,
            data: InstructionData::VectorShuffle {
                dest: *destination,
                left: *left,
                right: *right,
                mask: mask.clone(),
            },
        },
        mir::Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => Instruction {
            operation: InstructionOperation::VectorSelect,
            data: InstructionData::VectorSelect {
                dest: *destination,
                mask: *mask,
                then_value: *then_value,
                else_value: *else_value,
            },
        },

        mir::Instruction::VectorReduce {
            destination,
            operator,
            vector,
        } => Instruction {
            operation: InstructionOperation::VectorReduce,
            data: InstructionData::VectorReduce {
                dest: *destination,
                operator: *operator,
                vector: *vector,
            },
        },

        mir::Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        } => Instruction {
            operation: InstructionOperation::VectorCompare,
            data: InstructionData::VectorCompare {
                dest: *destination,
                operator: *operator,
                left: *left,
                right: *right,
            },
        },

        mir::Instruction::VectorConvert {
            destination,
            mode,
            vector,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*vector, value_types);
            Instruction {
                operation: InstructionOperation::VectorConvert,
                data: InstructionData::VectorConvert {
                    dest: *destination,
                    mode: *mode,
                    vector: *vector,
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorLoad {
            destination,
            view,
            indices,
        } => {
            let view_type = value_type_for_value(*view, value_types);
            let args = push_argument_range(argument_pool, tree.get_arguments(*indices));
            Instruction {
                operation: InstructionOperation::TensorLoad,
                data: InstructionData::TensorLoad {
                    dest: *destination,
                    view: *view,
                    indices: args,
                    view_type,
                },
            }
        }

        mir::Instruction::TensorStore {
            view,
            indices,
            value,
        } => {
            let view_type = value_type_for_value(*view, value_types);
            let args = push_argument_range(argument_pool, tree.get_arguments(*indices));
            Instruction {
                operation: InstructionOperation::TensorStore,
                data: InstructionData::TensorStore {
                    view: *view,
                    indices: args,
                    value: *value,
                    view_type,
                },
            }
        }

        mir::Instruction::TensorFill { view, value } => {
            let view_type = value_type_for_value(*view, value_types);
            Instruction {
                operation: InstructionOperation::TensorFill,
                data: InstructionData::TensorFill {
                    view: *view,
                    value: *value,
                    view_type,
                },
            }
        }

        mir::Instruction::TensorCopy { target, source } => {
            let target_type = value_type_for_value(*target, value_types);
            let source_type = value_type_for_value(*source, value_types);
            Instruction {
                operation: InstructionOperation::TensorCopy,
                data: InstructionData::TensorCopy {
                    target: *target,
                    source: *source,
                    target_type,
                    source_type,
                },
            }
        }

        mir::Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let args = push_argument_range(argument_pool, tree.get_arguments(*shape));
            Instruction {
                operation: InstructionOperation::TensorReshape,
                data: InstructionData::TensorReshape {
                    dest: *destination,
                    tensor: *tensor,
                    shape: args,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            Instruction {
                operation: InstructionOperation::TensorBroadcast,
                data: InstructionData::TensorBroadcast {
                    dest: *destination,
                    tensor: *tensor,
                    dimensions: dimensions.clone(),
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            Instruction {
                operation: InstructionOperation::TensorTranspose,
                data: InstructionData::TensorTranspose {
                    dest: *destination,
                    tensor: *tensor,
                    permutation: permutation.clone(),
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            Instruction {
                operation: InstructionOperation::TensorSlice,
                data: InstructionData::TensorSlice {
                    dest: *destination,
                    tensor: *tensor,
                    arguments: args,
                    offsets_count: *offsets_count,
                    sizes_count: *sizes_count,
                    strides_count: *strides_count,
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            Instruction {
                operation: InstructionOperation::TensorPad,
                data: InstructionData::TensorPad {
                    dest: *destination,
                    tensor: *tensor,
                    arguments: args,
                    low_count: *low_count,
                    high_count: *high_count,
                    interior_count: *interior_count,
                    value: *value,
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        } => {
            // resolve destination type
            let dest_type = value_type_for_value(*destination, value_types);

            // collect input values and types
            let tensor_values = tree.get_arguments(*tensors);
            let args = push_argument_range(argument_pool, tensor_values);

            let mut tensor_types = Vec::with_capacity(tensor_values.len());
            for value in tensor_values {
                let value_type = value_type_for_value(*value, value_types);
                tensor_types.push(value_type);
            }
            Instruction {
                operation: InstructionOperation::TensorConcat,
                data: InstructionData::TensorConcat {
                    dest: *destination,
                    tensors: args,
                    tensor_types,
                    axis: *axis,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            Instruction {
                operation: InstructionOperation::TensorReduce,
                data: InstructionData::TensorReduce {
                    dest: *destination,
                    operator: *operator,
                    tensor: *tensor,
                    initial: *initial,
                    axes: axes.clone(),
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let left_type = value_type_for_value(*left, value_types);
            let right_type = value_type_for_value(*right, value_types);
            Instruction {
                operation: InstructionOperation::TensorDot,
                data: InstructionData::TensorDot {
                    dest: *destination,
                    left: *left,
                    right: *right,
                    dimensions: dimensions.clone(),
                    left_type,
                    right_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let input_type = value_type_for_value(*input, value_types);
            let kernel_type = value_type_for_value(*kernel, value_types);
            Instruction {
                operation: InstructionOperation::TensorConvolution,
                data: InstructionData::TensorConvolution {
                    dest: *destination,
                    input: *input,
                    kernel: *kernel,
                    dimensions: dimensions.clone(),
                    window: window.clone(),
                    feature_group_count: *feature_group_count,
                    batch_group_count: *batch_group_count,
                    input_type,
                    kernel_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let operand_type = value_type_for_value(*operand, value_types);
            let indices_type = value_type_for_value(*indices, value_types);
            Instruction {
                operation: InstructionOperation::TensorGather,
                data: InstructionData::TensorGather {
                    dest: *destination,
                    operand: *operand,
                    indices: *indices,
                    dimensions: dimensions.clone(),
                    slice_sizes: slice_sizes.clone(),
                    operand_type,
                    indices_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let operand_type = value_type_for_value(*operand, value_types);
            let indices_type = value_type_for_value(*indices, value_types);
            let updates_type = value_type_for_value(*updates, value_types);
            Instruction {
                operation: InstructionOperation::TensorScatter,
                data: InstructionData::TensorScatter {
                    dest: *destination,
                    operand: *operand,
                    indices: *indices,
                    updates: *updates,
                    dimensions: dimensions.clone(),
                    mode: *mode,
                    operand_type,
                    indices_type,
                    updates_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let left_type = value_type_for_value(*left, value_types);
            let right_type = value_type_for_value(*right, value_types);
            Instruction {
                operation: InstructionOperation::TensorCompare,
                data: InstructionData::TensorCompare {
                    dest: *destination,
                    operator: *operator,
                    left: *left,
                    right: *right,
                    left_type,
                    right_type,
                    dest_type,
                },
            }
        }
        mir::Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            Instruction {
                operation: InstructionOperation::TensorSelect,
                data: InstructionData::TensorSelect {
                    dest: *destination,
                    mask: *mask,
                    then_value: *then_value,
                    else_value: *else_value,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        } => {
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*tensor, value_types);
            Instruction {
                operation: InstructionOperation::TensorConvert,
                data: InstructionData::TensorConvert {
                    dest: *destination,
                    mode: *mode,
                    tensor: *tensor,
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::TensorCast {
            destination,
            tensor,
        } => Instruction {
            operation: InstructionOperation::TensorCast,
            data: InstructionData::TensorCast {
                dest: *destination,
                tensor: *tensor,
            },
        },

        mir::Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            let dest_type = value_type_for_value(*destination, value_types);
            let source_type = value_type_for_value(*view, value_types);
            Instruction {
                operation: InstructionOperation::TensorView,
                data: InstructionData::TensorView {
                    dest: *destination,
                    view: *view,
                    arguments: args,
                    offsets_count: *offsets_count,
                    sizes_count: *sizes_count,
                    strides_count: *strides_count,
                    source_type,
                    dest_type,
                },
            }
        }

        mir::Instruction::ManagedAlloc {
            destination,
            layout,
            ..
        } => Instruction {
            operation: InstructionOperation::ManagedAlloc,
            data: InstructionData::ManagedAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                layout_id: tree.type_layout_id(*layout),
                byte_len: managed_byte_len_from_type(tree, *layout).unwrap_or(0),
                trace: managed_reference_map_from_type(tree, *layout)
                    .unwrap_or_else(ReferenceMap::empty),
            },
        },

        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
            ..
        } => Instruction {
            operation: InstructionOperation::ManagedAllocArray,
            data: InstructionData::ManagedAllocArray {
                dest: *destination,
                length: *length,
                reference: reference_meta_for_value(value_kinds, *destination),
                element_type: *element,
            },
        },

        mir::Instruction::RawAlloc {
            destination,
            layout,
            ..
        } => Instruction {
            operation: InstructionOperation::RawAlloc,
            data: InstructionData::RawAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                byte_len: raw_byte_len_from_type(tree, *layout).unwrap_or(0),
            },
        },

        mir::Instruction::RawFree { pointer } => Instruction {
            operation: InstructionOperation::RawFree,
            data: InstructionData::RawFree { pointer: *pointer },
        },

        mir::Instruction::StackAlloc {
            destination,
            layout,
            ..
        } => Instruction {
            operation: InstructionOperation::StackAlloc,
            data: InstructionData::StackAlloc {
                dest: *destination,
                reference: reference_meta_for_value(value_kinds, *destination),
                slot_count: slot_count_from_type(tree, *layout).unwrap_or(UNKNOWN_SLOT_COUNT),
            },
        },

        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            Instruction {
                operation: InstructionOperation::Intrinsic,
                data: InstructionData::Intrinsic {
                    dest: pack_optional_value(*destination),
                    intrinsic: *intrinsic,
                    arguments: args,
                },
            }
        }

        mir::Instruction::AtomicLoad {
            destination,
            pointer,
            ..
        } => Instruction {
            operation: InstructionOperation::AtomicLoad,
            data: InstructionData::AtomicLoad {
                dest: *destination,
                pointer: *pointer,
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::AtomicStore { pointer, value, .. } => Instruction {
            operation: InstructionOperation::AtomicStore,
            data: InstructionData::AtomicStore {
                pointer: *pointer,
                value: *value,
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::AtomicCompareExchange {
            destination,
            pointer,
            expected,
            new_value,
            ..
        } => Instruction {
            operation: InstructionOperation::AtomicCompareExchange,
            data: InstructionData::AtomicCompareExchange {
                dest: *destination,
                pointer: *pointer,
                expected: *expected,
                new_value: *new_value,
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::AtomicRmw {
            destination,
            operator,
            pointer,
            value,
            ..
        } => Instruction {
            operation: InstructionOperation::AtomicRmw,
            data: InstructionData::AtomicRmw {
                dest: *destination,
                operator: *operator,
                pointer: *pointer,
                value: *value,
                raw_pointee: raw_pointee_type_for_value(tree, value_types, *pointer),
            },
        },

        mir::Instruction::AtomicFence { .. } => Instruction {
            operation: InstructionOperation::AtomicFence,
            data: InstructionData::AtomicFence,
        },

        mir::Instruction::Barrier { .. } => Instruction {
            operation: InstructionOperation::Barrier,
            data: InstructionData::Barrier,
        },
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

    // seed explicit value types
    for (index, ty) in func.value_types.iter().enumerate() {
        let value = mir::Value(index as u32);
        value_kinds.set(value, kind_from_type(tree, *ty));
    }

    // seed function parameter kinds
    for param in &func.parameters {
        value_kinds.set(param.value, kind_from_type(tree, param.ty));
    }

    // seed block parameter kinds
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        for param in &block.parameters {
            let kind = kind_from_type(tree, param.ty);
            let block_kind = kind_for_block_param(kind);
            let next_kind = match value_kinds.get(param.value) {
                Some(existing) => merge_block_param_kind(existing, block_kind),
                None => block_kind,
            };

            value_kinds.set(param.value, next_kind);
        }
    }

    // iteratively infer instruction results
    let mut changed = true;
    while changed {
        // reset iteration flag
        changed = false;

        // propagate block parameter kinds from control flow edges
        if propagate_block_param_kinds(tree, mir_blocks, &mut value_kinds) {
            changed = true;
        }

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
                let Some(kind) =
                    infer_instruction_kind(tree, inst, &value_kinds, &func.value_types)
                else {
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

/// Get reference metadata for a type when available.
fn reference_meta_for_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> ReferenceMeta {
    match kind_from_type(tree, ty) {
        ValueKind::Pointer { reference, .. } => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Infer the value kind for a MIR instruction.
fn infer_instruction_kind(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kinds: &ValueKinds,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueKind> {
    // resolve instruction kind
    match inst {
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let ty = value_type_for_value(*destination, value_types);
                return Some(kind_from_type(tree, ty));
            }
            Some(kind_from_constant(value))
        }
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
        mir::Instruction::Select { then_value, .. } => value_kinds.get(*then_value),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            destination.as_ref()?;
            let func = tree.get(*function);
            Some(kind_from_type(tree, func.return_type))
        }
        mir::Instruction::CallVirtual {
            destination,
            signature,
            ..
        }
        | mir::Instruction::CallInterface {
            destination,
            signature,
            ..
        } => {
            destination.as_ref()?;
            let mir::Type::FunctionPointer { result, .. } = tree.get(*signature) else {
                return None;
            };
            Some(kind_from_type(tree, *result))
        }
        mir::Instruction::CallIndirect {
            destination,
            signature,
            ..
        } => {
            destination.as_ref()?;
            let mir::Type::FunctionPointer { result, .. } = tree.get(*signature) else {
                return None;
            };
            Some(kind_from_type(tree, *result))
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(*local);
            Some(kind_from_type(tree, local.ty))
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut kind = kind_from_type(tree, *result_type);
            let ValueKind::Pointer { storage, .. } = &mut kind else {
                return None;
            };
            *storage = PointerStorage::Local;
            Some(kind)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            Some(kind_from_type(tree, *result_type))
        }
        mir::Instruction::GlobalConst { global, .. } => {
            let global = tree.get(*global);
            Some(kind_from_type(tree, global.ty))
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(*function);
            Some(ValueKind::FunctionPointer {
                result: function.return_type,
            })
        }
        mir::Instruction::FunctionEnv { destination } => value_kinds.get(*destination),
        mir::Instruction::Load { result_type, .. } => Some(kind_from_type(tree, *result_type)),
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kinds.get(*aggregate)?;
            kind_from_field(tree, aggregate_kind, *index)
        }
        mir::Instruction::FieldAddr { result_type, .. } => Some(kind_from_type(tree, *result_type)),
        mir::Instruction::FieldSet { aggregate, .. } => value_kinds.get(*aggregate),
        mir::Instruction::ElementGet { array, .. } => {
            let array_kind = value_kinds.get(*array)?;
            kind_from_element(tree, array_kind)
        }
        mir::Instruction::ElementAddr { result_type, .. } => {
            Some(kind_from_type(tree, *result_type))
        }
        mir::Instruction::ElementSet { array, .. } => value_kinds.get(*array),
        mir::Instruction::Struct { ty, .. } => Some(kind_from_type(tree, *ty)),
        mir::Instruction::Tuple { ty, .. } => Some(kind_from_type(tree, *ty)),
        mir::Instruction::Array { ty, .. } => Some(kind_from_type(tree, *ty)),
        mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::TensorLoad { .. }
        | mir::Instruction::TensorStore { .. }
        | mir::Instruction::TensorFill { .. }
        | mir::Instruction::TensorCopy { .. }
        | mir::Instruction::TensorReshape { .. }
        | mir::Instruction::TensorBroadcast { .. }
        | mir::Instruction::TensorTranspose { .. }
        | mir::Instruction::TensorCast { .. }
        | mir::Instruction::TensorView { .. }
        | mir::Instruction::TensorSlice { .. }
        | mir::Instruction::TensorPad { .. }
        | mir::Instruction::TensorConcat { .. }
        | mir::Instruction::TensorReduce { .. }
        | mir::Instruction::TensorDot { .. }
        | mir::Instruction::TensorConvolution { .. }
        | mir::Instruction::TensorGather { .. }
        | mir::Instruction::TensorScatter { .. }
        | mir::Instruction::TensorCompare { .. }
        | mir::Instruction::TensorSelect { .. }
        | mir::Instruction::TensorConvert { .. } => None,
        mir::Instruction::ManagedAlloc { result_type, .. }
        | mir::Instruction::ManagedAllocArray { result_type, .. }
        | mir::Instruction::RawAlloc { result_type, .. }
        | mir::Instruction::StackAlloc { result_type, .. } => {
            Some(kind_from_type(tree, *result_type))
        }
        mir::Instruction::AtomicLoad { result_type, .. } => {
            Some(kind_from_type(tree, *result_type))
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let ty = value_type_for_value(*destination, value_types);
            Some(kind_from_type(tree, ty))
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_kind(tree, *intrinsic, *arguments, value_kinds),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::Barrier { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::RawDrop { .. }
        | mir::Instruction::StackDrop { .. }
        | mir::Instruction::Assume { .. } => None,
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
            width: usize::BITS as u8,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueKind::Int {
            width: usize::BITS as u8,
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
        mir::IntrinsicResultType::PointeeAndBool(_) => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::TypeDescriptor => Some(ValueKind::Unknown),
        mir::IntrinsicResultType::Explicit => Some(ValueKind::Unknown),
    }
}

/// Get the kind for a constant value.
fn kind_from_constant(constant: &mir::Constant) -> ValueKind {
    // map constant to value kind
    match constant {
        mir::Constant::Null => ValueKind::Unknown,
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
        mir::Type::Struct {
            fields,
            copyability: _,
        } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(kind_from_type(tree, field.ty))
        }
        mir::Type::Tuple {
            elements,
            copyability: _,
        } => {
            let field = elements.get(index as usize)?;
            Some(kind_from_type(tree, *field))
        }
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

/// Resolve the slot count for a concrete type layout.
fn slot_count_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> Option<u32> {
    // map types to slot counts
    match tree.get(ty) {
        mir::Type::Struct {
            fields,
            copyability: _,
        } => u32::try_from(fields.len()).ok(),
        mir::Type::Tuple {
            elements,
            copyability: _,
        } => u32::try_from(elements.len()).ok(),
        mir::Type::Array { length, .. } => u32::try_from(*length).ok(),
        mir::Type::Newtype { inner, .. } => slot_count_from_type(tree, *inner),
        mir::Type::FunctionValue { .. } => Some(2),
        mir::Type::Void => Some(1),
        mir::Type::Boolean
        | mir::Type::Int { .. }
        | mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::Float { .. }
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. }
        | mir::Type::TensorReference { .. } => Some(1),
    }
}

/// Resolve one raw pointee type from a pointer value when available.
fn raw_pointee_type_for_value_kind(
    value_kinds: &ValueKinds,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_kinds.get(value) {
        Some(ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Raw,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

fn managed_pointee_type_for_value_kind(
    value_kinds: &ValueKinds,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_kinds.get(value) {
        Some(ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Managed,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

fn managed_pointee_type_for_value(
    tree: &mir::NodeTree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = value_type_for_value(value, value_types);

    match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            pointee,
            ..
        } => Some(*pointee),
        mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            element,
            ..
        } => Some(*element),
        mir::Type::Newtype { inner, .. } => match tree.get(*inner) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                pointee,
                ..
            } => Some(*pointee),
            _ => None,
        },
        _ => None,
    }
}

fn raw_pointee_type_for_value(
    tree: &mir::NodeTree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = value_type_for_value(value, value_types);

    match tree.get(ty) {
        mir::Type::Reference {
            kind,
            address_space,
            pointee,
            ..
        } if matches!(
            pointer_storage_from_reference(*address_space, *kind),
            PointerStorage::Raw
        ) =>
        {
            Some(*pointee)
        }
        mir::Type::TensorReference {
            kind,
            address_space,
            element,
            ..
        } if matches!(
            pointer_storage_from_reference(*address_space, *kind),
            PointerStorage::Raw
        ) =>
        {
            Some(*element)
        }
        mir::Type::Newtype { inner, .. } => match tree.get(*inner) {
            mir::Type::Reference {
                kind,
                address_space,
                pointee,
                ..
            } if matches!(
                pointer_storage_from_reference(*address_space, *kind),
                PointerStorage::Raw
            ) =>
            {
                Some(*pointee)
            }
            _ => None,
        },
        _ => None,
    }
}

fn managed_byte_len_from_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<u32> {
    match tree.get(ty) {
        mir::Type::Void => Some(0),
        mir::Type::Boolean => Some(1),
        mir::Type::Int { width, .. } => Some((*width as u32).div_ceil(8)),
        mir::Type::Isize | mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            Some(tree.pointer_bytes() as u32)
        }
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            ..
        }
        | mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            ..
        } => Some(tree.data_layout.managed_reference_layout.bytes as u32),
        mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => Some(tree.pointer_bytes() as u32),
        mir::Type::Float { width } => Some((*width as u32).div_ceil(8)),
        mir::Type::Newtype { inner, .. } => managed_byte_len_from_type(tree, *inner),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => tree.type_layout(ty).map(|layout| layout.size),
    }
}

fn managed_reference_map_from_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<ReferenceMap> {
    match tree.get(ty) {
        mir::Type::Void
        | mir::Type::Boolean
        | mir::Type::Int { .. }
        | mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::Float { .. }
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::FunctionPointer { .. } => Some(ReferenceMap::empty()),
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            ..
        }
        | mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            ..
        } => Some(ReferenceMap::ReferenceOffsets { offsets: vec![0] }),
        mir::Type::Reference { .. } | mir::Type::TensorReference { .. } => {
            Some(ReferenceMap::empty())
        }
        mir::Type::Newtype { inner, .. } => managed_reference_map_from_type(tree, *inner),
        mir::Type::Struct { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => {
            let layout = tree.type_layout(ty)?;
            let mut offsets = Vec::new();

            for field in &layout.fields {
                append_managed_reference_offsets(tree, field.ty, field.offset, &mut offsets)?;
            }

            if offsets.is_empty() {
                Some(ReferenceMap::empty())
            } else {
                Some(ReferenceMap::ReferenceOffsets { offsets })
            }
        }
        mir::Type::Array {
            element, length, ..
        } => {
            let layout = tree.type_layout(ty)?;
            let mir::LayoutType::Array { element_stride, .. } = &layout.layout_type else {
                return None;
            };
            let mut offsets = Vec::new();
            append_managed_reference_offsets(tree, *element, 0, &mut offsets)?;

            if offsets.is_empty() {
                Some(ReferenceMap::empty())
            } else {
                Some(ReferenceMap::RepeatedReferenceOffsets {
                    count: *length as u32,
                    element_size: *element_stride,
                    offsets,
                })
            }
        }
    }
}

fn append_managed_reference_offsets(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    base_offset: u32,
    offsets: &mut Vec<u32>,
) -> Option<()> {
    match tree.get(ty) {
        mir::Type::Void
        | mir::Type::Boolean
        | mir::Type::Int { .. }
        | mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::Float { .. }
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::FunctionPointer { .. }
        | mir::Type::Reference { .. }
        | mir::Type::TensorReference { .. } => {
            if matches!(
                tree.get(ty),
                mir::Type::Reference {
                    kind: mir::ReferenceKind::Managed,
                    ..
                } | mir::Type::TensorReference {
                    kind: mir::ReferenceKind::Managed,
                    ..
                }
            ) {
                offsets.push(base_offset);
            }
        }
        mir::Type::Newtype { inner, .. } => {
            append_managed_reference_offsets(tree, *inner, base_offset, offsets)?;
        }
        mir::Type::Struct { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => {
            let layout = tree.type_layout(ty)?;
            for field in &layout.fields {
                append_managed_reference_offsets(
                    tree,
                    field.ty,
                    base_offset.saturating_add(field.offset),
                    offsets,
                )?;
            }
        }
        mir::Type::Array {
            element, length, ..
        } => {
            let layout = tree.type_layout(ty)?;
            let mir::LayoutType::Array { element_stride, .. } = &layout.layout_type else {
                return None;
            };

            for index in 0..*length as u32 {
                let element_base =
                    base_offset.saturating_add(index.saturating_mul(*element_stride));
                append_managed_reference_offsets(tree, *element, element_base, offsets)?;
            }
        }
    }

    Some(())
}

/// Resolve the byte size for one raw allocation type.
fn raw_byte_len_from_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> Option<u32> {
    match tree.get(ty) {
        mir::Type::Void => Some(0),
        mir::Type::Boolean => Some(1),
        mir::Type::Int { width, .. } => Some((*width as u32).div_ceil(8)),
        mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => Some(tree.pointer_bytes() as u32),
        mir::Type::Float { width } => Some((*width as u32).div_ceil(8)),
        mir::Type::Newtype { inner, .. } => raw_byte_len_from_type(tree, *inner),
        mir::Type::Array { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::FunctionValue { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => tree.type_layout(ty).map(|layout| layout.size),
    }
}

/// Resolve the field count for a struct or tuple kind.
fn field_count_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u32> {
    match kind {
        ValueKind::Aggregate { ty } => match tree.get(ty) {
            mir::Type::Struct {
                fields,
                copyability: _,
            } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct {
                fields,
                copyability: _,
            } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple {
                elements,
                copyability: _,
            } => u32::try_from(elements.len()).ok(),
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

    // detect contiguous argument ids
    let mut is_contiguous = true;
    let contiguous_start = arguments[0].0;
    for (offset, argument) in arguments.iter().enumerate() {
        let expected = contiguous_start + offset as u32;
        if argument.0 != expected {
            is_contiguous = false;
            break;
        }
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
        is_contiguous,
        contiguous_start: if is_contiguous { contiguous_start } else { 0 },
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

    // detect contiguous copy pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

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
        if index == 0 {
            contiguous_dest = param.0;
            contiguous_src = src;
            if src == INVALID_VALUE_ID {
                is_contiguous = false;
            }
        } else if is_contiguous {
            let expected_src = contiguous_src + index as u32;
            let expected_dest = contiguous_dest + index as u32;
            if src != expected_src || param.0 != expected_dest {
                is_contiguous = false;
            }
        }
        pool.push(CopyPair { dest: param.0, src });
    }

    // return range
    CopyRange {
        start: start as u32,
        len: parameters.len() as u32,
        is_contiguous,
        contiguous_src: if is_contiguous { contiguous_src } else { 0 },
        contiguous_dest: if is_contiguous { contiguous_dest } else { 0 },
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

    // detect contiguous copy pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

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
        if index == 0 {
            contiguous_dest = param.value.0;
            contiguous_src = src;
            if src == INVALID_VALUE_ID {
                is_contiguous = false;
            }
        } else if is_contiguous {
            let expected_src = contiguous_src + index as u32;
            let expected_dest = contiguous_dest + index as u32;
            if src != expected_src || param.value.0 != expected_dest {
                is_contiguous = false;
            }
        }
        pool.push(CopyPair {
            dest: param.value.0,
            src,
        });
    }

    // return range
    CopyRange {
        start: start as u32,
        len: parameters.len() as u32,
        is_contiguous,
        contiguous_src: if is_contiguous { contiguous_src } else { 0 },
        contiguous_dest: if is_contiguous { contiguous_dest } else { 0 },
    }
}

/// Resolve a lowered function index for the given function id.
fn lookup_function_index(
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    function: mir::LocalNodeId<mir::Function>,
) -> Option<u32> {
    function_indices.get(&function).copied()
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

/// Append a switch jump table to the pool when density is high enough.
fn push_switch_table_range(
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
    default_target: u32,
    default_copies: CopyRange,
) -> Option<(i64, SwitchRange)> {
    // bail if there are no cases
    if cases.is_empty() {
        return None;
    }

    // compute min and max case values
    let mut min_value = cases[0].value;
    let mut max_value = cases[0].value;
    for case in cases {
        min_value = min_value.min(case.value);
        max_value = max_value.max(case.value);
    }

    // compute range length with overflow protection
    let range_len = i128::from(max_value) - i128::from(min_value) + 1;
    if range_len <= 0 {
        return None;
    }
    if range_len > SWITCH_TABLE_MAX_RANGE as i128 {
        return None;
    }
    if range_len > u32::MAX as i128 {
        return None;
    }

    // require sufficient density
    let range_len = range_len as usize;
    let density = cases.len() as f64 / range_len as f64;
    if density < SWITCH_TABLE_MIN_DENSITY {
        return None;
    }

    // reserve table slots
    let start = switch_case_pool.len();
    debug_assert!(
        start + range_len <= u32::MAX as usize,
        "switch case pool overflow"
    );

    // seed with default targets
    for offset in 0..range_len {
        let value = min_value + offset as i64;
        switch_case_pool.push(SwitchCase {
            value,
            target: default_target,
            copies: default_copies,
        });
    }

    // populate explicit cases
    for case in cases {
        let target_index = block_index_map[&case.target];
        let target_parameters = block_parameters
            .get(target_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let copies = push_copy_range(copy_pool, target_parameters, &case.arguments);
        let offset = (case.value - min_value) as usize;
        let slot = &mut switch_case_pool[start + offset];
        slot.value = case.value;
        slot.target = target_index as u32;
        slot.copies = copies;
    }

    // return table range
    Some((
        min_value,
        SwitchRange {
            start: start as u32,
            len: range_len as u32,
        },
    ))
}

/// Convert a MIR terminator to lowered interpreter form.
#[allow(clippy::too_many_arguments)]
fn lower_terminator(
    tree: &mir::NodeTree,
    term: &mir::Terminator,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    current_function: mir::LocalNodeId<mir::Function>,
    entry_block: u32,
    current_block: mir::LocalNodeId<mir::Block>,
    yield_resume_points: &HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    value_kinds: &ValueKinds,
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
) -> Instruction {
    // map terminator opcode to lowered form
    match term {
        mir::Terminator::Return { value } => Instruction {
            operation: InstructionOperation::Return,
            data: InstructionData::Return {
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

            // assemble lowered jump
            Instruction {
                operation: InstructionOperation::Jump,
                data: InstructionData::Jump {
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

            // assemble lowered branch
            Instruction {
                operation: select_branch_operation(value_kinds, *condition),
                data: InstructionData::Branch {
                    condition: *condition,
                    then_target: then_index as u32,
                    then_copies,
                    else_target: else_index as u32,
                    else_copies,
                },
            }
        }

        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            // resolve check target parameters
            let success_index = block_index_map[&success.target];
            let failure_index = block_index_map[&failure.target];
            let success_parameters = block_parameters
                .get(success_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let failure_parameters = block_parameters
                .get(failure_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let success_copies = push_copy_range(copy_pool, success_parameters, &success.arguments);
            let failure_copies = push_copy_range(copy_pool, failure_parameters, &failure.arguments);

            // assemble lowered check
            Instruction {
                operation: select_branch_operation(value_kinds, *condition),
                data: InstructionData::Branch {
                    condition: *condition,
                    then_target: success_index as u32,
                    then_copies: success_copies,
                    else_target: failure_index as u32,
                    else_copies: failure_copies,
                },
            }
        }

        mir::Terminator::Switch {
            value,
            cases,
            default,
            default_arguments,
        } => {
            // resolve default block copies
            let default_index = block_index_map[default];
            let default_parameters = block_parameters
                .get(default_index)
                .map(|params| params.as_slice())
                .unwrap_or_default();
            let default_copies = push_copy_range(copy_pool, default_parameters, default_arguments);

            // try jump table for dense switches
            if let Some((min_value, table_range)) = push_switch_table_range(
                switch_case_pool,
                copy_pool,
                block_index_map,
                block_parameters,
                cases,
                default_index as u32,
                default_copies,
            ) {
                Instruction {
                    operation: select_switch_table_operation(value_kinds, *value),
                    data: InstructionData::SwitchTable {
                        value: *value,
                        min: min_value,
                        table: table_range,
                        default_target: default_index as u32,
                        default_copies,
                    },
                }
            }
            // fall back to linear scan
            else {
                let cases = push_switch_case_range(
                    switch_case_pool,
                    copy_pool,
                    block_index_map,
                    block_parameters,
                    cases,
                );
                Instruction {
                    operation: select_switch_operation(value_kinds, *value),
                    data: InstructionData::Switch {
                        value: *value,
                        cases,
                        default_target: default_index as u32,
                        default_copies,
                    },
                }
            }
        }

        mir::Terminator::Trap { kind, payload } => Instruction {
            operation: InstructionOperation::Trap,
            data: InstructionData::Trap {
                kind: *kind,
                payload: pack_optional_value(*payload),
            },
        },

        mir::Terminator::Unreachable => Instruction {
            operation: InstructionOperation::Unreachable,
            data: InstructionData::Unreachable,
        },

        mir::Terminator::Yield {
            value,
            resume: _,
            resume_arguments: _,
        } => {
            let resume_point = yield_resume_points
                .get(&current_block)
                .copied()
                .unwrap_or_else(|| {
                    panic!("missing yield resume point for block: {current_block:?}")
                });

            // assemble lowered yield
            Instruction {
                operation: InstructionOperation::Yield,
                data: InstructionData::Yield {
                    value: *value,
                    resume_point,
                },
            }
        }

        mir::Terminator::Throw { .. } => Instruction {
            operation: InstructionOperation::Unreachable,
            data: InstructionData::Unreachable,
        },

        // exception edges are rejected during threading, so reaching one here is a bug
        mir::Terminator::Call { .. }
        | mir::Terminator::CallIndirect { .. }
        | mir::Terminator::CallVirtual { .. }
        | mir::Terminator::CallInterface { .. } => {
            panic!("exceptional call terminators are not supported in the interpreter yet")
        }

        mir::Terminator::TailCall {
            function,
            arguments,
        } => {
            // fast path self tail calls by reusing the current frame
            if *function == current_function {
                let args = push_argument_range(argument_pool, arguments);
                Instruction {
                    operation: InstructionOperation::TailCallSelf,
                    data: InstructionData::TailCallSelf {
                        entry: entry_block,
                        arguments: args,
                    },
                }
            } else {
                // encode tail call metadata for the trampoline
                let callee = tree.get(*function);
                let copies = push_copy_range_from_params(copy_pool, &callee.parameters, arguments);
                let callee_index = lookup_function_index(function_indices, *function)
                    .unwrap_or(INVALID_FUNCTION_INDEX);

                Instruction {
                    operation: InstructionOperation::TailCall,
                    data: InstructionData::TailCall {
                        function: function.id,
                        callee_index,
                        copies,
                    },
                }
            }
        }

        mir::Terminator::TailCallIndirect {
            callee,
            env,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, arguments);
            Instruction {
                operation: InstructionOperation::TailCallIndirect,
                data: InstructionData::TailCallIndirect {
                    callee: *callee,
                    env: *env,
                    arguments: args,
                    cached_function: Cell::new(None),
                    cached_ptr: Cell::new(None),
                },
            }
        }

        mir::Terminator::TailCallVirtual {
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, arguments);
            Instruction {
                operation: InstructionOperation::TailCallVirtual,
                data: InstructionData::TailCallVirtual {
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value_kind(value_kinds, *receiver),
                    slot_id: slot_id.0,
                    arguments: args,
                },
            }
        }

        mir::Terminator::TailCallInterface {
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, arguments);
            Instruction {
                operation: InstructionOperation::TailCallInterface,
                data: InstructionData::TailCallInterface {
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value_kind(value_kinds, *receiver),
                    slot_id: slot_id.0,
                    arguments: args,
                },
            }
        }
    }
}
