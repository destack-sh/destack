use destack_mir as mir;

use crate::module::{Opcode, PointerClass, ValueKind};

use super::kind::ValueKindMap;

/// Pick a binary handler based on inferred operand kind.
pub(super) fn select_binary_opcode(
    value_kinds: &ValueKindMap,
    left: mir::Value,
    operator: mir::BinaryOperator,
) -> Opcode {
    use mir::BinaryOperator::*;

    // resolve operand kind
    let kind = value_kinds.get(left);

    // try specialized integer handlers first: no operator dispatch overhead
    if let Some(ValueKind::Int { signed, .. }) = kind
        && let Some(handler) = select_specialized_int_opcode(operator, signed)
    {
        return handler;
    }

    // fall back to typed handlers
    match kind {
        Some(ValueKind::Int { signed: true, .. }) => Opcode::BinaryInt,
        Some(ValueKind::Int { signed: false, .. }) => Opcode::BinaryUint,
        Some(ValueKind::Float { width: 32 }) => Opcode::BinaryFloat32,
        Some(ValueKind::Float { width: 64 }) => Opcode::BinaryFloat64,
        Some(ValueKind::Bool) if matches!(operator, And | Or | Xor) => Opcode::BinaryBool,
        _ => Opcode::Binary,
    }
}

/// Select a fully specialized integer handler if available.
pub(super) fn select_specialized_int_opcode(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<Opcode> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            // signed arithmetic
            Add => Opcode::AddInt,
            Subtract => Opcode::SubInt,
            Multiply => Opcode::MulInt,

            // signed bitwise
            And => Opcode::AndInt,
            Or => Opcode::OrInt,
            Xor => Opcode::XorInt,
            ShiftLeft => Opcode::ShlInt,
            ArithmeticShiftRight => Opcode::ShrInt,

            // signed comparisons
            Equal => Opcode::EqInt,
            NotEqual => Opcode::NeInt,
            SignedLessThan => Opcode::LtInt,
            SignedLessEqual => Opcode::LeInt,
            SignedGreaterThan => Opcode::GtInt,
            SignedGreaterEqual => Opcode::GeInt,
            _ => return None,
        }
    } else {
        match operator {
            // unsigned arithmetic
            Add => Opcode::AddUint,
            Subtract => Opcode::SubUint,
            Multiply => Opcode::MulUint,

            // unsigned bitwise
            And => Opcode::AndUint,
            Or => Opcode::OrUint,
            Xor => Opcode::XorUint,
            ShiftLeft => Opcode::ShlUint,
            LogicalShiftRight => Opcode::ShrUint,

            // unsigned comparisons: eq and ne produce bool, not int or uint
            Equal => Opcode::EqInt,
            NotEqual => Opcode::NeInt,
            UnsignedLessThan => Opcode::LtUint,
            UnsignedLessEqual => Opcode::LeUint,
            UnsignedGreaterThan => Opcode::GtUint,
            UnsignedGreaterEqual => Opcode::GeUint,
            _ => return None,
        }
    })
}

/// Select a specialized const-right handler if available.
pub(super) fn select_specialized_const_int_opcode(
    operator: mir::BinaryOperator,
    signed: bool,
) -> Option<Opcode> {
    use mir::BinaryOperator::*;

    Some(if signed {
        match operator {
            Add => Opcode::AddConstInt,
            Subtract => Opcode::SubConstInt,
            Multiply => Opcode::MulConstInt,
            Equal => Opcode::EqConstInt,
            NotEqual => Opcode::NeConstInt,
            SignedLessThan => Opcode::LtConstInt,
            SignedLessEqual => Opcode::LeConstInt,
            SignedGreaterThan => Opcode::GtConstInt,
            SignedGreaterEqual => Opcode::GeConstInt,
            _ => return None,
        }
    } else {
        match operator {
            Add => Opcode::AddConstUint,
            Subtract => Opcode::SubConstUint,
            Multiply => Opcode::MulConstUint,

            // eq and ne produce bool, can use signed version
            Equal => Opcode::EqConstInt,
            NotEqual => Opcode::NeConstInt,
            UnsignedLessThan => Opcode::LtConstUint,
            UnsignedLessEqual => Opcode::LeConstUint,
            UnsignedGreaterThan => Opcode::GtConstUint,
            UnsignedGreaterEqual => Opcode::GeConstUint,
            _ => return None,
        }
    })
}

