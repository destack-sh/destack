use destack_mir as mir;

use crate::diagnostic::Error;
use crate::program::{Op, PointerClass, Projection, ValueLayout, WordLayout};

use super::value::ValueLayoutMap;

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

/// Return the scalar load representation for one word layout.
fn scalar_load(layout: WordLayout) -> Result<ScalarLoad, Error> {
    Ok(match layout {
        WordLayout::Void => return Err(Error::InvalidInstruction),
        WordLayout::Bool => ScalarLoad::U8,
        WordLayout::Int { width } if width <= 8 => ScalarLoad::I8,
        WordLayout::Int { width } if width <= 16 => ScalarLoad::I16,
        WordLayout::Int { width } if width <= 32 => ScalarLoad::I32,
        WordLayout::Int { width } if width <= 64 => ScalarLoad::Width64,
        WordLayout::Uint { width } if width <= 8 => ScalarLoad::U8,
        WordLayout::Uint { width } if width <= 16 => ScalarLoad::U16,
        WordLayout::Uint { width } if width <= 32 => ScalarLoad::U32,
        WordLayout::Uint { width } if width <= 64 => ScalarLoad::Width64,
        WordLayout::Float32 => ScalarLoad::U32,
        WordLayout::Float64
        | WordLayout::HeapReference
        | WordLayout::SharedHeapReference
        | WordLayout::RawPointer
        | WordLayout::SharedRawPointer
        | WordLayout::StackPointer
        | WordLayout::FramePointer
        | WordLayout::StaticPointer
        | WordLayout::FunctionPointer => ScalarLoad::Width64,
        WordLayout::Int { .. } | WordLayout::Uint { .. } => return Err(Error::InvalidInstruction),
    })
}

/// Return the scalar store representation for one word layout.
fn scalar_store(layout: WordLayout) -> Result<ScalarStore, Error> {
    Ok(match layout {
        WordLayout::Void => return Err(Error::InvalidInstruction),
        WordLayout::Bool => ScalarStore::Width8,
        WordLayout::Int { width } | WordLayout::Uint { width } if width <= 8 => ScalarStore::Width8,
        WordLayout::Int { width } | WordLayout::Uint { width } if width <= 16 => {
            ScalarStore::Width16
        }
        WordLayout::Int { width } | WordLayout::Uint { width } if width <= 32 => {
            ScalarStore::Width32
        }
        WordLayout::Int { width } | WordLayout::Uint { width } if width <= 64 => {
            ScalarStore::Width64
        }
        WordLayout::Float32 => ScalarStore::Width32,
        WordLayout::Float64
        | WordLayout::HeapReference
        | WordLayout::SharedHeapReference
        | WordLayout::RawPointer
        | WordLayout::SharedRawPointer
        | WordLayout::StackPointer
        | WordLayout::FramePointer
        | WordLayout::StaticPointer
        | WordLayout::FunctionPointer => ScalarStore::Width64,
        WordLayout::Int { .. } | WordLayout::Uint { .. } => return Err(Error::InvalidInstruction),
    })
}

