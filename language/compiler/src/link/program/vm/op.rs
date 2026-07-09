use destack_mir as mir;

use destack_program::CellLayout;
use destack_program::vm::{Op, Projection};

use super::value::{Operand, OperandMap};

/// One scalar load representation.
#[derive(Clone, Copy)]
enum ScalarLoad {
    /// Unsigned 8-bit load.
    U8,
    /// Signed 8-bit load.
    I8,
    /// Unsigned 16-bit load.
    U16,
    /// Signed 16-bit load.
    I16,
    /// Unsigned 32-bit load.
    U32,
    /// Signed 32-bit load.
    I32,
    /// 64-bit load.
    Width64,
}

/// One scalar store representation.
#[derive(Clone, Copy)]
enum ScalarStore {
    /// 8-bit store.
    Width8,
    /// 16-bit store.
    Width16,
    /// 32-bit store.
    Width32,
    /// 64-bit store.
    Width64,
}

/// Return the scalar load representation for one cell layout.
fn scalar_load(layout: CellLayout) -> Option<ScalarLoad> {
    Some(match layout {
        CellLayout::Void => return None,
        CellLayout::Boolean => ScalarLoad::U8,
        CellLayout::Int { width } if width <= 8 => ScalarLoad::I8,
        CellLayout::Int { width } if width <= 16 => ScalarLoad::I16,
        CellLayout::Int { width } if width <= 32 => ScalarLoad::I32,
        CellLayout::Int { width } if width <= 64 => ScalarLoad::Width64,
        CellLayout::Uint { width } if width <= 8 => ScalarLoad::U8,
        CellLayout::Uint { width } if width <= 16 => ScalarLoad::U16,
        CellLayout::Uint { width } if width <= 32 => ScalarLoad::U32,
        CellLayout::Uint { width } if width <= 64 => ScalarLoad::Width64,
        CellLayout::Float16 | CellLayout::Bfloat16 => ScalarLoad::U16,
        CellLayout::Float32 => ScalarLoad::U32,
        CellLayout::Float64
        | CellLayout::HeapReference
        | CellLayout::SharedHeapReference
        | CellLayout::Address
        | CellLayout::StackPointer
        | CellLayout::FramePointer
        | CellLayout::GlobalAddress
        | CellLayout::FunctionPointer => ScalarLoad::Width64,
        CellLayout::Int { .. } | CellLayout::Uint { .. } => {
            return None;
        }
    })
}

/// Return the scalar store representation for one cell layout.
fn scalar_store(layout: CellLayout) -> Option<ScalarStore> {
    Some(match layout {
        CellLayout::Void => return None,
        CellLayout::Boolean => ScalarStore::Width8,
        CellLayout::Int { width } | CellLayout::Uint { width } if width <= 8 => ScalarStore::Width8,
        CellLayout::Int { width } | CellLayout::Uint { width } if width <= 16 => {
            ScalarStore::Width16
        }
        CellLayout::Int { width } | CellLayout::Uint { width } if width <= 32 => {
            ScalarStore::Width32
        }
        CellLayout::Int { width } | CellLayout::Uint { width } if width <= 64 => {
            ScalarStore::Width64
        }
        CellLayout::Float16 | CellLayout::Bfloat16 => ScalarStore::Width16,
        CellLayout::Float32 => ScalarStore::Width32,
        CellLayout::Float64
        | CellLayout::HeapReference
        | CellLayout::SharedHeapReference
        | CellLayout::Address
        | CellLayout::StackPointer
        | CellLayout::FramePointer
        | CellLayout::GlobalAddress
        | CellLayout::FunctionPointer => ScalarStore::Width64,
        CellLayout::Int { .. } | CellLayout::Uint { .. } => {
            return None;
        }
    })
}