/// Pick a unary handler based on inferred operand kind.
pub(super) fn select_unary_opcode(
    value_kinds: &ValueKindMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Opcode {
    // resolve operand kind
    let kind = value_kinds.get(argument);

    // select handler by kind
    match (kind, operator) {
        (Some(ValueKind::Int { signed: true, .. }), _) => Opcode::UnaryInt,
        (Some(ValueKind::Int { signed: false, .. }), _) => Opcode::UnaryUint,
        (Some(ValueKind::Float { width: 32 }), mir::UnaryOperator::FloatNegate) => {
            Opcode::UnaryFloat32
        }
        (Some(ValueKind::Float { width: 64 }), mir::UnaryOperator::FloatNegate) => {
            Opcode::UnaryFloat64
        }
        (Some(ValueKind::Bool), mir::UnaryOperator::Not) => Opcode::UnaryBool,
        _ => Opcode::Unary,
    }
}

/// Pick a load handler based on inferred pointer class.
pub(super) fn select_load_opcode(value_kinds: &ValueKindMap, pointer: mir::Value) -> Opcode {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::LoadHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::LoadRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::LoadStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::LoadFrame,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::LoadStatic,
        _ => Opcode::Load,
    }
}

/// Pick a store handler based on inferred pointer class.
pub(super) fn select_store_opcode(value_kinds: &ValueKindMap, pointer: mir::Value) -> Opcode {
    match value_kinds.get(pointer) {
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::StoreHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::StoreRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::StoreStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::StoreFrame,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::StoreStatic,
        _ => Opcode::Store,
    }
}

/// Pick a field get handler based on inferred aggregate value kind.
pub(super) fn select_field_get_opcode(value_kinds: &ValueKindMap, aggregate: mir::Value) -> Opcode {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => Opcode::FieldGet,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::FieldLoadHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::FieldLoadRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::FieldLoadStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::FieldLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::FieldLoadStatic,
        _ => Opcode::FieldLoad,
    }
}

/// Pick an element get handler based on inferred array value kind.
pub(super) fn select_element_get_opcode(value_kinds: &ValueKindMap, array: mir::Value) -> Opcode {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => Opcode::ElementGet,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::ElementLoadHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::ElementLoadRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::ElementLoadStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::ElementLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::ElementLoadStatic,
        _ => Opcode::ElementLoad,
    }
}

/// Pick a field address handler based on inferred aggregate value kind.
pub(super) fn select_field_addr_opcode(
    value_kinds: &ValueKindMap,
    aggregate: mir::Value,
) -> Opcode {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => Opcode::FieldAddr,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::FieldAddrHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::FieldAddrRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::FieldAddrStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::FieldAddr,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::FieldAddrStatic,
        _ => Opcode::FieldAddr,
    }
}

/// Pick an element address handler based on inferred array value kind.
pub(super) fn select_element_addr_opcode(value_kinds: &ValueKindMap, array: mir::Value) -> Opcode {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => Opcode::ElementAddr,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::ElementAddrHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::ElementAddrRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::ElementAddrStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::ElementAddr,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::ElementAddrStatic,
        _ => Opcode::ElementAddr,
    }
}

/// Pick a field load handler based on inferred aggregate value kind.
pub(super) fn select_field_load_opcode(
    value_kinds: &ValueKindMap,
    aggregate: mir::Value,
) -> Opcode {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => Opcode::FieldLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::FieldLoadHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::FieldLoadRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::FieldLoadStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::FieldLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::FieldLoadStatic,
        _ => Opcode::FieldLoad,
    }
}