/// Select a binary op from one known value layout.
pub(super) fn select_binary_op(
    layout: Option<ValueLayout>,
    operator: mir::BinaryOperator,
) -> Option<Op> {
    use mir::BinaryOperator::*;

    let op = match layout {
        Some(ValueLayout::Int { width, signed }) if width <= u64::BITS as u16 => {
            select_integer_op(operator, signed, width)?
        }
        Some(ValueLayout::Int { signed, .. }) => select_wide_integer_op(operator, signed)?,
        Some(ValueLayout::Float { width: 32 }) => select_float_op(operator, false)?,
        Some(ValueLayout::Float { width: 64 }) => select_float_op(operator, true)?,
        Some(ValueLayout::Bool) => match operator {
            And => Op::AndBool,
            Or => Op::OrBool,
            Xor => Op::XorBool,
            Equal => Op::EqWord,
            NotEqual => Op::NeWord,
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
        _ => select_integer_word_op(operator, signed),
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

/// Select an arbitrary-width integer op stored in one word.
fn select_integer_word_op(operator: mir::BinaryOperator, signed: bool) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, signed) {
        (Add, true) => Op::AddWordInt,
        (Add, false) => Op::AddWordUint,
        (Subtract, true) => Op::SubWordInt,
        (Subtract, false) => Op::SubWordUint,
        (Multiply, true) => Op::MulWordInt,
        (Multiply, false) => Op::MulWordUint,
        (SignedDivide, true) => Op::DivWordInt,
        (UnsignedDivide, _) => Op::DivWordUint,
        (SignedRemainder, true) => Op::RemWordInt,
        (UnsignedRemainder, _) => Op::RemWordUint,
        (And, _) => Op::AndWord,
        (Or, _) => Op::OrWord,
        (Xor, _) => Op::XorWord,
        (ShiftLeft, _) => Op::ShlWord,
        (ArithmeticShiftRight, true) => Op::ShrWordInt,
        (LogicalShiftRight, _) => Op::ShrWordUint,
        (Equal, _) => Op::EqWord,
        (NotEqual, _) => Op::NeWord,
        (SignedLessThan, true) => Op::LtWordInt,
        (SignedLessEqual, true) => Op::LeWordInt,
        (SignedGreaterThan, true) => Op::GtWordInt,
        (SignedGreaterEqual, true) => Op::GeWordInt,
        (UnsignedLessThan, _) => Op::LtWordUint,
        (UnsignedLessEqual, _) => Op::LeWordUint,
        (UnsignedGreaterThan, _) => Op::GtWordUint,
        (UnsignedGreaterEqual, _) => Op::GeWordUint,
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
        (mir::UnaryOperator::Negate, true, _) => Op::NegWordInt,
        (mir::UnaryOperator::Not, _, 32) => Op::Not32,
        (mir::UnaryOperator::Not, _, 64) => Op::Not64,
        (mir::UnaryOperator::Not, _, _) => Op::NotWord,
        _ => return None,
    })
}

