use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use {destack_engine as engine, destack_mir as mir};

use destack_heap::{ManagedReference, RawPointer, ReferenceMeta, SharedPointer, Value};

use super::storage::{StorageLayout, repr_type};
use super::value::{PointerStorage, ValueKind, kind_from_type, pointer_storage_from_reference};
use super::{
    ArgumentRange, Block, ConstValue, CopyPair, CopyRange, ElementAccess, FieldAccess, Function,
    INVALID_FUNCTION_INDEX, INVALID_VALUE_ID, Instruction, InstructionData, InstructionOperation,
    SwitchCase, SwitchRange, TypedAccess, UNKNOWN_ARRAY_LENGTH, UNKNOWN_FIELD_COUNT,
    pack_optional_value,
};

// switch table density threshold
const SWITCH_TABLE_MIN_DENSITY: f64 = 0.5;
// cap the number of jump table entries
const SWITCH_TABLE_MAX_RANGE: usize = 2048;

/// One lowered runtime value slot.
#[derive(Clone, Copy, Debug)]
pub(super) struct LoweredValueSlot {
    /// The logical source carried by this slot.
    pub source: engine::FrameSlotSource,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// One recursively decomposed value slot tree.
#[derive(Clone, Debug)]
struct ComponentSlots {
    /// The slot id for this value or component.
    slot: mir::Value,
    /// The semantic type stored at this node.
    ty: mir::LocalNodeId<mir::Type>,
    /// The child components in semantic source order.
    components: Vec<ComponentSlots>,
}

/// One lowering-time decomposition table for semantic values.
#[derive(Clone, Debug, Default)]
pub(super) struct DeferredBlockParams {
    /// The decomposed value trees by root semantic value.
    by_value: HashMap<mir::Value, ComponentSlots>,
    /// The decomposed parameter trees by block and root parameter value.
    by_block: HashMap<mir::LocalNodeId<mir::Block>, HashMap<mir::Value, ComponentSlots>>,
    /// The block parameter roots that are carried decomposed.
    roots: HashSet<mir::Value>,
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

/// Pick a field get handler based on inferred aggregate storage.
fn select_field_get_operation(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
    field_count: u32,
    index: u32,
) -> InstructionOperation {
    match value_kinds.get(aggregate) {
        // small aggregates (≤2 fields) use inline slot storage
        Some(ValueKind::Composite { .. })
            if field_count > 0 && field_count <= 2 && index < field_count =>
        {
            InstructionOperation::FieldGetInline
        }
        Some(ValueKind::Composite { .. }) => InstructionOperation::FieldGet,
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

/// Pick an element get handler based on inferred array storage.
fn select_element_get_operation(
    value_kinds: &ValueKinds,
    array: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Composite { .. }) => {
            InstructionOperation::ElementGet
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

/// Pick a field address handler based on inferred aggregate storage.
fn select_field_addr_operation(
    value_kinds: &ValueKinds,
    aggregate: mir::Value,
) -> InstructionOperation {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Composite { .. }) => InstructionOperation::FieldAddrComposite,
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
        Some(ValueKind::Array { .. }) | Some(ValueKind::Composite { .. }) => {
            InstructionOperation::ElementAddrComposite
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
        Some(ValueKind::Composite { .. }) => InstructionOperation::FieldLoadComposite,
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
    // small managed composites (≤2 fields) use inline slot storage
    if field_count > 0
        && field_count <= 2
        && index < field_count
        && let Some(ValueKind::Composite { .. }) = value_kinds.get(aggregate)
    {
        return InstructionOperation::FieldStoreInline;
    }

    match value_kinds.get(aggregate) {
        Some(ValueKind::Composite { .. }) => InstructionOperation::FieldStoreComposite,
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
        Some(ValueKind::Array { .. }) | Some(ValueKind::Composite { .. }) => {
            InstructionOperation::ElementLoadComposite
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
        Some(ValueKind::Array { .. }) | Some(ValueKind::Composite { .. }) => {
            InstructionOperation::ElementStoreComposite
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

/// Return whether this type can stay decomposed across ordinary CFG edges.
fn can_cross_block_decompose_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    matches!(
        tree.get(repr_type(tree, ty)),
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } | mir::Type::Array { .. }
    )
}

/// Return the blocks whose entry parameters must stay materialized for resume.
fn resume_target_blocks(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::LocalNodeId<mir::Block>> {
    let mut blocks = HashSet::new();

    // reserve the semantic resume targets
    for block_id in &function.blocks {
        let block = tree.get(*block_id);

        match &block.terminator {
            mir::Terminator::Yield { resume, .. } => {
                blocks.insert(*resume);
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
                blocks.insert(*normal_target);
                blocks.insert(*unwind_target);
            }
            _ => {}
        }
    }

    blocks
}

/// Append one disaggregated hidden slot tree for the given component type.
fn push_hidden_value_tree(
    tree: &mir::NodeTree,
    owner: mir::Value,
    ty: mir::LocalNodeId<mir::Type>,
    slots: &mut Vec<LoweredValueSlot>,
) -> ComponentSlots {
    let slot = mir::Value::new(slots.len() as u32);
    let repr_ty = repr_type(tree, ty);

    // allocate this node first so nested materialization has one destination
    slots.push(LoweredValueSlot {
        source: engine::FrameSlotSource::DisaggregatedValue(owner),
        ty,
    });

    let components = match tree.get(repr_ty) {
        mir::Type::Struct { fields, .. } => fields
            .iter()
            .map(|field| push_hidden_value_tree(tree, owner, tree.get(*field).ty, slots))
            .collect(),
        mir::Type::Tuple { elements, .. } => elements
            .iter()
            .map(|element| push_hidden_value_tree(tree, owner, *element, slots))
            .collect(),
        mir::Type::Array {
            element, length, ..
        } => (0..usize::try_from(*length).unwrap_or(usize::MAX))
            .map(|_| push_hidden_value_tree(tree, owner, *element, slots))
            .collect(),
        _ => Vec::new(),
    };

    ComponentSlots {
        slot,
        ty,
        components,
    }
}

/// Append lowered hidden child slots for the given semantic type.
fn push_hidden_components(
    tree: &mir::NodeTree,
    owner: mir::Value,
    ty: mir::LocalNodeId<mir::Type>,
    slots: &mut Vec<LoweredValueSlot>,
) -> Vec<ComponentSlots> {
    match tree.get(repr_type(tree, ty)) {
        mir::Type::Struct { fields, .. } => fields
            .iter()
            .map(|field| push_hidden_value_tree(tree, owner, tree.get(*field).ty, slots))
            .collect(),
        mir::Type::Tuple { elements, .. } => elements
            .iter()
            .map(|element| push_hidden_value_tree(tree, owner, *element, slots))
            .collect(),
        mir::Type::Array {
            element, length, ..
        } => (0..usize::try_from(*length).unwrap_or(usize::MAX))
            .map(|_| push_hidden_value_tree(tree, owner, *element, slots))
            .collect(),
        _ => Vec::new(),
    }
}

/// Analyze lowered runtime value slots and decomposed block parameters for one function.
pub(super) fn analyze_lowered_value_slots(
    tree: &mir::NodeTree,
    function: &mir::Function,
) -> (Vec<LoweredValueSlot>, DeferredBlockParams) {
    let mut slots = function
        .value_types
        .iter()
        .enumerate()
        .map(|(index, ty)| LoweredValueSlot {
            source: engine::FrameSlotSource::Value(mir::Value::new(index as u32)),
            ty: *ty,
        })
        .collect::<Vec<_>>();
    let mut decomposed = DeferredBlockParams::default();
    let resume_blocks = resume_target_blocks(function, tree);

    // reserve hidden child slots for all decomposable semantic values
    for (index, ty) in function.value_types.iter().enumerate() {
        let value = mir::Value::new(index as u32);
        if !can_cross_block_decompose_type(tree, *ty) {
            continue;
        }

        let components = push_hidden_components(tree, value, *ty, &mut slots);
        decomposed.by_value.insert(
            value,
            ComponentSlots {
                slot: value,
                ty: *ty,
                components,
            },
        );
    }

    // reserve hidden child slots for ordinary CFG block parameters
    for block_id in &function.blocks {
        if function.entry == Some(*block_id) || resume_blocks.contains(block_id) {
            continue;
        }

        let block = tree.get(*block_id);
        let mut params = HashMap::new();

        for parameter in &block.parameters {
            if !can_cross_block_decompose_type(tree, parameter.ty) {
                continue;
            }

            let components = decomposed
                .by_value
                .get(&parameter.value)
                .cloned()
                .unwrap_or_else(|| ComponentSlots {
                    slot: parameter.value,
                    ty: parameter.ty,
                    components: push_hidden_components(
                        tree,
                        parameter.value,
                        parameter.ty,
                        &mut slots,
                    ),
                });

            params.insert(parameter.value, components);
            decomposed.roots.insert(parameter.value);
        }

        if !params.is_empty() {
            decomposed.by_block.insert(*block_id, params);
        }
    }

    (slots, decomposed)
}

/// Lower a MIR function into the interpreter function form.
pub(super) fn lower_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
    frame_layout: destack_engine::FrameLayoutId,
    yield_resume_points: &HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    exceptional_call_resume_points: &HashMap<
        mir::LocalNodeId<mir::Block>,
        (destack_engine::ResumePointId, destack_engine::ResumePointId),
    >,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    value_slots: &[LoweredValueSlot],
    deferred_block_params: &DeferredBlockParams,
) -> Option<Function> {
    // load function
    let func = tree.get(func_id);

    // imported functions can't be lowered here
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
    let value_count = value_slots.len();
    let value_types = value_slots.iter().map(|slot| slot.ty).collect::<Vec<_>>();
    let value_kinds = build_value_kinds(tree, func, &mir_blocks, &value_types, value_count);
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
            exceptional_call_resume_points,
            function_indices,
            &value_kinds,
            &value_uses,
            &value_types,
            &mut argument_pool,
            &mut switch_case_pool,
            &mut copy_pool,
            storage_layouts,
            deferred_block_params,
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
        blocks,
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
    exceptional_call_resume_points: &HashMap<
        mir::LocalNodeId<mir::Block>,
        (destack_engine::ResumePointId, destack_engine::ResumePointId),
    >,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
    value_types: &[mir::LocalNodeId<mir::Type>],
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    deferred_block_params: &DeferredBlockParams,
) -> Block {
    // preallocate instruction list
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);
    let mut mir_instruction_offsets = Vec::with_capacity(block.instructions.len() + 2);
    mir_instruction_offsets.push(0);
    let mut deferred_composites = HashMap::new();

    // seed block-entry decomposed parameters from their hidden child slots
    if let Some(parameters) = deferred_block_params.by_block.get(&mir_block) {
        for tree in parameters.values() {
            seed_component_slots(tree, &mut deferred_composites);
        }
    }

    // convert regular instructions
    let mut inst_index = 0usize;
    while inst_index < block.instructions.len() {
        // load current instruction
        let inst_id = block.instructions[inst_index];
        let inst = tree.get(inst_id);

        // keep pure struct, tuple, and array values decomposed until a place is required
        if try_lower_deferred_composite_instruction(
            tree,
            inst,
            value_types,
            argument_pool,
            deferred_block_params,
            &mut deferred_composites,
            &mut instructions,
        ) {
            inst_index += 1;
            mir_instruction_offsets.push(instructions.len() as u32);
            continue;
        }

        // flush deferred composites before any instruction that may require a place
        flush_deferred_composites(&mut instructions, argument_pool, &mut deferred_composites);

        // attempt addr + load/store fusion
        if let Some((instruction, skip)) = try_fuse_addr_access(
            tree,
            inst,
            block.instructions.get(inst_index + 1).copied(),
            value_kinds,
            value_uses,
            storage_layouts,
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
            storage_layouts,
        );
        instructions.push(instruction);
        inst_index += 1;
        mir_instruction_offsets.push(inst_index as u32);
    }

    // try to fuse compare + branch
    let allow_compare_branch_fusion = deferred_composites.is_empty()
        && match &block.terminator {
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                !deferred_block_params.by_block.contains_key(then_target)
                    && !deferred_block_params.by_block.contains_key(else_target)
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                !deferred_block_params.by_block.contains_key(&success.target)
                    && !deferred_block_params.by_block.contains_key(&failure.target)
            }
            _ => true,
        };
    if allow_compare_branch_fusion
        && let Some(fused) = try_fuse_compare_branch(
            tree,
            block,
            &mut instructions,
            block_index_map,
            block_parameters,
            value_uses,
            copy_pool,
        )
    {
        instructions.push(fused);
    } else {
        let requires_materialized_boundary = matches!(
            block.terminator,
            mir::Terminator::Return { .. }
                | mir::Terminator::Throw { .. }
                | mir::Terminator::Trap { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::Yield { .. }
                | mir::Terminator::Call { .. }
                | mir::Terminator::CallIndirect { .. }
                | mir::Terminator::CallVirtual { .. }
                | mir::Terminator::CallInterface { .. }
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. }
                | mir::Terminator::TailCallVirtual { .. }
                | mir::Terminator::TailCallInterface { .. }
        );

        if requires_materialized_boundary {
            flush_deferred_composites(&mut instructions, argument_pool, &mut deferred_composites);
        }

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
            exceptional_call_resume_points,
            function_indices,
            value_kinds,
            value_types,
            argument_pool,
            switch_case_pool,
            copy_pool,
            deferred_block_params,
            &deferred_composites,
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

/// One block-local pure composite value kept decomposed in lowering.
#[derive(Clone, Debug)]
struct DeferredComposite {
    /// The semantic component values in source order.
    elements: Vec<mir::Value>,
}

/// One deferred dynamic element.get plan.
struct IndexSelectPlan {
    /// The destination slot to write.
    dest: mir::Value,
    /// The candidate values in source order.
    elements: Vec<mir::Value>,
}

/// One deferred dynamic element.set plan for one slot.
struct SelectByIndexPlan {
    /// The destination slot to write.
    dest: mir::Value,
    /// The matching array index for this destination.
    match_index: u64,
    /// The replacement value when the index matches.
    then_value: mir::Value,
    /// The original value when the index does not match.
    else_value: mir::Value,
}

/// Seed one decomposed slot tree into the deferred composite map.
fn seed_component_slots(
    tree: &ComponentSlots,
    deferred_composites: &mut HashMap<mir::Value, DeferredComposite>,
) {
    if tree.components.is_empty() {
        return;
    }

    deferred_composites.insert(
        tree.slot,
        DeferredComposite {
            elements: tree
                .components
                .iter()
                .map(|component| component.slot)
                .collect(),
        },
    );

    for component in &tree.components {
        seed_component_slots(component, deferred_composites);
    }
}

/// Return whether this semantic type can stay decomposed during lowering.
fn can_defer_composite_type(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    matches!(
        tree.get(repr_type(tree, ty)),
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } | mir::Type::Array { .. }
    )
}

/// Project one deferred composite value through a constant component path.
fn project_deferred_value_path(
    value: mir::Value,
    path: &[usize],
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
) -> Option<mir::Value> {
    let mut current = value;

    for index in path {
        let composite = deferred_composites.get(&current)?;
        current = *composite.elements.get(*index)?;
    }

    Some(current)
}

/// Collect one dynamic index-select plan for a destination component tree.
fn collect_index_select_plan(
    target: &ComponentSlots,
    source_elements: &[mir::Value],
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
    path: &mut Vec<usize>,
    plans: &mut Vec<IndexSelectPlan>,
) -> bool {
    if target.components.is_empty() {
        let mut elements = Vec::with_capacity(source_elements.len());

        for source in source_elements {
            let Some(projected) = project_deferred_value_path(*source, path, deferred_composites)
            else {
                return false;
            };
            elements.push(projected);
        }

        plans.push(IndexSelectPlan {
            dest: target.slot,
            elements,
        });
        return true;
    }

    for (index, component) in target.components.iter().enumerate() {
        path.push(index);

        let is_valid =
            collect_index_select_plan(component, source_elements, deferred_composites, path, plans);

        path.pop();

        if !is_valid {
            return false;
        }
    }

    true
}

/// Collect one dynamic element-set plan for a destination component tree.
fn collect_select_by_index_plan(
    target: &ComponentSlots,
    source_value: mir::Value,
    replacement_value: mir::Value,
    match_index: u64,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
    path: &mut Vec<usize>,
    plans: &mut Vec<SelectByIndexPlan>,
) -> bool {
    if target.components.is_empty() {
        let Some(then_value) =
            project_deferred_value_path(replacement_value, path, deferred_composites)
        else {
            return false;
        };
        let Some(else_value) = project_deferred_value_path(source_value, path, deferred_composites)
        else {
            return false;
        };

        plans.push(SelectByIndexPlan {
            dest: target.slot,
            match_index,
            then_value,
            else_value,
        });
        return true;
    }

    for (index, component) in target.components.iter().enumerate() {
        path.push(index);

        let is_valid = collect_select_by_index_plan(
            component,
            source_value,
            replacement_value,
            match_index,
            deferred_composites,
            path,
            plans,
        );

        path.pop();

        if !is_valid {
            return false;
        }
    }

    true
}

/// Emit one collected dynamic element.get plan.
fn emit_index_select_plan(
    instructions: &mut Vec<Instruction>,
    argument_pool: &mut Vec<mir::Value>,
    index: mir::Value,
    plans: Vec<IndexSelectPlan>,
) {
    for plan in plans {
        let elements = push_argument_range(argument_pool, &plan.elements);

        instructions.push(Instruction {
            operation: InstructionOperation::IndexSelect,
            data: InstructionData::IndexSelect {
                dest: plan.dest,
                index,
                elements,
            },
        });
    }
}

/// Emit one collected dynamic element.set plan.
fn emit_select_by_index_plan(
    instructions: &mut Vec<Instruction>,
    index: mir::Value,
    plans: Vec<SelectByIndexPlan>,
) {
    for plan in plans {
        instructions.push(Instruction {
            operation: InstructionOperation::SelectByIndex,
            data: InstructionData::SelectByIndex {
                dest: plan.dest,
                index,
                match_index: plan.match_index,
                then_value: plan.then_value,
                else_value: plan.else_value,
            },
        });
    }
}

/// Try to lower one instruction through the block-local deferred composite model.
fn try_lower_deferred_composite_instruction(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_types: &[mir::LocalNodeId<mir::Type>],
    argument_pool: &mut Vec<mir::Value>,
    deferred_block_params: &DeferredBlockParams,
    deferred_composites: &mut HashMap<mir::Value, DeferredComposite>,
    instructions: &mut Vec<Instruction>,
) -> bool {
    match inst {
        // pure construction
        mir::Instruction::Struct {
            destination,
            fields,
            ..
        } => {
            deferred_composites.insert(
                *destination,
                DeferredComposite {
                    elements: tree.get_arguments(*fields).to_vec(),
                },
            );

            true
        }
        mir::Instruction::Tuple {
            destination,
            elements,
            ..
        } => {
            deferred_composites.insert(
                *destination,
                DeferredComposite {
                    elements: tree.get_arguments(*elements).to_vec(),
                },
            );

            true
        }
        mir::Instruction::Array {
            destination,
            elements,
            ..
        } => {
            deferred_composites.insert(
                *destination,
                DeferredComposite {
                    elements: tree.get_arguments(*elements).to_vec(),
                },
            );

            true
        }

        // pure projection
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => {
            let Some(aggregate) = deferred_composites.get(aggregate) else {
                return false;
            };
            let Some(source) = aggregate.elements.get(*index as usize).copied() else {
                return false;
            };
            let destination_type = value_type_for_value(*destination, value_types);

            if can_defer_composite_type(tree, destination_type)
                && let Some(source_composite) = deferred_composites.get(&source).cloned()
            {
                deferred_composites.insert(*destination, source_composite);
                return true;
            }

            instructions.push(Instruction {
                operation: InstructionOperation::Copy,
                data: InstructionData::Copy {
                    dest: *destination,
                    source,
                },
            });

            true
        }
        // pure updates
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => {
            let Some(aggregate) = deferred_composites.get(aggregate).cloned() else {
                return false;
            };
            if aggregate.elements.get(*index as usize).is_none() {
                return false;
            }

            let mut elements = aggregate.elements;
            elements[*index as usize] = *value;

            deferred_composites.insert(*destination, DeferredComposite { elements });

            true
        }
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => {
            let Some(array) = deferred_composites.get(array) else {
                return false;
            };

            let destination_type = value_type_for_value(*destination, value_types);

            if can_defer_composite_type(tree, destination_type) {
                let Some(destination_tree) = deferred_block_params.by_value.get(destination) else {
                    return false;
                };

                let mut plans = Vec::new();
                let mut path = Vec::new();
                let is_valid = collect_index_select_plan(
                    destination_tree,
                    &array.elements,
                    deferred_composites,
                    &mut path,
                    &mut plans,
                );
                if !is_valid {
                    return false;
                }

                emit_index_select_plan(instructions, argument_pool, *index, plans);
                seed_component_slots(destination_tree, deferred_composites);
                return true;
            }

            let elements = push_argument_range(argument_pool, &array.elements);
            instructions.push(Instruction {
                operation: InstructionOperation::IndexSelect,
                data: InstructionData::IndexSelect {
                    dest: *destination,
                    index: *index,
                    elements,
                },
            });

            true
        }
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => {
            let Some(array) = deferred_composites.get(array) else {
                return false;
            };
            let Some(destination_tree) = deferred_block_params.by_value.get(destination) else {
                return false;
            };

            if destination_tree.components.len() != array.elements.len() {
                return false;
            }

            let mut plans = Vec::new();

            for (match_index, (destination_element, source_element)) in destination_tree
                .components
                .iter()
                .zip(&array.elements)
                .enumerate()
            {
                let mut path = Vec::new();
                let is_valid = collect_select_by_index_plan(
                    destination_element,
                    *source_element,
                    *value,
                    match_index as u64,
                    deferred_composites,
                    &mut path,
                    &mut plans,
                );
                if !is_valid {
                    return false;
                }
            }

            emit_select_by_index_plan(instructions, *index, plans);
            seed_component_slots(destination_tree, deferred_composites);
            true
        }

        _ => false,
    }
}

/// Flush all deferred composite values into explicit composite instructions.
fn flush_deferred_composites(
    instructions: &mut Vec<Instruction>,
    argument_pool: &mut Vec<mir::Value>,
    deferred_composites: &mut HashMap<mir::Value, DeferredComposite>,
) {
    // collect roots before recursive emission mutates the map
    let roots = deferred_composites.keys().copied().collect::<Vec<_>>();
    let mut emitted = HashSet::new();

    // emit each deferred value in dependency order
    for value in roots {
        emit_deferred_composite(
            value,
            deferred_composites,
            instructions,
            argument_pool,
            &mut emitted,
        );
    }

    // clear the block-local deferred state after emission
    deferred_composites.clear();
}

/// Emit one deferred composite value and all deferred dependencies it references.
fn emit_deferred_composite(
    value: mir::Value,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
    instructions: &mut Vec<Instruction>,
    argument_pool: &mut Vec<mir::Value>,
    emitted: &mut HashSet<mir::Value>,
) {
    // skip already emitted or already materialized values
    if emitted.contains(&value) {
        return;
    }
    let Some(composite) = deferred_composites.get(&value) else {
        return;
    };

    // materialize deferred children before this parent
    for element in &composite.elements {
        emit_deferred_composite(
            *element,
            deferred_composites,
            instructions,
            argument_pool,
            emitted,
        );
    }

    // emit one explicit materialization at the boundary
    let elements = push_argument_range(argument_pool, &composite.elements);
    instructions.push(Instruction {
        operation: InstructionOperation::Composite,
        data: InstructionData::Composite {
            dest: value,
            elements,
        },
    });
    emitted.insert(value);
}

/// Try to fuse addr + load/store into a single lowered instruction.
fn try_fuse_addr_access(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    value_kinds: &ValueKinds,
    value_uses: &[u32],
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
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

            // precompute the field access descriptor when the pointee is known
            let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *aggregate)
                .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *aggregate));
            let field = pointee_type.and_then(|pointee_type| {
                field_access_for_pointee(storage_layouts, pointee_type, *index)
            });

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
                            composite: *aggregate,
                            index: *index,
                            field_count,
                            field,
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
                            composite: *aggregate,
                            index: *index,
                            value: *value,
                            reference: reference_meta_for_value(value_kinds, *destination),
                            field_count,
                            field,
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

            // precompute the element access descriptor when the pointee is known
            let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *array)
                .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *array));
            let element = pointee_type
                .and_then(|pointee_type| element_access_for_pointee(storage_layouts, pointee_type));

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
                            element,
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
                            element,
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
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
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
            signature,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, tree.get_arguments(*arguments));
            Instruction {
                operation: InstructionOperation::CallIndirect,
                data: InstructionData::CallIndirect {
                    dest: pack_optional_value(*destination),
                    callee: *callee,
                    signature: *signature,
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
        mir::Instruction::FunctionValue {
            destination,
            function,
            environment,
        } => Instruction {
            operation: InstructionOperation::FunctionValue,
            data: InstructionData::FunctionValue {
                dest: *destination,
                function: function.id,
                environment: *environment,
            },
        },
        mir::Instruction::FunctionEnvironment { destination } => Instruction {
            operation: InstructionOperation::FunctionEnvironment,
            data: InstructionData::FunctionEnvironment { dest: *destination },
        },
        mir::Instruction::Load {
            destination,
            pointer,
            ..
        } => Instruction {
            operation: select_load_operation(value_kinds, *pointer),
            data: {
                // resolve one compiled typed access descriptor when the pointee is known
                let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *pointer)
                    .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *pointer))
                    .or_else(|| managed_pointee_type_for_value(tree, value_types, *pointer))
                    .or_else(|| raw_pointee_type_for_value(tree, value_types, *pointer));
                let access = pointee_type.and_then(|pointee_type| {
                    typed_access_for_pointee(storage_layouts, pointee_type)
                });

                InstructionData::Load {
                    dest: *destination,
                    pointer: *pointer,
                    access,
                }
            },
        },

        mir::Instruction::Store { pointer, value } => Instruction {
            operation: select_store_operation(value_kinds, *pointer),
            data: {
                // resolve one compiled typed access descriptor when the pointee is known
                let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *pointer)
                    .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *pointer))
                    .or_else(|| managed_pointee_type_for_value(tree, value_types, *pointer))
                    .or_else(|| raw_pointee_type_for_value(tree, value_types, *pointer));
                let access = pointee_type.and_then(|pointee_type| {
                    typed_access_for_pointee(storage_layouts, pointee_type)
                });

                InstructionData::Store {
                    pointer: *pointer,
                    value: *value,
                    reference: reference_meta_for_value(value_kinds, *pointer),
                    access,
                }
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
            let operation =
                select_field_get_operation(value_kinds, *aggregate, field_count, *index);

            // resolve one compiled field descriptor when the pointee type is known
            let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *aggregate)
                .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *aggregate))
                .or_else(|| managed_pointee_type_for_value(tree, value_types, *aggregate))
                .or_else(|| raw_pointee_type_for_value(tree, value_types, *aggregate));
            let field = pointee_type.and_then(|pointee_type| {
                field_access_for_pointee(storage_layouts, pointee_type, *index)
            });

            match operation {
                InstructionOperation::FieldGet | InstructionOperation::FieldGetInline => {
                    Instruction {
                        operation,
                        data: InstructionData::FieldGet {
                            dest: *destination,
                            composite: *aggregate,
                            index: *index,
                        },
                    }
                }
                _ => Instruction {
                    operation,
                    data: InstructionData::FieldLoad {
                        dest: *destination,
                        composite: *aggregate,
                        index: *index,
                        field_count,
                        field,
                    },
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
            data: {
                // resolve one compiled field descriptor when the pointee type is known
                let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *aggregate)
                    .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *aggregate))
                    .or_else(|| managed_pointee_type_for_value(tree, value_types, *aggregate))
                    .or_else(|| raw_pointee_type_for_value(tree, value_types, *aggregate));
                let field = pointee_type.and_then(|pointee_type| {
                    field_access_for_pointee(storage_layouts, pointee_type, *index)
                });

                InstructionData::FieldAddr {
                    dest: *destination,
                    composite: *aggregate,
                    index: *index,
                    reference: reference_meta_for_value(value_kinds, *destination),
                    field_count: value_kinds
                        .get(*aggregate)
                        .and_then(|kind| field_count_from_kind(tree, kind))
                        .unwrap_or(UNKNOWN_FIELD_COUNT),
                    field,
                }
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
                composite: *aggregate,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => {
            let operation = select_element_get_operation(value_kinds, *array);
            let array_length = value_kinds
                .get(*array)
                .and_then(|kind| array_length_from_kind(tree, kind))
                .unwrap_or(UNKNOWN_ARRAY_LENGTH);

            // resolve one compiled element descriptor when the pointee type is known
            let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *array)
                .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *array))
                .or_else(|| managed_pointee_type_for_value(tree, value_types, *array))
                .or_else(|| raw_pointee_type_for_value(tree, value_types, *array));
            let element = pointee_type
                .and_then(|pointee_type| element_access_for_pointee(storage_layouts, pointee_type));

            match operation {
                InstructionOperation::ElementGet => Instruction {
                    operation,
                    data: InstructionData::ElementGet {
                        dest: *destination,
                        array: *array,
                        index: *index,
                    },
                },
                _ => Instruction {
                    operation,
                    data: InstructionData::ElementLoad {
                        dest: *destination,
                        array: *array,
                        index: *index,
                        array_length,
                        element,
                    },
                },
            }
        }

        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            ..
        } => Instruction {
            operation: select_element_addr_operation(value_kinds, *array),
            data: {
                // resolve one compiled element descriptor when the pointee type is known
                let pointee_type = managed_pointee_type_for_value_kind(value_kinds, *array)
                    .or_else(|| raw_pointee_type_for_value_kind(value_kinds, *array))
                    .or_else(|| managed_pointee_type_for_value(tree, value_types, *array))
                    .or_else(|| raw_pointee_type_for_value(tree, value_types, *array));
                let element = pointee_type.and_then(|pointee_type| {
                    element_access_for_pointee(storage_layouts, pointee_type)
                });

                InstructionData::ElementAddr {
                    dest: *destination,
                    array: *array,
                    index: *index,
                    reference: reference_meta_for_value(value_kinds, *destination),
                    array_length: value_kinds
                        .get(*array)
                        .and_then(|kind| array_length_from_kind(tree, kind))
                        .unwrap_or(UNKNOWN_ARRAY_LENGTH),
                    element,
                }
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
                operation: InstructionOperation::Composite,
                data: InstructionData::Composite {
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
                operation: InstructionOperation::Composite,
                data: InstructionData::Composite {
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
                operation: InstructionOperation::Composite,
                data: InstructionData::Composite {
                    dest: *destination,
                    elements: args,
                },
            }
        }

        mir::Instruction::VectorSplat { destination, value } => Instruction {
            operation: InstructionOperation::VectorSplat,
            data: InstructionData::VectorSplat {
                dest: *destination,
                value: *value,
            },
        },

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
            let element = tensor_element_type_for_view_type(tree, view_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
            Instruction {
                operation: InstructionOperation::TensorLoad,
                data: InstructionData::TensorLoad {
                    dest: *destination,
                    view: *view,
                    indices: args,
                    view_type,
                    element,
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
            let element = tensor_element_type_for_view_type(tree, view_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
            Instruction {
                operation: InstructionOperation::TensorStore,
                data: InstructionData::TensorStore {
                    view: *view,
                    indices: args,
                    value: *value,
                    view_type,
                    element,
                },
            }
        }

        mir::Instruction::TensorFill { view, value } => {
            let view_type = value_type_for_value(*view, value_types);
            let element = tensor_element_type_for_view_type(tree, view_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
            Instruction {
                operation: InstructionOperation::TensorFill,
                data: InstructionData::TensorFill {
                    view: *view,
                    value: *value,
                    view_type,
                    element,
                },
            }
        }

        mir::Instruction::TensorCopy { target, source } => {
            let target_type = value_type_for_value(*target, value_types);
            let source_type = value_type_for_value(*source, value_types);
            let target_element = tensor_element_type_for_view_type(tree, target_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
            let source_element = tensor_element_type_for_view_type(tree, source_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
            Instruction {
                operation: InstructionOperation::TensorCopy,
                data: InstructionData::TensorCopy {
                    target: *target,
                    source: *source,
                    target_type,
                    source_type,
                    target_element,
                    source_element,
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
            let element = tensor_element_type_for_view_type(tree, source_type)
                .and_then(|element_type| tensor_element_access(storage_layouts, element_type));
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
                    element,
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
                storage_type: *layout,
                layout_id: tree.type_layout_id(*layout),
                byte_len: storage_layouts
                    .get(layout)
                    .map(|layout| layout.byte_len)
                    .unwrap_or(0),
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
                byte_len: storage_layouts
                    .get(layout)
                    .map(|layout| layout.byte_len)
                    .unwrap_or(0),
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
                storage_type: *layout,
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
    value_types: &[mir::LocalNodeId<mir::Type>],
    value_count: usize,
) -> ValueKinds {
    // allocate value kinds
    let mut value_kinds = ValueKinds::new(value_count);

    // seed explicit value types
    for (index, ty) in value_types.iter().enumerate() {
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
                let Some(kind) = infer_instruction_kind(tree, inst, &value_kinds, value_types)
                else {
                    continue;
                };
                let next_kind = match value_kinds.get(dest) {
                    Some(existing) => merge_block_param_kind(existing, kind),
                    None => kind,
                };

                if value_kinds.get(dest) != Some(next_kind) {
                    value_kinds.set(dest, next_kind);
                    changed = true;
                }
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

/// Rebuild one address-producing result kind from the source storage class.
fn pointer_result_kind_from_source(
    tree: &mir::NodeTree,
    result_type: mir::LocalNodeId<mir::Type>,
    source_kind: ValueKind,
) -> Option<ValueKind> {
    let ValueKind::Pointer { storage, .. } = source_kind else {
        return Some(kind_from_type(tree, result_type));
    };

    let ValueKind::Pointer {
        pointee, reference, ..
    } = kind_from_type(tree, result_type)
    else {
        return None;
    };

    Some(ValueKind::Pointer {
        pointee,
        storage,
        reference,
    })
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
        mir::Instruction::FunctionValue { destination, .. } => {
            let ty = value_type_for_value(*destination, value_types);
            Some(kind_from_type(tree, ty))
        }
        mir::Instruction::FunctionEnvironment { destination } => value_kinds.get(*destination),
        mir::Instruction::Load { result_type, .. } => Some(kind_from_type(tree, *result_type)),
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kinds.get(*aggregate)?;
            kind_from_field(tree, aggregate_kind, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate,
            result_type,
            ..
        } => {
            let source_kind = value_kinds.get(*aggregate)?;
            pointer_result_kind_from_source(tree, *result_type, source_kind)
        }
        mir::Instruction::FieldSet { aggregate, .. } => value_kinds.get(*aggregate),
        mir::Instruction::ElementGet { array, .. } => {
            let array_kind = value_kinds.get(*array)?;
            kind_from_element(tree, array_kind)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_kind = value_kinds.get(*array)?;
            pointer_result_kind_from_source(tree, *result_type, source_kind)
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
    let ValueKind::Composite { ty } = kind else {
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
        ValueKind::Composite { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(kind_from_type(tree, *element)),
            _ => None,
        },
        _ => None,
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
            storage: PointerStorage::Raw | PointerStorage::Stack,
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
    let ty = repr_type(tree, value_type_for_value(value, value_types));

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
        _ => None,
    }
}

fn raw_pointee_type_for_value(
    tree: &mir::NodeTree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, value_type_for_value(value, value_types));

    match tree.get(ty) {
        mir::Type::Reference {
            kind,
            address_space,
            pointee,
            ..
        } if matches!(
            pointer_storage_from_reference(*address_space, *kind),
            PointerStorage::Raw | PointerStorage::Stack
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
            PointerStorage::Raw | PointerStorage::Stack
        ) =>
        {
            Some(*element)
        }
        _ => None,
    }
}

/// Build one field access descriptor from one compiled storage layout.
fn field_access_for_pointee(
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Option<FieldAccess> {
    let layout = storage_layouts.get(&pointee_type)?;
    let field = layout.field(index)?;
    let is_scalar = storage_layouts
        .get(&field.ty)
        .is_some_and(StorageLayout::is_scalar);

    Some(FieldAccess {
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        is_scalar,
    })
}

/// Build one element access descriptor from one compiled storage layout.
fn element_access_for_pointee(
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<ElementAccess> {
    let layout = storage_layouts.get(&pointee_type)?;
    let element = layout.element()?;
    let is_scalar = storage_layouts
        .get(&element.ty)
        .is_some_and(StorageLayout::is_scalar);

    Some(ElementAccess {
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar,
    })
}

/// Build one typed pointee access descriptor from one compiled storage layout.
fn typed_access_for_pointee(
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<TypedAccess> {
    let layout = storage_layouts.get(&pointee_type)?;
    let is_scalar = layout.is_scalar();

    Some(TypedAccess {
        value_type: pointee_type,
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Build one tensor element descriptor from one compiled element storage layout.
fn tensor_element_access(
    storage_layouts: &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    element_type: mir::LocalNodeId<mir::Type>,
) -> Option<ElementAccess> {
    let layout = storage_layouts.get(&element_type)?;
    let is_scalar = layout.is_scalar();

    Some(ElementAccess {
        value_type: element_type,
        byte_stride: layout.stride(),
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Resolve the tensor element type for one tensor value or reference type.
fn tensor_element_type_for_view_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Tensor { element, .. } | mir::Type::TensorReference { element, .. } => {
            Some(*element)
        }
        _ => None,
    }
}

/// Resolve the field count for a struct or tuple kind.
fn field_count_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u32> {
    match kind {
        ValueKind::Composite { ty } => match tree.get(ty) {
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
        ValueKind::Composite { ty } => match tree.get(ty) {
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

/// Append undefined copies for one decomposed destination tree.
fn push_undefined_component_copies(pool: &mut Vec<CopyPair>, target: &ComponentSlots) {
    if target.components.is_empty() {
        pool.push(CopyPair {
            dest: target.slot.0,
            src: INVALID_VALUE_ID,
        });
        return;
    }

    for component in &target.components {
        push_undefined_component_copies(pool, component);
    }
}

/// Append one decomposed source value into one destination component tree.
fn push_component_copies(
    pool: &mut Vec<CopyPair>,
    target: &ComponentSlots,
    source: mir::Value,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
) {
    if target.components.is_empty() {
        pool.push(CopyPair {
            dest: target.slot.0,
            src: source.0,
        });
        return;
    }

    let source_components = deferred_composites.get(&source).unwrap_or_else(|| {
        panic!("missing deferred composite for cross block transfer: {source:?}")
    });

    debug_assert_eq!(
        source_components.elements.len(),
        target.components.len(),
        "mismatched deferred component count for {:?}",
        target.ty
    );

    for (component, source_component) in target.components.iter().zip(&source_components.elements) {
        push_component_copies(pool, component, *source_component, deferred_composites);
    }
}

/// Append one block-edge copy plan, expanding decomposed destination parameters.
fn push_block_copy_range(
    pool: &mut Vec<CopyPair>,
    parameters: &[mir::Value],
    arguments: &[mir::Value],
    deferred_parameters: Option<&HashMap<mir::Value, ComponentSlots>>,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
) -> CopyRange {
    if parameters.is_empty() {
        return CopyRange::empty();
    }

    let start = pool.len();

    for (index, parameter) in parameters.iter().enumerate() {
        if let Some(target) = deferred_parameters.and_then(|targets| targets.get(parameter)) {
            if let Some(source) = arguments.get(index) {
                push_component_copies(pool, target, *source, deferred_composites);
            } else {
                push_undefined_component_copies(pool, target);
            }

            continue;
        }

        let src = arguments
            .get(index)
            .map(|value| value.0)
            .unwrap_or(INVALID_VALUE_ID);
        pool.push(CopyPair {
            dest: parameter.0,
            src,
        });
    }

    CopyRange {
        start: start as u32,
        len: (pool.len() - start) as u32,
        is_contiguous: false,
        contiguous_src: 0,
        contiguous_dest: 0,
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
    deferred_block_params: &DeferredBlockParams,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
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
        let deferred_parameters = deferred_block_params.by_block.get(&case.target);
        let copies = push_block_copy_range(
            copy_pool,
            target_parameters,
            &case.arguments,
            deferred_parameters,
            deferred_composites,
        );
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
    deferred_block_params: &DeferredBlockParams,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
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
        let deferred_parameters = deferred_block_params.by_block.get(&case.target);
        let copies = push_block_copy_range(
            copy_pool,
            target_parameters,
            &case.arguments,
            deferred_parameters,
            deferred_composites,
        );
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
    exceptional_call_resume_points: &HashMap<
        mir::LocalNodeId<mir::Block>,
        (destack_engine::ResumePointId, destack_engine::ResumePointId),
    >,
    function_indices: &HashMap<mir::LocalNodeId<mir::Function>, u32>,
    value_kinds: &ValueKinds,
    value_types: &[mir::LocalNodeId<mir::Type>],
    argument_pool: &mut Vec<mir::Value>,
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    deferred_block_params: &DeferredBlockParams,
    deferred_composites: &HashMap<mir::Value, DeferredComposite>,
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
            let deferred_parameters = deferred_block_params.by_block.get(target);
            let copies = push_block_copy_range(
                copy_pool,
                target_parameters,
                arguments,
                deferred_parameters,
                deferred_composites,
            );

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
            let then_deferred = deferred_block_params.by_block.get(then_target);
            let else_deferred = deferred_block_params.by_block.get(else_target);
            let then_copies = push_block_copy_range(
                copy_pool,
                then_parameters,
                then_arguments,
                then_deferred,
                deferred_composites,
            );
            let else_copies = push_block_copy_range(
                copy_pool,
                else_parameters,
                else_arguments,
                else_deferred,
                deferred_composites,
            );

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
            let success_deferred = deferred_block_params.by_block.get(&success.target);
            let failure_deferred = deferred_block_params.by_block.get(&failure.target);
            let success_copies = push_block_copy_range(
                copy_pool,
                success_parameters,
                &success.arguments,
                success_deferred,
                deferred_composites,
            );
            let failure_copies = push_block_copy_range(
                copy_pool,
                failure_parameters,
                &failure.arguments,
                failure_deferred,
                deferred_composites,
            );

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
            let default_deferred = deferred_block_params.by_block.get(default);
            let default_copies = push_block_copy_range(
                copy_pool,
                default_parameters,
                default_arguments,
                default_deferred,
                deferred_composites,
            );

            // try jump table for dense switches
            if let Some((min_value, table_range)) = push_switch_table_range(
                switch_case_pool,
                copy_pool,
                block_index_map,
                block_parameters,
                cases,
                default_index as u32,
                default_copies,
                deferred_block_params,
                deferred_composites,
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
                    deferred_block_params,
                    deferred_composites,
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

        mir::Terminator::Throw { value } => Instruction {
            operation: InstructionOperation::Throw,
            data: InstructionData::Throw {
                value: pack_optional_value(Some(*value)),
            },
        },

        mir::Terminator::Call {
            function,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, arguments);
            let &(normal_resume_point, unwind_resume_point) = exceptional_call_resume_points
                .get(&current_block)
                .unwrap_or_else(|| {
                    panic!("missing exceptional call resume points for block: {current_block:?}")
                });
            let callee_index = lookup_function_index(function_indices, *function)
                .unwrap_or(INVALID_FUNCTION_INDEX);

            Instruction {
                operation: InstructionOperation::CallBranch,
                data: InstructionData::CallBranch {
                    function: function.id,
                    callee_index,
                    arguments: args,
                    normal_resume_point,
                    unwind_resume_point,
                },
            }
        }

        mir::Terminator::CallIndirect {
            callee,
            signature,
            arguments,
            ..
        } => {
            let arguments = push_argument_range(argument_pool, arguments);
            let &(normal_resume_point, unwind_resume_point) = exceptional_call_resume_points
                .get(&current_block)
                .unwrap_or_else(|| {
                    panic!("missing exceptional call resume points for block: {current_block:?}")
                });

            Instruction {
                operation: InstructionOperation::CallIndirectBranch,
                data: InstructionData::CallIndirectBranch {
                    callee: *callee,
                    signature: *signature,
                    arguments,
                    normal_resume_point,
                    unwind_resume_point,
                    cached_function: Cell::new(None),
                    cached_index: Cell::new(None),
                },
            }
        }

        mir::Terminator::CallVirtual {
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let arguments = push_argument_range(argument_pool, arguments);
            let &(normal_resume_point, unwind_resume_point) = exceptional_call_resume_points
                .get(&current_block)
                .unwrap_or_else(|| {
                    panic!("missing exceptional call resume points for block: {current_block:?}")
                });

            Instruction {
                operation: InstructionOperation::CallVirtualBranch,
                data: InstructionData::CallVirtualBranch {
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value(tree, value_types, *receiver),
                    slot_id: slot_id.0,
                    arguments,
                    normal_resume_point,
                    unwind_resume_point,
                },
            }
        }

        mir::Terminator::CallInterface {
            receiver,
            slot_id,
            arguments,
            ..
        } => {
            let arguments = push_argument_range(argument_pool, arguments);
            let &(normal_resume_point, unwind_resume_point) = exceptional_call_resume_points
                .get(&current_block)
                .unwrap_or_else(|| {
                    panic!("missing exceptional call resume points for block: {current_block:?}")
                });

            Instruction {
                operation: InstructionOperation::CallInterfaceBranch,
                data: InstructionData::CallInterfaceBranch {
                    receiver: *receiver,
                    managed_pointee: managed_pointee_type_for_value(tree, value_types, *receiver),
                    slot_id: slot_id.0,
                    arguments,
                    normal_resume_point,
                    unwind_resume_point,
                },
            }
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
            signature,
            arguments,
            ..
        } => {
            let args = push_argument_range(argument_pool, arguments);
            Instruction {
                operation: InstructionOperation::TailCallIndirect,
                data: InstructionData::TailCallIndirect {
                    callee: *callee,
                    signature: *signature,
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