/// Select a binary op from one known operand.
pub(super) fn select_binary_op(
    layout: Option<Operand>,
    operator: mir::BinaryOperator,
) -> Option<Op> {
    use mir::BinaryOperator::*;

    let op = match layout {
        Some(Operand::Int { width, signed }) if width <= u64::BITS as u16 => {
            select_integer_op(operator, signed, width)?
        }
        Some(Operand::Int { signed, .. }) => select_wide_integer_op(operator, signed)?,
        Some(Operand::Float {
            format: mir::FloatType::Float32,
        }) => select_float_op(operator, false)?,
        Some(Operand::Float {
            format: mir::FloatType::Float64,
        }) => select_float_op(operator, true)?,
        Some(Operand::Boolean) => match operator {
            And => Op::AndBool,
            Or => Op::OrBool,
            Xor => Op::XorBool,
            Equal => Op::EqCell,
            NotEqual => Op::NeCell,
            _ => return None,
        },
        _ => return None,
    };

    Some(op)
}

/// Select a machine integer op when the operator has one.
pub(super) fn select_integer_op(
    operator: mir::BinaryOperator,
    signed: bool,
    width: u16,
) -> Option<Op> {
    if width > u64::BITS as u16 {
        return None;
    }

    match width {
        32 => select_integer_32_op(operator, signed),
        64 => select_integer_64_op(operator, signed),
        _ => select_integer_cell_op(operator, signed),
    }
}

/// Select a 32-bit integer op.
fn select_integer_32_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, signed) {
        (Add, true) => Op::AddI32,
        (Add, false) => Op::AddU32,
        (Subtract, true) => Op::SubI32,
        (Subtract, false) => Op::SubU32,
        (Multiply, true) => Op::MulI32,
        (Multiply, false) => Op::MulU32,
        (SignedDivide, true) => Op::DivI32,
        (UnsignedDivide, _) => Op::DivU32,
        (SignedRemainder, true) => Op::RemI32,
        (UnsignedRemainder, _) => Op::RemU32,
        (And, _) => Op::And32,
        (Or, _) => Op::Or32,
        (Xor, _) => Op::Xor32,
        (ShiftLeft, _) => Op::Shl32,
        (ArithmeticShiftRight, true) => Op::ShrI32,
        (LogicalShiftRight, _) => Op::ShrU32,
        (Equal, _) => Op::Eq32,
        (NotEqual, _) => Op::Ne32,
        (SignedLessThan, true) => Op::LtI32,
        (SignedLessEqual, true) => Op::LeI32,
        (SignedGreaterThan, true) => Op::GtI32,
        (SignedGreaterEqual, true) => Op::GeI32,
        (UnsignedLessThan, _) => Op::LtU32,
        (UnsignedLessEqual, _) => Op::LeU32,
        (UnsignedGreaterThan, _) => Op::GtU32,
        (UnsignedGreaterEqual, _) => Op::GeU32,
        _ => return None,
    })
}

/// Select a 64-bit integer op.
fn select_integer_64_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, signed) {
        (Add, true) => Op::AddI64,
        (Add, false) => Op::AddU64,
        (Subtract, true) => Op::SubI64,
        (Subtract, false) => Op::SubU64,
        (Multiply, true) => Op::MulI64,
        (Multiply, false) => Op::MulU64,
        (SignedDivide, true) => Op::DivI64,
        (UnsignedDivide, _) => Op::DivU64,
        (SignedRemainder, true) => Op::RemI64,
        (UnsignedRemainder, _) => Op::RemU64,
        (And, _) => Op::And64,
        (Or, _) => Op::Or64,
        (Xor, _) => Op::Xor64,
        (ShiftLeft, _) => Op::Shl64,
        (ArithmeticShiftRight, true) => Op::ShrI64,
        (LogicalShiftRight, _) => Op::ShrU64,
        (Equal, _) => Op::Eq64,
        (NotEqual, _) => Op::Ne64,
        (SignedLessThan, true) => Op::LtI64,
        (SignedLessEqual, true) => Op::LeI64,
        (SignedGreaterThan, true) => Op::GtI64,
        (SignedGreaterEqual, true) => Op::GeI64,
        (UnsignedLessThan, _) => Op::LtU64,
        (UnsignedLessEqual, _) => Op::LeU64,
        (UnsignedGreaterThan, _) => Op::GtU64,
        (UnsignedGreaterEqual, _) => Op::GeU64,
        _ => return None,
    })
}