/// Select a unary op from one known value layout.
pub(super) fn select_unary_op(
    value_layouts: &ValueLayoutMap,
    argument: mir::Value,
    operator: mir::UnaryOperator,
) -> Option<Op> {
    match value_layouts.get(argument) {
        Some(ValueLayout::Int { width, signed }) if width <= u64::BITS as u16 => {
            select_integer_unary_op(operator, signed, width)
        }
        Some(ValueLayout::Int { signed, .. }) => select_wide_integer_unary_op(operator, signed),
        Some(ValueLayout::Float { width: 32 }) if operator == mir::UnaryOperator::FloatNegate => {
            Some(Op::NegF32)
        }
        Some(ValueLayout::Float { width: 64 }) if operator == mir::UnaryOperator::FloatNegate => {
            Some(Op::NegF64)
        }
        Some(ValueLayout::Bool) if operator == mir::UnaryOperator::Not => Some(Op::NotBool),
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

/// Select a load handler for one known pointer class and projection.
pub(super) fn select_load_op(
    pointer_class: PointerClass,
    projection: Projection,
) -> Result<Op, Error> {
    if !projection.is_word() {
        return select_bytes_load_op(pointer_class);
    }

    let layout = projection.word_layout.ok_or(Error::InvalidInstruction)?;
    let load = scalar_load(layout)?;

    select_scalar_load_op(pointer_class, load)
}

/// Select a store handler for one known pointer class and projection.
pub(super) fn select_store_op(
    pointer_class: PointerClass,
    projection: Projection,
) -> Result<Op, Error> {
    if !projection.is_word() {
        return select_bytes_store_op(pointer_class);
    }

    let layout = projection.word_layout.ok_or(Error::InvalidInstruction)?;
    let store = scalar_store(layout)?;

    select_scalar_store_op(pointer_class, store)
}

/// Select one scalar load operation.
fn select_scalar_load_op(pointer_class: PointerClass, load: ScalarLoad) -> Result<Op, Error> {
    Ok(match (pointer_class, load) {
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::U8) => Op::LoadHeapU8,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::I8) => Op::LoadHeapI8,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::U16) => Op::LoadHeapU16,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::I16) => Op::LoadHeapI16,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::U32) => Op::LoadHeapU32,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::I32) => Op::LoadHeapI32,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarLoad::Width64) => Op::LoadHeap64,
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::U8) => {
            Op::LoadSharedHeapU8
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::I8) => {
            Op::LoadSharedHeapI8
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::U16) => {
            Op::LoadSharedHeapU16
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::I16) => {
            Op::LoadSharedHeapI16
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::U32) => {
            Op::LoadSharedHeapU32
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::I32) => {
            Op::LoadSharedHeapI32
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarLoad::Width64) => {
            Op::LoadSharedHeap64
        }
        (PointerClass::Raw, ScalarLoad::U8) => Op::LoadRawU8,
        (PointerClass::Raw, ScalarLoad::I8) => Op::LoadRawI8,
        (PointerClass::Raw, ScalarLoad::U16) => Op::LoadRawU16,
        (PointerClass::Raw, ScalarLoad::I16) => Op::LoadRawI16,
        (PointerClass::Raw, ScalarLoad::U32) => Op::LoadRawU32,
        (PointerClass::Raw, ScalarLoad::I32) => Op::LoadRawI32,
        (PointerClass::Raw, ScalarLoad::Width64) => Op::LoadRaw64,
        (PointerClass::SharedRaw, ScalarLoad::U8) => Op::LoadSharedRawU8,
        (PointerClass::SharedRaw, ScalarLoad::I8) => Op::LoadSharedRawI8,
        (PointerClass::SharedRaw, ScalarLoad::U16) => Op::LoadSharedRawU16,
        (PointerClass::SharedRaw, ScalarLoad::I16) => Op::LoadSharedRawI16,
        (PointerClass::SharedRaw, ScalarLoad::U32) => Op::LoadSharedRawU32,
        (PointerClass::SharedRaw, ScalarLoad::I32) => Op::LoadSharedRawI32,
        (PointerClass::SharedRaw, ScalarLoad::Width64) => Op::LoadSharedRaw64,
        (PointerClass::Stack, ScalarLoad::U8) => Op::LoadStackU8,
        (PointerClass::Stack, ScalarLoad::I8) => Op::LoadStackI8,
        (PointerClass::Stack, ScalarLoad::U16) => Op::LoadStackU16,
        (PointerClass::Stack, ScalarLoad::I16) => Op::LoadStackI16,
        (PointerClass::Stack, ScalarLoad::U32) => Op::LoadStackU32,
        (PointerClass::Stack, ScalarLoad::I32) => Op::LoadStackI32,
        (PointerClass::Stack, ScalarLoad::Width64) => Op::LoadStack64,
        (PointerClass::Frame, ScalarLoad::U8) => Op::LoadFrameU8,
        (PointerClass::Frame, ScalarLoad::I8) => Op::LoadFrameI8,
        (PointerClass::Frame, ScalarLoad::U16) => Op::LoadFrameU16,
        (PointerClass::Frame, ScalarLoad::I16) => Op::LoadFrameI16,
        (PointerClass::Frame, ScalarLoad::U32) => Op::LoadFrameU32,
        (PointerClass::Frame, ScalarLoad::I32) => Op::LoadFrameI32,
        (PointerClass::Frame, ScalarLoad::Width64) => Op::LoadFrame64,
        (PointerClass::Static, ScalarLoad::U8) => Op::LoadStaticU8,
        (PointerClass::Static, ScalarLoad::I8) => Op::LoadStaticI8,
        (PointerClass::Static, ScalarLoad::U16) => Op::LoadStaticU16,
        (PointerClass::Static, ScalarLoad::I16) => Op::LoadStaticI16,
        (PointerClass::Static, ScalarLoad::U32) => Op::LoadStaticU32,
        (PointerClass::Static, ScalarLoad::I32) => Op::LoadStaticI32,
        (PointerClass::Static, ScalarLoad::Width64) => Op::LoadStatic64,
        (PointerClass::Unknown, _) => return Err(Error::InvalidInstruction),
    })
}

