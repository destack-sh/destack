use crate::{Value, ValueTag};
use smallvec::SmallVec;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::Error;
use crate::interpreter::Frame;
use crate::module::{ArgumentRange, CopyPair, CopyRange, INVALID_VALUE_ID, Module};
use destack_heap::Heap;

/// One value transfer between frame ownership domains.
pub(crate) type TransferredValue = Value;

/// Return the MIR type stored in one frame value slot.
pub(crate) fn frame_value_type(
    module: &Module,
    frame: &Frame,
    value: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let frame_layout = module
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let slot_index = frame_layout
        .value_slots
        .start
        .checked_add(value.0)
        .ok_or(Error::InvalidInstruction)?;
    let slot = frame_layout
        .slot(slot_index)
        .ok_or(Error::InvalidInstruction)?;

    Ok(slot.ty)
}

/// Capture one value into an owned transfer payload for the given type.
pub(crate) fn capture_transferred_value(
    _module: &Module,
    _heap: &Heap,
    _frames: &[Frame],
    _ty: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<TransferredValue, Error> {
    Ok(value)
}

/// Bind one owned transfer payload into a destination frame slot.
pub(crate) fn bind_transferred_value(
    module: &Module,
    dest_frame: &mut Frame,
    _dest_frame_index: usize,
    destination: mir::Value,
    transferred: TransferredValue,
) -> Result<(), Error> {
    let _destination_type = frame_value_type(module, dest_frame, destination)?;
    dest_frame.set_value(destination, transferred);

    Ok(())
}

/// Materialize one transfer payload into one durable boundary value.
pub(crate) fn materialize_transferred_value(
    module: &Module,
    heap: &mut Heap,
    transferred: TransferredValue,
) -> Result<engine::MaterializedValue, Error> {
    let _module = module;

    materialize_plain_value(heap, transferred)
}

/// Rebuild one transfer payload from one materialized boundary value.
pub(crate) fn transferred_value_from_materialized(
    value: &engine::MaterializedValue,
) -> Result<TransferredValue, Error> {
    match value {
        engine::MaterializedValue::Void => Ok(Value::VOID),
        engine::MaterializedValue::Bool(value) => Ok(Value::bool(*value)),
        engine::MaterializedValue::Int { value, width } => Ok(Value::int(*value, *width)),
        engine::MaterializedValue::UInt { value, width } => Ok(Value::uint(*value, *width)),
        engine::MaterializedValue::Float32 { bits } => Ok(Value::float32(f32::from_bits(*bits))),
        engine::MaterializedValue::Float64 { bits } => Ok(Value::float64(f64::from_bits(*bits))),
        engine::MaterializedValue::Char(value) => Ok(Value::char(*value)),
        engine::MaterializedValue::HeapReference(reference) => {
            Ok(Value::heap_reference(*reference))
        }
        engine::MaterializedValue::SharedHeapReference(reference) => {
            Ok(Value::shared_heap_reference(*reference))
        }
        engine::MaterializedValue::RawPointer(pointer) => Ok(Value::raw_pointer(*pointer)),
        engine::MaterializedValue::SharedRawPointer(pointer) => {
            Ok(Value::shared_raw_pointer(*pointer))
        }
        engine::MaterializedValue::Undefined
        | engine::MaterializedValue::FrameAddress(_)
        | engine::MaterializedValue::StaticAddress(_)
        | engine::MaterializedValue::Function(_) => Err(Error::TypeMismatch {
            expected: "runtime value".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Materialize one owned transfer payload into external runtime storage.
pub(crate) fn materialize_transferred_value_for_external_call(
    module: &Module,
    heap: &mut Heap,
    transferred: TransferredValue,
) -> Result<Value, Error> {
    let _module = module;
    let _heap = heap;

    Ok(transferred)
}

/// Materialize one scalar or pointer-like runtime value.
pub(crate) fn materialize_plain_value(
    heap: &mut Heap,
    value: Value,
) -> Result<engine::MaterializedValue, Error> {
    match value.tag() {
        ValueTag::Void => Ok(engine::MaterializedValue::Void),
        ValueTag::Bool => {
            let value = value.as_bool().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::Bool(value))
        }
        ValueTag::Int => {
            let (value, width) = value.as_int_with_width().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::Int { value, width })
        }
        ValueTag::UInt => {
            let (value, width) = value
                .as_uint_with_width()
                .ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::UInt { value, width })
        }
        ValueTag::Float32 => {
            let value = value.as_float32().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::Float32 {
                bits: value.to_bits(),
            })
        }
        ValueTag::Float64 => {
            let value = value.as_float64().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::Float64 {
                bits: value.to_bits(),
            })
        }
        ValueTag::Char => {
            let value = value.as_char().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::Char(value))
        }
        ValueTag::HeapReference => {
            let reference = value.as_heap_reference().ok_or(Error::InvalidInstruction)?;
            let reference = if reference.is_null() {
                reference
            } else {
                heap.stabilize_heap(reference).map_err(Error::from)?
            };

            Ok(engine::MaterializedValue::HeapReference(reference))
        }
        ValueTag::SharedHeapReference => {
            let reference = value
                .as_shared_heap_reference()
                .ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::SharedHeapReference(reference))
        }
        ValueTag::RawPointer => {
            let pointer = value.as_raw_pointer().ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::RawPointer(pointer))
        }
        ValueTag::SharedRawPointer => {
            let pointer = value
                .as_shared_raw_pointer()
                .ok_or(Error::InvalidInstruction)?;

            Ok(engine::MaterializedValue::SharedRawPointer(pointer))
        }

        ValueTag::StackPointer
        | ValueTag::FramePointer
        | ValueTag::GlobalPointer
        | ValueTag::FunctionPointer => Err(Error::TypeMismatch {
            expected: "plain boundary value".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Capture one ordered argument list into owned transfer payloads.
pub(crate) fn collect_transferred_values_range(
    module: &Module,
    heap: &Heap,
    frames: &[Frame],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<SmallVec<[TransferredValue; 16]>, Error> {
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());

    for argument in argument_slice {
        let value = frame.get_value_or_error(*argument)?;
        let argument_type = frame_value_type(module, frame, *argument)?;
        let transferred = capture_transferred_value(module, heap, frames, argument_type, value)?;

        collected_arguments.push(transferred);
    }

    Ok(collected_arguments)
}

/// Capture one argument copy plan into owned transfer payloads.
pub(crate) fn collect_transferred_values_from_copies(
    module: &Module,
    heap: &Heap,
    frames: &[Frame],
    frame: &Frame,
    copy_pool: &[CopyPair],
    copies: CopyRange,
) -> Result<SmallVec<[TransferredValue; 16]>, Error> {
    let pairs = copies.slice(copy_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());

    for pair in pairs {
        let transferred = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_value = mir::Value::new(pair.src);
            let value = frame.get_value_or_error(src_value)?;
            let source_type = frame_value_type(module, frame, src_value)?;
            capture_transferred_value(module, heap, frames, source_type, value)?
        };

        arguments.push(transferred);
    }

    Ok(arguments)
}

/// Bind owned transfer payloads to parameter slots in one frame.
pub(crate) fn bind_parameters_from_transferred_values(
    module: &Module,
    frame: &mut Frame,
    frame_index: usize,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[TransferredValue],
) -> Result<(), Error> {
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let transferred = arguments.get(index).copied().unwrap_or(Value::VOID);

        bind_transferred_value(module, frame, frame_index, *param, transferred)?;
    }

    Ok(())
}

