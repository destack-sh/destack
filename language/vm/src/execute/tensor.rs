use super::prelude::*;

/// The flattened layout for one tensor.
#[derive(Debug, Clone)]
pub(crate) struct TensorLayout {
    /// The static tensor shape.
    pub(crate) shape: Vec<u64>,
    /// The per-dimension strides in element units.
    pub(crate) strides: Vec<u64>,
    /// The total storage length in elements.
    pub(crate) storage_len: usize,
}

/// Convert tensor dimensions to a static shape.
pub(crate) fn static_shape(shape: &[mir::TensorDimension]) -> Result<Vec<u64>, Error> {
    // reject dynamic shapes for the interpreter
    let mut dims = Vec::with_capacity(shape.len());
    for dim in shape {
        match dim {
            mir::TensorDimension::Static(value) => dims.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic shape".to_string(),
                });
            }
        }
    }

    Ok(dims)
}

/// Convert tensor strides to a static list.
pub(crate) fn static_strides(strides: &[mir::TensorDimension]) -> Result<Vec<u64>, Error> {
    // reject dynamic strides for the interpreter
    let mut values = Vec::with_capacity(strides.len());
    for dim in strides {
        match dim {
            mir::TensorDimension::Static(value) => values.push(*value),
            mir::TensorDimension::Dynamic => {
                return Err(Error::UnsupportedInstruction {
                    name: "tensor dynamic stride".to_string(),
                });
            }
        }
    }

    Ok(values)
}

/// Compute row-major strides for a shape.
pub(crate) fn row_major_strides(shape: &[u64]) -> Vec<u64> {
    // compute row-major strides
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate().rev() {
        strides[index] = stride;
        stride = stride.saturating_mul(*dim);
    }

    strides
}

/// Compute column-major strides for a shape.
pub(crate) fn column_major_strides(shape: &[u64]) -> Vec<u64> {
    // compute column-major strides
    let mut strides = vec![1; shape.len()];
    let mut stride = 1u64;
    for (index, dim) in shape.iter().enumerate() {
        strides[index] = stride;
        stride = stride.saturating_mul(*dim);
    }

    strides
}

/// Compute the storage length for a shape and stride list.
pub(crate) fn tensor_storage_len(shape: &[u64], strides: &[u64]) -> Result<usize, Error> {
    // empty shape stores a single scalar
    if shape.is_empty() {
        return Ok(1);
    }

    // zero-sized shapes have zero elements
    if shape.contains(&0) {
        return Ok(0);
    }

    // compute max linear index
    let mut max_index = 0u64;
    for (dim, stride) in shape.iter().zip(strides.iter()) {
        let count = dim.saturating_sub(1);
        max_index = max_index.saturating_add(count.saturating_mul(*stride));
    }

    let len = max_index.saturating_add(1);
    usize::try_from(len).map_err(|_| Error::TypeMismatch {
        expected: "tensor storage length".to_string(),
        actual: len.to_string(),
    })
}

