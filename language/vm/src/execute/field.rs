use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    FieldAccess, FieldAddr, FieldLoad, FieldStore, Instruction, PointerClass, Transfer,
};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Load a field using lowered pointer metadata.
#[inline(always)]
pub(super) fn load_field(
    state: &mut DispatchState<'_, '_>,
    base: Word,
    index: u32,
    field_count: u32,
    field: FieldAccess,
) -> Result<Word, Error> {
    match field.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            access::load_field_heap(state, base.as_heap_reference(), field, index, field_count)
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            access::load_field_shared_heap(
                state,
                base.as_shared_heap_reference(),
                field,
                index,
                field_count,
            )
        }
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
            access::load_frame_word(state, pointer, field.into())
        }
        PointerClass::Static => {
            access::load_field_static(state, base.as_static_pointer(), field, index, field_count)
        }
        PointerClass::Unknown => Err(access::invalid_pointer_type(base)),
    }
}

/// Publish one checked field address.
#[inline(always)]
fn publish_field_address(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    reference: ReferenceMeta,
    value: Word,
) -> Result<(), Error> {
    check_reference_address_space(state, reference)?;
    state.set_word(dest, value);

    Ok(())
}

/// Check one field store destination.
#[inline(always)]
fn check_field_store(state: &DispatchState<'_, '_>, reference: ReferenceMeta) -> Result<(), Error> {
    check_reference_address_space(state, reference)?;
    check_reference_mutability(state, reference)
}

/// Execute field addr on heap references.
pub(crate) fn execute_address_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value = match access::address_field_heap(
        state,
        base.as_heap_reference(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on shared heap references.
pub(crate) fn execute_address_shared_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // compute concrete address
    let value = match access::address_field_shared_heap(
        state,
        base.as_shared_heap_reference(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on raw pointers.
pub(crate) fn execute_address_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value = match access::address_field_raw(
        state,
        base.as_raw_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on shared raw pointers.
pub(crate) fn execute_address_shared_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // compute concrete address
    let value = match access::address_field_shared_raw(
        state,
        base.as_shared_raw_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on stack pointers.
pub(crate) fn execute_address_stack_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value = match access::address_field_stack(
        state,
        base.as_stack_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field addr on static pointers.
pub(crate) fn execute_address_static_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldAddr {
        dest,
        base,
        index,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldAddr>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value = match access::address_field_static(
        state,
        base.as_static_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = publish_field_address(state, *dest, *reference, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field load on heap references.
pub(crate) fn execute_load_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value =
        match access::load_field_heap(state, base.as_heap_reference(), field, *index, *field_count)
        {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on shared heap references.
pub(crate) fn execute_load_shared_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // load field word
    let value = match access::load_field_shared_heap(
        state,
        base.as_shared_heap_reference(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on raw pointers.
pub(crate) fn execute_load_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    let value =
        match access::load_field_raw(state, base.as_raw_pointer(), field, *index, *field_count) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on shared raw pointers.
pub(crate) fn execute_load_shared_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // load field word
    let value = match access::load_field_shared_raw(
        state,
        base.as_shared_raw_pointer(),
        field,
        *index,
        *field_count,
    ) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute field load on stack pointers.
pub(crate) fn execute_load_stack_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
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
pub(crate) fn execute_load_static_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldLoad {
        dest,
        base,
        index,
        field_count,
        field,
    } = instruction.payload_as::<FieldLoad>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
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

/// Execute field store on heap references.
pub(crate) fn execute_store_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // validate store destination
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_field_heap(
        state,
        base.as_heap_reference(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on shared heap references.
pub(crate) fn execute_store_shared_heap_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    // store field word
    let value = state.get_word(*value);
    if let Err(error) = access::store_field_shared_heap(
        state,
        base.as_shared_heap_reference(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on raw pointers.
pub(crate) fn execute_store_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();

    // decode access metadata
    let field = state.field_access(*field);
    let base = state.get_word(*base);

    // validate store destination
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_field_raw(
        state,
        base.as_raw_pointer(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on shared raw pointers.
pub(crate) fn execute_store_shared_raw_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    // store field word
    let value = state.get_word(*value);
    if let Err(error) = access::store_field_shared_raw(
        state,
        base.as_shared_raw_pointer(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on stack pointers.
pub(crate) fn execute_store_stack_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_field_stack(
        state,
        base.as_stack_pointer(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute field store on static pointers.
pub(crate) fn execute_store_static_field(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FieldStore {
        base,
        index,
        value,
        reference,
        field_count,
        field,
    } = instruction.payload_as::<FieldStore>();
    let field = state.field_access(*field);
    let base = state.get_word(*base);
    if let Err(error) = check_field_store(state, *reference) {
        return Transfer::Error(error);
    }

    let value = state.get_word(*value);
    if let Err(error) = access::store_field_static(
        state,
        base.as_static_pointer(),
        field,
        *index,
        *field_count,
        value,
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}
