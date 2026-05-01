use destack_mir as mir;

use crate::Word;
use crate::diagnostic::Error;
use crate::program::{ElementAccess, FieldAccess, Opcode, PointeeAccess, PointerClass, ValueRepr};

use super::repr::ValueReprMap;

/// Require one lowered access to fit in a word.
fn require_word_access(
    is_word: bool,
    expected: &'static str,
    actual: mir::LocalNodeId<mir::Type>,
) -> Result<(), Error> {
    if !is_word {
        return Err(Error::TypeMismatch {
            expected: expected.to_string(),
            actual: format!("{actual:?}"),
        });
    }

    Ok(())
}

/// Pick a binary handler from one known operand representation.
pub(super) fn select_binary_opcode(
    repr: Option<ValueRepr>,
    operator: mir::BinaryOperator,
) -> Opcode {
    use mir::BinaryOperator::*;

    // try specialized integer handlers first: no operator dispatch overhead
    if let Some(ValueRepr::Int { width, signed }) = repr
        && width <= Word::BIT_LEN as u16
        && let Some(handler) = select_specialized_int_opcode(operator, signed)
    {
        return handler;
    }

    // fall back to typed handlers
    match repr {
        Some(ValueRepr::Int {
            width,
            signed: true,
        }) if width <= Word::BIT_LEN as u16 => Opcode::BinaryInt,
        Some(ValueRepr::Int {
            width,
            signed: false,
        }) if width <= Word::BIT_LEN as u16 => Opcode::BinaryUint,
        Some(ValueRepr::Float { width: 32 }) => Opcode::BinaryFloat32,
        Some(ValueRepr::Float { width: 64 }) => Opcode::BinaryFloat64,
        Some(ValueRepr::Bool) if matches!(operator, And | Or | Xor) => Opcode::BinaryBool,
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

/// Pick a unary handler based on inferred operand representation.
pub(super) fn select_unary_opcode(
    value_reprs: &ValueReprMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Opcode {
    // resolve operand representation
    let repr = value_reprs.get(argument);

    // select handler by representation
    match (repr, operator) {
        (
            Some(ValueRepr::Int {
                width,
                signed: true,
            }),
            _,
        ) if width <= Word::BIT_LEN as u16 => Opcode::UnaryInt,
        (
            Some(ValueRepr::Int {
                width,
                signed: false,
            }),
            _,
        ) if width <= Word::BIT_LEN as u16 => Opcode::UnaryUint,
        (Some(ValueRepr::Float { width: 32 }), mir::UnaryOperator::FloatNegate) => {
            Opcode::UnaryFloat32
        }
        (Some(ValueRepr::Float { width: 64 }), mir::UnaryOperator::FloatNegate) => {
            Opcode::UnaryFloat64
        }
        (Some(ValueRepr::Bool), mir::UnaryOperator::Not) => Opcode::UnaryBool,
        _ => Opcode::Unary,
    }
}

/// Pick a load handler for one known pointer access.
pub(super) fn select_load_opcode(access: PointeeAccess) -> Result<Opcode, Error> {
    if !access.is_word() {
        return Ok(Opcode::CopyFromAddress);
    }

    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Opcode::LoadHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Opcode::LoadSharedHeap),
        PointerClass::Raw => Ok(Opcode::LoadRaw),
        PointerClass::SharedRaw => Ok(Opcode::LoadSharedRaw),
        PointerClass::Stack => Ok(Opcode::LoadStack),
        PointerClass::Frame => Ok(Opcode::LoadFrame),
        PointerClass::Static => Ok(Opcode::LoadStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Pick a store handler for one known pointer access.
pub(super) fn select_store_opcode(access: PointeeAccess) -> Result<Opcode, Error> {
    if !access.is_word() {
        return Ok(Opcode::CopyToAddress);
    }

    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Opcode::StoreHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Opcode::StoreSharedHeap),
        PointerClass::Raw => Ok(Opcode::StoreRaw),
        PointerClass::SharedRaw => Ok(Opcode::StoreSharedRaw),
        PointerClass::Stack => Ok(Opcode::StoreStack),
        PointerClass::Frame => Ok(Opcode::StoreFrame),
        PointerClass::Static => Ok(Opcode::StoreStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Pick a field address handler based on inferred value representation.
pub(super) fn select_field_addr_opcode(
    value_reprs: &ValueReprMap,
    base: mir::Value,
) -> Result<Opcode, Error> {
    match value_reprs.get(base) {
        Some(ValueRepr::FrameBytes { .. }) => Ok(Opcode::FieldAddr),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::FieldAddrSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::FieldAddrSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::FieldAddrHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::FieldAddrRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::FieldAddrStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::FieldAddr),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::FieldAddrStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick an element address handler based on inferred value representation.
pub(super) fn select_element_addr_opcode(
    value_reprs: &ValueReprMap,
    array: mir::Value,
) -> Result<Opcode, Error> {
    match value_reprs.get(array) {
        Some(ValueRepr::Array { .. }) | Some(ValueRepr::FrameBytes { .. }) => {
            Ok(Opcode::ElementAddr)
        }
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::ElementAddrSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::ElementAddrSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::ElementAddrHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::ElementAddrRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::ElementAddrStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::ElementAddr),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::ElementAddrStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick a field load handler based on inferred value representation.
pub(super) fn select_field_load_opcode(
    value_reprs: &ValueReprMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Opcode, Error> {
    require_word_access(field.is_word(), "word field load", field.value_type)?;

    match value_reprs.get(base) {
        Some(ValueRepr::FrameBytes { .. }) => Ok(Opcode::FieldLoad),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::FieldLoadSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::FieldLoadSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::FieldLoadHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::FieldLoadRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::FieldLoadStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::FieldLoad),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::FieldLoadStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick a field store handler based on inferred value representation.
pub(super) fn select_field_store_opcode(
    value_reprs: &ValueReprMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Opcode, Error> {
    require_word_access(field.is_word(), "word field store", field.value_type)?;

    match value_reprs.get(base) {
        Some(ValueRepr::FrameBytes { .. }) => Ok(Opcode::FieldStore),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::FieldStoreSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::FieldStoreSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::FieldStoreHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::FieldStoreRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::FieldStoreStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::FieldStore),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::FieldStoreStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick an element load handler based on inferred value representation.
pub(super) fn select_element_load_opcode(
    value_reprs: &ValueReprMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Opcode, Error> {
    require_word_access(element.is_word(), "word element load", element.value_type)?;

    match value_reprs.get(array) {
        Some(ValueRepr::Array { .. }) | Some(ValueRepr::FrameBytes { .. }) => {
            Ok(Opcode::ElementLoad)
        }
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::ElementLoadSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::ElementLoadSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::ElementLoadHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::ElementLoadRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::ElementLoadStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::ElementLoad),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::ElementLoadStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick an element store handler based on inferred value representation.
pub(super) fn select_element_store_opcode(
    value_reprs: &ValueReprMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Opcode, Error> {
    require_word_access(element.is_word(), "word element store", element.value_type)?;

    match value_reprs.get(array) {
        Some(ValueRepr::Array { .. }) | Some(ValueRepr::FrameBytes { .. }) => {
            Ok(Opcode::ElementStore)
        }
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap | PointerClass::SharedHeapAddress,
            ..
        }) => Ok(Opcode::ElementStoreSharedHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        }) => Ok(Opcode::ElementStoreSharedRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Heap | PointerClass::HeapAddress,
            ..
        }) => Ok(Opcode::ElementStoreHeap),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        }) => Ok(Opcode::ElementStoreRaw),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Stack,
            ..
        }) => Ok(Opcode::ElementStoreStack),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Frame,
            ..
        }) => Ok(Opcode::ElementStore),
        Some(ValueRepr::Pointer {
            pointer_class: PointerClass::Static,
            ..
        }) => Ok(Opcode::ElementStoreStatic),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick a branch handler based on inferred condition representation.
pub(super) fn select_branch_opcode(value_reprs: &ValueReprMap, condition: mir::Value) -> Opcode {
    // resolve condition representation
    match value_reprs.get(condition) {
        Some(ValueRepr::Bool) => Opcode::BranchBool,
        _ => Opcode::Branch,
    }
}

/// Pick a switch handler based on inferred value representation.
pub(super) fn select_switch_opcode(value_reprs: &ValueReprMap, value: mir::Value) -> Opcode {
    // resolve switch value representation
    match value_reprs.get(value) {
        Some(ValueRepr::Int { width, .. }) if width <= Word::BIT_LEN as u16 => Opcode::SwitchInt,
        _ => Opcode::Switch,
    }
}

/// Pick a switch table handler based on inferred value representation.
pub(super) fn select_switch_table_opcode(value_reprs: &ValueReprMap, value: mir::Value) -> Opcode {
    // resolve switch value representation
    match value_reprs.get(value) {
        Some(ValueRepr::Int { width, .. }) if width <= Word::BIT_LEN as u16 => {
            Opcode::SwitchTableInt
        }
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

        // default for non comparison operators
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

        // default for non comparison operators
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
