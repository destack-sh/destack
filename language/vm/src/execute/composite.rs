use super::prelude::*;
use crate::diagnostic::Error;
use crate::module::TypedAccess;

/// Apply reference metadata and validate the resulting pointer value.
#[inline(always)]
fn build_reference_result(
    state: &mut StepState<'_, '_>,
    reference: crate::ReferenceMeta,
    value: Value,
) -> Result<Value, Error> {
    // apply reference metadata
    let value = value.with_reference_meta(reference);

    // validate reference kind
    check_reference_kind(state, reference, value)?;

    Ok(value)
}

/// Load one array index operand as an unsigned value.
#[inline(always)]
fn load_array_index(state: &StepState<'_, '_>, index: mir::Value) -> Result<u64, Error> {
    let value = state.get(index);

    value.as_uint().ok_or_else(|| Error::TypeMismatch {
        expected: "unsigned integer".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Build one composite type mismatch.
#[inline(always)]
fn invalid_composite(value: Value) -> Error {
    Error::TypeMismatch {
        expected: "composite".to_string(),
        actual: format!("{value:?}"),
    }
}

/// Require one heap composite value.
#[inline(always)]
fn heap_composite(value: Value) -> Result<HeapReference, Error> {
    value
        .as_heap_reference()
        .ok_or_else(|| invalid_composite(value))
}

/// Require one shared heap composite value.
#[inline(always)]
fn shared_heap_composite(value: Value) -> Result<destack_heap::SharedHeapReference, Error> {
    value
        .as_shared_heap_reference()
        .ok_or_else(|| invalid_composite(value))
}

/// Require one raw composite pointer.
#[inline(always)]
fn raw_composite(value: Value) -> Result<RawPointer, Error> {
    value
        .as_raw_pointer()
        .ok_or_else(|| invalid_composite(value))
}

/// Require one stack composite pointer.
#[inline(always)]
fn stack_composite(value: Value) -> Result<StackPointer, Error> {
    value
        .as_stack_pointer()
        .ok_or_else(|| invalid_composite(value))
}

/// Require one global composite pointer.
#[inline(always)]
fn global_composite(value: Value) -> Result<GlobalPointer, Error> {
    value
        .as_global_pointer()
        .ok_or_else(|| invalid_composite(value))
}

/// Step field get.
pub(crate) fn step_field_get(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldGet {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // load field value
    let value = match access::get_field(state, agg, *index, *field_count, *field) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field addr.
pub(crate) fn step_field_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // compute field address
    let value = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::SharedHeapReference, Some(field)) => {
            let handle = match shared_heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_shared_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::RawPointer, Some(field)) => {
            let pointer = match raw_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_raw(state, pointer, field, *index, *field_count)
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_stack(state, pointer, field, *index, *field_count)
        }
        _ => access::field_addr(state, agg, *index, *field_count),
    };
    let value = match value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field addr on heap references.
pub(crate) fn step_field_addr_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = match heap_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::field_addr_heap(state, handle, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step field addr on raw pointers.
pub(crate) fn step_field_addr_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match raw_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::field_addr_raw(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step field addr on stack pointers.
pub(crate) fn step_field_addr_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match stack_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::field_addr_stack(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step field addr on global pointers.
pub(crate) fn step_field_addr_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match global_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::field_addr_global(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step field load.
pub(crate) fn step_field_load(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // load the selected field through the runtime storage class
    let value = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::SharedHeapReference, Some(field)) => {
            let handle = match shared_heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_shared_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::RawPointer, Some(field)) => {
            let pointer = match raw_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_raw(state, pointer, field, *index, *field_count)
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_stack(state, pointer, field, *index, *field_count)
        }
        _ => {
            let pointer = match access::field_addr(state, agg, *index, *field_count) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            let access = field.map(TypedAccess::from);

            access::load_from_pointer_with_access(state, pointer, access)
        }
    };
    let value = match value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field load on heap references.
pub(crate) fn step_field_load_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = match heap_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_field_heap(state, handle, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field load on raw pointers.
pub(crate) fn step_field_load_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match raw_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_field_raw(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field load on stack pointers.
pub(crate) fn step_field_load_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match stack_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_field_stack(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field load on global pointers.
pub(crate) fn step_field_load_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match global_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_field_global(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field set.
pub(crate) fn step_field_set(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldSet {
        dest,
        composite,
        index,
        value,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    let val = state.get(*value);

    // write field
    let aggregate_type = match state.value_type(*dest) {
        Ok(aggregate_type) => aggregate_type,
        Err(error) => return Transfer::Error(error),
    };

    let result = match access::set_field(
        state,
        agg,
        *index,
        val,
        aggregate_type,
        *field_count,
        *field,
    ) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store.
pub(crate) fn step_field_store(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    let val = state.get(*value);

    // store the selected field through the runtime storage class
    let result = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::heap_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_field_heap(state, handle, field, *index, *field_count, val)
        }
        (ValueTag::SharedHeapReference, Some(field)) => {
            let handle = match shared_heap_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::shared_heap_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_field_shared_heap(state, handle, field, *index, *field_count, val)
        }
        (ValueTag::RawPointer, Some(field)) => {
            let pointer = match raw_composite(agg) {
                Ok(pointer) => Value::raw_pointer_with_meta(pointer, *reference),
                Err(error) => return Transfer::Error(error),
            };

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_field_raw(
                state,
                match raw_composite(agg) {
                    Ok(pointer) => pointer,
                    Err(error) => return Transfer::Error(error),
                },
                field,
                *index,
                *field_count,
                val,
            )
        }
        _ => {
            let pointer = match access::field_addr(state, agg, *index, *field_count) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = pointer.with_reference_meta(*reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_to_pointer_with_access(state, pointer, field.map(TypedAccess::from), val)
        }
    };
    if let Err(error) = result {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on heap references.
pub(crate) fn step_field_store_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = match heap_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::heap_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) = access::store_field_heap(state, handle, field, *index, *field_count, val) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on raw pointers.
pub(crate) fn step_field_store_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let raw_pointer = match raw_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_field_raw(state, raw_pointer, field, *index, *field_count, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on stack pointers.
pub(crate) fn step_field_store_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let stack_pointer = match stack_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match stack_pointer_value_with_meta(stack_pointer, *reference) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_field_stack(state, stack_pointer, field, *index, *field_count, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on global pointers.
pub(crate) fn step_field_store_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let global_pointer = match global_composite(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match global_pointer_value_with_meta(
        global_pointer.id,
        global_pointer.byte_offset,
        *reference,
    ) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_field_global(state, global_pointer, field, *index, *field_count, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element get.
pub(crate) fn step_element_get(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementGet {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let value = match access::get_element(state, arr, idx_val, *array_length, *element) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element addr.
pub(crate) fn step_element_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let value = match (arr.tag(), *element) {
        (ValueTag::HeapReference, Some(element)) => {
            let handle = match heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::SharedHeapReference, Some(element)) => {
            let handle = match shared_heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_shared_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::RawPointer, Some(element)) => {
            let pointer = match raw_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_raw(state, pointer, element, idx_val, *array_length)
        }
        (ValueTag::StackPointer, Some(element)) => {
            let pointer = match stack_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_stack(state, pointer, element, idx_val, *array_length)
        }
        _ => access::element_addr(state, arr, idx_val, *array_length),
    };
    let value = match value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element addr on heap references.
pub(crate) fn step_element_addr_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let handle = match heap_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::element_addr_heap(state, handle, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element addr on raw pointers.
pub(crate) fn step_element_addr_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let pointer = match raw_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::element_addr_raw(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element addr on stack pointers.
pub(crate) fn step_element_addr_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let pointer = match stack_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::element_addr_stack(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element addr on global pointers.
pub(crate) fn step_element_addr_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let pointer = match global_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::element_addr_global(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element load.
pub(crate) fn step_element_load(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load the selected element through the runtime storage class
    let value = match (arr.tag(), *element) {
        (ValueTag::HeapReference, Some(element)) => {
            let handle = match heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::SharedHeapReference, Some(element)) => {
            let handle = match shared_heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_shared_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::RawPointer, Some(element)) => {
            let pointer = match raw_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_raw(state, pointer, element, idx_val, *array_length)
        }
        (ValueTag::StackPointer, Some(element)) => {
            let pointer = match stack_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_stack(state, pointer, element, idx_val, *array_length)
        }
        _ => {
            let pointer = match access::element_addr(state, arr, idx_val, *array_length) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            let access = element.map(TypedAccess::from);

            access::load_from_pointer_with_access(state, pointer, access)
        }
    };
    let value = match value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element load on heap references.
pub(crate) fn step_element_load_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let handle = match heap_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_element_heap(state, handle, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element load on raw pointers.
pub(crate) fn step_element_load_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let pointer = match raw_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_element_raw(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element load on stack pointers.
pub(crate) fn step_element_load_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let pointer = match stack_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_element_stack(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element load on global pointers.
pub(crate) fn step_element_load_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let pointer = match global_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_element_global(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element set.
pub(crate) fn step_element_set(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementSet {
        dest,
        array,
        index,
        value,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // write element
    let aggregate_type = match state.value_type(*dest) {
        Ok(aggregate_type) => aggregate_type,
        Err(error) => return Transfer::Error(error),
    };

    let result = match access::set_element(
        state,
        arr,
        idx_val,
        val,
        aggregate_type,
        *array_length,
        *element,
    ) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store.
pub(crate) fn step_element_store(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // store the selected element through the runtime storage class
    let result = match (arr.tag(), *element) {
        (ValueTag::HeapReference, Some(element)) => {
            let handle = match heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::heap_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_element_heap(state, handle, element, idx_val, *array_length, val)
        }
        (ValueTag::SharedHeapReference, Some(element)) => {
            let handle = match shared_heap_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::shared_heap_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_element_shared_heap(state, handle, element, idx_val, *array_length, val)
        }
        (ValueTag::RawPointer, Some(element)) => {
            let pointer = match raw_composite(arr) {
                Ok(pointer) => Value::raw_pointer_with_meta(pointer, *reference),
                Err(error) => return Transfer::Error(error),
            };

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_element_raw(
                state,
                match raw_composite(arr) {
                    Ok(pointer) => pointer,
                    Err(error) => return Transfer::Error(error),
                },
                element,
                idx_val,
                *array_length,
                val,
            )
        }
        _ => {
            let pointer = match access::element_addr(state, arr, idx_val, *array_length) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = pointer.with_reference_meta(*reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_to_pointer_with_access(
                state,
                pointer,
                element.map(TypedAccess::from),
                val,
            )
        }
    };
    if let Err(error) = result {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store on heap references.
pub(crate) fn step_element_store_heap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    if arr.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference semantics
    let handle = match heap_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::heap_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_element_heap(state, handle, element, idx_val, *array_length, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store on raw pointers.
pub(crate) fn step_element_store_raw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference semantics
    let raw_pointer = match raw_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_element_raw(state, raw_pointer, element, idx_val, *array_length, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store on stack pointers.
pub(crate) fn step_element_store_stack(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference semantics
    let stack_pointer = match stack_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match stack_pointer_value_with_meta(stack_pointer, *reference) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_element_stack(state, stack_pointer, element, idx_val, *array_length, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store on global pointers.
pub(crate) fn step_element_store_global(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference semantics
    let global_pointer = match global_composite(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match global_pointer_value_with_meta(
        global_pointer.id,
        global_pointer.byte_offset,
        *reference,
    ) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    if let Err(error) =
        access::store_element_global(state, global_pointer, element, idx_val, *array_length, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step composite construction.
pub(crate) fn step_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Composite { dest, elements } = &block[pc].immediate else {
        unreachable!()
    };

    // materialize the composite with the destination storage policy
    let element_slice = state.argument_slice(*elements).to_vec();
    let result = match super::value::materialize_composite_by_index(
        state,
        *dest,
        |state, index, _value_type| {
            let element = element_slice
                .get(index as usize)
                .copied()
                .ok_or(Error::InvalidInstruction)?;

            Ok(state.get(element))
        },
    ) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