/// Resolve tensor layout information from a tensor type.
pub(crate) fn tensor_layout_info(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<TensorLayout, Error> {
    // resolve tensor shape and layout
    let (shape, layout) = match tree.get(ty) {
        mir::Type::Tensor { shape, layout, .. } => (shape, layout),
        mir::Type::TensorView { shape, layout, .. } => (shape, layout),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    // compute static shape and strides
    let shape = static_shape(shape)?;
    let strides = match layout {
        mir::TensorLayout::RowMajor => row_major_strides(&shape),
        mir::TensorLayout::ColumnMajor => column_major_strides(&shape),
        mir::TensorLayout::Strided { strides } => static_strides(strides)?,
    };

    // compute storage length
    let storage_len = tensor_storage_len(&shape, &strides)?;

    Ok(TensorLayout {
        shape,
        strides,
        storage_len,
    })
}

/// Return the scalar element type for one tensor or tensor view.
fn tensor_element_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout, Error> {
    let element = match tree.get(ty) {
        mir::Type::Tensor { element, .. } | mir::Type::TensorView { element, .. } => {
            element.ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "tensor element type".to_string(),
            })?
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    scalar_layout(tree, element)
}

/// Write one tensor result into frame bytes.
fn write_tensor_by_element<F>(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    storage_len: usize,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut DispatchState<'_, '_>, usize) -> Result<Word, Error>,
{
    super::bytes::write_frame_elements(state, dest, |state, element_index, _value_type| {
        if element_index >= storage_len {
            return Err(Error::IndexOutOfBounds {
                index: element_index as u64,
                length: storage_len as u64,
            });
        }

        element_value(state, element_index)
    })
}

/// Load one tensor element through one concrete pointer class.
#[inline(always)]
fn load_tensor_element(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    element: ElementAccess,
) -> Result<Word, Error> {
    let access = PointeeAccess::from(element);

    match element.pointer_class {
        PointerClass::Heap => access::load_heap_reference(state, pointer, access),
        PointerClass::SharedHeap => access::load_shared_heap_reference(state, pointer, access),
        PointerClass::Raw => access::load_raw_pointer(state, pointer, access),
        PointerClass::SharedRaw => access::load_shared_raw_pointer(state, pointer, access),
        PointerClass::Stack => {
            access::load_stack_pointer(state, pointer.as_stack_pointer(), access)
        }
        PointerClass::Frame => {
            access::load_frame_pointer(state, pointer.as_frame_pointer(), access)
        }
        PointerClass::Static => {
            access::load_static_pointer(state, pointer.as_static_pointer(), access)
        }
        PointerClass::Unknown => Err(access::invalid_pointer_type(pointer)),
    }
}

/// Store one tensor element through one concrete pointer class.
#[inline(always)]
fn store_tensor_element(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    element: ElementAccess,
    value: Word,
) -> Result<(), Error> {
    let access = PointeeAccess::from(element);

    match element.pointer_class {
        PointerClass::Heap => access::store_heap_reference(state, pointer, access, value),
        PointerClass::SharedHeap => {
            access::store_shared_heap_reference(state, pointer, access, value)
        }
        PointerClass::Raw => access::store_raw_pointer(state, pointer, access, value),
        PointerClass::SharedRaw => access::store_shared_raw_pointer(state, pointer, access, value),
        PointerClass::Stack => {
            access::store_stack_pointer(state, pointer.as_stack_pointer(), access, value)
        }
        PointerClass::Frame => {
            access::store_frame_pointer(state, pointer.as_frame_pointer(), access, value)
        }
        PointerClass::Static => {
            access::store_static_pointer(state, pointer.as_static_pointer(), access, value)
        }
        PointerClass::Unknown => Err(access::invalid_pointer_type(pointer)),
    }
}

/// Store one tensor element into one tensor value.
fn store_tensor_element_at(
    state: &mut DispatchState<'_, '_>,
    tensor: Word,
    tensor_type: mir::LocalNodeId<mir::Type>,
    element_index: usize,
    value: Word,
) -> Result<(), Error> {
    let element_index = u32::try_from(element_index).map_err(|_| Error::TypeMismatch {
        expected: "tensor element index".to_string(),
        actual: element_index.to_string(),
    })?;
    let (element, element_count) =
        access::element_access_for_type(state, tensor_type, element_index.into())?;
    let element_count = usize::try_from(element_count).map_err(|_| Error::TypeMismatch {
        expected: "tensor storage length".to_string(),
        actual: element_count.to_string(),
    })?;
    let pointer = offset_pointer(tensor, element, element_index as usize, element_count)?;

    store_tensor_element(state, pointer, element, value)
}

/// Allocate one tensor result by destination index.
fn allocate_tensor_by_index<F>(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    layout: &TensorLayout,
    mut index_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut DispatchState<'_, '_>, &[u64]) -> Result<Word, Error>,
{
    let dest_type = state.value_type(dest)?;
    let result = Word::frame_pointer(state.value_address(dest)?);
    state.value_bytes_mut(dest)?.fill(0);
    let mut error = None;

    // fill active tensor indices directly into the destination place
    for_each_index(&layout.shape, |output_index| {
        if error.is_some() {
            return;
        }

        let dst_offset = match tensor_linear_index(output_index, &layout.shape, &layout.strides) {
            Ok(offset) => offset,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };
        let value = match index_value(state, output_index) {
            Ok(value) => value,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };

        if let Err(current_error) =
            store_tensor_element_at(state, result, dest_type, dst_offset, value)
        {
            error = Some(current_error);
        }
    });

    if let Some(error) = error {
        return Err(error);
    }

    Ok(())
}

/// Compute the linear index for a multi-dimensional index.
pub(crate) fn tensor_linear_index(
    indices: &[u64],
    shape: &[u64],
    strides: &[u64],
) -> Result<usize, Error> {
    // validate index length
    if indices.len() != shape.len() || shape.len() != strides.len() {
        return Err(Error::TypeMismatch {
            expected: "tensor index rank".to_string(),
            actual: format!(
                "indices={}, shape={}, strides={}",
                indices.len(),
                shape.len(),
                strides.len()
            ),
        });
    }

    // compute linear index
    let mut offset = 0u64;
    for ((index, dim), stride) in indices.iter().zip(shape.iter()).zip(strides.iter()) {
        // reject out of bounds indices before accumulating the stride
        if *index >= *dim {
            return Err(Error::IndexOutOfBounds {
                index: *index,
                length: *dim,
            });
        }

        // accumulate the linear offset in element units
        offset = offset.saturating_add(index.saturating_mul(*stride));
    }

    // convert the final offset into host indexing
    usize::try_from(offset).map_err(|_| Error::TypeMismatch {
        expected: "tensor index".to_string(),
        actual: offset.to_string(),
    })
}

/// Load one tensor element through indexed access.
pub(crate) fn load_tensor_element_at(
    state: &mut DispatchState<'_, '_>,
    tensor: Word,
    tensor_type: mir::LocalNodeId<mir::Type>,
    element_index: usize,
) -> Result<Word, Error> {
    let index = u32::try_from(element_index).map_err(|_| Error::TypeMismatch {
        expected: "tensor element index".to_string(),
        actual: element_index.to_string(),
    })?;
    let (element, element_count) =
        access::element_access_for_type(state, tensor_type, index.into())?;
    let element_count = usize::try_from(element_count).map_err(|_| Error::TypeMismatch {
        expected: "tensor storage length".to_string(),
        actual: element_count.to_string(),
    })?;
    let pointer = offset_pointer(tensor, element, element_index, element_count)?;

    load_tensor_element(state, pointer, element)
}

/// Execute tensor.splat.
pub(crate) fn execute_tensor_splat(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorSplat {
        dest,
        value,
        tensor_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve result layout
    let layout = match tensor_layout_info(state.tree(), *tensor_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let value = state.get(*value);

    // allocate the result one active index at a time
    if let Err(error) = allocate_tensor_by_index(state, *dest, &layout, |_state, _index| Ok(value))
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.extract.
pub(crate) fn execute_tensor_extract(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorExtract {
        dest,
        tensor,
        indices,
        tensor_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(state.tree(), *tensor_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // resolve indices
    let index_values = state.argument_slice(*indices);
    let mut index = Vec::with_capacity(index_values.len());
    for value_id in index_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => index.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    let element_index = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(element_index) => element_index,
        Err(error) => return Transfer::Error(error),
    };

    // load the tensor value
    let tensor_value = state.get(*tensor);
    let value = match load_tensor_element_at(state, tensor_value, *tensor_type, element_index) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Iterate over all indices in a tensor shape.
pub(crate) fn for_each_index<F: FnMut(&[u64])>(shape: &[u64], mut f: F) {
    // handle scalar or empty shapes
    if shape.is_empty() {
        f(&[]);
        return;
    }
    if shape.contains(&0) {
        return;
    }

    // initialize index vector
    let mut index = vec![0u64; shape.len()];
    loop {
        // visit the current tensor index
        f(&index);

        // increment the odometer
        let mut dim = shape.len();
        while dim > 0 {
            dim -= 1;
            index[dim] += 1;
            if index[dim] < shape[dim] {
                break;
            }
            index[dim] = 0;
            if dim == 0 {
                return;
            }
        }
    }
}

/// Offset a pointer by one tensor element index.
pub(crate) fn offset_pointer(
    value: Word,
    element: ElementAccess,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    // validate bounds
    if offset >= length {
        return Err(Error::IndexOutOfBounds {
            index: offset as u64,
            length: length as u64,
        });
    }

    let byte_offset = offset
        .checked_mul(element.byte_stride)
        .ok_or(Error::InvalidPointerType {
            actual: format!("{value:?}"),
        })?;

    // offset the pointer according to its pointer kind
    match element.pointer_class {
        PointerClass::Heap => {
            let reference = value.as_heap_reference();
            let reference = reference
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::heap_reference(reference))
        }
        PointerClass::SharedHeap => {
            let reference = value.as_shared_heap_reference();
            let reference = reference
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::shared_heap_reference(reference))
        }
        PointerClass::Raw => {
            let pointer = value.as_raw_pointer();
            let pointer = pointer
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::raw_pointer(pointer))
        }
        PointerClass::SharedRaw => {
            let pointer = value.as_shared_raw_pointer();
            let pointer = pointer
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::shared_raw_pointer(pointer))
        }
        PointerClass::Stack => {
            let pointer = value.as_stack_pointer();
            let pointer = pointer
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::stack_pointer(pointer))
        }
        PointerClass::Frame => {
            let pointer = value.as_frame_pointer();
            let pointer = pointer
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::frame_pointer(pointer))
        }
        PointerClass::Static => {
            let pointer = value.as_static_pointer();
            let pointer = pointer
                .add_bytes(byte_offset)
                .ok_or(Error::InvalidPointerType {
                    actual: format!("{value:?}"),
                })?;

            Ok(Word::static_pointer(pointer))
        }
        PointerClass::Unknown => Err(Error::InvalidPointerType {
            actual: format!("{value:?}"),
        }),
    }
}

/// Execute tensor.load.
pub(crate) fn execute_tensor_load(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorLoad {
        dest,
        view,
        indices,
        view_type,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(state.tree(), *view_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    // resolve indices
    let index_values = state.argument_slice(*indices);
    let mut index = Vec::with_capacity(index_values.len());
    for value_id in index_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => index.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let Some(element) = *element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{view_type:?}"),
        });
    };
    let pointer = match offset_pointer(view_value, element, offset, layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // load element
    let value = match load_tensor_element(state, pointer, element) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.store.
pub(crate) fn execute_tensor_store(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorStore {
        view,
        indices,
        value,
        view_type,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(state.tree(), *view_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    // resolve indices
    let index_values = state.argument_slice(*indices);
    let mut index = Vec::with_capacity(index_values.len());
    for value_id in index_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => index.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let Some(element) = *element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{view_type:?}"),
        });
    };
    let pointer = match offset_pointer(view_value, element, offset, layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // store element
    let value = state.get(*value);
    if let Err(error) = store_tensor_element(state, pointer, element, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.fill.
pub(crate) fn execute_tensor_fill(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorFill {
        view,
        value,
        view_type,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layout info
    let layout = match tensor_layout_info(state.tree(), *view_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let fill_value = state.get(*value);
    let base_pointer = state.get(*view);

    // fill each element
    let Some(element) = *element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{view_type:?}"),
        });
    };
    for offset in 0..layout.storage_len {
        let pointer = match offset_pointer(base_pointer, element, offset, layout.storage_len) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };
        if let Err(error) = store_tensor_element(state, pointer, element, fill_value) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.move.
pub(crate) fn execute_tensor_copy(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorCopy {
        target,
        source,
        target_type,
        source_type,
        target_element,
        source_element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let target_layout = match tensor_layout_info(state.tree(), *target_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    // validate element counts
    if target_layout.storage_len != source_layout.storage_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor sizes".to_string(),
            actual: format!(
                "{} vs {}",
                target_layout.storage_len, source_layout.storage_len
            ),
        });
    }

    // move elements
    let target_ptr = state.get(*target);
    let source_ptr = state.get(*source);
    let Some(source_element) = *source_element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{source_type:?}"),
        });
    };
    let Some(target_element) = *target_element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{target_type:?}"),
        });
    };
    for offset in 0..target_layout.storage_len {
        let src = match offset_pointer(
            source_ptr,
            source_element,
            offset,
            source_layout.storage_len,
        ) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };
        let dst = match offset_pointer(
            target_ptr,
            target_element,
            offset,
            target_layout.storage_len,
        ) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };
        let value = match load_tensor_element(state, src, source_element) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        if let Err(error) = store_tensor_element(state, dst, target_element, value) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.reshape.
