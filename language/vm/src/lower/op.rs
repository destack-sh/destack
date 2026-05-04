use destack_mir as mir;

use crate::diagnostic::Error;
use crate::program::{ElementAccess, FieldAccess, Op, PointeeAccess, PointerClass, ValueLayout};

use super::value::ValueLayoutMap;

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

/// Select a binary handler from one known operand layout.
pub(super) fn select_binary_op(layout: Option<ValueLayout>, operator: mir::BinaryOperator) -> Op {
    use mir::BinaryOperator::*;

    match layout {
        Some(ValueLayout::Int { width, signed }) => select_integer_op(operator, signed, width)
            .or_else(|| select_wide_integer_op(operator, signed))
            .unwrap_or(Op::BinaryElementwise),
        Some(ValueLayout::Float { width: 32 }) => select_float_op(operator, false),
        Some(ValueLayout::Float { width: 64 }) => select_float_op(operator, true),
        Some(ValueLayout::Bool) => match operator {
            And => Op::AndBool,
            Or => Op::OrBool,
            Xor => Op::XorBool,
            Equal => Op::EqInt,
            NotEqual => Op::NeInt,
            _ => Op::BinaryElementwise,
        },
        _ => Op::BinaryElementwise,
    }
}

/// Select a machine integer op when the operator has one.
pub(super) fn select_integer_op(
    operator: mir::BinaryOperator,
    signed: bool,
    width: u16,
) -> Option<Op> {
    use mir::BinaryOperator::*;

    if width > u64::BITS as u16 {
        return None;
    }

    Some(match (operator, signed) {
        (Add, _) => Op::AddInt,
        (Subtract, _) => Op::SubInt,
        (Multiply, _) => Op::MulInt,
        (SignedDivide, true) => Op::DivInt,
        (UnsignedDivide, _) => Op::DivUint,
        (SignedRemainder, true) => Op::RemInt,
        (UnsignedRemainder, _) => Op::RemUint,
        (And, _) => Op::AndInt,
        (Or, _) => Op::OrInt,
        (Xor, _) => Op::XorInt,
        (ShiftLeft, _) => Op::ShlInt,
        (ArithmeticShiftRight, true) => Op::ShrInt,
        (LogicalShiftRight, _) => Op::ShrUint,
        (Equal, _) => Op::EqInt,
        (NotEqual, _) => Op::NeInt,
        (SignedLessThan, true) => Op::LtInt,
        (SignedLessEqual, true) => Op::LeInt,
        (SignedGreaterThan, true) => Op::GtInt,
        (SignedGreaterEqual, true) => Op::GeInt,
        (UnsignedLessThan, _) => Op::LtUint,
        (UnsignedLessEqual, _) => Op::LeUint,
        (UnsignedGreaterThan, _) => Op::GtUint,
        (UnsignedGreaterEqual, _) => Op::GeUint,
        _ => return None,
    })
}

/// Select a wide integer op when the operator has one.
pub(super) fn select_wide_integer_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, signed) {
        (Add, _) => Op::AddWideInt,
        (Subtract, _) => Op::SubWideInt,
        (Multiply, _) => Op::MulWideInt,
        (SignedDivide, true) => Op::DivWideInt,
        (UnsignedDivide, _) => Op::DivWideUint,
        (SignedRemainder, true) => Op::RemWideInt,
        (UnsignedRemainder, _) => Op::RemWideUint,
        (And, _) => Op::AndWideInt,
        (Or, _) => Op::OrWideInt,
        (Xor, _) => Op::XorWideInt,
        (ShiftLeft, _) => Op::ShlWideInt,
        (ArithmeticShiftRight, true) => Op::ShrWideInt,
        (LogicalShiftRight, _) => Op::ShrWideUint,
        (Equal, _) => Op::EqWideInt,
        (NotEqual, _) => Op::NeWideInt,
        (SignedLessThan, true) => Op::LtWideInt,
        (SignedLessEqual, true) => Op::LeWideInt,
        (SignedGreaterThan, true) => Op::GtWideInt,
        (SignedGreaterEqual, true) => Op::GeWideInt,
        (UnsignedLessThan, _) => Op::LtWideUint,
        (UnsignedLessEqual, _) => Op::LeWideUint,
        (UnsignedGreaterThan, _) => Op::GtWideUint,
        (UnsignedGreaterEqual, _) => Op::GeWideUint,
        _ => return None,
    })
}

