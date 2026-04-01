use destack_mir as mir;

use super::super::value::{PointerStorage, ValueKind};
use super::kind::ValueKindMap;
use crate::executable::InstructionOperation;

/// Pick a binary handler based on inferred operand kind.
pub(super) fn select_binary_operation(
    value_kinds: &ValueKindMap,
    left: mir::Value,
    operator: mir::BinaryOperator,
) -> InstructionOperation {
    use mir::BinaryOperator::*;

    // resolve operand kind
    let kind = value_kinds.get(left);

    // try specialized integer handlers first: no operator dispatch overhead
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
pub(super) fn select_specialized_int_operation(
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

            // unsigned comparisons: eq and ne produce bool, not int or uint
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
pub(super) fn select_specialized_const_int_operation(
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

            // eq and ne produce bool, can use signed version
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
pub(super) fn select_unary_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_load_operation(
    value_kinds: &ValueKindMap,
    pointer: mir::Value,
) -> InstructionOperation {
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
pub(super) fn select_store_operation(
    value_kinds: &ValueKindMap,
    pointer: mir::Value,
) -> InstructionOperation {
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
pub(super) fn select_field_get_operation(
    value_kinds: &ValueKindMap,
    aggregate: mir::Value,
    field_count: u32,
    index: u32,
) -> InstructionOperation {
    match value_kinds.get(aggregate) {
        // small aggregates: inline slot storage
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
pub(super) fn select_element_get_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_field_addr_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_element_addr_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_field_load_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_field_store_operation(
    value_kinds: &ValueKindMap,
    aggregate: mir::Value,
    field_count: u32,
    index: u32,
) -> InstructionOperation {
    // small managed composites: inline slot storage
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
pub(super) fn select_element_load_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_element_store_operation(
    value_kinds: &ValueKindMap,
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
pub(super) fn select_branch_operation(
    value_kinds: &ValueKindMap,
    condition: mir::Value,
) -> InstructionOperation {
    // resolve condition kind
    match value_kinds.get(condition) {
        Some(ValueKind::Bool) => InstructionOperation::BranchBool,
        _ => InstructionOperation::Branch,
    }
}

/// Pick a switch handler based on inferred value kind.
pub(super) fn select_switch_operation(
    value_kinds: &ValueKindMap,
    value: mir::Value,
) -> InstructionOperation {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => InstructionOperation::SwitchInt,
        _ => InstructionOperation::Switch,
    }
}

/// Pick a switch table handler based on inferred value kind.
pub(super) fn select_switch_table_operation(
    value_kinds: &ValueKindMap,
    value: mir::Value,
) -> InstructionOperation {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => InstructionOperation::SwitchTableInt,
        _ => InstructionOperation::SwitchTable,
    }
}

/// Pick a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_operation(
    operator: mir::BinaryOperator,
) -> InstructionOperation {
    match operator {
        // signed integer comparisons: most common in loops
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

        // fallback for non comparison operators
        _ => InstructionOperation::CompareAndBranch,
    }
}

/// Pick a compare and branch handler for constant right operands.
pub(super) fn select_compare_branch_const_operation(
    operator: mir::BinaryOperator,
) -> InstructionOperation {
    match operator {
        // signed integer comparisons: most common in loops
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

        // fallback for non comparison operators
        _ => InstructionOperation::CompareAndBranchConst,
    }
}

/// Swap the comparison operator when the constant appears on the left.
pub(super) fn swap_compare_operator(operator: mir::BinaryOperator) -> mir::BinaryOperator {
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