/// Copy values between different frames using owned transfer payloads.
pub(crate) fn copy_values_between_frames_typed(
    module: &Module,
    heap: &Heap,
    frames: &[Frame],
    source_frame: &Frame,
    dest_frame: &mut Frame,
    dest_frame_index: usize,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<(), Error> {
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let transferred = if let Some(argument) = argument_slice.get(index) {
            let value = source_frame.get_value_or_error(*argument)?;
            let destination_type = frame_value_type(module, dest_frame, *param)?;
            capture_transferred_value(module, heap, frames, destination_type, value)?
        } else {
            Value::VOID
        };

        bind_transferred_value(module, dest_frame, dest_frame_index, *param, transferred)?;
    }

    Ok(())
}

/// Copy values between different frames using one precomputed plan.
pub(crate) fn copy_values_with_plan_typed(
    module: &Module,
    heap: &Heap,
    frames: &[Frame],
    source_frame: &Frame,
    dest_frame: &mut Frame,
    dest_frame_index: usize,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) -> Result<(), Error> {
    let pairs = copies.slice(copy_pool);

    for pair in pairs {
        let destination = mir::Value::new(pair.dest);
        let transferred = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let source = mir::Value::new(pair.src);
            let value = source_frame.get_value_or_error(source)?;
            let destination_type = frame_value_type(module, dest_frame, destination)?;
            capture_transferred_value(module, heap, frames, destination_type, value)?
        };

        bind_transferred_value(
            module,
            dest_frame,
            dest_frame_index,
            destination,
            transferred,
        )?;
    }

    Ok(())
}

/// Copy values between frames using one precomputed plan.
pub(crate) fn copy_values_with_plan(
    source_frame: &Frame,
    dest_frame: &mut Frame,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) -> Result<(), Error> {
    // apply each logical copy pair
    let pairs = copies.slice(copy_pool);
    let copied_values = pairs
        .iter()
        .map(|pair| {
            if pair.src == INVALID_VALUE_ID {
                Ok(Value::VOID)
            } else {
                source_frame.get_value_or_error(mir::Value::new(pair.src))
            }
        })
        .collect::<Result<SmallVec<[Value; 16]>, Error>>()?;

    for (pair, value) in pairs.iter().zip(copied_values) {
        dest_frame.set_value(mir::Value::new(pair.dest), value);
    }

    Ok(())
}