/// Pick a field store handler based on inferred aggregate value kind.
pub(super) fn select_field_store_opcode(
    value_kinds: &ValueKindMap,
    aggregate: mir::Value,
) -> Opcode {
    match value_kinds.get(aggregate) {
        Some(ValueKind::Aggregate { .. }) => Opcode::FieldStore,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::FieldStoreHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::FieldStoreRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::FieldStoreStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::FieldStore,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::FieldStoreStatic,
        _ => Opcode::FieldStore,
    }
}

/// Pick an element load handler based on inferred array value kind.
pub(super) fn select_element_load_opcode(value_kinds: &ValueKindMap, array: mir::Value) -> Opcode {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => Opcode::ElementLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::ElementLoadHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::ElementLoadRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::ElementLoadStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::ElementLoad,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::ElementLoadStatic,
        _ => Opcode::ElementLoad,
    }
}

/// Pick an element store handler based on inferred array value kind.
pub(super) fn select_element_store_opcode(value_kinds: &ValueKindMap, array: mir::Value) -> Opcode {
    match value_kinds.get(array) {
        Some(ValueKind::Array { .. }) | Some(ValueKind::Aggregate { .. }) => Opcode::ElementStore,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        }) => Opcode::ElementStoreHeap,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Opcode::ElementStoreRaw,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Opcode::ElementStoreStack,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Opcode::ElementStore,
        Some(ValueKind::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Opcode::ElementStoreStatic,
        _ => Opcode::ElementStore,
    }
}

/// Pick a branch handler based on inferred condition kind.
pub(super) fn select_branch_opcode(value_kinds: &ValueKindMap, condition: mir::Value) -> Opcode {
    // resolve condition kind
    match value_kinds.get(condition) {
        Some(ValueKind::Bool) => Opcode::BranchBool,
        _ => Opcode::Branch,
    }
}

/// Pick a switch handler based on inferred value kind.
pub(super) fn select_switch_opcode(value_kinds: &ValueKindMap, value: mir::Value) -> Opcode {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => Opcode::SwitchInt,
        _ => Opcode::Switch,
    }
}

/// Pick a switch table handler based on inferred value kind.
pub(super) fn select_switch_table_opcode(value_kinds: &ValueKindMap, value: mir::Value) -> Opcode {
    // resolve switch value kind
    match value_kinds.get(value) {
        Some(ValueKind::Int { .. }) => Opcode::SwitchTableInt,
        _ => Opcode::SwitchTable,
    }
}

/// Pick a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_opcode(operator: mir::BinaryOperator) -> Opcode {
    match operator {
        // signed integer comparisons: most common in loops
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual => Opcode::CompareAndBranchInt,

        // unsigned integer comparisons
        mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual => Opcode::CompareAndBranchUint,

        // float comparisons
        mir::BinaryOperator::FloatEqual
        | mir::BinaryOperator::FloatNotEqual
        | mir::BinaryOperator::FloatLessThan
        | mir::BinaryOperator::FloatLessEqual
        | mir::BinaryOperator::FloatGreaterThan
        | mir::BinaryOperator::FloatGreaterEqual => Opcode::CompareAndBranchFloat,

        // fallback for non comparison operators
        _ => Opcode::CompareAndBranch,
    }
}

/// Pick a compare and branch handler for constant right operands.
pub(super) fn select_compare_branch_const_opcode(operator: mir::BinaryOperator) -> Opcode {
    match operator {
        // signed integer comparisons: most common in loops
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual => Opcode::CompareAndBranchConstInt,

        // unsigned integer comparisons
        mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual => Opcode::CompareAndBranchConstUint,

        // float comparisons
        mir::BinaryOperator::FloatEqual
        | mir::BinaryOperator::FloatNotEqual
        | mir::BinaryOperator::FloatLessThan
        | mir::BinaryOperator::FloatLessEqual
        | mir::BinaryOperator::FloatGreaterThan
        | mir::BinaryOperator::FloatGreaterEqual => Opcode::CompareAndBranchConstFloat,

        // fallback for non comparison operators
        _ => Opcode::CompareAndBranchConst,
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
