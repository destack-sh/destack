use smallvec::SmallVec;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::Error;
use crate::interpreter::Frame;
use crate::program::{
    ArgumentRange, INVALID_VALUE_ID, MovePair, MoveRange, PointerClass, Program, ValueRepr,
    value_repr_from_type,
};
use crate::{FramePointer, Word};
use destack_heap::{Heap, Payload};

/// One owned value copied out of a frame.
#[derive(Clone, Debug)]
pub(crate) enum FrameValueData {
    /// One scalar or pointer word.
    Word(Word),
    /// One non-word frame byte range.
    Bytes(Box<[u8]>),
}

/// One owned value copied out of a frame with its MIR type.
#[derive(Clone, Debug)]
pub(crate) struct FrameValue {
    /// The MIR type carried with the raw value bits.
    ty: mir::LocalNodeId<mir::Type>,
    /// The owned value data.
    data: FrameValueData,
}

impl FrameValue {
    /// Create one word value.
    #[inline]
    pub(crate) fn word(ty: mir::LocalNodeId<mir::Type>, value: Word) -> Self {
        Self {
            ty,
            data: FrameValueData::Word(value),
        }
    }

    /// Create one byte value.
    #[inline]
    pub(crate) fn bytes(ty: mir::LocalNodeId<mir::Type>, bytes: Box<[u8]>) -> Self {
        Self {
            ty,
            data: FrameValueData::Bytes(bytes),
        }
    }

    /// Return this value as one word.
    #[inline]
    pub(crate) fn as_word(&self) -> Result<Word, Error> {
        match self.data {
            FrameValueData::Word(value) => Ok(value),
            FrameValueData::Bytes(_) => Err(Error::TypeMismatch {
                expected: "word frame value".to_string(),
                actual: format!("byte frame value: type={:?}", self.ty),
            }),
        }
    }
}

/// Return the MIR type stored in one frame value.
pub(crate) fn frame_value_type(
    program: &Program,
    frame: &Frame,
    value: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value_index(value.0)
        .ok_or(Error::InvalidInstruction)?;

    Ok(program.type_for_id(region.ty))
}

/// Return the addressable word for one frame value.
fn frame_value_word(program: &Program, frame: &Frame, value: mir::Value) -> Result<Word, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value_index(value.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout = program
        .layout_for_id(region.ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing frame value representation: type={:?}", region.ty),
        })?;

    if layout.is_scalar() {
        return Ok(frame.read_word(region));
    }

    Ok(Word::frame_pointer(FramePointer::from_address(
        frame.region_address(region),
    )))
}

/// Return one function return type.
pub(crate) fn function_return_type(
    program: &Program,
    function: mir::LocalNodeId<mir::Function>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let function = program.tree.get(function);

    (function.return_type)
        .ty()
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: "function return type".to_string(),
        })
}

/// Copy one word or frame byte range into an owned frame value.
pub(crate) fn frame_value_from_word(
    program: &Program,
    frames: &[Frame],
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<FrameValue, Error> {
    let layout = program
        .layout(ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing frame value layout: type={ty:?}"),
        })?;
    if layout.is_scalar() {
        return Ok(FrameValue::word(ty, value));
    }

    let pointer = value.as_frame_pointer();
    let frame = frames
        .iter()
        .find(|frame| frame.owns_stack_range(pointer.address(), layout.byte_len))
        .ok_or(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("{value:?}"),
        })?;
    let start =
        pointer
            .address()
            .checked_sub(frame.base_address())
            .ok_or(Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: format!("{value:?}"),
            })?;
    let end = start
        .checked_add(layout.byte_len)
        .ok_or(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("{value:?}"),
        })?;
    let bytes = frame
        .bytes()
        .get(start..end)
        .ok_or(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("{value:?}"),
        })?
        .to_vec()
        .into_boxed_slice();

    Ok(FrameValue::bytes(ty, bytes))
}

/// Copy one frame value into an owned value.
fn read_frame_value(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    value: mir::Value,
) -> Result<FrameValue, Error> {
    let ty = frame_value_type(program, frame, value)?;
    let value = frame_value_word(program, frame, value)?;

    frame_value_from_word(program, frames, ty, value)
}

