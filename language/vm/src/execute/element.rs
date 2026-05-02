use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    ElementAccess, Instruction, PointerClass, Transfer, word_layout_from_type, *,
};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Load one array index operand as an unsigned value.
#[inline(always)]
pub(super) fn load_array_index(state: &DispatchState<'_, '_>, index: mir::Value) -> u64 {
    state.get_word(index).as_u64()
}

/// Return one element byte offset.
#[inline(always)]
pub(crate) fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Return one lowered element access for an already known indexed type.
#[inline(always)]
pub(crate) fn element_access_for_type(
    state: &DispatchState<'_, '_>,
    indexed_type: mir::LocalNodeId<mir::Type>,
    index: u64,
) -> Result<(ElementAccess, u64), Error> {
    let layout = state.layout(indexed_type)?;
    let element_count = layout.element_count().ok_or(Error::InvalidInstruction)? as u64;
    let element = layout.element().ok_or(Error::InvalidArrayAccess {
        index,
        length: element_count,
    })?;

    if index >= element_count {
        return Err(Error::InvalidArrayAccess {
            index,
            length: element_count,
        });
    }

    let access = ElementAccess {
        pointer_class: PointerClass::Frame,
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        word_layout: word_layout_from_type(state.tree(), element.ty),
    };

    Ok((access, element_count))
}

/// Compute an element address using lowered pointer metadata.
#[inline(always)]
pub(super) fn address_element(
    state: &mut DispatchState<'_, '_>,
    array: Word,
    index: u64,
    reference: ReferenceMeta,
    array_length: u64,
    element: ElementAccess,
) -> Result<Word, Error> {
    let value = match element.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => access::address_element_heap(
            state,
            array.as_heap_reference(),
            element,
            index,
            array_length,
        )?,
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            access::address_element_shared_heap(
                state,
                array.as_shared_heap_reference(),
                element,
                index,
                array_length,
            )?
        }
        PointerClass::Raw => access::address_element_raw(
            state,
            array.as_raw_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::SharedRaw => access::address_element_shared_raw(
            state,
            array.as_shared_raw_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Stack => access::address_element_stack(
            state,
            array.as_stack_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Frame => {
            let offset = element_byte_offset(index, element.byte_stride);
            let pointer = array.as_frame_pointer().add_bytes(offset);
            Word::frame_pointer(pointer)
        }
        PointerClass::Static => access::address_element_static(
            state,
            array.as_static_pointer(),
            element,
            index,
            array_length,
        )?,
        PointerClass::Unknown => return Err(access::invalid_pointer_type(array)),
    };

    check_reference_address_space(state, reference)?;

    Ok(value)
}

/// Publish one checked element address.
#[inline(always)]
fn publish_element_address(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(state, reference)?;
    state.set_word(dest, value);

    Ok(())
}

/// Check one element store destination.
#[inline(always)]
fn check_element_store(
    state: &DispatchState<'_, '_>,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    check_reference_address_space(state, reference)?;
    check_reference_mutability(state, reference)
}

/// Execute element addr on heap references.
pub(crate) fn execute_address_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    let value = match access::address_element_heap(
        state,
        array.as_heap_reference(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on shared heap references.
pub(crate) fn execute_address_shared_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // compute concrete address
    let value = match access::address_element_shared_heap(
        state,
        array.as_shared_heap_reference(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on raw pointers.
pub(crate) fn execute_address_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    let value = match access::address_element_raw(
        state,
        array.as_raw_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // compute concrete address
    let value = match access::address_element_shared_raw(
        state,
        array.as_shared_raw_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on stack pointers.
pub(crate) fn execute_address_stack_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    let value = match access::address_element_stack(
        state,
        array.as_stack_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element addr on static pointers.
pub(crate) fn execute_address_static_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementAddr>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    let value = match access::address_element_static(
        state,
        array.as_static_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_element_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element load on heap references.
pub(crate) fn execute_load_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
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

/// Execute element load on shared heap references.
pub(crate) fn execute_load_shared_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // load element word
    let value = match access::load_element_shared_heap(
        state,
        array.as_shared_heap_reference(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on raw pointers.
pub(crate) fn execute_load_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
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

/// Execute element load on shared raw pointers.
pub(crate) fn execute_load_shared_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // load element word
    let value = match access::load_element_shared_raw(
        state,
        array.as_shared_raw_pointer(),
        element,
        index,
        *array_length,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute element load on stack pointers.
pub(crate) fn execute_load_stack_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
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
pub(crate) fn execute_load_static_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = instruction.payload_as::<ElementLoad>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
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

/// Execute element store on heap references.
pub(crate) fn execute_store_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // validate store destination
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_element_heap(
        state,
        array.as_heap_reference(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on shared heap references.
pub(crate) fn execute_store_shared_heap_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    // store element word
    let value = state.get_word(*value);
    if let Err(error) = access::store_element_shared_heap(
        state,
        array.as_shared_heap_reference(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on raw pointers.
pub(crate) fn execute_store_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();

    // decode access metadata
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);

    // validate store destination
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_element_raw(
        state,
        array.as_raw_pointer(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on shared raw pointers.
pub(crate) fn execute_store_shared_raw_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    // store element word
    let value = state.get_word(*value);
    if let Err(error) = access::store_element_shared_raw(
        state,
        array.as_shared_raw_pointer(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on stack pointers.
pub(crate) fn execute_store_stack_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_element_stack(
        state,
        array.as_stack_pointer(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute element store on static pointers.
pub(crate) fn execute_store_static_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = instruction.payload_as::<ElementStore>();
    let element = state.element_access(*element);
    let array = state.get_word(*array);
    let index = load_array_index(state, *index);
    if let Err(error) = check_element_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_element_static(
        state,
        array.as_static_pointer(),
        element,
        index,
        *array_length,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