/// Select a machine float op for one binary operator.
fn select_float_op(operator: mir::BinaryOperator, is_64: bool) -> Op {
    use mir::BinaryOperator::*;

    match (operator, is_64) {
        (FloatAdd, false) => Op::AddF32,
        (FloatAdd, true) => Op::AddF64,
        (FloatSubtract, false) => Op::SubF32,
        (FloatSubtract, true) => Op::SubF64,
        (FloatMultiply, false) => Op::MulF32,
        (FloatMultiply, true) => Op::MulF64,
        (FloatDivide, false) => Op::DivF32,
        (FloatDivide, true) => Op::DivF64,
        (FloatEqual, false) => Op::EqF32,
        (FloatEqual, true) => Op::EqF64,
        (FloatNotEqual, false) => Op::NeF32,
        (FloatNotEqual, true) => Op::NeF64,
        (FloatLessThan, false) => Op::LtF32,
        (FloatLessThan, true) => Op::LtF64,
        (FloatLessEqual, false) => Op::LeF32,
        (FloatLessEqual, true) => Op::LeF64,
        (FloatGreaterThan, false) => Op::GtF32,
        (FloatGreaterThan, true) => Op::GtF64,
        (FloatGreaterEqual, false) => Op::GeF32,
        (FloatGreaterEqual, true) => Op::GeF64,
        _ => Op::BinaryElementwise,
    }
}

/// Select a machine integer unary op when the operator has one.
pub(super) fn select_integer_unary_op(
    operator: mir::UnaryOperator,
    signed: bool,
    width: u16,
) -> Option<Op> {
    if width > u64::BITS as u16 {
        return None;
    }

    Some(match (operator, signed) {
        (mir::UnaryOperator::Negate, true) => Op::NegInt,
        (mir::UnaryOperator::Not, _) => Op::NotInt,
        _ => return None,
    })
}

/// Select a unary handler from one known operand layout.
pub(super) fn select_unary_op(
    value_layouts: &ValueLayoutMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Op {
    match value_layouts.get(argument) {
        Some(ValueLayout::Int { width, signed }) => {
            select_integer_unary_op(operator, signed, width)
                .or_else(|| select_wide_integer_unary_op(operator, signed))
                .unwrap_or(Op::UnaryElementwise)
        }
        Some(ValueLayout::Float { width: 32 }) if operator == mir::UnaryOperator::FloatNegate => {
            Op::NegF32
        }
        Some(ValueLayout::Float { width: 64 }) if operator == mir::UnaryOperator::FloatNegate => {
            Op::NegF64
        }
        Some(ValueLayout::Bool) if operator == mir::UnaryOperator::Not => Op::NotBool,
        _ => Op::UnaryElementwise,
    }
}

/// Select a wide integer unary op when the operator has one.
pub(super) fn select_wide_integer_unary_op(
    operator: mir::UnaryOperator,
    signed: bool,
) -> Option<Op> {
    Some(match (operator, signed) {
        (mir::UnaryOperator::Negate, true) => Op::NegWideInt,
        (mir::UnaryOperator::Not, _) => Op::NotWideInt,
        _ => return None,
    })
}

/// Select a load handler for one known pointer access.
pub(super) fn select_load_op(access: PointeeAccess) -> Result<Op, Error> {
    if !access.is_word() {
        return select_bytes_load_op(access.pointer_class);
    }

    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::LoadHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::LoadSharedHeap),
        PointerClass::Raw => Ok(Op::LoadRaw),
        PointerClass::SharedRaw => Ok(Op::LoadSharedRaw),
        PointerClass::Stack => Ok(Op::LoadStack),
        PointerClass::Frame => Ok(Op::LoadFrame),
        PointerClass::Static => Ok(Op::LoadStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select a store handler for one known pointer access.
pub(super) fn select_store_op(access: PointeeAccess) -> Result<Op, Error> {
    if !access.is_word() {
        return select_bytes_store_op(access.pointer_class);
    }

    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::StoreHeap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::StoreSharedHeap),
        PointerClass::Raw => Ok(Op::StoreRaw),
        PointerClass::SharedRaw => Ok(Op::StoreSharedRaw),
        PointerClass::Stack => Ok(Op::StoreStack),
        PointerClass::Frame => Ok(Op::StoreFrame),
        PointerClass::Static => Ok(Op::StoreStatic),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select one byte load operation.
fn select_bytes_load_op(pointer_class: PointerClass) -> Result<Op, Error> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::LoadHeapBytes),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::LoadSharedHeapBytes),
        PointerClass::Raw => Ok(Op::LoadRawBytes),
        PointerClass::SharedRaw => Ok(Op::LoadSharedRawBytes),
        PointerClass::Stack => Ok(Op::LoadStackBytes),
        PointerClass::Frame => Ok(Op::LoadFrameBytes),
        PointerClass::Static => Ok(Op::LoadStaticBytes),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select one byte store operation.
