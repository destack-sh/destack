use super::prelude::*;
use crate::diagnostic::Error;
use crate::execute::bytes::checked_frame_value_byte_range;

/// Load one array index operand as an unsigned value.
#[inline(always)]
fn load_array_index(state: &DispatchState<'_, '_>, index: mir::Value) -> u64 {
    state.get(index).as_u64()
}

/// Return one lowered field access.
#[inline(always)]
fn field_access(field: Option<FieldAccess>) -> Result<FieldAccess, Error> {
    field.ok_or(Error::InvalidInstruction)
}

/// Return one lowered element access.
#[inline(always)]
fn element_access(element: Option<ElementAccess>) -> Result<ElementAccess, Error> {
    element.ok_or(Error::InvalidInstruction)
}

/// Compute a field address using lowered pointer metadata.
#[inline(always)]
fn field_addr(
    state: &mut DispatchState<'_, '_>,
    base: Word,
    index: u32,
    reference: ReferenceMeta,
    field_count: Option<u32>,
    field: FieldAccess,
) -> Result<Word, Error> {
    let value = match field.pointer_class {
        PointerClass::Heap => {
            access::field_addr_heap(state, base.as_heap_reference(), field, index, field_count)?
        }
        PointerClass::SharedHeap => access::field_addr_shared_heap(
            state,
            base.as_shared_heap_reference(),
            field,
            index,
            field_count,
        )?,
        PointerClass::Raw => {
            access::field_addr_raw(state, base.as_raw_pointer(), field, index, field_count)?
        }
        PointerClass::SharedRaw => access::field_addr_shared_raw(
            state,
            base.as_shared_raw_pointer(),
            field,
            index,
            field_count,
        )?,
        PointerClass::Stack => {
            access::field_addr_stack(state, base.as_stack_pointer(), field, index, field_count)?
        }
        PointerClass::Frame => {
            let field_count = field_count.ok_or(Error::InvalidInstruction)?;
            let pointer = base.as_frame_pointer().add_bytes(field.byte_offset).ok_or(
                Error::InvalidFieldAccess {
                    index,
                    field_count: field_count as usize,
                },
            )?;
            Word::frame_pointer(pointer)
        }
        PointerClass::Static => {
            access::field_addr_static(state, base.as_static_pointer(), field, index, field_count)?
        }
        PointerClass::Unknown => return Err(access::invalid_pointer_type(base)),
    };

    check_reference_kind(state, reference, value)?;

    Ok(value)
}

/// Load a field using lowered pointer metadata.
#[inline(always)]
fn field_load(
    state: &mut DispatchState<'_, '_>,
    base: Word,
    index: u32,
    field_count: Option<u32>,
    field: FieldAccess,
) -> Result<Word, Error> {
    match field.pointer_class {
        PointerClass::Heap => {
            access::load_field_heap(state, base.as_heap_reference(), field, index, field_count)
        }
        PointerClass::SharedHeap => access::load_field_shared_heap(
            state,
            base.as_shared_heap_reference(),
            field,
            index,
            field_count,
        ),
        PointerClass::Raw => {
            access::load_field_raw(state, base.as_raw_pointer(), field, index, field_count)
        }
        PointerClass::SharedRaw => access::load_field_shared_raw(
            state,
            base.as_shared_raw_pointer(),
            field,
            index,
            field_count,
        ),
        PointerClass::Stack => {
            access::load_field_stack(state, base.as_stack_pointer(), field, index, field_count)
        }
        PointerClass::Frame => {
            let pointer = base.as_frame_pointer();
            access::load_frame_pointer(state, pointer, field.into())
        }
        PointerClass::Static => {
            access::load_field_static(state, base.as_static_pointer(), field, index, field_count)
        }
        PointerClass::Unknown => Err(access::invalid_pointer_type(base)),
    }
}