/// Select one scalar store operation.
fn select_scalar_store_op(pointer_class: PointerClass, store: ScalarStore) -> Result<Op, Error> {
    Ok(match (pointer_class, store) {
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarStore::Width8) => Op::StoreHeap8,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarStore::Width16) => Op::StoreHeap16,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarStore::Width32) => Op::StoreHeap32,
        (PointerClass::Heap | PointerClass::HeapAddress, ScalarStore::Width64) => Op::StoreHeap64,
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarStore::Width8) => {
            Op::StoreSharedHeap8
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarStore::Width16) => {
            Op::StoreSharedHeap16
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarStore::Width32) => {
            Op::StoreSharedHeap32
        }
        (PointerClass::SharedHeap | PointerClass::SharedHeapAddress, ScalarStore::Width64) => {
            Op::StoreSharedHeap64
        }
        (PointerClass::Raw, ScalarStore::Width8) => Op::StoreRaw8,
        (PointerClass::Raw, ScalarStore::Width16) => Op::StoreRaw16,
        (PointerClass::Raw, ScalarStore::Width32) => Op::StoreRaw32,
        (PointerClass::Raw, ScalarStore::Width64) => Op::StoreRaw64,
        (PointerClass::SharedRaw, ScalarStore::Width8) => Op::StoreSharedRaw8,
        (PointerClass::SharedRaw, ScalarStore::Width16) => Op::StoreSharedRaw16,
        (PointerClass::SharedRaw, ScalarStore::Width32) => Op::StoreSharedRaw32,
        (PointerClass::SharedRaw, ScalarStore::Width64) => Op::StoreSharedRaw64,
        (PointerClass::Stack, ScalarStore::Width8) => Op::StoreStack8,
        (PointerClass::Stack, ScalarStore::Width16) => Op::StoreStack16,
        (PointerClass::Stack, ScalarStore::Width32) => Op::StoreStack32,
        (PointerClass::Stack, ScalarStore::Width64) => Op::StoreStack64,
        (PointerClass::Frame, ScalarStore::Width8) => Op::StoreFrame8,
        (PointerClass::Frame, ScalarStore::Width16) => Op::StoreFrame16,
        (PointerClass::Frame, ScalarStore::Width32) => Op::StoreFrame32,
        (PointerClass::Frame, ScalarStore::Width64) => Op::StoreFrame64,
        (PointerClass::Static, ScalarStore::Width8) => Op::StoreStatic8,
        (PointerClass::Static, ScalarStore::Width16) => Op::StoreStatic16,
        (PointerClass::Static, ScalarStore::Width32) => Op::StoreStatic32,
        (PointerClass::Static, ScalarStore::Width64) => Op::StoreStatic64,
        (PointerClass::Unknown, _) => return Err(Error::InvalidInstruction),
    })
}

