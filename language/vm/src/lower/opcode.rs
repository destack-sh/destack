use destack_mir as mir;

use crate::diagnostic::Error;
use crate::program::{
    ElementAccess, FieldAccess, Opcode, PointeeAccess, PointerClass, ValueLayout,
};

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

/// Pick a binary handler from one known operand layout.
pub(super) fn select_binary_opcode(
    layout: Option<ValueLayout>,
    operator: mir::BinaryOperator,
) -> Opcode {
    use mir::BinaryOperator::*;

    match layout {
        Some(ValueLayout::Int { width, signed }) => {
            let wide_opcode = if signed {
                Opcode::BinaryWideInt
            } else {
                Opcode::BinaryWideUint
            };

            select_integer_opcode(operator, signed, width).unwrap_or(wide_opcode)
        }
        Some(ValueLayout::Float { width: 32 }) => select_float_opcode(operator, false),
        Some(ValueLayout::Float { width: 64 }) => select_float_opcode(operator, true),
        Some(ValueLayout::Bool) => match operator {
            And => Opcode::AndBool,
            Or => Opcode::OrBool,
            Xor => Opcode::XorBool,
            Equal => Opcode::EqInt,
            NotEqual => Opcode::NeInt,
            _ => Opcode::BinaryWideUint,
        },
        _ => Opcode::BinaryWideInt,
    }
}

/// Select a machine integer opcode when the operator has one.
pub(super) fn select_integer_opcode(
    operator: mir::BinaryOperator,
    signed: bool,
    width: u16,
) -> Option<Opcode> {
    use mir::BinaryOperator::*;

    if width > u64::BITS as u16 {
        return None;
    }

    Some(match (operator, signed) {
        (Add, _) => Opcode::AddInt,
        (Subtract, _) => Opcode::SubInt,
        (Multiply, _) => Opcode::MulInt,
        (SignedDivide, true) => Opcode::DivInt,
        (UnsignedDivide, _) => Opcode::DivUint,
        (SignedRemainder, true) => Opcode::RemInt,
        (UnsignedRemainder, _) => Opcode::RemUint,
        (And, _) => Opcode::AndInt,
        (Or, _) => Opcode::OrInt,
        (Xor, _) => Opcode::XorInt,
        (ShiftLeft, _) => Opcode::ShlInt,
        (ArithmeticShiftRight, true) => Opcode::ShrInt,
        (LogicalShiftRight, _) => Opcode::ShrUint,
        (Equal, _) => Opcode::EqInt,
        (NotEqual, _) => Opcode::NeInt,
        (SignedLessThan, true) => Opcode::LtInt,
        (SignedLessEqual, true) => Opcode::LeInt,
        (SignedGreaterThan, true) => Opcode::GtInt,
        (SignedGreaterEqual, true) => Opcode::GeInt,
        (UnsignedLessThan, _) => Opcode::LtUint,
        (UnsignedLessEqual, _) => Opcode::LeUint,
        (UnsignedGreaterThan, _) => Opcode::GtUint,
        (UnsignedGreaterEqual, _) => Opcode::GeUint,
        _ => return None,
    })
}

/// Select a machine float opcode for one binary operator.
fn select_float_opcode(operator: mir::BinaryOperator, is_64: bool) -> Opcode {
    use mir::BinaryOperator::*;

    match (operator, is_64) {
        (FloatAdd, false) => Opcode::AddF32,
        (FloatAdd, true) => Opcode::AddF64,
        (FloatSubtract, false) => Opcode::SubF32,
        (FloatSubtract, true) => Opcode::SubF64,
        (FloatMultiply, false) => Opcode::MulF32,
        (FloatMultiply, true) => Opcode::MulF64,
        (FloatDivide, false) => Opcode::DivF32,
        (FloatDivide, true) => Opcode::DivF64,
        (FloatEqual, false) => Opcode::EqF32,
        (FloatEqual, true) => Opcode::EqF64,
        (FloatNotEqual, false) => Opcode::NeF32,
        (FloatNotEqual, true) => Opcode::NeF64,
        (FloatLessThan, false) => Opcode::LtF32,
        (FloatLessThan, true) => Opcode::LtF64,
        (FloatLessEqual, false) => Opcode::LeF32,
        (FloatLessEqual, true) => Opcode::LeF64,
        (FloatGreaterThan, false) => Opcode::GtF32,
        (FloatGreaterThan, true) => Opcode::GtF64,
        (FloatGreaterEqual, false) => Opcode::GeF32,
        (FloatGreaterEqual, true) => Opcode::GeF64,
        _ => Opcode::BinaryWideInt,
    }
}

/// Select a machine integer unary opcode when the operator has one.
pub(super) fn select_integer_unary_opcode(
    operator: mir::UnaryOperator,
    signed: bool,
    width: u16,
) -> Option<Opcode> {
    if width > u64::BITS as u16 {
        return None;
    }

    Some(match (operator, signed) {
        (mir::UnaryOperator::Negate, true) => Opcode::NegInt,
        (mir::UnaryOperator::Not, _) => Opcode::NotInt,
        _ => return None,
    })
}