/// Select an arbitrary-width integer op stored in one cell.
fn select_integer_cell_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, signed) {
        (Add, true) => Op::AddCellInt,
        (Add, false) => Op::AddCellUint,
        (Subtract, true) => Op::SubCellInt,
        (Subtract, false) => Op::SubCellUint,
        (Multiply, true) => Op::MulCellInt,
        (Multiply, false) => Op::MulCellUint,
        (SignedDivide, true) => Op::DivCellInt,
        (UnsignedDivide, _) => Op::DivCellUint,
        (SignedRemainder, true) => Op::RemCellInt,
        (UnsignedRemainder, _) => Op::RemCellUint,
        (And, _) => Op::AndCell,
        (Or, _) => Op::OrCell,
        (Xor, _) => Op::XorCell,
        (ShiftLeft, _) => Op::ShlCell,
        (ArithmeticShiftRight, true) => Op::ShrCellInt,
        (LogicalShiftRight, _) => Op::ShrCellUint,
        (Equal, _) => Op::EqCell,
        (NotEqual, _) => Op::NeCell,
        (SignedLessThan, true) => Op::LtCellInt,
        (SignedLessEqual, true) => Op::LeCellInt,
        (SignedGreaterThan, true) => Op::GtCellInt,
        (SignedGreaterEqual, true) => Op::GeCellInt,
        (UnsignedLessThan, _) => Op::LtCellUint,
        (UnsignedLessEqual, _) => Op::LeCellUint,
        (UnsignedGreaterThan, _) => Op::GtCellUint,
        (UnsignedGreaterEqual, _) => Op::GeCellUint,
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
fn select_float_op(operator: mir::BinaryOperator, is_64: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    let op = match (operator, is_64) {
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
        _ => return None,
    };

    Some(op)
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

    Some(match (operator, signed, width) {
        (mir::UnaryOperator::Negate, true, 32) => Op::NegI32,
        (mir::UnaryOperator::Negate, true, 64) => Op::NegI64,
        (mir::UnaryOperator::Negate, true, _) => Op::NegCellInt,
        (mir::UnaryOperator::Not, _, 32) => Op::Not32,
        (mir::UnaryOperator::Not, _, 64) => Op::Not64,
        (mir::UnaryOperator::Not, _, _) => Op::NotCell,
        _ => return None,
    })
}

/// Select a unary op from one known operand.
pub(super) fn select_unary_op(
    operand_map: &OperandMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Option<Op> {
    match operand_map.get(argument) {
        Some(Operand::Int { width, signed }) if width <= u64::BITS as u16 => {
            select_integer_unary_op(operator, signed, width)
        }
        Some(Operand::Int { signed, .. }) => select_wide_integer_unary_op(operator, signed),
        Some(Operand::Float {
            format: mir::FloatType::Float32,
        }) if operator == mir::UnaryOperator::FloatNegate => Some(Op::NegF32),
        Some(Operand::Float {
            format: mir::FloatType::Float64,
        }) if operator == mir::UnaryOperator::FloatNegate => Some(Op::NegF64),
        Some(Operand::Boolean) if operator == mir::UnaryOperator::Not => Some(Op::NotBool),
        _ => None,
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

/// Select a load handler for one pointer cell layout and projection.
pub(super) fn select_load_op(pointer: CellLayout, projection: Projection) -> Option<Op> {
    if !projection.is_cell() {
        return select_aggregate_load_op(pointer);
    }

    let layout = projection.cell_layout()?;
    let load = scalar_load(layout)?;

    select_scalar_load_op(pointer, load)
}

/// Select a store handler for one pointer cell layout and projection.
pub(super) fn select_store_op(pointer: CellLayout, projection: Projection) -> Option<Op> {
    if !projection.is_cell() {
        return select_aggregate_store_op(pointer);
    }

    let layout = projection.cell_layout()?;
    let store = scalar_store(layout)?;

    select_scalar_store_op(pointer, store)
}

/// Select one scalar load operation.
fn select_scalar_load_op(pointer: CellLayout, load: ScalarLoad) -> Option<Op> {
    Some(match (pointer, load) {
        (CellLayout::HeapReference, ScalarLoad::U8) => Op::LoadHeapU8,
        (CellLayout::HeapReference, ScalarLoad::I8) => Op::LoadHeapI8,
        (CellLayout::HeapReference, ScalarLoad::U16) => Op::LoadHeapU16,
        (CellLayout::HeapReference, ScalarLoad::I16) => Op::LoadHeapI16,
        (CellLayout::HeapReference, ScalarLoad::U32) => Op::LoadHeapU32,
        (CellLayout::HeapReference, ScalarLoad::I32) => Op::LoadHeapI32,
        (CellLayout::HeapReference, ScalarLoad::Width64) => Op::LoadHeap64,
        (CellLayout::SharedHeapReference, ScalarLoad::U8) => Op::LoadSharedHeapU8,
        (CellLayout::SharedHeapReference, ScalarLoad::I8) => Op::LoadSharedHeapI8,
        (CellLayout::SharedHeapReference, ScalarLoad::U16) => Op::LoadSharedHeapU16,
        (CellLayout::SharedHeapReference, ScalarLoad::I16) => Op::LoadSharedHeapI16,
        (CellLayout::SharedHeapReference, ScalarLoad::U32) => Op::LoadSharedHeapU32,
        (CellLayout::SharedHeapReference, ScalarLoad::I32) => Op::LoadSharedHeapI32,
        (CellLayout::SharedHeapReference, ScalarLoad::Width64) => Op::LoadSharedHeap64,
        (CellLayout::Address, ScalarLoad::U8) => Op::LoadRawU8,
        (CellLayout::Address, ScalarLoad::I8) => Op::LoadRawI8,
        (CellLayout::Address, ScalarLoad::U16) => Op::LoadRawU16,
        (CellLayout::Address, ScalarLoad::I16) => Op::LoadRawI16,
        (CellLayout::Address, ScalarLoad::U32) => Op::LoadRawU32,
        (CellLayout::Address, ScalarLoad::I32) => Op::LoadRawI32,
        (CellLayout::Address, ScalarLoad::Width64) => Op::LoadRaw64,
        (CellLayout::StackPointer, ScalarLoad::U8) => Op::LoadStackU8,
        (CellLayout::StackPointer, ScalarLoad::I8) => Op::LoadStackI8,
        (CellLayout::StackPointer, ScalarLoad::U16) => Op::LoadStackU16,
        (CellLayout::StackPointer, ScalarLoad::I16) => Op::LoadStackI16,
        (CellLayout::StackPointer, ScalarLoad::U32) => Op::LoadStackU32,
        (CellLayout::StackPointer, ScalarLoad::I32) => Op::LoadStackI32,
        (CellLayout::StackPointer, ScalarLoad::Width64) => Op::LoadStack64,
        (CellLayout::FramePointer, ScalarLoad::U8) => Op::LoadFrameU8,
        (CellLayout::FramePointer, ScalarLoad::I8) => Op::LoadFrameI8,
        (CellLayout::FramePointer, ScalarLoad::U16) => Op::LoadFrameU16,
        (CellLayout::FramePointer, ScalarLoad::I16) => Op::LoadFrameI16,
        (CellLayout::FramePointer, ScalarLoad::U32) => Op::LoadFrameU32,
        (CellLayout::FramePointer, ScalarLoad::I32) => Op::LoadFrameI32,
        (CellLayout::FramePointer, ScalarLoad::Width64) => Op::LoadFrame64,
        (CellLayout::GlobalAddress, ScalarLoad::U8) => Op::LoadStaticU8,
        (CellLayout::GlobalAddress, ScalarLoad::I8) => Op::LoadStaticI8,
        (CellLayout::GlobalAddress, ScalarLoad::U16) => Op::LoadStaticU16,
        (CellLayout::GlobalAddress, ScalarLoad::I16) => Op::LoadStaticI16,
        (CellLayout::GlobalAddress, ScalarLoad::U32) => Op::LoadStaticU32,
        (CellLayout::GlobalAddress, ScalarLoad::I32) => Op::LoadStaticI32,
        (CellLayout::GlobalAddress, ScalarLoad::Width64) => Op::LoadStatic64,
        _ => return None,
    })
}

/// Select one scalar store operation.
fn select_scalar_store_op(pointer: CellLayout, store: ScalarStore) -> Option<Op> {
    Some(match (pointer, store) {
        (CellLayout::HeapReference, ScalarStore::Width8) => Op::StoreHeap8,
        (CellLayout::HeapReference, ScalarStore::Width16) => Op::StoreHeap16,
        (CellLayout::HeapReference, ScalarStore::Width32) => Op::StoreHeap32,
        (CellLayout::HeapReference, ScalarStore::Width64) => Op::StoreHeap64,
        (CellLayout::SharedHeapReference, ScalarStore::Width8) => Op::StoreSharedHeap8,
        (CellLayout::SharedHeapReference, ScalarStore::Width16) => Op::StoreSharedHeap16,
        (CellLayout::SharedHeapReference, ScalarStore::Width32) => Op::StoreSharedHeap32,
        (CellLayout::SharedHeapReference, ScalarStore::Width64) => Op::StoreSharedHeap64,
        (CellLayout::Address, ScalarStore::Width8) => Op::StoreRaw8,
        (CellLayout::Address, ScalarStore::Width16) => Op::StoreRaw16,
        (CellLayout::Address, ScalarStore::Width32) => Op::StoreRaw32,
        (CellLayout::Address, ScalarStore::Width64) => Op::StoreRaw64,
        (CellLayout::StackPointer, ScalarStore::Width8) => Op::StoreStack8,
        (CellLayout::StackPointer, ScalarStore::Width16) => Op::StoreStack16,
        (CellLayout::StackPointer, ScalarStore::Width32) => Op::StoreStack32,
        (CellLayout::StackPointer, ScalarStore::Width64) => Op::StoreStack64,
        (CellLayout::FramePointer, ScalarStore::Width8) => Op::StoreFrame8,
        (CellLayout::FramePointer, ScalarStore::Width16) => Op::StoreFrame16,
        (CellLayout::FramePointer, ScalarStore::Width32) => Op::StoreFrame32,
        (CellLayout::FramePointer, ScalarStore::Width64) => Op::StoreFrame64,
        (CellLayout::GlobalAddress, ScalarStore::Width8) => Op::StoreStatic8,
        (CellLayout::GlobalAddress, ScalarStore::Width16) => Op::StoreStatic16,
        (CellLayout::GlobalAddress, ScalarStore::Width32) => Op::StoreStatic32,
        (CellLayout::GlobalAddress, ScalarStore::Width64) => Op::StoreStatic64,
        _ => return None,
    })
}

/// Select one frame value scalar load operation.
pub(super) fn select_frame_value_load_op(projection: Projection) -> Option<Op> {
    let layout = projection.cell_layout()?;

    Some(match scalar_load(layout)? {
        ScalarLoad::U8 => Op::LoadFrameValueU8,
        ScalarLoad::I8 => Op::LoadFrameValueI8,
        ScalarLoad::U16 => Op::LoadFrameValueU16,
        ScalarLoad::I16 => Op::LoadFrameValueI16,
        ScalarLoad::U32 => Op::LoadFrameValueU32,
        ScalarLoad::I32 => Op::LoadFrameValueI32,
        ScalarLoad::Width64 => Op::LoadFrameValue64,
    })
}

/// Select one frame value scalar store operation.
pub(super) fn select_frame_value_store_op(projection: Projection) -> Option<Op> {
    let layout = projection.cell_layout()?;

    Some(match scalar_store(layout)? {
        ScalarStore::Width8 => Op::StoreFrameValue8,
        ScalarStore::Width16 => Op::StoreFrameValue16,
        ScalarStore::Width32 => Op::StoreFrameValue32,
        ScalarStore::Width64 => Op::StoreFrameValue64,
    })
}

/// Select one aggregate load operation.
fn select_aggregate_load_op(pointer: CellLayout) -> Option<Op> {
    Some(match pointer {
        CellLayout::HeapReference => Op::LoadHeapAggregate,
        CellLayout::SharedHeapReference => Op::LoadSharedHeapAggregate,
        CellLayout::Address => Op::LoadRawAggregate,
        CellLayout::StackPointer => Op::LoadStackAggregate,
        CellLayout::FramePointer => Op::LoadFrameAggregate,
        CellLayout::GlobalAddress => Op::LoadStaticAggregate,
        _ => return None,
    })
}

/// Select one aggregate store operation.
fn select_aggregate_store_op(pointer: CellLayout) -> Option<Op> {
    Some(match pointer {
        CellLayout::HeapReference => Op::StoreHeapAggregate,
        CellLayout::SharedHeapReference => Op::StoreSharedHeapAggregate,
        CellLayout::Address => Op::StoreRawAggregate,
        CellLayout::StackPointer => Op::StoreStackAggregate,
        CellLayout::FramePointer => Op::StoreFrameAggregate,
        CellLayout::GlobalAddress => Op::StoreStaticAggregate,
        _ => return None,
    })
}

/// Select a field address handler from the lowered operand.
pub(super) fn select_field_addr_op(operand_map: &OperandMap, base: mir::Value) -> Option<Op> {
    let operand = operand_map.get(base)?;
    match operand {
        Operand::Aggregate { .. } => Some(Op::AddressFrameValueOffset),
        Operand::Reference { cell_layout, .. } => select_offset_address_op(cell_layout),
        _ => None,
    }
}

/// Select an element address handler from the lowered operand.
pub(super) fn select_element_addr_op(operand_map: &OperandMap, array: mir::Value) -> Option<Op> {
    let operand = operand_map.get(array)?;
    match operand {
        Operand::Aggregate { .. } | Operand::Sequence { .. } => Some(Op::AddressFrameValueElement),
        Operand::Reference { cell_layout, .. } => select_index_address_op(cell_layout),
        _ => None,
    }
}

/// Select a slice element address handler based on the backing pointer layout.
pub(super) fn select_slice_element_addr_op(pointer: CellLayout) -> Option<Op> {
    Some(match pointer {
        CellLayout::HeapReference => Op::AddressHeapSliceElement,
        CellLayout::SharedHeapReference => Op::AddressSharedHeapSliceElement,
        CellLayout::Address => Op::AddressRawSliceElement,
        CellLayout::StackPointer => Op::AddressStackSliceElement,
        CellLayout::FramePointer => Op::AddressFrameSliceElement,
        CellLayout::GlobalAddress => Op::GlobalAddressSliceElement,
        _ => return None,
    })
}

/// Select one indexed address operation.
fn select_index_address_op(pointer: CellLayout) -> Option<Op> {
    Some(match pointer {
        CellLayout::FramePointer => Op::AddressFrameElement,
        CellLayout::HeapReference => Op::AddressHeapElement,
        CellLayout::SharedHeapReference => Op::AddressSharedHeapElement,
        CellLayout::Address => Op::AddressRawElement,
        CellLayout::StackPointer => Op::AddressStackElement,
        CellLayout::GlobalAddress => Op::GlobalAddressElement,
        _ => return None,
    })
}

/// Select one fixed-offset address operation.
fn select_offset_address_op(pointer: CellLayout) -> Option<Op> {
    Some(match pointer {
        CellLayout::FramePointer => Op::AddressFrameOffset,
        CellLayout::HeapReference => Op::AddressHeapOffset,
        CellLayout::SharedHeapReference => Op::AddressSharedHeapOffset,
        CellLayout::Address => Op::AddressRawOffset,
        CellLayout::StackPointer => Op::AddressStackOffset,
        CellLayout::GlobalAddress => Op::GlobalAddressOffset,
        _ => return None,
    })
}

/// Select a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_op(
    operator: mir::BinaryOperator,
    layout: Option<Operand>,
) -> Option<Op> {
    let op = match layout {
        Some(Operand::Int { width: 32, signed }) => select_compare_branch_32_op(operator, signed)?,
        Some(Operand::Int { width: 64, signed }) => select_compare_branch_64_op(operator, signed)?,
        Some(Operand::Float {
            format: mir::FloatType::Float32,
        }) => select_float_branch_op(operator, false)?,
        Some(Operand::Float {
            format: mir::FloatType::Float64,
        }) => select_float_branch_op(operator, true)?,
        Some(Operand::Float { .. }) => return None,
        _ => select_compare_branch_cell_op(operator)?,
    };

    Some(op)
}

/// Select a 32-bit integer compare branch op.
fn select_compare_branch_32_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    Some(match (operator, signed) {
        (mir::BinaryOperator::Equal, _) => Op::BranchEq32,
        (mir::BinaryOperator::NotEqual, _) => Op::BranchNe32,
        (mir::BinaryOperator::SignedLessThan, true) => Op::BranchLtI32,
        (mir::BinaryOperator::SignedLessEqual, true) => Op::BranchLeI32,
        (mir::BinaryOperator::SignedGreaterThan, true) => Op::BranchGtI32,
        (mir::BinaryOperator::SignedGreaterEqual, true) => Op::BranchGeI32,
        (mir::BinaryOperator::UnsignedLessThan, _) => Op::BranchLtU32,
        (mir::BinaryOperator::UnsignedLessEqual, _) => Op::BranchLeU32,
        (mir::BinaryOperator::UnsignedGreaterThan, _) => Op::BranchGtU32,
        (mir::BinaryOperator::UnsignedGreaterEqual, _) => Op::BranchGeU32,
        _ => return None,
    })
}

/// Select a 64-bit integer compare branch op.
fn select_compare_branch_64_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    Some(match (operator, signed) {
        (mir::BinaryOperator::Equal, _) => Op::BranchEq64,
        (mir::BinaryOperator::NotEqual, _) => Op::BranchNe64,
        (mir::BinaryOperator::SignedLessThan, true) => Op::BranchLtI64,
        (mir::BinaryOperator::SignedLessEqual, true) => Op::BranchLeI64,
        (mir::BinaryOperator::SignedGreaterThan, true) => Op::BranchGtI64,
        (mir::BinaryOperator::SignedGreaterEqual, true) => Op::BranchGeI64,
        (mir::BinaryOperator::UnsignedLessThan, _) => Op::BranchLtU64,
        (mir::BinaryOperator::UnsignedLessEqual, _) => Op::BranchLeU64,
        (mir::BinaryOperator::UnsignedGreaterThan, _) => Op::BranchGtU64,
        (mir::BinaryOperator::UnsignedGreaterEqual, _) => Op::BranchGeU64,
        _ => return None,
    })
}

/// Select an arbitrary-width cell compare branch op.
fn select_compare_branch_cell_op(operator: mir::BinaryOperator) -> Option<Op> {
    Some(match operator {
        mir::BinaryOperator::Equal => Op::BranchEqCell,
        mir::BinaryOperator::NotEqual => Op::BranchNeCell,
        mir::BinaryOperator::SignedLessThan => Op::BranchLtCellInt,
        mir::BinaryOperator::SignedLessEqual => Op::BranchLeCellInt,
        mir::BinaryOperator::SignedGreaterThan => Op::BranchGtCellInt,
        mir::BinaryOperator::SignedGreaterEqual => Op::BranchGeCellInt,
        mir::BinaryOperator::UnsignedLessThan => Op::BranchLtCellUint,
        mir::BinaryOperator::UnsignedLessEqual => Op::BranchLeCellUint,
        mir::BinaryOperator::UnsignedGreaterThan => Op::BranchGtCellUint,
        mir::BinaryOperator::UnsignedGreaterEqual => Op::BranchGeCellUint,
        _ => return None,
    })
}

/// Select a float compare branch op.
fn select_float_branch_op(operator: mir::BinaryOperator, is_float64: bool) -> Option<Op> {
    Some(match (operator, is_float64) {
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