pub(crate) fn execute_tensor_reshape(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorReshape {
        dest,
        tensor,
        shape,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve source tensor
    let tensor_value = state.get(*tensor);
    let tensor_type = match state.value_type(*tensor) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve output layout
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // compute expected element count from shape values when provided
    let shape_values = state.argument_slice(*shape);
    let mut shape_len = 1u64;
    for value_id in shape_values {
        let value = state.get(*value_id);
        let size = match value_to_u64(value) {
            Ok(size) => size,
            Err(error) => return Transfer::Error(error),
        };
        shape_len = shape_len.saturating_mul(size);
    }
    if shape_values.is_empty() {
        shape_len = dest_layout.storage_len as u64;
    }

    // validate element counts
    if dest_layout.storage_len as u64 != shape_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "reshape element count".to_string(),
            actual: format!("{} vs {}", dest_layout.storage_len, shape_len),
        });
    }

    // move the source storage into the reshaped result
    if let Err(error) = write_tensor_by_element(
        state,
        *dest,
        dest_layout.storage_len,
        |state, element_index| {
            load_tensor_element_at(state, tensor_value, tensor_type, element_index)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.broadcast.
pub(crate) fn execute_tensor_broadcast(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorBroadcast {
        dest,
        tensor,
        dimensions,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // validate dimension mapping
    if dimensions.len() != source_layout.shape.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "broadcast dimension mapping".to_string(),
            actual: format!("{} vs {}", dimensions.len(), source_layout.shape.len()),
        });
    }

    // resolve source elements
    let tensor_value = state.get(*tensor);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            for (i, dim) in dimensions.iter().enumerate() {
                let output_value = output_index[*dim as usize];
                let source_dim = source_layout.shape[i];
                input_index[i] = if source_dim == 1 { 0 } else { output_value };
            }

            let src_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_tensor_element_at(state, tensor_value, *source_type, src_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.transpose.
pub(crate) fn execute_tensor_transpose(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorTranspose {
        dest,
        tensor,
        permutation,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // validate permutation
    if permutation.len() != source_layout.shape.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "transpose permutation".to_string(),
            actual: format!("{} vs {}", permutation.len(), source_layout.shape.len()),
        });
    }

    // resolve source tensor
    let tensor_value = state.get(*tensor);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            for (out_dim, in_dim) in permutation.iter().enumerate() {
                input_index[*in_dim as usize] = output_index[out_dim];
            }

            let src_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_tensor_element_at(state, tensor_value, *source_type, src_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.slice.
pub(crate) fn execute_tensor_slice(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorSlice {
        dest,
        tensor,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // resolve arguments
    let args = state.argument_slice(*arguments);
    let (offset_values, rest) = args.split_at(*offsets_count as usize);
    let (size_values, stride_values) = rest.split_at(*sizes_count as usize);
    if stride_values.len() != *strides_count as usize {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for value_id in offset_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in size_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in stride_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // resolve source tensor
    let tensor_value = state.get(*tensor);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            for i in 0..input_index.len() {
                let offset = offsets.get(i).copied().unwrap_or(0);
                let stride = strides.get(i).copied().unwrap_or(1);
                input_index[i] = offset + output_index[i] * stride;
            }

            let src_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_tensor_element_at(state, tensor_value, *source_type, src_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.pad.
pub(crate) fn execute_tensor_pad(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorPad {
        dest,
        tensor,
        arguments,
        low_count,
        high_count,
        interior_count,
        value,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // resolve arguments
    let args = state.argument_slice(*arguments);
    let (low_values, rest) = args.split_at(*low_count as usize);
    let (high_values, interior_values) = rest.split_at(*high_count as usize);
    if interior_values.len() != *interior_count as usize {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut low = Vec::with_capacity(low_values.len());
    let mut high = Vec::with_capacity(high_values.len());
    let mut interior = Vec::with_capacity(interior_values.len());
    for value_id in low_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => low.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in high_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => high.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in interior_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => interior.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // resolve source tensor
    let tensor_value = state.get(*tensor);
    let pad_value = state.get(*value);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            for i in 0..input_index.len() {
                let low_pad = low.get(i).copied().unwrap_or(0);
                let interior_pad = interior.get(i).copied().unwrap_or(0);
                let mut idx = output_index[i] as i64 - low_pad as i64;
                if idx < 0 {
                    return Ok(pad_value);
                }

                let stride = interior_pad + 1;
                if !(idx as u64).is_multiple_of(stride) {
                    return Ok(pad_value);
                }

                idx /= stride as i64;
                if idx < 0 || idx as u64 >= source_layout.shape[i] {
                    return Ok(pad_value);
                }

                input_index[i] = idx as u64;
            }

            let src_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_tensor_element_at(state, tensor_value, *source_type, src_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.concat.
pub(crate) fn execute_tensor_concat(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorConcat {
        dest,
        tensors,
        tensor_types,
        axis,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve destination layout
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // resolve input tensors
    let tensor_ids = state.argument_slice(*tensors).to_vec();
    // validate input metadata
    if tensor_ids.len() != tensor_types.len() {
        return Transfer::Error(Error::InvalidInstruction);
    }
    let mut inputs = Vec::with_capacity(tensor_ids.len());
    let mut axis_sizes = Vec::with_capacity(tensor_ids.len());
    for (value_id, type_id) in tensor_ids.iter().zip(tensor_types.iter()) {
        let value = state.get(*value_id);
        let layout = match tensor_layout_info(state.tree(), *type_id) {
            Ok(layout) => layout,
            Err(error) => return Transfer::Error(error),
        };
        let axis_index = *axis as usize;
        if axis_index >= layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
        axis_sizes.push(layout.shape[axis_index]);
        inputs.push((value, *type_id, layout));
    }

    // compute axis offsets
    let mut axis_offsets = Vec::with_capacity(axis_sizes.len());
    let mut running = 0u64;
    for size in &axis_sizes {
        axis_offsets.push(running);
        running = running.saturating_add(*size);
    }

    let axis_index = *axis as usize;
    let mut input_index = vec![0u64; dest_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
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
                return Err(Error::InvalidInstruction);
            };

            input_index.clone_from_slice(output_index);
            input_index[axis_index] = local_axis;
            let (tensor_value, tensor_type, layout) = &inputs[input_idx];

            let src_offset = tensor_linear_index(&input_index, &layout.shape, &layout.strides)?;

            load_tensor_element_at(state, *tensor_value, *tensor_type, src_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.reduce.
pub(crate) fn execute_tensor_reduce(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorReduce {
        dest,
        operator,
        tensor,
        initial,
        axes,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let element_type = match tensor_element_type(state.tree(), *source_type) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve source tensor
    let tensor_value = state.get(*tensor);
    let init_value = state.get(*initial);
    let reduce_axes: std::collections::HashSet<u32> = axes.iter().copied().collect();
    let reduced_shape = axes
        .iter()
        .map(|axis| {
            source_layout
                .shape
                .get(*axis as usize)
                .copied()
                .ok_or(Error::InvalidInstruction)
        })
        .collect::<Result<Vec<_>, _>>();
    let reduced_shape = match reduced_shape {
        Ok(shape) => shape,
        Err(error) => return Transfer::Error(error),
    };
    let mut source_index = vec![0u64; source_layout.shape.len()];

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            let mut output_cursor = 0usize;
            for (dim, element) in source_index.iter_mut().enumerate() {
                if reduce_axes.contains(&(dim as u32)) {
                    *element = 0;
                    continue;
                }

                *element = output_index
                    .get(output_cursor)
                    .copied()
                    .ok_or(Error::InvalidInstruction)?;
                output_cursor += 1;
            }

            let mut accum = init_value;
            let mut reduce_error = None;

            // accumulate the reduced axes for this destination index
            for_each_index(&reduced_shape, |reduced_index| {
                if reduce_error.is_some() {
                    return;
                }

                for (reduce_position, axis) in axes.iter().enumerate() {
                    source_index[*axis as usize] = reduced_index[reduce_position];
                }

                let src_offset = match tensor_linear_index(
                    &source_index,
                    &source_layout.shape,
                    &source_layout.strides,
                ) {
                    Ok(offset) => offset,
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                };
                let src_value =
                    match load_tensor_element_at(state, tensor_value, *source_type, src_offset) {
                        Ok(value) => value,
                        Err(error) => {
                            reduce_error = Some(error);
                            return;
                        }
                    };

                accum = match apply_reduce_operator(
                    element_type,
                    ReduceOperator::from(*operator),
                    accum,
                    src_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                };
            });

            if let Some(error) = reduce_error {
                return Err(error);
            }

            Ok(accum)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.dot.
pub(crate) fn execute_tensor_dot(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorDot {
        dest,
        left,
        right,
        dimensions,
        left_type,
        right_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let left_layout = match tensor_layout_info(state.tree(), *left_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let right_layout = match tensor_layout_info(state.tree(), *right_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let element_type = match tensor_element_type(state.tree(), *dest_type) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve source tensors
    let left_value = state.get(*left);
    let right_value = state.get(*right);

    // validate storage lengths
    if left_layout.storage_len != right_layout.storage_len
        || left_layout.storage_len != dest_layout.storage_len
    {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor storage".to_string(),
            actual: format!(
                "{} vs {} vs {}",
                left_layout.storage_len, right_layout.storage_len, dest_layout.storage_len
            ),
        });
    }

    // compute axis sets
    let lhs_batch = &dimensions.lhs_batch;
    let rhs_batch = &dimensions.rhs_batch;
    let lhs_contract = &dimensions.lhs_contracting;
    let rhs_contract = &dimensions.rhs_contracting;
    if lhs_batch.len() != rhs_batch.len() || lhs_contract.len() != rhs_contract.len() {
        return Transfer::Error(Error::InvalidInstruction);
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

    let contract_shape: Vec<u64> = lhs_contract
        .iter()
        .map(|dim| left_layout.shape[*dim as usize])
        .collect();
    let mut lhs_index = vec![0u64; lhs_rank];
    let mut rhs_index = vec![0u64; rhs_rank];

    // allocate result
    if let Err(error) = allocate_tensor_by_index(state, *dest, &dest_layout, |state, out_index| {
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

        let mut accum = None;
        let mut contract_error = None;

        // iterate the contracting dimensions for this destination element
        for_each_index(&contract_shape, |contract_index| {
            if contract_error.is_some() {
                return;
            }

            for (i, dim) in lhs_contract.iter().enumerate() {
                lhs_index[*dim as usize] = contract_index[i];
            }
            for (i, dim) in rhs_contract.iter().enumerate() {
                rhs_index[*dim as usize] = contract_index[i];
            }

            let lhs_offset =
                match tensor_linear_index(&lhs_index, &left_layout.shape, &left_layout.strides) {
                    Ok(offset) => offset,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
            let rhs_offset =
                match tensor_linear_index(&rhs_index, &right_layout.shape, &right_layout.strides) {
                    Ok(offset) => offset,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
            let lhs_val = match load_tensor_element_at(state, left_value, *left_type, lhs_offset) {
                Ok(value) => value,
                Err(error) => {
                    contract_error = Some(error);
                    return;
                }
            };
            let rhs_val = match load_tensor_element_at(state, right_value, *right_type, rhs_offset)
            {
                Ok(value) => value,
                Err(error) => {
                    contract_error = Some(error);
                    return;
                }
            };
            let product = match apply_reduce_operator(
                element_type,
                ReduceOperator::Multiply,
                lhs_val,
                rhs_val,
            ) {
                Ok(value) => value,
                Err(error) => {
                    contract_error = Some(error);
                    return;
                }
            };

            accum = match accum {
                None => Some(product),
                Some(current) => {
                    match apply_reduce_operator(element_type, ReduceOperator::Add, current, product)
                    {
                        Ok(value) => Some(value),
                        Err(error) => {
                            contract_error = Some(error);
                            return;
                        }
                    }
                }
            };
        });

        if let Some(error) = contract_error {
            return Err(error);
        }

        accum.ok_or(Error::InvalidInstruction)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.convolution.
pub(crate) fn execute_tensor_convolution(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorConvolution {
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
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let input_layout = match tensor_layout_info(state.tree(), *input_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let kernel_layout = match tensor_layout_info(state.tree(), *kernel_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let element_type = match tensor_element_type(state.tree(), *dest_type) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve source tensors
    let input_value = state.get(*input);
    let kernel_value = state.get(*kernel);

    // derive dimension mappings
    let spatial_rank = dimensions.input_spatial.len();
    let output_spatial = &dimensions.output_spatial;
    let kernel_spatial = &dimensions.kernel_spatial;
    if output_spatial.len() != spatial_rank || kernel_spatial.len() != spatial_rank {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // validate window shapes
    if window.strides.len() != spatial_rank
        || window.padding_low.len() != spatial_rank
        || window.padding_high.len() != spatial_rank
        || window.lhs_dilation.len() != spatial_rank
        || window.rhs_dilation.len() != spatial_rank
        || window.window_reversal.len() != spatial_rank
    {
        return Transfer::Error(Error::InvalidInstruction);
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
        return Transfer::Error(Error::InvalidInstruction);
    }

    for dim in dimensions.input_spatial.iter() {
        if *dim as usize >= input_layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
    }
    for dim in output_spatial.iter() {
        if *dim as usize >= dest_layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
    }

    let input_batch_size = input_layout.shape[input_batch_dim];
    let input_feature_size = input_layout.shape[input_feature_dim];
    let output_batch_size = dest_layout.shape[output_batch_dim];
    let output_feature_size = dest_layout.shape[output_feature_dim];

    let mut kernel_spatial_shape = Vec::with_capacity(kernel_spatial.len());
    for dim in kernel_spatial {
        let Some(size) = kernel_layout.shape.get(*dim as usize) else {
            return Transfer::Error(Error::InvalidInstruction);
        };
        kernel_spatial_shape.push(*size);
    }

    // validate group counts
    if *feature_group_count == 0 || *batch_group_count == 0 {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let out_features_per_group = output_feature_size / (*feature_group_count as u64);
    let in_features_per_group = input_feature_size / (*feature_group_count as u64);
    let out_batches_per_group = output_batch_size / (*batch_group_count as u64);
    let in_batches_per_group = input_batch_size / (*batch_group_count as u64);
    let mut input_index = vec![0u64; input_layout.shape.len()];
    let mut kernel_index = vec![0u64; kernel_layout.shape.len()];

    // allocate result
    if let Err(error) = allocate_tensor_by_index(state, *dest, &dest_layout, |state, out_index| {
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
        let mut convolution_error = None;

        for in_feature in 0..in_features_per_group {
            input_index[input_feature_dim] = input_feature_base + in_feature;
            kernel_index[kernel_input_feature_dim] = in_feature;

            for_each_index(&kernel_spatial_shape, |kernel_spatial_index| {
                if convolution_error.is_some() {
                    return;
                }

                let mut is_valid = true;
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
                        is_valid = false;
                        break;
                    }
                    input_pos -= padding_low;
                    if lhs_dilation > 1 {
                        if input_pos % lhs_dilation != 0 {
                            is_valid = false;
                            break;
                        }
                        input_pos /= lhs_dilation;
                    }
                    if input_pos >= input_layout.shape[dim as usize] {
                        is_valid = false;
                        break;
                    }

                    input_index[dim as usize] = input_pos;
                    kernel_index[kernel_dim] = kernel_pos;
                }

                if !is_valid {
                    return;
                }

                let input_offset = match tensor_linear_index(
                    &input_index,
                    &input_layout.shape,
                    &input_layout.strides,
                ) {
                    Ok(offset) => offset,
                    Err(error) => {
                        convolution_error = Some(error);
                        return;
                    }
                };
                let kernel_offset = match tensor_linear_index(
                    &kernel_index,
                    &kernel_layout.shape,
                    &kernel_layout.strides,
                ) {
                    Ok(offset) => offset,
                    Err(error) => {
                        convolution_error = Some(error);
                        return;
                    }
                };
                let input_val =
                    match load_tensor_element_at(state, input_value, *input_type, input_offset) {
                        Ok(value) => value,
                        Err(error) => {
                            convolution_error = Some(error);
                            return;
                        }
                    };
                let kernel_val = match load_tensor_element_at(
                    state,
                    kernel_value,
                    *kernel_type,
                    kernel_offset,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        convolution_error = Some(error);
                        return;
                    }
                };
                let product = match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Multiply,
                    input_val,
                    kernel_val,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        convolution_error = Some(error);
                        return;
                    }
                };

                accum = match accum {
                    None => Some(product),
                    Some(current) => {
                        match apply_reduce_operator(
                            element_type,
                            ReduceOperator::Add,
                            current,
                            product,
                        ) {
                            Ok(value) => Some(value),
                            Err(error) => {
                                convolution_error = Some(error);
                                return;
                            }
                        }
                    }
                };
            });
        }

        if let Some(error) = convolution_error {
            return Err(error);
        }

        accum.ok_or(Error::InvalidInstruction)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.gather.
pub(crate) fn execute_tensor_gather(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorGather {
        dest,
        operand,
        indices,
        dimensions,
        slice_sizes,
        operand_type,
        indices_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let operand_layout = match tensor_layout_info(state.tree(), *operand_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let indices_layout = match tensor_layout_info(state.tree(), *indices_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    // resolve tensors
    let operand_value = state.get(*operand);
    let indices_value = state.get(*indices);
    let offset_dims: std::collections::HashSet<u32> =
        dimensions.offset_dims.iter().copied().collect();
    let collapsed_dims: std::collections::HashSet<u32> =
        dimensions.collapsed_slice_dims.iter().copied().collect();

    // allocate result
    if let Err(error) = allocate_tensor_by_index(state, *dest, &dest_layout, |state, out_index| {
        let mut index_coords = Vec::new();
        for (dim, value) in out_index.iter().enumerate() {
            if !offset_dims.contains(&(dim as u32)) {
                index_coords.push(*value);
            }
        }

        let mut index_vec = vec![0u64; dimensions.start_index_map.len()];
        let mut indices_index = vec![0u64; indices_layout.shape.len()];
        let mut coord_iter = index_coords.iter();
        for (dim, element) in indices_index.iter_mut().enumerate() {
            if dim as u32 == dimensions.index_vector_dim {
                continue;
            }
            *element = *coord_iter.next().unwrap_or(&0);
        }

        let index_offset = tensor_linear_index(
            &indices_index,
            &indices_layout.shape,
            &indices_layout.strides,
        )?;
        let index_base = index_offset;
        for (i, &_map_dim) in dimensions.start_index_map.iter().enumerate() {
            let element_index = index_base + i;
            let value = load_tensor_element_at(state, indices_value, *indices_type, element_index)?;
            index_vec[i] = value_to_u64(value)?;
            indices_index[dimensions.index_vector_dim as usize] = index_vec[i];
        }

        let mut operand_index = vec![0u64; operand_layout.shape.len()];
        for (i, &map_dim) in dimensions.start_index_map.iter().enumerate() {
            operand_index[map_dim as usize] = index_vec[i];
        }

        let mut offset_iter = out_index.iter();
        for (dim, element) in operand_index.iter_mut().enumerate() {
            if collapsed_dims.contains(&(dim as u32)) {
                continue;
            }

            let offset = if offset_dims.contains(&(dim as u32)) {
                *offset_iter.next().unwrap_or(&0)
            } else {
                0
            };
            let size = slice_sizes.get(dim).copied().unwrap_or(1) as u64;
            let start = *element;
            *element = start + offset.min(size.saturating_sub(1));
        }

        let src_offset = tensor_linear_index(
            &operand_index,
            &operand_layout.shape,
            &operand_layout.strides,
        )?;

        load_tensor_element_at(state, operand_value, *operand_type, src_offset)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.scatter.
pub(crate) fn execute_tensor_scatter(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorScatter {
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
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let operand_layout = match tensor_layout_info(state.tree(), *operand_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let indices_layout = match tensor_layout_info(state.tree(), *indices_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let updates_layout = match tensor_layout_info(state.tree(), *updates_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let element_type = match tensor_element_type(state.tree(), *dest_type) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve tensors
    let operand_value = state.get(*operand);
    let indices_value = state.get(*indices);
    let updates_value = state.get(*updates);

    if let Err(error) = write_tensor_by_element(
        state,
        *dest,
        dest_layout.storage_len,
        |state, element_index| {
            load_tensor_element_at(state, operand_value, *operand_type, element_index)
        },
    ) {
        return Transfer::Error(error);
    }
    let result = match state.value_address(*dest) {
        Ok(pointer) => Word::frame_pointer(pointer),
        Err(error) => return Transfer::Error(error),
    };
    let update_window_dims: std::collections::HashSet<u32> =
        dimensions.update_window_dims.iter().copied().collect();
    let inserted_window_dims: std::collections::HashSet<u32> =
        dimensions.inserted_window_dims.iter().copied().collect();
    let mut scatter_error = None;

    // scatter updates
    for_each_index(&updates_layout.shape, |update_index| {
        if scatter_error.is_some() {
            return;
        }

        let mut index_coords = Vec::new();
        for (dim, value) in update_index.iter().enumerate() {
            if !update_window_dims.contains(&(dim as u32)) {
                index_coords.push(*value);
            }
        }

        let mut indices_index = vec![0u64; indices_layout.shape.len()];
        let mut coord_iter = index_coords.iter();
        for (dim, element) in indices_index.iter_mut().enumerate() {
            if dim as u32 == dimensions.index_vector_dim {
                continue;
            }
            *element = *coord_iter.next().unwrap_or(&0);
        }

        let index_offset = tensor_linear_index(
            &indices_index,
            &indices_layout.shape,
            &indices_layout.strides,
        );
        let Ok(index_offset) = index_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };

        let mut scatter_indices = Vec::with_capacity(dimensions.scatter_dims_to_operand_dims.len());
        for i in 0..dimensions.scatter_dims_to_operand_dims.len() {
            let element_index = index_offset + i;
            let Ok(value) =
                load_tensor_element_at(state, indices_value, *indices_type, element_index)
            else {
                scatter_error = Some(Error::InvalidInstruction);
                return;
            };
            if let Ok(coord) = value_to_u64(value) {
                scatter_indices.push(coord);
            } else {
                scatter_error = Some(Error::InvalidInstruction);
                return;
            }
        }

        let mut operand_index = vec![0u64; operand_layout.shape.len()];
        for (i, &dim) in dimensions.scatter_dims_to_operand_dims.iter().enumerate() {
            operand_index[dim as usize] = scatter_indices[i];
        }

        let mut update_iter = update_index.iter();
        for (dim, element) in operand_index.iter_mut().enumerate() {
            if inserted_window_dims.contains(&(dim as u32)) {
                continue;
            }
            if update_window_dims.contains(&(dim as u32)) {
                *element = *update_iter.next().unwrap_or(&0);
            }
        }

        let dst_offset =
            tensor_linear_index(&operand_index, &dest_layout.shape, &dest_layout.strides);
        let Ok(dst_offset) = dst_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let update_offset =
            tensor_linear_index(update_index, &updates_layout.shape, &updates_layout.strides);
        let Ok(update_offset) = update_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let Ok(update_value) =
            load_tensor_element_at(state, updates_value, *updates_type, update_offset)
        else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let current_value = match load_tensor_element_at(state, result, *dest_type, dst_offset) {
            Ok(value) => value,
            Err(error) => {
                scatter_error = Some(error);
                return;
            }
        };
        let new_value = match mode {
            mir::TensorScatterMode::Replace => update_value,
            mir::TensorScatterMode::Add => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Add,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::Multiply => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Multiply,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::Min => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Min,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::Max => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Max,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::And => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::And,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::Or => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Or,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
            mir::TensorScatterMode::Xor => {
                match apply_reduce_operator(
                    element_type,
                    ReduceOperator::Xor,
                    current_value,
                    update_value,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        scatter_error = Some(error);
                        return;
                    }
                }
            }
        };

        if let Err(error) =
            store_tensor_element_at(state, result, *dest_type, dst_offset, new_value)
        {
            scatter_error = Some(error);
        }
    });

    if let Some(error) = scatter_error {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.convert.
pub(crate) fn execute_tensor_convert(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorConvert {
        dest,
        mode,
        tensor,
        source_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve tensor types
    let (source_element, source_layout) = match state.tree().get(*source_type) {
        mir::Type::Tensor { element, .. } => {
            let layout = match tensor_layout_info(state.tree(), *source_type) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            (*element, layout)
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{source_type:?}"),
            });
        }
    };
    let dest_type_info = state.tree().get(*dest_type);
    let (dest_element, dest_layout) = match dest_type_info {
        mir::Type::Tensor { element, .. } => {
            let layout = match tensor_layout_info(state.tree(), *dest_type) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            (*element, layout)
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor type".to_string(),
                actual: format!("{dest_type:?}"),
            });
        }
    };

    let tensor_value = state.get(*tensor);
    if source_layout.storage_len != dest_layout.storage_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor storage".to_string(),
            actual: format!(
                "{} vs {}",
                source_layout.storage_len, dest_layout.storage_len
            ),
        });
    }

    let Some(source_element) = source_element.ty() else {
        return Transfer::Error(Error::ConcreteMirRequired {
            context: "tensor convert source element".to_string(),
        });
    };
    let Some(dest_element) = dest_element.ty() else {
        return Transfer::Error(Error::ConcreteMirRequired {
            context: "tensor convert destination element".to_string(),
        });
    };
    let source_info = match scalar_layout(state.tree(), source_element) {
        Ok(info) => info,
        Err(error) => return Transfer::Error(error),
    };
    let dest_info = match scalar_layout(state.tree(), dest_element) {
        Ok(info) => info,
        Err(error) => return Transfer::Error(error),
    };
    let convert_mode = ScalarConvertMode::from(*mode);

    // allocate result
    if let Err(error) = write_tensor_by_element(
        state,
        *dest,
        dest_layout.storage_len,
        |state, element_index| {
            let src = load_tensor_element_at(state, tensor_value, *source_type, element_index)?;

            convert_scalar_value(src, source_info, dest_info, convert_mode)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.compare.
pub(crate) fn execute_tensor_compare(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorCompare {
        dest,
        operator,
        left,
        right,
        left_type,
        right_type,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let left_layout = match tensor_layout_info(state.tree(), *left_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let right_layout = match tensor_layout_info(state.tree(), *right_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let element_type = match tensor_element_type(state.tree(), *left_type) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // resolve operand tensors
    let left_value = state.get(*left);
    let right_value = state.get(*right);

    // allocate result
    if let Err(error) =
        allocate_tensor_by_index(state, *dest, &dest_layout, |state, output_index| {
            let left_offset =
                tensor_linear_index(output_index, &left_layout.shape, &left_layout.strides)?;
            let right_offset =
                tensor_linear_index(output_index, &right_layout.shape, &right_layout.strides)?;
            let left_value = load_tensor_element_at(state, left_value, *left_type, left_offset)?;
            let right_value =
                load_tensor_element_at(state, right_value, *right_type, right_offset)?;

            apply_binary_operator(element_type, *operator, left_value, right_value)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.select.
pub(crate) fn execute_tensor_select(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::TensorSelect {
        dest,
        mask,
        then_value,
        else_value,
        dest_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    let mask_type = match state.value_type(*mask) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let then_type = match state.value_type(*then_value) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let else_type = match state.value_type(*else_value) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let mask_value = state.get(*mask);
    let then_value = state.get(*then_value);
    let else_value = state.get(*else_value);
    if let Err(error) = write_tensor_by_element(
        state,
        *dest,
        dest_layout.storage_len,
        |state, element_index| {
            let mask_value = load_tensor_element_at(state, mask_value, mask_type, element_index)?;
            let then_element = load_tensor_element_at(state, then_value, then_type, element_index)?;
            let else_element = load_tensor_element_at(state, else_value, else_type, element_index)?;

            let select = mask_value.as_bool();
            Ok(if select { then_element } else { else_element })
        },
    ) {
        return Transfer::Error(error);
    }
    Transfer::Continue
}

/// Execute tensor.cast.
pub(crate) fn execute_tensor_cast(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorCast { dest, tensor } = &block[pc].operands else {
        unreachable!()
    };

    // forward the tensor bytes
    if let Err(error) = state.move_value_to_value(*tensor, *dest) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.view.
pub(crate) fn execute_tensor_view(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::TensorView {
        dest,
        view,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_type,
        dest_type,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve layouts
    let source_layout = match tensor_layout_info(state.tree(), *source_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match tensor_layout_info(state.tree(), *dest_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };

    // resolve view arguments
    let args = state.argument_slice(*arguments);
    let (offset_values, rest) = args.split_at(*offsets_count as usize);
    let (size_values, stride_values) = rest.split_at(*sizes_count as usize);
    if stride_values.len() != *strides_count as usize {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for value_id in offset_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in size_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for value_id in stride_values {
        let value = state.get(*value_id);
        match value_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // validate sizes and strides against the destination layout
    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if *expected != *actual {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor.view size".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }
    for (expected, actual) in dest_layout.strides.iter().zip(strides.iter()) {
        if *expected != *actual {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor.view stride".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }

    // compute offset into the source view
    let offset = match tensor_linear_index(&offsets, &source_layout.shape, &source_layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = state.get(*view);
    let Some(element) = *element else {
        return Transfer::Error(Error::TypeMismatch {
            expected: "compiled tensor element access".to_string(),
            actual: format!("{source_type:?}"),
        });
    };
    let pointer = match offset_pointer(view_value, element, offset, source_layout.storage_len) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // set the view result
    state.set_word(*dest, pointer);

    // continue to next instruction
    Transfer::Continue
}