/// Pick a unary handler based on inferred operand layout.
pub(super) fn select_unary_opcode(
    value_layouts: &ValueLayoutMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Opcode {
    match value_layouts.get(argument) {
        Some(ValueLayout::Int { width, signed }) => {
            select_integer_unary_opcode(operator, signed, width).unwrap_or(Opcode::UnaryWideInt)
        }
        Some(ValueLayout::Float { width: 32 }) if operator == mir::UnaryOperator::FloatNegate => {
            Opcode::NegF32
        }
        Some(ValueLayout::Float { width: 64 }) if operator == mir::UnaryOperator::FloatNegate => {
            Opcode::NegF64
        }
        Some(ValueLayout::Bool) if operator == mir::UnaryOperator::Not => Opcode::NotBool,
        _ => Opcode::UnaryWideInt,
    }
}

/// Pick a load handler for one known pointer access.
pub(super) fn select_load_opcode(access: PointeeAccess) -> Result<Opcode, Error> {
    if !access.is_word() {
        return Ok(Opcode::LoadFrameBytes);
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
        return Ok(Opcode::StoreFrameBytes);
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

/// Pick a field address handler based on inferred value layout.
pub(super) fn select_field_addr_opcode(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
) -> Result<Opcode, Error> {
    let pointer_class = projection_pointer_class(value_layouts.get(base), false)?;

    select_address_opcode(pointer_class, Projection::Field)
}

/// Pick an element address handler based on inferred value layout.
pub(super) fn select_element_addr_opcode(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
) -> Result<Opcode, Error> {
    let pointer_class = projection_pointer_class(value_layouts.get(array), true)?;

    select_address_opcode(pointer_class, Projection::Element)
}

/// Pick a field load handler based on inferred value layout.
pub(super) fn select_field_load_opcode(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Opcode, Error> {
    require_word_access(field.is_word(), "word field load", field.value_type)?;

    let pointer_class = projection_pointer_class(value_layouts.get(base), false)?;

    select_projection_load_opcode(pointer_class, Projection::Field)
}

/// Pick a field store handler based on inferred value layout.
pub(super) fn select_field_store_opcode(
    value_layouts: &ValueLayoutMap,
    base: mir::Value,
    field: FieldAccess,
) -> Result<Opcode, Error> {
    require_word_access(field.is_word(), "word field store", field.value_type)?;

    let pointer_class = projection_pointer_class(value_layouts.get(base), false)?;

    select_projection_store_opcode(pointer_class, Projection::Field)
}

/// Pick an element load handler based on inferred value layout.
pub(super) fn select_element_load_opcode(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Opcode, Error> {
    require_word_access(element.is_word(), "word element load", element.value_type)?;

    let pointer_class = projection_pointer_class(value_layouts.get(array), true)?;

    select_projection_load_opcode(pointer_class, Projection::Element)
}

/// Pick an element store handler based on inferred value layout.
pub(super) fn select_element_store_opcode(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
    element: ElementAccess,
) -> Result<Opcode, Error> {
    require_word_access(element.is_word(), "word element store", element.value_type)?;

    let pointer_class = projection_pointer_class(value_layouts.get(array), true)?;

    select_projection_store_opcode(pointer_class, Projection::Element)
}

/// Field or element projection.
#[derive(Clone, Copy)]
enum Projection {
    /// Struct or tuple field.
    Field,
    /// Fixed array element.
    Element,
}

/// Return the pointer class addressed by one projection.
fn projection_pointer_class(
    layout: Option<ValueLayout>,
    is_array_allowed: bool,
) -> Result<PointerClass, Error> {
    match layout {
        Some(ValueLayout::FrameBytes { .. }) => Ok(PointerClass::Frame),
        Some(ValueLayout::Array { .. }) if is_array_allowed => Ok(PointerClass::Frame),
        Some(ValueLayout::Pointer { pointer_class, .. }) => Ok(pointer_class),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Pick one address opcode for a projection.
fn select_address_opcode(pointer_class: PointerClass, access: Projection) -> Result<Opcode, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Opcode::AddressFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Opcode::AddressHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Opcode::AddressHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Opcode::AddressSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Opcode::AddressSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Opcode::AddressRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Opcode::AddressRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Opcode::AddressSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Opcode::AddressSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Opcode::AddressStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Opcode::AddressStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Opcode::AddressStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Opcode::AddressStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Pick one load opcode for a projection.
fn select_projection_load_opcode(
    pointer_class: PointerClass,
    access: Projection,
) -> Result<Opcode, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Opcode::LoadFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Opcode::LoadHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Opcode::LoadHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Opcode::LoadSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Opcode::LoadSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Opcode::LoadRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Opcode::LoadRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Opcode::LoadSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Opcode::LoadSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Opcode::LoadStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Opcode::LoadStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Opcode::LoadStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Opcode::LoadStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Pick one store opcode for a projection.
fn select_projection_store_opcode(
    pointer_class: PointerClass,
    access: Projection,
) -> Result<Opcode, Error> {
    match (pointer_class, access) {
        (PointerClass::Frame, _) => Ok(Opcode::StoreFrame),
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Field) => {
            Ok(Opcode::StoreHeapField)
        }
        (PointerClass::Heap | PointerClass::HeapAddress, Projection::Element) => {
            Ok(Opcode::StoreHeapElement)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Field) => {
            Ok(Opcode::StoreSharedHeapField)
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, Projection::Element) => {
            Ok(Opcode::StoreSharedHeapElement)
        }
        (PointerClass::Raw, Projection::Field) => Ok(Opcode::StoreRawField),
        (PointerClass::Raw, Projection::Element) => Ok(Opcode::StoreRawElement),
        (PointerClass::SharedRaw, Projection::Field) => Ok(Opcode::StoreSharedRawField),
        (PointerClass::SharedRaw, Projection::Element) => Ok(Opcode::StoreSharedRawElement),
        (PointerClass::Stack, Projection::Field) => Ok(Opcode::StoreStackField),
        (PointerClass::Stack, Projection::Element) => Ok(Opcode::StoreStackElement),
        (PointerClass::Static, Projection::Field) => Ok(Opcode::StoreStaticField),
        (PointerClass::Static, Projection::Element) => Ok(Opcode::StoreStaticElement),
        (PointerClass::Unknown, _) => Err(Error::InvalidInstruction),
    }
}

/// Pick a branch handler based on inferred condition layout.
pub(super) fn select_branch_opcode(
    value_layouts: &ValueLayoutMap,
    condition: mir::Value,
) -> Opcode {
    match value_layouts.get(condition) {
        Some(ValueLayout::Bool) => Opcode::BranchBool,
        _ => Opcode::BranchBool,
    }
}

/// Pick a switch handler based on inferred value layout.
pub(super) fn select_switch_opcode(value_layouts: &ValueLayoutMap, value: mir::Value) -> Opcode {
    match value_layouts.get(value) {
        Some(ValueLayout::Int { width, .. }) if width <= u32::BITS as u16 => Opcode::Switch32,
        Some(ValueLayout::Int { width, .. }) if width <= u64::BITS as u16 => Opcode::Switch64,
        _ => Opcode::SwitchWideInt,
    }
}

/// Pick a switch table handler based on inferred value layout.
pub(super) fn select_switch_table_opcode(
    value_layouts: &ValueLayoutMap,
    value: mir::Value,
) -> Opcode {
    match value_layouts.get(value) {
        Some(ValueLayout::Int { width, .. }) if width <= u32::BITS as u16 => Opcode::SwitchTable32,
        Some(ValueLayout::Int { width, .. }) if width <= u64::BITS as u16 => Opcode::SwitchTable64,
        _ => Opcode::SwitchTableWideInt,
    }
}

/// Pick a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_opcode(
    operator: mir::BinaryOperator,
    layout: Option<ValueLayout>,
) -> Option<Opcode> {
    let is_float64 = matches!(layout, Some(ValueLayout::Float { width: 64 }));

    Some(match (operator, is_float64) {
        (mir::BinaryOperator::Equal, _) => Opcode::BranchEqInt,
        (mir::BinaryOperator::NotEqual, _) => Opcode::BranchNeInt,
        (mir::BinaryOperator::SignedLessThan, _) => Opcode::BranchLtInt,
        (mir::BinaryOperator::SignedLessEqual, _) => Opcode::BranchLeInt,
        (mir::BinaryOperator::SignedGreaterThan, _) => Opcode::BranchGtInt,
        (mir::BinaryOperator::SignedGreaterEqual, _) => Opcode::BranchGeInt,
        (mir::BinaryOperator::UnsignedLessThan, _) => Opcode::BranchLtUint,
        (mir::BinaryOperator::UnsignedLessEqual, _) => Opcode::BranchLeUint,
        (mir::BinaryOperator::UnsignedGreaterThan, _) => Opcode::BranchGtUint,
        (mir::BinaryOperator::UnsignedGreaterEqual, _) => Opcode::BranchGeUint,
        (mir::BinaryOperator::FloatEqual, false) => Opcode::BranchEqF32,
        (mir::BinaryOperator::FloatEqual, true) => Opcode::BranchEqF64,
        (mir::BinaryOperator::FloatNotEqual, false) => Opcode::BranchNeF32,
        (mir::BinaryOperator::FloatNotEqual, true) => Opcode::BranchNeF64,
        (mir::BinaryOperator::FloatLessThan, false) => Opcode::BranchLtF32,
        (mir::BinaryOperator::FloatLessThan, true) => Opcode::BranchLtF64,
        (mir::BinaryOperator::FloatLessEqual, false) => Opcode::BranchLeF32,
        (mir::BinaryOperator::FloatLessEqual, true) => Opcode::BranchLeF64,
        (mir::BinaryOperator::FloatGreaterThan, false) => Opcode::BranchGtF32,
        (mir::BinaryOperator::FloatGreaterThan, true) => Opcode::BranchGtF64,
        (mir::BinaryOperator::FloatGreaterEqual, false) => Opcode::BranchGeF32,
        (mir::BinaryOperator::FloatGreaterEqual, true) => Opcode::BranchGeF64,
        _ => return None,
    })
}