/// Select one frame value scalar load operation.
pub(super) fn select_frame_value_load_op(projection: Projection) -> Result<Op, Error> {
    let layout = projection.word_layout.ok_or(Error::InvalidInstruction)?;

    Ok(match scalar_load(layout)? {
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
pub(super) fn select_frame_value_store_op(projection: Projection) -> Result<Op, Error> {
    let layout = projection.word_layout.ok_or(Error::InvalidInstruction)?;

    Ok(match scalar_store(layout)? {
        ScalarStore::Width8 => Op::StoreFrameValue8,
        ScalarStore::Width16 => Op::StoreFrameValue16,
        ScalarStore::Width32 => Op::StoreFrameValue32,
        ScalarStore::Width64 => Op::StoreFrameValue64,
    })
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

    select_offset_address_op(pointer_class)
}

/// Select an element address handler based on inferred value layout.
pub(super) fn select_element_addr_op(
    value_layouts: &ValueLayoutMap,
    array: mir::Value,
) -> Result<Op, Error> {
    let layout = value_layouts.get(array).ok_or(Error::InvalidInstruction)?;
    let pointer_class = element_base_pointer_class(layout)?;

    select_index_address_op(pointer_class)
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

/// Return the pointer class addressed by one field projection.
fn field_base_pointer_class(layout: ValueLayout) -> Result<PointerClass, Error> {
    match layout {
        ValueLayout::FrameBytes { .. } => Ok(PointerClass::Frame),
        ValueLayout::Pointer { pointer_class, .. } => Ok(pointer_class),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return the pointer class addressed by one element projection.
fn element_base_pointer_class(layout: ValueLayout) -> Result<PointerClass, Error> {
    match layout {
        ValueLayout::FrameBytes { .. } | ValueLayout::Array { .. } => Ok(PointerClass::Frame),
        ValueLayout::Pointer { pointer_class, .. } => Ok(pointer_class),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Select one indexed address operation.
fn select_index_address_op(pointer_class: PointerClass) -> Result<Op, Error> {
    match pointer_class {
        PointerClass::Frame => Ok(Op::AddressFrameElement),
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::AddressHeapElement),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            Ok(Op::AddressSharedHeapElement)
        }
        PointerClass::Raw => Ok(Op::AddressRawElement),
        PointerClass::SharedRaw => Ok(Op::AddressSharedRawElement),
        PointerClass::Stack => Ok(Op::AddressStackElement),
        PointerClass::Static => Ok(Op::AddressStaticElement),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select one fixed-offset address operation.
fn select_offset_address_op(pointer_class: PointerClass) -> Result<Op, Error> {
    match pointer_class {
        PointerClass::Frame => Ok(Op::AddressFrameOffset),
        PointerClass::Heap | PointerClass::HeapAddress => Ok(Op::AddressHeapOffset),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            Ok(Op::AddressSharedHeapOffset)
        }
        PointerClass::Raw => Ok(Op::AddressRawOffset),
        PointerClass::SharedRaw => Ok(Op::AddressSharedRawOffset),
        PointerClass::Stack => Ok(Op::AddressStackOffset),
        PointerClass::Static => Ok(Op::AddressStaticOffset),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Select a compare and branch handler based on operator type.
pub(super) fn select_compare_branch_op(
    operator: mir::BinaryOperator,
    layout: Option<ValueLayout>,
) -> Option<Op> {
    let op = match layout {
        Some(ValueLayout::Int { width: 32, signed }) => {
            select_compare_branch_32_op(operator, signed)?
        }
        Some(ValueLayout::Int { width: 64, signed }) => {
            select_compare_branch_64_op(operator, signed)?
        }
        Some(ValueLayout::Float { width }) => select_float_branch_op(operator, width == 64)?,
        _ => select_compare_branch_word_op(operator)?,
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

/// Select an arbitrary-width word compare branch op.
fn select_compare_branch_word_op(operator: mir::BinaryOperator) -> Option<Op> {
    Some(match operator {
        mir::BinaryOperator::Equal => Op::BranchEqWord,
        mir::BinaryOperator::NotEqual => Op::BranchNeWord,
        mir::BinaryOperator::SignedLessThan => Op::BranchLtWordInt,
        mir::BinaryOperator::SignedLessEqual => Op::BranchLeWordInt,
        mir::BinaryOperator::SignedGreaterThan => Op::BranchGtWordInt,
        mir::BinaryOperator::SignedGreaterEqual => Op::BranchGeWordInt,
        mir::BinaryOperator::UnsignedLessThan => Op::BranchLtWordUint,
        mir::BinaryOperator::UnsignedLessEqual => Op::BranchLeWordUint,
        mir::BinaryOperator::UnsignedGreaterThan => Op::BranchGtWordUint,
        mir::BinaryOperator::UnsignedGreaterEqual => Op::BranchGeWordUint,
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