/// Write one owned frame value into a destination frame region.
pub(crate) fn write_frame_value(
    program: &Program,
    dest_frame: &mut Frame,
    destination: mir::Value,
    value: FrameValue,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(dest_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value_index(destination.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout = program
        .layout_for_id(region.ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!(
                "missing destination value representation: type={:?}",
                region.ty
            ),
        })?;

    match (layout.is_scalar(), value.data) {
        (true, FrameValueData::Word(value)) => dest_frame.write_word(region, value),
        (false, FrameValueData::Bytes(bytes)) if bytes.len() == region.byte_len as usize => {
            dest_frame.region_bytes_mut(region).copy_from_slice(&bytes);
        }
        (false, FrameValueData::Word(value)) => {
            return Err(Error::TypeMismatch {
                expected: "byte frame value".to_string(),
                actual: format!("{value:?}"),
            });
        }
        (true, FrameValueData::Bytes(bytes)) => {
            return Err(Error::TypeMismatch {
                expected: "word frame value".to_string(),
                actual: format!("{} bytes", bytes.len()),
            });
        }
        (false, FrameValueData::Bytes(bytes)) => {
            return Err(Error::TypeMismatch {
                expected: format!("{} bytes", region.byte_len),
                actual: format!("{} bytes", bytes.len()),
            });
        }
    }

    Ok(())
}

/// Materialize one owned frame value into one engine boundary value.
pub(crate) fn materialize_frame_value(
    program: &Program,
    heap: &mut Heap,
    value: FrameValue,
) -> Result<engine::Value, Error> {
    match value.data {
        FrameValueData::Word(word) => materialize_word(program, value.ty, word),
        FrameValueData::Bytes(bytes) => {
            let layout_id = program
                .layout_id_for_type(value.ty)
                .ok_or(Error::InvalidInstruction)?;
            let layout = program.allocation_layout(layout_id)?;
            let reference = heap
                .allocate(layout, Payload::Bytes(&bytes))
                .map_err(Error::from)?;

            Ok(engine::Value::HeapReference(reference))
        }
    }
}

/// Build one frame value from one engine boundary value.
pub(crate) fn frame_value_from_materialized(
    ty: mir::LocalNodeId<mir::Type>,
    value: &engine::Value,
) -> Result<FrameValue, Error> {
    let value = match value {
        engine::Value::Void => Word::VOID,
        engine::Value::Bool(value) => Word::bool(*value),
        engine::Value::Int { value, width } => Word::int(*value, *width),
        engine::Value::UInt { value, width } => Word::uint(*value, *width),
        engine::Value::Float32 { bits } => Word::float32(f32::from_bits(*bits)),
        engine::Value::Float64 { bits } => Word::float64(f64::from_bits(*bits)),
        engine::Value::Char(value) => Word::char(*value),
        engine::Value::HeapReference(reference) => Word::heap_reference(*reference),
        engine::Value::SharedHeapReference(reference) => Word::shared_heap_reference(*reference),
        engine::Value::RawPointer(pointer) => Word::raw_pointer(*pointer),
        engine::Value::SharedRawPointer(pointer) => Word::shared_raw_pointer(*pointer),
    };

    Ok(FrameValue::word(ty, value))
}

/// Return one frame value as an external call word.
pub(crate) fn frame_value_as_external_word(value: FrameValue) -> Result<Word, Error> {
    value.as_word()
}

/// Materialize one word into one engine boundary value.
pub(crate) fn materialize_word(
    program: &Program,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<engine::Value, Error> {
    match value_repr_from_type(&program.tree, ty) {
        ValueRepr::Void => Ok(engine::Value::Void),
        ValueRepr::Bool => Ok(engine::Value::Bool(value.as_bool())),
        ValueRepr::Int {
            width,
            signed: true,
        } => Ok(engine::Value::Int {
            value: value.as_int(),
            width,
        }),
        ValueRepr::Int {
            width,
            signed: false,
        } => Ok(engine::Value::UInt {
            value: value.as_uint(),
            width,
        }),
        ValueRepr::Float { width: 32 } => Ok(engine::Value::Float32 {
            bits: value.as_float32().to_bits(),
        }),
        ValueRepr::Float { .. } => Ok(engine::Value::Float64 {
            bits: value.as_float64().to_bits(),
        }),
        ValueRepr::Char => {
            let value = value.as_char().ok_or(Error::InvalidInstruction)?;

            Ok(engine::Value::Char(value))
        }
        ValueRepr::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        } => Ok(engine::Value::HeapReference(value.as_heap_reference())),
        ValueRepr::Pointer {
            pointer_class: PointerClass::SharedHeap,
            ..
        } => Ok(engine::Value::SharedHeapReference(
            value.as_shared_heap_reference(),
        )),
        ValueRepr::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        } => Ok(engine::Value::RawPointer(value.as_raw_pointer())),
        ValueRepr::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        } => Ok(engine::Value::SharedRawPointer(
            value.as_shared_raw_pointer(),
        )),
        ValueRepr::FrameBytes { .. } | ValueRepr::Array { .. } => {
            Ok(engine::Value::HeapReference(value.as_heap_reference()))
        }
        _ => Err(Error::TypeMismatch {
            expected: "word value".to_string(),
            actual: format!("{:?}", value_repr_from_type(&program.tree, ty)),
        }),
    }
}

