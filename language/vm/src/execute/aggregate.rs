use super::prelude::*;
use crate::diagnostic::Error;
use crate::module::TypedAccess;

/// Apply reference metadata and validate the resulting pointer value.
#[inline(always)]
fn build_reference_result(
    state: &mut ExecutionState<'_, '_>,
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
fn load_array_index(state: &ExecutionState<'_, '_>, index: mir::Value) -> Result<u64, Error> {
    let value = state.get(index);

    value.as_uint().ok_or_else(|| Error::TypeMismatch {
        expected: "unsigned integer".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Build one aggregate type mismatch.
#[inline(always)]
fn invalid_aggregate(value: Value) -> Error {
    Error::TypeMismatch {
        expected: "aggregate".to_string(),
        actual: format!("{value:?}"),
    }
}

/// Require one heap aggregate value.
#[inline(always)]
fn heap_aggregate(value: Value) -> Result<HeapReference, Error> {
    value
        .as_heap_reference()
        .ok_or_else(|| invalid_aggregate(value))
}

/// Require one shared heap aggregate value.
#[inline(always)]
fn shared_heap_aggregate(value: Value) -> Result<destack_heap::SharedHeapReference, Error> {
    value
        .as_shared_heap_reference()
        .ok_or_else(|| invalid_aggregate(value))
}

/// Require one raw aggregate pointer.
#[inline(always)]
fn raw_aggregate(value: Value) -> Result<RawPointer, Error> {
    value
        .as_raw_pointer()
        .ok_or_else(|| invalid_aggregate(value))
}

/// Require one stack aggregate pointer.
#[inline(always)]
fn stack_aggregate(value: Value) -> Result<StackPointer, Error> {
    value
        .as_stack_pointer()
        .ok_or_else(|| invalid_aggregate(value))
}

/// Require one static aggregate pointer.
#[inline(always)]
fn static_aggregate(value: Value) -> Result<StaticPointer, Error> {
    value
        .as_static_pointer()
        .ok_or_else(|| invalid_aggregate(value))
}

/// Execute field get.
pub(crate) fn execute_field_get(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldGet {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

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

/// Execute field addr.
pub(crate) fn execute_field_addr(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // compute field address
    let value = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_aggregate(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::SharedHeapReference, Some(field)) => {
            let handle = match shared_heap_aggregate(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_shared_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::RawPointer, Some(field)) => {
            let pointer = match raw_aggregate(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_raw(state, pointer, field, *index, *field_count)
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_aggregate(agg) {
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

/// Execute field addr on heap references.
pub(crate) fn execute_field_addr_heap(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = match heap_aggregate(agg) {
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

/// Execute field addr on raw pointers.
pub(crate) fn execute_field_addr_raw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match raw_aggregate(agg) {
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

/// Execute field addr on stack pointers.
pub(crate) fn execute_field_addr_stack(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match stack_aggregate(agg) {
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

/// Execute field addr on static pointers.
pub(crate) fn execute_field_addr_static(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StaticPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = match static_aggregate(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::field_addr_static(state, pointer, field, *index, *field_count) {
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

/// Execute field load.
pub(crate) fn execute_field_load(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // load the selected field through the pointer class
    let value = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_aggregate(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::SharedHeapReference, Some(field)) => {
            let handle = match shared_heap_aggregate(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_shared_heap(state, handle, field, *index, *field_count)
        }
        (ValueTag::RawPointer, Some(field)) => {
            let pointer = match raw_aggregate(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_raw(state, pointer, field, *index, *field_count)
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_aggregate(agg) {
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

/// Execute field load on heap references.
pub(crate) fn execute_field_load_heap(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = match heap_aggregate(agg) {
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

/// Execute field load on raw pointers.
pub(crate) fn execute_field_load_raw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match raw_aggregate(agg) {
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

/// Execute field load on stack pointers.
pub(crate) fn execute_field_load_stack(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match stack_aggregate(agg) {
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

/// Execute field load on static pointers.
pub(crate) fn execute_field_load_static(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StaticPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = match static_aggregate(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_field_static(state, pointer, field, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute field set.
pub(crate) fn execute_field_set(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldSet {
        dest,
        aggregate,
        index,
        value,
        field_count,
        field,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // load values
    let agg = state.get(*aggregate);
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

/// Execute field store.
pub(crate) fn execute_field_store(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        aggregate,
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
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // store the selected field through the pointer class
    let result = match (agg.tag(), *field) {
        (ValueTag::HeapReference, Some(field)) => {
            let handle = match heap_aggregate(agg) {
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
            let handle = match shared_heap_aggregate(agg) {
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
            let pointer = match raw_aggregate(agg) {
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
                match raw_aggregate(agg) {
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

/// Execute field store on heap references.
pub(crate) fn execute_field_store_heap(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        aggregate,
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
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::HeapReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = match heap_aggregate(agg) {
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

/// Execute field store on raw pointers.
pub(crate) fn execute_field_store_raw(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        aggregate,
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
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let raw_pointer = match raw_aggregate(agg) {
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

/// Execute field store on stack pointers.
pub(crate) fn execute_field_store_stack(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        aggregate,
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
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let stack_pointer = match stack_aggregate(agg) {
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

/// Execute field store on static pointers.
pub(crate) fn execute_field_store_static(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::FieldStore {
        aggregate,
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
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StaticPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let static_pointer = match static_aggregate(agg) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match static_pointer_value_with_meta(
        static_pointer.id,
        static_pointer.byte_offset,
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
        access::store_field_static(state, static_pointer, field, *index, *field_count, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute element get.
pub(crate) fn execute_element_get(
    state: &mut ExecutionState<'_, '_>,
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

/// Execute element addr.
pub(crate) fn execute_element_addr(
    state: &mut ExecutionState<'_, '_>,
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
            let handle = match heap_aggregate(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::SharedHeapReference, Some(element)) => {
            let handle = match shared_heap_aggregate(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_shared_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::RawPointer, Some(element)) => {
            let pointer = match raw_aggregate(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_raw(state, pointer, element, idx_val, *array_length)
        }
        (ValueTag::StackPointer, Some(element)) => {
            let pointer = match stack_aggregate(arr) {
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

/// Execute element addr on heap references.
pub(crate) fn execute_element_addr_heap(
    state: &mut ExecutionState<'_, '_>,
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
    let handle = match heap_aggregate(arr) {
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

/// Execute element addr on raw pointers.
pub(crate) fn execute_element_addr_raw(
    state: &mut ExecutionState<'_, '_>,
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
    let pointer = match raw_aggregate(arr) {
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

/// Execute element addr on stack pointers.
pub(crate) fn execute_element_addr_stack(
    state: &mut ExecutionState<'_, '_>,
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
    let pointer = match stack_aggregate(arr) {
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

/// Execute element addr on static pointers.
pub(crate) fn execute_element_addr_static(
    state: &mut ExecutionState<'_, '_>,
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
    if arr.tag() != ValueTag::StaticPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let pointer = match static_aggregate(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::element_addr_static(state, pointer, element, idx_val, *array_length) {
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

/// Execute element load.
pub(crate) fn execute_element_load(
    state: &mut ExecutionState<'_, '_>,
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

    // load the selected element through the pointer class
    let value = match (arr.tag(), *element) {
        (ValueTag::HeapReference, Some(element)) => {
            let handle = match heap_aggregate(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::SharedHeapReference, Some(element)) => {
            let handle = match shared_heap_aggregate(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_shared_heap(state, handle, element, idx_val, *array_length)
        }
        (ValueTag::RawPointer, Some(element)) => {
            let pointer = match raw_aggregate(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_raw(state, pointer, element, idx_val, *array_length)
        }
        (ValueTag::StackPointer, Some(element)) => {
            let pointer = match stack_aggregate(arr) {
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

/// Execute element load on heap references.
pub(crate) fn execute_element_load_heap(
    state: &mut ExecutionState<'_, '_>,
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
    let handle = match heap_aggregate(arr) {
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

/// Execute element load on raw pointers.
pub(crate) fn execute_element_load_raw(
    state: &mut ExecutionState<'_, '_>,
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
    let pointer = match raw_aggregate(arr) {
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

/// Execute element load on stack pointers.
pub(crate) fn execute_element_load_stack(
    state: &mut ExecutionState<'_, '_>,
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
    let pointer = match stack_aggregate(arr) {
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

/// Execute element load on static pointers.
pub(crate) fn execute_element_load_static(
    state: &mut ExecutionState<'_, '_>,
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
    if arr.tag() != ValueTag::StaticPointer {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let pointer = match static_aggregate(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidHeapReference);
    };
    let value = match access::load_element_static(state, pointer, element, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute element set.
pub(crate) fn execute_element_set(
    state: &mut ExecutionState<'_, '_>,
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

/// Execute element store.
pub(crate) fn execute_element_store(
    state: &mut ExecutionState<'_, '_>,
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

    // store the selected element through the pointer class
    let result = match (arr.tag(), *element) {
        (ValueTag::HeapReference, Some(element)) => {
            let handle = match heap_aggregate(arr) {
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
            let handle = match shared_heap_aggregate(arr) {
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
            let pointer = match raw_aggregate(arr) {
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
                match raw_aggregate(arr) {
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

/// Execute element store on heap references.
pub(crate) fn execute_element_store_heap(
    state: &mut ExecutionState<'_, '_>,
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
    let handle = match heap_aggregate(arr) {
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

/// Execute element store on raw pointers.
pub(crate) fn execute_element_store_raw(
    state: &mut ExecutionState<'_, '_>,
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
    let raw_pointer = match raw_aggregate(arr) {
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

/// Execute element store on stack pointers.
pub(crate) fn execute_element_store_stack(
    state: &mut ExecutionState<'_, '_>,
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
    let stack_pointer = match stack_aggregate(arr) {
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

/// Execute element store on static pointers.
pub(crate) fn execute_element_store_static(
    state: &mut ExecutionState<'_, '_>,
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
    if arr.tag() != ValueTag::StaticPointer {
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
    let static_pointer = match static_aggregate(arr) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = match static_pointer_value_with_meta(
        static_pointer.id,
        static_pointer.byte_offset,
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
        access::store_element_static(state, static_pointer, element, idx_val, *array_length, val)
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute aggregate construction.
pub(crate) fn execute_aggregate(
    state: &mut ExecutionState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Aggregate { dest, elements } = &block[pc].immediate else {
        unreachable!()
    };

    // allocate the aggregate payload with the destination type
    let element_slice = state.argument_slice(*elements).to_vec();
    let result =
        match super::value::allocate_payload_by_index(state, *dest, |state, index, _value_type| {
            let element = element_slice
                .get(index as usize)
                .copied()
                .ok_or(Error::InvalidInstruction)?;

            Ok(state.get(element))
        }) {
            Ok(result) => result,
            Err(error) => return Transfer::Error(error),
        };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
