use super::*;

// FUGU #Performance: improve VM tensor performance

/// Handle tensor.load.
pub(crate) fn handle_tensor_load(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorLoad {
        dest,
        view,
        indices,
        view_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(&state.interpreter.isolate.tree, *view_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve indices
    let index_values = state.argument_slice(*indices);
    let mut index = Vec::with_capacity(index_values.len());
    for value_id in index_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => index.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return ControlFlow::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let pointer = match offset_pointer(view_value, offset, layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return ControlFlow::Error(error),
    };

    // load element
    let value = match instruction::load_from_pointer(state, pointer) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.store.
pub(crate) fn handle_tensor_store(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorStore {
        view,
        indices,
        value,
        view_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(&state.interpreter.isolate.tree, *view_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve indices
    let index_values = state.argument_slice(*indices);
    let mut index = Vec::with_capacity(index_values.len());
    for value_id in index_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => index.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return ControlFlow::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let pointer = match offset_pointer(view_value, offset, layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return ControlFlow::Error(error),
    };

    // store element
    let value = state.get(*value);
    if let Err(error) = instruction::store_to_pointer(state, pointer, value) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.fill.
pub(crate) fn handle_tensor_fill(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorFill {
        view,
        value,
        view_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(&state.interpreter.isolate.tree, *view_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let fill_value = state.get(*value);
    let base_pointer = state.get(*view);

    // fill each slot
    for offset in 0..layout.storage_len {
        let pointer = match offset_pointer(base_pointer, offset, layout.storage_len) {
            Ok(pointer) => pointer,
            Err(error) => return ControlFlow::Error(error),
        };
        if let Err(error) = instruction::store_to_pointer(state, pointer, fill_value) {
            return ControlFlow::Error(error);
        }
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.copy.
pub(crate) fn handle_tensor_copy(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorCopy {
        target,
        source,
        target_type,
        source_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let target_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *target_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // validate element counts
    if target_layout.storage_len != source_layout.storage_len {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "matching tensor sizes".to_string(),
            actual: format!(
                "{} vs {}",
                target_layout.storage_len, source_layout.storage_len
            ),
        });
    }

    // copy elements
    let target_ptr = state.get(*target);
    let source_ptr = state.get(*source);
    for offset in 0..target_layout.storage_len {
        let src = match offset_pointer(source_ptr, offset, source_layout.storage_len) {
            Ok(pointer) => pointer,
            Err(error) => return ControlFlow::Error(error),
        };
        let dst = match offset_pointer(target_ptr, offset, target_layout.storage_len) {
            Ok(pointer) => pointer,
            Err(error) => return ControlFlow::Error(error),
        };
        let value = match instruction::load_from_pointer(state, src) {
            Ok(value) => value,
            Err(error) => return ControlFlow::Error(error),
        };
        if let Err(error) = instruction::store_to_pointer(state, dst, value) {
            return ControlFlow::Error(error);
        }
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.reshape.
pub(crate) fn handle_tensor_reshape(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorReshape {
        dest,
        tensor,
        shape,
        source_type: _,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve output layout
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // compute expected element count from shape values when provided
    let shape_values = state.argument_slice(*shape);
    let mut shape_len = 1u64;
    for value_id in shape_values {
        let value = state.get(*value_id);
        let size = match value_to_u64(value) {
            Ok(size) => size,
            Err(error) => return ControlFlow::Error(error),
        };
        shape_len = shape_len.saturating_mul(size);
    }
    if shape_values.is_empty() {
        shape_len = dest_layout.storage_len as u64;
    }

    // validate element counts
    if source_slots.len() as u64 != shape_len {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "reshape element count".to_string(),
            actual: format!("{} vs {}", source_slots.len(), shape_len),
        });
    }
    if dest_layout.storage_len != source_slots.len() {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "reshape destination size".to_string(),
            actual: format!("{} vs {}", dest_layout.storage_len, source_slots.len()),
        });
    }

    // allocate reshaped tensor
    let result = state.interpreter.allocate_aggregate(source_slots.to_vec());
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.broadcast.
pub(crate) fn handle_tensor_broadcast(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorBroadcast {
        dest,
        tensor,
        dimensions,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // validate dimension mapping
    if dimensions.len() != source_layout.shape.len() {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "broadcast dimension mapping".to_string(),
            actual: format!("{} vs {}", dimensions.len(), source_layout.shape.len()),
        });
    }

    // allocate destination storage
    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // populate broadcasted values
    for_each_index(&dest_layout.shape, |output_index| {
        for (i, dim) in dimensions.iter().enumerate() {
            let output_value = output_index[*dim as usize];
            let source_dim = source_layout.shape[i];
            input_index[i] = if source_dim == 1 { 0 } else { output_value };
        }

        let Ok(src_offset) =
            tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)
        else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = source_slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.transpose.
pub(crate) fn handle_tensor_transpose(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorTranspose {
        dest,
        tensor,
        permutation,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // validate permutation
    if permutation.len() != source_layout.shape.len() {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "transpose permutation".to_string(),
            actual: format!("{} vs {}", permutation.len(), source_layout.shape.len()),
        });
    }

    // allocate destination storage
    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // transpose values
    for_each_index(&dest_layout.shape, |output_index| {
        for (out_dim, in_dim) in permutation.iter().enumerate() {
            input_index[*in_dim as usize] = output_index[out_dim];
        }

        let Ok(src_offset) =
            tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)
        else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = source_slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.slice.
pub(crate) fn handle_tensor_slice(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorSlice {
        dest,
        tensor,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve arguments
    let args = state.argument_slice(*arguments);
    let (offset_values, rest) = args.split_at(*offsets_count as usize);
    let (size_values, stride_values) = rest.split_at(*sizes_count as usize);
    if stride_values.len() != *strides_count as usize {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for value_id in offset_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in size_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in stride_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // allocate destination storage
    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // slice values
    for_each_index(&dest_layout.shape, |output_index| {
        for i in 0..input_index.len() {
            let offset = offsets.get(i).copied().unwrap_or(0);
            let stride = strides.get(i).copied().unwrap_or(1);
            input_index[i] = offset + output_index[i] * stride;
        }

        let Ok(src_offset) =
            tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)
        else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = source_slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.pad.
pub(crate) fn handle_tensor_pad(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorPad {
        dest,
        tensor,
        arguments,
        low_count,
        high_count,
        interior_count,
        value,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve arguments
    let args = state.argument_slice(*arguments);
    let (low_values, rest) = args.split_at(*low_count as usize);
    let (high_values, interior_values) = rest.split_at(*high_count as usize);
    if interior_values.len() != *interior_count as usize {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let mut low = Vec::with_capacity(low_values.len());
    let mut high = Vec::with_capacity(high_values.len());
    let mut interior = Vec::with_capacity(interior_values.len());
    for value_id in low_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => low.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in high_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => high.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in interior_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => interior.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // allocate destination storage
    let pad_value = state.get(*value);
    let mut output = vec![pad_value; dest_layout.storage_len];
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // map output indices to input indices
    for_each_index(&dest_layout.shape, |output_index| {
        let mut is_padding = false;
        for i in 0..input_index.len() {
            let low_pad = low.get(i).copied().unwrap_or(0);
            let interior_pad = interior.get(i).copied().unwrap_or(0);
            let mut idx = output_index[i] as i64 - low_pad as i64;
            if idx < 0 {
                is_padding = true;
                break;
            }
            let stride = interior_pad + 1;
            if !(idx as u64).is_multiple_of(stride) {
                is_padding = true;
                break;
            }
            idx /= stride as i64;
            if idx < 0 || idx as u64 >= source_layout.shape[i] {
                is_padding = true;
                break;
            }
            input_index[i] = idx as u64;
        }

        if is_padding {
            return;
        }
        let Ok(src_offset) =
            tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)
        else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = source_slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.concat.
pub(crate) fn handle_tensor_concat(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorConcat {
        dest,
        tensors,
        tensor_types,
        axis,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve destination layout
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve input tensors
    let tensor_ids = state.argument_slice(*tensors);
    // validate input metadata
    if tensor_ids.len() != tensor_types.len() {
        return ControlFlow::Error(Error::InvalidInstruction);
    }
    let mut inputs = Vec::with_capacity(tensor_ids.len());
    let mut axis_sizes = Vec::with_capacity(tensor_ids.len());
    for (value_id, type_id) in tensor_ids.iter().zip(tensor_types.iter()) {
        let value = state.get(*value_id);
        let slots = match aggregate_slots(state, value) {
            Ok(slots) => slots,
            Err(error) => return ControlFlow::Error(error),
        };
        let layout = match tensor_layout_info(&state.interpreter.isolate.tree, *type_id) {
            Ok(layout) => layout,
            Err(error) => return ControlFlow::Error(error),
        };
        let axis_index = *axis as usize;
        if axis_index >= layout.shape.len() {
            return ControlFlow::Error(Error::InvalidInstruction);
        }
        axis_sizes.push(layout.shape[axis_index]);
        inputs.push((layout, slots.to_vec()));
    }

    // compute axis offsets
    let mut axis_offsets = Vec::with_capacity(axis_sizes.len());
    let mut running = 0u64;
    for size in &axis_sizes {
        axis_offsets.push(running);
        running = running.saturating_add(*size);
    }

    // allocate destination storage
    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let axis_index = *axis as usize;
    let mut input_index = vec![0u64; dest_layout.shape.len()];

    // concatenate values
    for_each_index(&dest_layout.shape, |output_index| {
        let axis_value = output_index[axis_index];
        let mut selected = None;
        for (i, offset) in axis_offsets.iter().enumerate() {
            let size = axis_sizes[i];
            if axis_value >= *offset && axis_value < *offset + size {
                selected = Some((i, axis_value - *offset));
                break;
            }
        }
        let Some((input_idx, local_axis)) = selected else {
            return;
        };

        input_index.clone_from_slice(output_index);
        input_index[axis_index] = local_axis;
        let (layout, slots) = &inputs[input_idx];

        let Ok(src_offset) = tensor_linear_index(&input_index, &layout.shape, &layout.strides)
        else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.reduce.
pub(crate) fn handle_tensor_reduce(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorReduce {
        dest,
        operator,
        tensor,
        initial,
        axes,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // allocate output storage
    let init_value = state.get(*initial);
    let mut output = vec![init_value; dest_layout.storage_len];
    let reduce_axes: std::collections::HashSet<u32> = axes.iter().copied().collect();
    let mut output_index = Vec::new();

    // reduce values
    for_each_index(&source_layout.shape, |index| {
        output_index.clear();
        for (dim, value) in index.iter().enumerate() {
            if !reduce_axes.contains(&(dim as u32)) {
                output_index.push(*value);
            }
        }

        let src_offset =
            match tensor_linear_index(index, &source_layout.shape, &source_layout.strides) {
                Ok(offset) => offset,
                Err(_) => return,
            };
        let dst_offset =
            match tensor_linear_index(&output_index, &dest_layout.shape, &dest_layout.strides) {
                Ok(offset) => offset,
                Err(_) => return,
            };
        let Some(src_value) = source_slots.get(src_offset) else {
            return;
        };
        if let Some(slot) = output.get_mut(dst_offset) {
            let op = ReduceOperator::from(*operator);
            if let Ok(value) = apply_reduce_operator(op, *slot, *src_value) {
                *slot = value;
            }
        }
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.dot.
pub(crate) fn handle_tensor_dot(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorDot {
        dest,
        left,
        right,
        dimensions,
        left_type,
        right_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let left_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *left_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let right_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *right_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve source slots
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_slots = match aggregate_slots(state, left_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let right_slots = match aggregate_slots(state, right_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // validate storage lengths
    if left_slots.len() != right_slots.len() || left_slots.len() != dest_layout.storage_len {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "matching tensor storage".to_string(),
            actual: format!(
                "{} vs {} vs {}",
                left_slots.len(),
                right_slots.len(),
                dest_layout.storage_len
            ),
        });
    }

    // compute axis sets
    let lhs_batch = &dimensions.lhs_batch;
    let rhs_batch = &dimensions.rhs_batch;
    let lhs_contract = &dimensions.lhs_contracting;
    let rhs_contract = &dimensions.rhs_contracting;
    if lhs_batch.len() != rhs_batch.len() || lhs_contract.len() != rhs_contract.len() {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let lhs_rank = left_layout.shape.len();
    let rhs_rank = right_layout.shape.len();
    let mut lhs_free = Vec::new();
    let mut rhs_free = Vec::new();
    for i in 0..lhs_rank {
        if !lhs_batch.contains(&(i as u32)) && !lhs_contract.contains(&(i as u32)) {
            lhs_free.push(i as u32);
        }
    }
    for i in 0..rhs_rank {
        if !rhs_batch.contains(&(i as u32)) && !rhs_contract.contains(&(i as u32)) {
            rhs_free.push(i as u32);
        }
    }

    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let mut lhs_index = vec![0u64; lhs_rank];
    let mut rhs_index = vec![0u64; rhs_rank];

    // compute dot product
    for_each_index(&dest_layout.shape, |out_index| {
        for (i, dim) in lhs_batch.iter().enumerate() {
            let value = out_index[i];
            lhs_index[*dim as usize] = value;
            rhs_index[rhs_batch[i] as usize] = value;
        }
        for (i, dim) in lhs_free.iter().enumerate() {
            lhs_index[*dim as usize] = out_index[lhs_batch.len() + i];
        }
        for (i, dim) in rhs_free.iter().enumerate() {
            let offset = lhs_batch.len() + lhs_free.len() + i;
            rhs_index[*dim as usize] = out_index[offset];
        }

        let contract_shape: Vec<u64> = lhs_contract
            .iter()
            .map(|dim| left_layout.shape[*dim as usize])
            .collect();
        let mut accum = None;
        for_each_index(&contract_shape, |contract_index| {
            for (i, dim) in lhs_contract.iter().enumerate() {
                lhs_index[*dim as usize] = contract_index[i];
            }
            for (i, dim) in rhs_contract.iter().enumerate() {
                rhs_index[*dim as usize] = contract_index[i];
            }

            let lhs_offset =
                tensor_linear_index(&lhs_index, &left_layout.shape, &left_layout.strides);
            let rhs_offset =
                tensor_linear_index(&rhs_index, &right_layout.shape, &right_layout.strides);
            let (Ok(lhs_offset), Ok(rhs_offset)) = (lhs_offset, rhs_offset) else {
                return;
            };
            let Some(lhs_val) = left_slots.get(lhs_offset) else {
                return;
            };
            let Some(rhs_val) = right_slots.get(rhs_offset) else {
                return;
            };

            let product = match apply_reduce_operator(ReduceOperator::Multiply, *lhs_val, *rhs_val)
            {
                Ok(value) => value,
                Err(_) => return,
            };
            accum = match accum {
                None => Some(product),
                Some(current) => apply_reduce_operator(ReduceOperator::Add, current, product).ok(),
            };
        });

        if let Some(value) = accum
            && let Ok(dst_offset) =
                tensor_linear_index(out_index, &dest_layout.shape, &dest_layout.strides)
            && let Some(slot) = output.get_mut(dst_offset)
        {
            *slot = value;
        }
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.convolution.
pub(crate) fn handle_tensor_convolution(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorConvolution {
        dest,
        input,
        kernel,
        dimensions,
        window,
        feature_group_count,
        batch_group_count,
        input_type,
        kernel_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let input_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *input_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let kernel_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *kernel_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve source slots
    let input_value = state.get(*input);
    let kernel_value = state.get(*kernel);
    let input_slots = match aggregate_slots(state, input_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let kernel_slots = match aggregate_slots(state, kernel_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // derive dimension mappings
    let spatial_rank = dimensions.input_spatial.len();
    let output_spatial = &dimensions.output_spatial;
    let kernel_spatial = &dimensions.kernel_spatial;
    if output_spatial.len() != spatial_rank || kernel_spatial.len() != spatial_rank {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    // validate window shapes
    if window.strides.len() != spatial_rank
        || window.padding_low.len() != spatial_rank
        || window.padding_high.len() != spatial_rank
        || window.lhs_dilation.len() != spatial_rank
        || window.rhs_dilation.len() != spatial_rank
        || window.window_reversal.len() != spatial_rank
    {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let output_batch_dim = dimensions.output_batch as usize;
    let output_feature_dim = dimensions.output_feature as usize;
    let input_batch_dim = dimensions.input_batch as usize;
    let input_feature_dim = dimensions.input_feature as usize;
    let kernel_input_feature_dim = dimensions.kernel_input_feature as usize;
    let kernel_output_feature_dim = dimensions.kernel_output_feature as usize;

    // validate dimension indices
    if input_batch_dim >= input_layout.shape.len()
        || input_feature_dim >= input_layout.shape.len()
        || output_batch_dim >= dest_layout.shape.len()
        || output_feature_dim >= dest_layout.shape.len()
        || kernel_input_feature_dim >= kernel_layout.shape.len()
        || kernel_output_feature_dim >= kernel_layout.shape.len()
    {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    for dim in dimensions.input_spatial.iter() {
        if *dim as usize >= input_layout.shape.len() {
            return ControlFlow::Error(Error::InvalidInstruction);
        }
    }
    for dim in output_spatial.iter() {
        if *dim as usize >= dest_layout.shape.len() {
            return ControlFlow::Error(Error::InvalidInstruction);
        }
    }

    let input_batch_size = input_layout.shape[input_batch_dim];
    let input_feature_size = input_layout.shape[input_feature_dim];
    let output_batch_size = dest_layout.shape[output_batch_dim];
    let output_feature_size = dest_layout.shape[output_feature_dim];

    let mut kernel_spatial_shape = Vec::with_capacity(kernel_spatial.len());
    for dim in kernel_spatial {
        let Some(size) = kernel_layout.shape.get(*dim as usize) else {
            return ControlFlow::Error(Error::InvalidInstruction);
        };
        kernel_spatial_shape.push(*size);
    }

    // validate group counts
    if *feature_group_count == 0 || *batch_group_count == 0 {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let out_features_per_group = output_feature_size / (*feature_group_count as u64);
    let in_features_per_group = input_feature_size / (*feature_group_count as u64);
    let out_batches_per_group = output_batch_size / (*batch_group_count as u64);
    let in_batches_per_group = input_batch_size / (*batch_group_count as u64);

    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let mut input_index = vec![0u64; input_layout.shape.len()];
    let mut kernel_index = vec![0u64; kernel_layout.shape.len()];

    // compute convolution
    for_each_index(&dest_layout.shape, |out_index| {
        let out_batch = out_index[output_batch_dim];
        let out_feature = out_index[output_feature_dim];

        let batch_group = out_batch / out_batches_per_group;
        let feature_group = out_feature / out_features_per_group;

        let input_batch = batch_group * in_batches_per_group + (out_batch % out_batches_per_group);
        let input_feature_base = feature_group * in_features_per_group;
        let kernel_out_feature = out_feature % out_features_per_group;

        input_index[input_batch_dim] = input_batch;
        kernel_index[kernel_output_feature_dim] = kernel_out_feature;

        let mut accum = None;
        for in_feature in 0..in_features_per_group {
            input_index[input_feature_dim] = input_feature_base + in_feature;
            kernel_index[kernel_input_feature_dim] = in_feature;

            for_each_index(&kernel_spatial_shape, |kernel_spatial_index| {
                let mut valid = true;
                for (i, &dim) in dimensions.input_spatial.iter().enumerate() {
                    let out_spatial_dim = output_spatial[i] as usize;
                    let kernel_dim = kernel_spatial[i] as usize;
                    let stride = window.strides[i];
                    let padding_low = window.padding_low[i];
                    let lhs_dilation = window.lhs_dilation[i];
                    let rhs_dilation = window.rhs_dilation[i];
                    let kernel_size = kernel_layout.shape[kernel_dim];
                    let mut kernel_pos = kernel_spatial_index[i];
                    if window.window_reversal[i] {
                        kernel_pos = kernel_size.saturating_sub(1).saturating_sub(kernel_pos);
                    }

                    let out_pos = out_index[out_spatial_dim];
                    let mut input_pos = out_pos
                        .saturating_mul(stride)
                        .saturating_add(kernel_pos * rhs_dilation);
                    if input_pos < padding_low {
                        valid = false;
                        break;
                    }
                    input_pos -= padding_low;
                    if lhs_dilation > 1 {
                        if input_pos % lhs_dilation != 0 {
                            valid = false;
                            break;
                        }
                        input_pos /= lhs_dilation;
                    }
                    if input_pos >= input_layout.shape[dim as usize] {
                        valid = false;
                        break;
                    }

                    input_index[dim as usize] = input_pos;
                    kernel_index[kernel_dim] = kernel_pos;
                }

                if !valid {
                    return;
                }

                let input_offset =
                    tensor_linear_index(&input_index, &input_layout.shape, &input_layout.strides);
                let kernel_offset = tensor_linear_index(
                    &kernel_index,
                    &kernel_layout.shape,
                    &kernel_layout.strides,
                );
                let (Ok(input_offset), Ok(kernel_offset)) = (input_offset, kernel_offset) else {
                    return;
                };
                let Some(input_val) = input_slots.get(input_offset) else {
                    return;
                };
                let Some(kernel_val) = kernel_slots.get(kernel_offset) else {
                    return;
                };

                let product =
                    apply_reduce_operator(ReduceOperator::Multiply, *input_val, *kernel_val).ok();
                let Some(product) = product else {
                    return;
                };
                accum = match accum {
                    None => Some(product),
                    Some(current) => {
                        apply_reduce_operator(ReduceOperator::Add, current, product).ok()
                    }
                };
            });
        }

        if let Some(value) = accum
            && let Ok(dst_offset) =
                tensor_linear_index(out_index, &dest_layout.shape, &dest_layout.strides)
            && let Some(slot) = output.get_mut(dst_offset)
        {
            *slot = value;
        }
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.gather.
pub(crate) fn handle_tensor_gather(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorGather {
        dest,
        operand,
        indices,
        dimensions,
        slice_sizes,
        operand_type,
        indices_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let operand_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *operand_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let indices_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *indices_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve slots
    let operand_value = state.get(*operand);
    let indices_value = state.get(*indices);
    let operand_slots = match aggregate_slots(state, operand_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let indices_slots = match aggregate_slots(state, indices_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    let mut output = vec![Value::VOID; dest_layout.storage_len];
    let offset_dims: std::collections::HashSet<u32> =
        dimensions.offset_dims.iter().copied().collect();
    let collapsed_dims: std::collections::HashSet<u32> =
        dimensions.collapsed_slice_dims.iter().copied().collect();

    // gather slices
    for_each_index(&dest_layout.shape, |out_index| {
        let mut index_coords = Vec::new();
        for (dim, value) in out_index.iter().enumerate() {
            if !offset_dims.contains(&(dim as u32)) {
                index_coords.push(*value);
            }
        }

        let mut index_vec = vec![0u64; dimensions.start_index_map.len()];
        let mut indices_index = vec![0u64; indices_layout.shape.len()];
        let mut coord_iter = index_coords.iter();
        for (dim, slot) in indices_index.iter_mut().enumerate() {
            if dim as u32 == dimensions.index_vector_dim {
                continue;
            }
            *slot = *coord_iter.next().unwrap_or(&0);
        }

        let index_offset = tensor_linear_index(
            &indices_index,
            &indices_layout.shape,
            &indices_layout.strides,
        );
        let Ok(index_offset) = index_offset else {
            return;
        };
        let index_base = index_offset;
        for (i, &_map_dim) in dimensions.start_index_map.iter().enumerate() {
            let slot_index = index_base + i;
            let Some(value) = indices_slots.get(slot_index) else {
                return;
            };
            if let Ok(coord) = value_to_u64(*value) {
                index_vec[i] = coord;
            } else {
                return;
            }
            indices_index[dimensions.index_vector_dim as usize] = index_vec[i];
        }

        let mut operand_index = vec![0u64; operand_layout.shape.len()];
        for (i, &map_dim) in dimensions.start_index_map.iter().enumerate() {
            operand_index[map_dim as usize] = index_vec[i];
        }

        let mut offset_iter = out_index.iter();
        for (dim, slot) in operand_index.iter_mut().enumerate() {
            if collapsed_dims.contains(&(dim as u32)) {
                continue;
            }
            let offset = if offset_dims.contains(&(dim as u32)) {
                *offset_iter.next().unwrap_or(&0)
            } else {
                0
            };
            let size = slice_sizes.get(dim).copied().unwrap_or(1) as u64;
            let start = *slot;
            *slot = start + offset.min(size.saturating_sub(1));
        }

        let Ok(src_offset) = tensor_linear_index(
            &operand_index,
            &operand_layout.shape,
            &operand_layout.strides,
        ) else {
            return;
        };
        let Ok(dst_offset) =
            tensor_linear_index(out_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(value) = operand_slots.get(src_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dst_offset) else {
            return;
        };
        *slot = *value;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.scatter.
pub(crate) fn handle_tensor_scatter(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorScatter {
        dest,
        operand,
        indices,
        updates,
        dimensions,
        mode,
        operand_type,
        indices_type,
        updates_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let operand_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *operand_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let indices_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *indices_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let updates_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *updates_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve slots
    let operand_value = state.get(*operand);
    let indices_value = state.get(*indices);
    let updates_value = state.get(*updates);
    let operand_slots = match aggregate_slots(state, operand_value) {
        Ok(slots) => slots.to_vec(),
        Err(error) => return ControlFlow::Error(error),
    };
    let indices_slots = match aggregate_slots(state, indices_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let updates_slots = match aggregate_slots(state, updates_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    let mut output = operand_slots;
    let update_window_dims: std::collections::HashSet<u32> =
        dimensions.update_window_dims.iter().copied().collect();
    let inserted_window_dims: std::collections::HashSet<u32> =
        dimensions.inserted_window_dims.iter().copied().collect();

    // scatter updates
    for_each_index(&updates_layout.shape, |update_index| {
        let mut index_coords = Vec::new();
        for (dim, value) in update_index.iter().enumerate() {
            if !update_window_dims.contains(&(dim as u32)) {
                index_coords.push(*value);
            }
        }

        let mut indices_index = vec![0u64; indices_layout.shape.len()];
        let mut coord_iter = index_coords.iter();
        for (dim, slot) in indices_index.iter_mut().enumerate() {
            if dim as u32 == dimensions.index_vector_dim {
                continue;
            }
            *slot = *coord_iter.next().unwrap_or(&0);
        }

        let index_offset = tensor_linear_index(
            &indices_index,
            &indices_layout.shape,
            &indices_layout.strides,
        );
        let Ok(index_offset) = index_offset else {
            return;
        };

        let mut scatter_indices = Vec::with_capacity(dimensions.scatter_dims_to_operand_dims.len());
        for i in 0..dimensions.scatter_dims_to_operand_dims.len() {
            let slot_index = index_offset + i;
            let Some(value) = indices_slots.get(slot_index) else {
                return;
            };
            if let Ok(coord) = value_to_u64(*value) {
                scatter_indices.push(coord);
            } else {
                return;
            }
        }

        let mut operand_index = vec![0u64; operand_layout.shape.len()];
        for (i, &dim) in dimensions.scatter_dims_to_operand_dims.iter().enumerate() {
            operand_index[dim as usize] = scatter_indices[i];
        }

        let mut update_iter = update_index.iter();
        for (dim, slot) in operand_index.iter_mut().enumerate() {
            if inserted_window_dims.contains(&(dim as u32)) {
                continue;
            }
            if update_window_dims.contains(&(dim as u32)) {
                *slot = *update_iter.next().unwrap_or(&0);
            }
        }

        let dst_offset =
            tensor_linear_index(&operand_index, &dest_layout.shape, &dest_layout.strides);
        let Ok(dst_offset) = dst_offset else {
            return;
        };
        let update_offset =
            tensor_linear_index(update_index, &updates_layout.shape, &updates_layout.strides);
        let Ok(update_offset) = update_offset else {
            return;
        };
        let Some(update_value) = updates_slots.get(update_offset) else {
            return;
        };
        if let Some(slot) = output.get_mut(dst_offset) {
            let op = match mode {
                mir::TensorScatterMode::Replace => {
                    *slot = *update_value;
                    return;
                }
                mir::TensorScatterMode::Add => ReduceOperator::Add,
                mir::TensorScatterMode::Multiply => ReduceOperator::Multiply,
                mir::TensorScatterMode::Min => ReduceOperator::Min,
                mir::TensorScatterMode::Max => ReduceOperator::Max,
                mir::TensorScatterMode::And => ReduceOperator::And,
                mir::TensorScatterMode::Or => ReduceOperator::Or,
                mir::TensorScatterMode::Xor => ReduceOperator::Xor,
            };
            let new_value = match apply_reduce_operator(op, *slot, *update_value) {
                Ok(value) => value,
                Err(_) => return,
            };
            *slot = new_value;
        }
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.convert.
pub(crate) fn handle_tensor_convert(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorConvert {
        dest,
        mode,
        tensor,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve tensor types
    let source_type = match state.interpreter.isolate.tree.get(*source_type) {
        mir::Type::Tensor { element, .. } => *element,
        _ => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{source_type:?}"),
            });
        }
    };
    let dest_type_info = state.interpreter.isolate.tree.get(*dest_type);
    let (dest_element, dest_layout) = match dest_type_info {
        mir::Type::Tensor { element, .. } => {
            let layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
                Ok(layout) => layout,
                Err(error) => return ControlFlow::Error(error),
            };
            (*element, layout)
        }
        _ => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{dest_type:?}"),
            });
        }
    };

    // resolve source slots
    let tensor_value = state.get(*tensor);
    let source_slots = match aggregate_slots(state, tensor_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // validate storage length
    if source_slots.len() != dest_layout.storage_len {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "matching tensor storage".to_string(),
            actual: format!("{} vs {}", source_slots.len(), dest_layout.storage_len),
        });
    }

    // resolve conversion types
    let source_info = match scalar_type_info(&state.interpreter.isolate.tree, source_type) {
        Ok(info) => info,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_info = match scalar_type_info(&state.interpreter.isolate.tree, dest_element) {
        Ok(info) => info,
        Err(error) => return ControlFlow::Error(error),
    };
    let convert_mode = ScalarConvertMode::from(*mode);

    // allocate output storage
    let mut output = Vec::with_capacity(source_slots.len());
    for src in source_slots {
        let converted = match convert_scalar_value(*src, source_info, dest_info, convert_mode) {
            Ok(value) => value,
            Err(error) => return ControlFlow::Error(error),
        };
        output.push(converted);
    }

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.compare.
pub(crate) fn handle_tensor_compare(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorCompare {
        dest,
        operator,
        left,
        right,
        left_type,
        right_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let left_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *left_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let right_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *right_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve operand slots
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_slots = match aggregate_slots(state, left_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let right_slots = match aggregate_slots(state, right_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // allocate destination storage
    let mut output = vec![Value::VOID; dest_layout.storage_len];

    // compare elements
    for_each_index(&dest_layout.shape, |output_index| {
        let Ok(left_offset) =
            tensor_linear_index(output_index, &left_layout.shape, &left_layout.strides)
        else {
            return;
        };
        let Ok(right_offset) =
            tensor_linear_index(output_index, &right_layout.shape, &right_layout.strides)
        else {
            return;
        };
        let Ok(dest_offset) =
            tensor_linear_index(output_index, &dest_layout.shape, &dest_layout.strides)
        else {
            return;
        };
        let Some(left_value) = left_slots.get(left_offset) else {
            return;
        };
        let Some(right_value) = right_slots.get(right_offset) else {
            return;
        };
        let Some(slot) = output.get_mut(dest_offset) else {
            return;
        };
        let Ok(result) = operator::execute_binary(*operator, *left_value, *right_value) else {
            return;
        };
        *slot = result;
    });

    // allocate result
    let result = state.interpreter.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.cast.
pub(crate) fn handle_tensor_cast(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorCast { dest, tensor } = &block[pc].data else {
        unreachable!()
    };

    // forward the tensor value
    let value = state.get(*tensor);
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle tensor.view.
pub(crate) fn handle_tensor_view(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::TensorView {
        dest,
        view,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *source_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };
    let dest_layout = match tensor_layout_info(&state.interpreter.isolate.tree, *dest_type) {
        Ok(layout) => layout,
        Err(error) => return ControlFlow::Error(error),
    };

    // resolve view arguments
    let args = state.argument_slice(*arguments);
    let (offset_values, rest) = args.split_at(*offsets_count as usize);
    let (size_values, stride_values) = rest.split_at(*sizes_count as usize);
    if stride_values.len() != *strides_count as usize {
        return ControlFlow::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for value_id in offset_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in size_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }
    for value_id in stride_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return ControlFlow::Error(error),
        }
    }

    // validate sizes and strides against the destination layout
    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if *expected != *actual {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "tensor.view size".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }
    for (expected, actual) in dest_layout.strides.iter().zip(strides.iter()) {
        if *expected != *actual {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "tensor.view stride".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }

    // compute offset into the source view
    let offset = match tensor_linear_index(&offsets, &source_layout.shape, &source_layout.strides) {
        Ok(offset) => offset,
        Err(error) => return ControlFlow::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let pointer = match offset_pointer(view_value, offset, source_layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return ControlFlow::Error(error),
    };

    // set the view result
    state.set(*dest, pointer);

    // continue to next instruction
    next!(state, block, pc)
}