/// Move one frame value into another frame.
pub(crate) fn move_frame_value(
    program: &Program,
    source_frame: &Frame,
    source: mir::Value,
    dest_frame: &mut Frame,
    destination: mir::Value,
) -> Result<(), Error> {
    let source_layout = program
        .frame_layout_by_id(source_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let dest_layout = program
        .frame_layout_by_id(dest_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;

    let source_region = source_layout
        .value_index(source.0)
        .ok_or(Error::InvalidInstruction)?;
    let dest_region = dest_layout
        .value_index(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if source_region.byte_len != dest_region.byte_len
        || source_region.is_word != dest_region.is_word
    {
        return Err(Error::TypeMismatch {
            expected: format!(
                "{} bytes, word={}",
                dest_region.byte_len, dest_region.is_word
            ),
            actual: format!(
                "{} bytes, word={}",
                source_region.byte_len, source_region.is_word
            ),
        });
    }

    if dest_region.is_word {
        let value = source_frame.read_word(source_region);
        dest_frame.write_word(dest_region, value);

        return Ok(());
    }

    let bytes = source_frame.region_bytes(source_region);
    dest_frame
        .region_bytes_mut(dest_region)
        .copy_from_slice(bytes);

    Ok(())
}

/// Write void to one frame value.
fn write_void_value(
    program: &Program,
    frame: &mut Frame,
    destination: mir::Value,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value_index(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if region.is_word {
        frame.write_word(region, Word::VOID);

        return Ok(());
    }

    frame.region_bytes_mut(region).fill(0);

    Ok(())
}

/// Move call arguments between two frames.
pub(crate) fn move_arguments_between_frames(
    program: &Program,
    source_frame: &Frame,
    dest_frame: &mut Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<(), Error> {
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let Some(argument) = argument_slice.get(index) else {
            write_void_value(program, dest_frame, *param)?;

            continue;
        };

        move_frame_value(program, source_frame, *argument, dest_frame, *param)?;
    }

    Ok(())
}

/// Move frame values using one lowered move plan.
pub(crate) fn move_values(
    program: &Program,
    source_frame: &Frame,
    dest_frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    for pair in pairs {
        let destination = mir::Value::new(pair.dest);
        if pair.src == INVALID_VALUE_ID {
            write_void_value(program, dest_frame, destination)?;

            continue;
        }

        move_frame_value(
            program,
            source_frame,
            mir::Value::new(pair.src),
            dest_frame,
            destination,
        )?;
    }

    Ok(())
}

/// Copy one ordered argument list out of the current frame.
pub(crate) fn read_arguments(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());
    for argument in argument_slice {
        let value = read_frame_value(program, frames, frame, *argument)?;

        collected_arguments.push(value);
    }

    Ok(collected_arguments)
}

/// Copy one lowered argument plan out of the current frame.
pub(crate) fn read_planned_arguments(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    move_pool: &[MovePair],
    moves: MoveRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let pairs = moves.slice(move_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    for pair in pairs {
        let value = if pair.src == INVALID_VALUE_ID {
            let destination = mir::Value::new(pair.dest);
            let destination_type = frame_value_type(program, frame, destination)?;

            FrameValue::word(destination_type, Word::VOID)
        } else {
            let src_value = mir::Value::new(pair.src);
            read_frame_value(program, frames, frame, src_value)?
        };

        arguments.push(value);
    }

    Ok(arguments)
}

/// Write owned frame values into parameter regions.
pub(crate) fn write_parameters(
    program: &Program,
    frame: &mut Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[FrameValue],
) -> Result<(), Error> {
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let value = match arguments.get(index).cloned() {
            Some(value) => value,
            None => {
                let ty = frame_value_type(program, frame, *param)?;
                FrameValue::word(ty, Word::VOID)
            }
        };

        write_frame_value(program, frame, *param, value)?;
    }

    Ok(())
}