/// Store a field using lowered pointer metadata.
#[inline(always)]
fn field_store(
    state: &mut DispatchState<'_, '_>,
    base: Word,
    index: u32,
    value: mir::Value,
    reference: ReferenceMeta,
    field_count: Option<u32>,
    field: FieldAccess,
) -> Result<(), Error> {
    check_reference_kind(state, reference, base)?;
    check_reference_mutability(state, reference)?;

    if field.is_scalar {
        let value = state.get(value);

        return match field.pointer_class {
            PointerClass::Heap => access::store_field_heap(
                state,
                base.as_heap_reference(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::SharedHeap => access::store_field_shared_heap(
                state,
                base.as_shared_heap_reference(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::Raw => access::store_field_raw(
                state,
                base.as_raw_pointer(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::SharedRaw => access::store_field_shared_raw(
                state,
                base.as_shared_raw_pointer(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::Stack => access::store_field_stack(
                state,
                base.as_stack_pointer(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::Frame => {
                let pointer = base.as_frame_pointer();
                access::store_frame_pointer(state, pointer, field.into(), value)
            }
            PointerClass::Static => access::store_field_static(
                state,
                base.as_static_pointer(),
                field,
                index,
                field_count,
                value,
            ),
            PointerClass::Unknown => Err(access::invalid_pointer_type(base)),
        };
    }

    if field.pointer_class == PointerClass::Heap {
        return Err(Error::TypeMismatch {
            expected: "scalar heap field store".to_string(),
            actual: format!("{:?}", field.value_type),
        });
    }
    if field.pointer_class == PointerClass::SharedHeap {
        return Err(Error::TypeMismatch {
            expected: "scalar shared heap field store".to_string(),
            actual: format!("{:?}", field.value_type),
        });
    }

    let (source, source_len) = checked_frame_value_byte_range(state, value, field.byte_len)?;
    let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };

    match field.pointer_class {
        PointerClass::Heap => Err(Error::TypeMismatch {
            expected: "scalar heap field store".to_string(),
            actual: format!("{:?}", field.value_type),
        }),
        PointerClass::SharedHeap => Err(Error::TypeMismatch {
            expected: "scalar shared heap field store".to_string(),
            actual: format!("{:?}", field.value_type),
        }),
        PointerClass::Raw => access::store_field_raw_bytes(
            state,
            base.as_raw_pointer(),
            field,
            index,
            field_count,
            bytes,
        ),
        PointerClass::SharedRaw => access::store_field_shared_raw_bytes(
            state,
            base.as_shared_raw_pointer(),
            field,
            index,
            field_count,
            bytes,
        ),
        PointerClass::Stack => access::store_field_stack_bytes(
            state,
            base.as_stack_pointer(),
            field,
            index,
            field_count,
            bytes,
        ),
        PointerClass::Frame => {
            let pointer = base.as_frame_pointer();
            access::store_frame_pointer_bytes(state, pointer, field.into(), bytes)
        }
        PointerClass::Static => access::store_field_static_bytes(
            state,
            base.as_static_pointer(),
            field,
            index,
            field_count,
            bytes,
        ),
        PointerClass::Unknown => Err(access::invalid_pointer_type(base)),
    }
}

/// Compute an element address using lowered pointer metadata.
#[inline(always)]
fn element_addr(
    state: &mut DispatchState<'_, '_>,
    array: Word,
    index: u64,
    reference: ReferenceMeta,
    array_length: Option<u64>,
    element: ElementAccess,
) -> Result<Word, Error> {
    let value = match element.pointer_class {
        PointerClass::Heap => access::element_addr_heap(
            state,
            array.as_heap_reference(),
            element,
            index,
            array_length,
        )?,
        PointerClass::SharedHeap => access::element_addr_shared_heap(
            state,
            array.as_shared_heap_reference(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Raw => {
            access::element_addr_raw(state, array.as_raw_pointer(), element, index, array_length)?
        }
        PointerClass::SharedRaw => access::element_addr_shared_raw(
            state,
            array.as_shared_raw_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Stack => access::element_addr_stack(
            state,
            array.as_stack_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Frame => {
            let length = array_length.ok_or(Error::InvalidInstruction)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess { index, length })?;
            let pointer = array
                .as_frame_pointer()
                .add_bytes(offset)
                .ok_or(Error::InvalidArrayAccess { index, length })?;
            Word::frame_pointer(pointer)
        }
        PointerClass::Static => access::element_addr_static(
            state,
            array.as_static_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Unknown => return Err(access::invalid_pointer_type(array)),
    };

    check_reference_kind(state, reference, value)?;

    Ok(value)
}

/// Load an element using lowered pointer metadata.
#[inline(always)]
fn element_load(
    state: &mut DispatchState<'_, '_>,
    array: Word,
    index: u64,
    array_length: Option<u64>,
    element: ElementAccess,
) -> Result<Word, Error> {
    match element.pointer_class {
        PointerClass::Heap => access::load_element_heap(
            state,
            array.as_heap_reference(),
            element,
            index,
            array_length,
        ),
        PointerClass::SharedHeap => access::load_element_shared_heap(
            state,
            array.as_shared_heap_reference(),
            element,
            index,
            array_length,
        ),
        PointerClass::Raw => {
            access::load_element_raw(state, array.as_raw_pointer(), element, index, array_length)
        }
        PointerClass::SharedRaw => access::load_element_shared_raw(
            state,
            array.as_shared_raw_pointer(),
            element,
            index,
            array_length,
        ),
        PointerClass::Stack => access::load_element_stack(
            state,
            array.as_stack_pointer(),
            element,
            index,
            array_length,
        ),
        PointerClass::Frame => {
            let length = array_length.ok_or(Error::InvalidInstruction)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess { index, length })?;
            let pointer = array
                .as_frame_pointer()
                .add_bytes(offset)
                .ok_or(Error::InvalidArrayAccess { index, length })?;

            access::load_frame_pointer(state, pointer, element.into())
        }
        PointerClass::Static => access::load_element_static(
            state,
            array.as_static_pointer(),
            element,
            index,
            array_length,
        ),
        PointerClass::Unknown => Err(access::invalid_pointer_type(array)),
    }
}

/// Store an element using lowered pointer metadata.
#[inline(always)]
fn element_store(
    state: &mut DispatchState<'_, '_>,
    array: Word,
    index: u64,
    value: mir::Value,
    reference: ReferenceMeta,
    array_length: Option<u64>,
    element: ElementAccess,
) -> Result<(), Error> {
    check_reference_kind(state, reference, array)?;
    check_reference_mutability(state, reference)?;

    if element.is_scalar {
        let value = state.get(value);

        return match element.pointer_class {
            PointerClass::Heap => access::store_element_heap(
                state,
                array.as_heap_reference(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::SharedHeap => access::store_element_shared_heap(
                state,
                array.as_shared_heap_reference(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::Raw => access::store_element_raw(
                state,
                array.as_raw_pointer(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::SharedRaw => access::store_element_shared_raw(
                state,
                array.as_shared_raw_pointer(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::Stack => access::store_element_stack(
                state,
                array.as_stack_pointer(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::Frame => {
                let length = array_length.ok_or(Error::InvalidInstruction)?;
                let offset = usize::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_mul(element.byte_stride))
                    .ok_or(Error::InvalidArrayAccess { index, length })?;
                let pointer = array
                    .as_frame_pointer()
                    .add_bytes(offset)
                    .ok_or(Error::InvalidArrayAccess { index, length })?;

                access::store_frame_pointer(state, pointer, element.into(), value)
            }
            PointerClass::Static => access::store_element_static(
                state,
                array.as_static_pointer(),
                element,
                index,
                array_length,
                value,
            ),
            PointerClass::Unknown => Err(access::invalid_pointer_type(array)),
        };
    }

    if element.pointer_class == PointerClass::Heap {
        return Err(Error::TypeMismatch {
            expected: "scalar heap element store".to_string(),
            actual: format!("{:?}", element.value_type),
        });
    }
    if element.pointer_class == PointerClass::SharedHeap {
        return Err(Error::TypeMismatch {
            expected: "scalar shared heap element store".to_string(),
            actual: format!("{:?}", element.value_type),
        });
    }

    let (source, source_len) = checked_frame_value_byte_range(state, value, element.byte_len)?;
    let bytes = unsafe { std::slice::from_raw_parts(source, source_len) };

    match element.pointer_class {
        PointerClass::Heap => Err(Error::TypeMismatch {
            expected: "scalar heap element store".to_string(),
            actual: format!("{:?}", element.value_type),
        }),
        PointerClass::SharedHeap => Err(Error::TypeMismatch {
            expected: "scalar shared heap element store".to_string(),
            actual: format!("{:?}", element.value_type),
        }),
        PointerClass::Raw => access::store_element_raw_bytes(
            state,
            array.as_raw_pointer(),
            element,
            index,
            array_length,
            bytes,
        ),
        PointerClass::SharedRaw => access::store_element_shared_raw_bytes(
            state,
            array.as_shared_raw_pointer(),
            element,
            index,
            array_length,
            bytes,
        ),
        PointerClass::Stack => access::store_element_stack_bytes(
            state,
            array.as_stack_pointer(),
            element,
            index,
            array_length,
            bytes,
        ),
        PointerClass::Frame => {
            let length = array_length.ok_or(Error::InvalidInstruction)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess { index, length })?;
            let pointer = array
                .as_frame_pointer()
                .add_bytes(offset)
                .ok_or(Error::InvalidArrayAccess { index, length })?;

            access::store_frame_pointer_bytes(state, pointer, element.into(), bytes)
        }
        PointerClass::Static => access::store_element_static_bytes(
            state,
            array.as_static_pointer(),
            element,
            index,
            array_length,
            bytes,
        ),
        PointerClass::Unknown => Err(access::invalid_pointer_type(array)),
    }
}

/// Execute field get.
pub(crate) fn execute_field_get(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldGet {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let value = match access::get_field(state, base, *index, *field_count, *field) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field addr.
pub(crate) fn execute_field_addr(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let value = match field_addr(state, base, *index, *reference, *field_count, field) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field addr on heap references.
pub(crate) fn execute_field_addr_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::field_addr_heap(state, base.as_heap_reference(), field, *index, *field_count)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field addr on raw pointers.
pub(crate) fn execute_field_addr_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::field_addr_raw(state, base.as_raw_pointer(), field, *index, *field_count) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field addr on stack pointers.
pub(crate) fn execute_field_addr_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::field_addr_stack(state, base.as_stack_pointer(), field, *index, *field_count)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field addr on static pointers.
pub(crate) fn execute_field_addr_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value = match access::field_addr_static(
        state,
        base.as_static_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load.
pub(crate) fn execute_field_load(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value = match field_load(state, base, *index, *field_count, field) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on heap references.
pub(crate) fn execute_field_load_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::load_field_heap(state, base.as_heap_reference(), field, *index, *field_count)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on raw pointers.
pub(crate) fn execute_field_load_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::load_field_raw(state, base.as_raw_pointer(), field, *index, *field_count) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on stack pointers.
pub(crate) fn execute_field_load_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value =
        match access::load_field_stack(state, base.as_stack_pointer(), field, *index, *field_count)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on static pointers.
pub(crate) fn execute_field_load_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    let value = match access::load_field_static(
        state,
        base.as_static_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field store.
pub(crate) fn execute_field_store(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    if let Err(error) = field_store(state, base, *index, *value, *reference, *field_count, field) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on heap references.
pub(crate) fn execute_field_store_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    if let Err(error) = field_store(state, base, *index, *value, *reference, *field_count, field) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on raw pointers.
pub(crate) fn execute_field_store_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    if let Err(error) = field_store(state, base, *index, *value, *reference, *field_count, field) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on stack pointers.
pub(crate) fn execute_field_store_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    if let Err(error) = field_store(state, base, *index, *value, *reference, *field_count, field) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on static pointers.
pub(crate) fn execute_field_store_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let field = match field_access(*field) {
        Ok(field) => field,
        Err(error) => return Transfer::Error(error),
    };
    let base = state.get(*base);
    if let Err(error) = field_store(state, base, *index, *value, *reference, *field_count, field) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element get.
pub(crate) fn execute_element_get(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementGet {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let array = match state.value_operand(*array) {
        Ok(array) => array,
        Err(error) => return Transfer::Error(error),
    };
    let index = load_array_index(state, *index);
    let value = match access::get_element(state, array, index, *array_length, *element) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element addr.
pub(crate) fn execute_element_addr(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = match state.value_operand(*array) {
        Ok(array) => array,
        Err(error) => return Transfer::Error(error),
    };
    let index = load_array_index(state, *index);
    let value = match element_addr(state, array, index, *reference, *array_length, element) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element addr on heap references.
pub(crate) fn execute_element_addr_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::element_addr_heap(
        state,
        array.as_heap_reference(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element addr on raw pointers.
pub(crate) fn execute_element_addr_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::element_addr_raw(
        state,
        array.as_raw_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element addr on stack pointers.
pub(crate) fn execute_element_addr_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::element_addr_stack(
        state,
        array.as_stack_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element addr on static pointers.
pub(crate) fn execute_element_addr_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::element_addr_static(
        state,
        array.as_static_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load.
pub(crate) fn execute_element_load(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match element_load(state, array, index, *array_length, element) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on heap references.
pub(crate) fn execute_element_load_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::load_element_heap(
        state,
        array.as_heap_reference(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on raw pointers.
pub(crate) fn execute_element_load_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::load_element_raw(
        state,
        array.as_raw_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on stack pointers.
pub(crate) fn execute_element_load_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::load_element_stack(
        state,
        array.as_stack_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on static pointers.
pub(crate) fn execute_element_load_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    let value = match access::load_element_static(
        state,
        array.as_static_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element store.
pub(crate) fn execute_element_store(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = element_store(
        state,
        array,
        index,
        *value,
        *reference,
        *array_length,
        element,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on heap references.
pub(crate) fn execute_element_store_heap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = element_store(
        state,
        array,
        index,
        *value,
        *reference,
        *array_length,
        element,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on raw pointers.
pub(crate) fn execute_element_store_raw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = element_store(
        state,
        array,
        index,
        *value,
        *reference,
        *array_length,
        element,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on stack pointers.
pub(crate) fn execute_element_store_stack(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = element_store(
        state,
        array,
        index,
        *value,
        *reference,
        *array_length,
        element,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on static pointers.
pub(crate) fn execute_element_store_static(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let element = match element_access(*element) {
        Ok(element) => element,
        Err(error) => return Transfer::Error(error),
    };
    let array = state.get(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = element_store(
        state,
        array,
        index,
        *value,
        *reference,
        *array_length,
        element,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
