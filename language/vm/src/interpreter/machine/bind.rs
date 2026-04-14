use destack_heap::{Heap, StackPointer, Value};
use destack_mir as mir;
use smallvec::SmallVec;

use super::super::state::Frame;
use crate::diagnostic::Error;
use crate::executable::{ArgumentRange, CopyPair, CopyRange, Executable, INVALID_VALUE_ID};
use crate::interpreter::StackAllocation;

/// One by-value transfer payload between frame ownership domains.
#[derive(Clone, Debug)]
pub(crate) enum TransferredValue {
    /// One scalar or pointer-like value copied directly.
    Plain(Value),
    /// One owned typed storage payload.
    TypedStorage {
        /// The stored MIR type.
        ty: mir::LocalNodeId<mir::Type>,
        /// The owned typed bytes.
        bytes: Vec<u8>,
    },
}

/// Return the MIR type stored in one frame value slot.
pub(crate) fn frame_value_type(
    executable: &Executable,
    frame: &Frame,
    value: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let frame_layout = executable
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
    executable: &Executable,
    heap: &Heap,
    frames: &[Frame],
    ty: mir::LocalNodeId<mir::Type>,
    value: Value,
) -> Result<TransferredValue, Error> {
    let layout = executable.layout(ty).ok_or(Error::InvalidInstruction)?;

    // keep scalar and pointer-like values as plain payloads
    if layout.is_scalar() || value == Value::VOID {
        return Ok(TransferredValue::Plain(value));
    }

    // clone stack-backed whole-object storage eagerly
    if let Some(pointer) = value.as_stack_pointer() {
        if pointer.slot_offset != 0 {
            return Err(Error::TypeMismatch {
                expected: "whole-object stack-backed composite".to_string(),
                actual: format!("{value:?}"),
            });
        }

        let frame = frames
            .get(pointer.frame_idx)
            .ok_or(Error::InvalidManagedReference)?;
        let allocation = frame
            .stack_allocation(pointer.slot)
            .ok_or(Error::InvalidManagedReference)?;
        if allocation.storage_type() != ty {
            return Err(Error::TypeMismatch {
                expected: format!("{ty:?}"),
                actual: format!("{:?}", allocation.storage_type()),
            });
        }

        return Ok(TransferredValue::TypedStorage {
            ty,
            bytes: allocation.clone_bytes(),
        });
    }

    // clone managed whole-object storage eagerly
    if let Some(reference) = value.as_managed_reference() {
        let source_type = heap
            .managed_type_id(reference)
            .map(mir::LocalNodeId::new)
            .ok_or(Error::InvalidManagedReference)?;
        if source_type != ty {
            return Err(Error::TypeMismatch {
                expected: format!("{ty:?}"),
                actual: format!("{source_type:?}"),
            });
        }

        let bytes = heap
            .managed_bytes(reference)
            .ok_or(Error::InvalidManagedReference)?
            .into_owned();
        if bytes.len() != layout.byte_len {
            return Err(Error::InvalidManagedReference);
        }

        return Ok(TransferredValue::TypedStorage { ty, bytes });
    }

    Err(Error::TypeMismatch {
        expected: "plain value or typed composite storage".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Bind one owned transfer payload into a destination frame slot.
pub(crate) fn bind_transferred_value(
    executable: &Executable,
    values: &mut [Value],
    dest_frame: &mut Frame,
    dest_frame_index: usize,
    destination: mir::Value,
    transferred: TransferredValue,
) -> Result<(), Error> {
    let destination_type = frame_value_type(executable, dest_frame, destination)?;
    let layout = executable
        .layout(destination_type)
        .ok_or(Error::InvalidInstruction)?;

    // scalar destinations keep plain runtime values
    if layout.is_scalar() {
        let TransferredValue::Plain(value) = transferred else {
            return Err(Error::TypeMismatch {
                expected: "plain scalar payload".to_string(),
                actual: format!("{transferred:?}"),
            });
        };

        dest_frame.set_value(values, destination, value);
        return Ok(());
    }

    // allow missing defaults to flow through unchanged
    if let TransferredValue::Plain(value) = &transferred
        && value.is_void()
    {
        dest_frame.set_value(values, destination, *value);
        return Ok(());
    }

    // non-scalars always rematerialize into destination frame storage
    let TransferredValue::TypedStorage { ty, bytes } = transferred else {
        return Err(Error::TypeMismatch {
            expected: "typed storage payload".to_string(),
            actual: format!("{transferred:?}"),
        });
    };
    if ty != destination_type || bytes.len() != layout.byte_len {
        return Err(Error::TypeMismatch {
            expected: format!("{destination_type:?}"),
            actual: format!("{ty:?}"),
        });
    }

    let allocation = StackAllocation::from_bytes(bytes, destination_type);
    let slot = dest_frame.allocate_stack_allocation(allocation);
    let pointer = StackPointer::new(dest_frame_index, slot);

    dest_frame.set_value(values, destination, Value::stack_pointer(pointer));
    Ok(())
}

/// Materialize one owned transfer payload into escaped runtime storage.
pub(crate) fn materialize_transferred_value_for_escape(
    executable: &Executable,
    heap: &mut Heap,
    transferred: TransferredValue,
) -> Result<Value, Error> {
    match transferred {
        TransferredValue::Plain(value) => Ok(value),
        TransferredValue::TypedStorage { ty, bytes } => {
            let layout = executable.layout(ty).ok_or(Error::InvalidInstruction)?;
            let layout_id = executable.tree.type_layout_id(ty);
            let reference = heap
                .allocate_managed_bytes_borrowed_typed(
                    &bytes,
                    &layout.reference_map,
                    layout_id,
                    ty.id,
                )
                .map_err(Error::from)?;

            Ok(Value::managed_reference(reference))
        }
    }
}

/// Capture one ordered argument list into owned transfer payloads.
pub(crate) fn collect_transferred_values_range(
    executable: &Executable,
    heap: &Heap,
    frames: &[Frame],
    values: &[Value],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<SmallVec<[TransferredValue; 16]>, Error> {
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());

    for argument in argument_slice {
        let index = frame.value_base + argument.0 as usize;
        debug_assert!(
            (argument.0 as usize) < frame.value_count,
            "ssa value out of bounds: {argument:?}"
        );

        let value = unsafe { *values.get_unchecked(index) };
        let argument_type = frame_value_type(executable, frame, *argument)?;
        let transferred =
            capture_transferred_value(executable, heap, frames, argument_type, value)?;

        collected_arguments.push(transferred);
    }

    Ok(collected_arguments)
}

/// Capture one argument copy plan into owned transfer payloads.
pub(crate) fn collect_transferred_values_from_copies(
    executable: &Executable,
    heap: &Heap,
    frames: &[Frame],
    values: &[Value],
    frame: &Frame,
    copy_pool: &[CopyPair],
    copies: CopyRange,
) -> Result<SmallVec<[TransferredValue; 16]>, Error> {
    let pairs = copies.slice(copy_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    let values_ptr = values.as_ptr();

    for pair in pairs {
        let transferred = if pair.src == INVALID_VALUE_ID {
            TransferredValue::Plain(Value::VOID)
        } else {
            let src_value = mir::Value::new(pair.src);
            let src_index = frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );

            let value = unsafe { *values_ptr.add(src_index) };
            let source_type = frame_value_type(executable, frame, src_value)?;
            capture_transferred_value(executable, heap, frames, source_type, value)?
        };

        arguments.push(transferred);
    }

    Ok(arguments)
}

/// Bind owned transfer payloads to parameter slots in one frame.
pub(crate) fn bind_parameters_from_transferred_values(
    executable: &Executable,
    values: &mut [Value],
    frame: &mut Frame,
    frame_index: usize,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[TransferredValue],
) -> Result<(), Error> {
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let transferred = arguments
            .get(index)
            .cloned()
            .unwrap_or(TransferredValue::Plain(Value::VOID));

        bind_transferred_value(executable, values, frame, frame_index, *param, transferred)?;
    }

    Ok(())
}

/// Copy values between different frames using owned transfer payloads.
pub(crate) fn copy_values_between_frames_typed(
    executable: &Executable,
    heap: &Heap,
    frames: &[Frame],
    values: &mut [Value],
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
    let values_ptr = values.as_ptr();

    for (index, param) in param_slice.iter().enumerate() {
        let transferred = if let Some(argument) = argument_slice.get(index) {
            let arg_index = source_frame.value_base + argument.0 as usize;
            debug_assert!(
                (argument.0 as usize) < source_frame.value_count,
                "ssa value out of bounds: {argument:?}"
            );

            let value = unsafe { *values_ptr.add(arg_index) };
            let destination_type = frame_value_type(executable, dest_frame, *param)?;
            capture_transferred_value(executable, heap, frames, destination_type, value)?
        } else {
            TransferredValue::Plain(Value::VOID)
        };

        bind_transferred_value(
            executable,
            values,
            dest_frame,
            dest_frame_index,
            *param,
            transferred,
        )?;
    }

    Ok(())
}

/// Copy values between different frames using one precomputed plan.
pub(crate) fn copy_values_with_plan_typed(
    executable: &Executable,
    heap: &Heap,
    frames: &[Frame],
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &mut Frame,
    dest_frame_index: usize,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) -> Result<(), Error> {
    let pairs = copies.slice(copy_pool);
    let values_ptr = values.as_ptr();

    for pair in pairs {
        let destination = mir::Value::new(pair.dest);
        let transferred = if pair.src == INVALID_VALUE_ID {
            TransferredValue::Plain(Value::VOID)
        } else {
            let source = mir::Value::new(pair.src);
            let src_index = source_frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < source_frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );

            let value = unsafe { *values_ptr.add(src_index) };
            let destination_type = frame_value_type(executable, dest_frame, destination)?;
            let _ = source;
            capture_transferred_value(executable, heap, frames, destination_type, value)?
        };

        bind_transferred_value(
            executable,
            values,
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
    values: &mut [Value],
    source_frame: &Frame,
    dest_frame: &Frame,
    copies: CopyRange,
    copy_pool: &[CopyPair],
) {
    // copy contiguous plans directly when they line up
    if let Some((src_start, dest_start, len)) = copies.contiguous_plan() {
        let src_index = source_frame.value_base + src_start as usize;
        let dest_index = dest_frame.value_base + dest_start as usize;

        debug_assert!(
            (src_start as usize) + len <= source_frame.value_count,
            "ssa value out of bounds: {src_start}"
        );
        debug_assert!(
            (dest_start as usize) + len <= dest_frame.value_count,
            "ssa value out of bounds: {dest_start}"
        );

        let values_ptr = values.as_mut_ptr();
        unsafe {
            if std::ptr::eq(source_frame, dest_frame) {
                std::ptr::copy(values_ptr.add(src_index), values_ptr.add(dest_index), len);
            } else {
                std::ptr::copy_nonoverlapping(
                    values_ptr.add(src_index),
                    values_ptr.add(dest_index),
                    len,
                );
            }
        }
        return;
    }

    // apply each logical copy pair
    let values_ptr = values.as_mut_ptr();
    let pairs = copies.slice(copy_pool);

    for pair in pairs {
        let dest_index = dest_frame.value_base + pair.dest as usize;
        debug_assert!(
            (pair.dest as usize) < dest_frame.value_count,
            "ssa value out of bounds: {}",
            pair.dest
        );

        let value = if pair.src == INVALID_VALUE_ID {
            Value::VOID
        } else {
            let src_index = source_frame.value_base + pair.src as usize;
            debug_assert!(
                (pair.src as usize) < source_frame.value_count,
                "ssa value out of bounds: {}",
                pair.src
            );

            unsafe { *values_ptr.add(src_index) }
        };

        unsafe {
            *values_ptr.add(dest_index) = value;
        }
    }
}