fn select_bytes_store_op(pointer_class: PointerClass) -> Result<Op, Error> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::StoreHeapBytes),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(Op::StoreSharedHeapBytes),
        PointerClass::Raw => Ok(Op::StoreRawBytes),
        PointerClass::SharedRaw => Ok(Op::StoreSharedRawBytes),
        PointerClass::Stack => Ok(Op::StoreStackBytes),
        PointerClass::Frame => Ok(Op::StoreFrameBytes),
        PointerClass::Static => Ok(Op::StoreStaticBytes),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select a field address handler based on inferred value layout.
pub(super) fn select_field_addr_op(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
) -> Result<Op, Error> {
    let layout = value_layouts.get(base).ok_or(Error::InvalidInstruction)?;
    let pointer_class = field_base_pointer_class(layout)?;

    select_address_op(pointer_class, Projection::Field)
}

/// Select an element address handler based on inferred value layout.
pub(super) fn select_element_addr_op(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
) -> Result<Op, Error> {
    let layout = value_layouts.get(array).ok_or(Error::InvalidInstruction)?;
    let pointer_class = element_base_pointer_class(layout)?;

    select_address_op(pointer_class, Projection::Element)
}

/// Select a slice element address handler based on the backing pointer class.
pub(super) fn select_slice_element_addr_op(pointer_class: PointerClass) -> Result<Op, Error> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::AddressHeapSliceElement),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            Ok(Op::AddressSharedHeapSliceElement)
        }
        PointerClass::Raw => Ok(Op::AddressRawSliceElement),
        PointerClass::SharedRaw => Ok(Op::AddressSharedRawSliceElement),
        PointerClass::Stack => Ok(Op::AddressStackSliceElement),
        PointerClass::Frame => Ok(Op::AddressFrameSliceElement),
        PointerClass::Static => Ok(Op::AddressStaticSliceElement),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select a field load handler based on inferred value layout.
pub(super) fn select_field_load_op(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Op, Error> {
    require_word_access(field.is_word(), "word field load", field.value_type)?;

    let layout = value_layouts.get(base).ok_or(Error::InvalidInstruction)?;
    let pointer_class = field_base_pointer_class(layout)?;

    select_projection_load_op(pointer_class, Projection::Field)
}

/// Select a field store handler based on inferred value layout.
pub(super) fn select_field_store_op(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Op, Error> {
    require_word_access(field.is_word(), "word field store", field.value_type)?;

    let layout = value_layouts.get(base).ok_or(Error::InvalidInstruction)?;
    let pointer_class = field_base_pointer_class(layout)?;

    select_projection_store_op(pointer_class, Projection::Field)
}

/// Select an element load handler based on inferred value layout.
pub(super) fn select_element_load_op(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Op, Error> {
    require_word_access(element.is_word(), "word element load", element.value_type)?;

    let layout = value_layouts.get(array).ok_or(Error::InvalidInstruction)?;
    let pointer_class = element_base_pointer_class(layout)?;

    select_projection_load_op(pointer_class, Projection::Element)
}

/// Select an element store handler based on inferred value layout.
pub(super) fn select_element_store_op(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Op, Error> {
    require_word_access(element.is_word(), "word element store", element.value_type)?;

    let layout = value_layouts.get(array).ok_or(Error::InvalidInstruction)?;
    let pointer_class = element_base_pointer_class(layout)?;

    select_projection_store_op(pointer_class, Projection::Element)
}
/// Field or element projection.
#[derive(Clone, Copy)]
enum Projection {
    /// Struct or tuple field.
    Field,
    /// Fixed array element.
    Element,
}

/// Return the pointer class addressed by one field access.
fn field_base_pointer_class(layout: ValueLayout) -> Result<PointerClass, Error> {
    match layout {
        ValueLayout::FrameBytes { .. } => Ok(PointerClass::Frame),
        ValueLayout::Pointer { pointer_class, .. } => Ok(pointer_class),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return the pointer class addressed by one element access.
fn element_base_pointer_class(layout: ValueLayout) -> Result<PointerClass, Error> {
    match layout {
        ValueLayout::FrameBytes { .. } | ValueLayout::Array { .. } => Ok(PointerClass::Frame),
        ValueLayout::Pointer { pointer_class, .. } => Ok(pointer_class),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Select one address operation for a projection.
fn select_address_op(pointer_class: PointerClass, access: Projection) -> Result<Op, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Op::AddressFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Op::AddressHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Op::AddressHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Op::AddressSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Op::AddressSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Op::AddressRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Op::AddressRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Op::AddressSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Op::AddressSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Op::AddressStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Op::AddressStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Op::AddressStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Op::AddressStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Select one load operation for a projection.
fn select_projection_load_op(pointer_class: PointerClass, access: Projection) -> Result<Op, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Op::LoadFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Op::LoadHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Op::LoadHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Op::LoadSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Op::LoadSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Op::LoadRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Op::LoadRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Op::LoadSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Op::LoadSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Op::LoadStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Op::LoadStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Op::LoadStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Op::LoadStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Select one store operation for a projection.
fn select_projection_store_op(
    pointer_class: PointerClass,
    access: Projection,
) -> Result<Op, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Op::StoreFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Op::StoreHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Op::StoreHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Op::StoreSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Op::StoreSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Op::StoreRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Op::StoreRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Op::StoreSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Op::StoreSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Op::StoreStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Op::StoreStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Op::StoreStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Op::StoreStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Select a switch handler based on inferred value layout.
pub(super) fn select_switch_op(value_layouts: &ValueLayoutMap, value: mir::Value) -> Op {
    match value_layouts.get(value) {
        Some(ValueLayout::Int { width, .. }) if width <= u32::BITS as u16 => Op::Switch32,
        Some(ValueLayout::Int { width, .. }) if width <= u64::BITS as u16 => Op::Switch64,
        _ => Op::SwitchWideInt,
    }
}

/// Select a switch table handler based on inferred value layout.
pub(super) fn select_switch_table_op(value_layouts: &ValueLayoutMap, value: mir::Value) -> Op {
    match value_layouts.get(value) {
        Some(ValueLayout::Int { width, .. }) if width <= u32::BITS as u16 => Op::SwitchTable32,
        Some(ValueLayout::Int { width, .. }) if width <= u64::BITS as u16 => Op::SwitchTable64,
        _ => Op::SwitchTableWideInt,
    }
}

/// Select a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_op(
    operator: mir::BinaryOperator,
    layout: Option<ValueLayout>,
) -> Option<Op> {
    let is_float64 = matches!(layout, Some(ValueLayout::Float { width: 64 }));

    Some(match (operator, is_float64) {
        (mir::BinaryOperator::Equal, _) => Op::BranchEqInt,
        (mir::BinaryOperator::NotEqual, _) => Op::BranchNeInt,
        (mir::BinaryOperator::SignedLessThan, _) => Op::BranchLtInt,
        (mir::BinaryOperator::SignedLessEqual, _) => Op::BranchLeInt,
        (mir::BinaryOperator::SignedGreaterThan, _) => Op::BranchGtInt,
        (mir::BinaryOperator::SignedGreaterEqual, _) => Op::BranchGeInt,
        (mir::BinaryOperator::UnsignedLessThan, _) => Op::BranchLtUint,
        (mir::BinaryOperator::UnsignedLessEqual, _) => Op::BranchLeUint,
        (mir::BinaryOperator::UnsignedGreaterThan, _) => Op::BranchGtUint,
        (mir::BinaryOperator::UnsignedGreaterEqual, _) => Op::BranchGeUint,
        (mir::BinaryOperator::FloatEqual, false) => Op::BranchEqF32,
        (mir::BinaryOperator::FloatEqual, true) => Op::BranchEqF64,
        (mir::BinaryOperator::FloatNotEqual, false) => Op::BranchNeF32,
        (mir::BinaryOperator::FloatNotEqual, true) => Op::BranchNeF64,
        (mir::BinaryOperator::FloatLessThan, false) => Op::BranchLtF32,
        (mir::BinaryOperator::FloatLessThan, true) => Op::BranchLtF64,
        (mir::BinaryOperator::FloatLessEqual, false) => Op::BranchLeF32,
        (mir::BinaryOperator::FloatLessEqual, true) => Op::BranchLeF64,
        (mir::BinaryOperator::FloatGreaterThan, false) => Op::BranchGtF32,
        (mir::BinaryOperator::FloatGreaterThan, true) => Op::BranchGtF64,
        (mir::BinaryOperator::FloatGreaterEqual, false) => Op::BranchGeF32,
        (mir::BinaryOperator::FloatGreaterEqual, true) => Op::BranchGeF64,
        _ => return None,
    })
}
