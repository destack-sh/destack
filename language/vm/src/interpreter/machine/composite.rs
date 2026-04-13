use super::prelude::*;
use crate::diagnostic::Error;
use crate::executable::TypedAccess;
use crate::telemetry::stat_inc;

/// Apply reference metadata and validate the resulting pointer value.
#[inline(always)]
fn build_reference_result(
    state: &mut StepState<'_, '_>,
    reference: destack_heap::ReferenceMeta,
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

/// Require one managed composite value.
#[inline(always)]
fn managed_composite(value: Value) -> Result<ManagedReference, Error> {
    value
        .as_managed_reference()
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
    // decode instruction data
    let InstructionData::FieldGet {
        dest,
        composite,
        index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // load field value
    let value = match access::get_field(state, agg, *index) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field get for small inline aggregates (≤2 fields).
#[inline(always)]
pub(crate) fn step_field_get_inline(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldGet {
        dest,
        composite,
        index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // resolve the field through the generic composite path
    let value = match access::get_field(state, agg, *index) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on small managed composites (≤2 fields, inline storage).
#[inline(always)]
pub(crate) fn step_field_store_inline(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference: _,
        field_count: _,
        field: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    if state.collect_stats {
        stat_inc!(state.engine.statistics, stores);
    }

    // load composite
    let agg = state.get(*composite);

    // load value to store
    let val = state.get(*value);

    // store through the generic composite path
    if let Err(error) = access::set_field(state, agg, *index, val) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field addr.
pub(crate) fn step_field_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // compute field address
    let value = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::field_addr_managed(state, handle, field, *index, *field_count)
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

/// Step field addr on composite values.
pub(crate) fn step_field_addr_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    let value = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            match access::field_addr_managed(state, handle, field, *index, *field_count) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            match access::field_addr_stack(state, pointer, field, *index, *field_count) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "composite".to_string(),
                actual: format!("{agg:?}"),
            });
        }
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

/// Step field addr on managed references.
pub(crate) fn step_field_addr_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::ManagedReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = match managed_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::field_addr_managed(state, handle, field, *index, *field_count) {
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
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldAddr {
        dest,
        composite,
        index,
        reference,
        field_count,
        field: _,
    } = &block[pc].data
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
    let value = match access::field_addr_global(state, pointer, *index, *field_count) {
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
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);

    // load the selected field through the runtime storage class
    let value = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_field_managed(state, handle, field, *index, *field_count)
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

/// Step field load on composite values.
pub(crate) fn step_field_load_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    let value = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            match access::load_field_managed(state, handle, field, *index, *field_count) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_composite(agg) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            match access::load_field_stack(state, pointer, field, *index, *field_count) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "composite".to_string(),
                actual: format!("{agg:?}"),
            });
        }
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field load on managed references.
pub(crate) fn step_field_load_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load composite
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::ManagedReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = match managed_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::load_field_managed(state, handle, field, *index, *field_count) {
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
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldLoad {
        dest,
        composite,
        index,
        field_count,
        field: _,
    } = &block[pc].data
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
    let value = match access::load_field_global(state, pointer, *index, *field_count) {
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
    // decode instruction data
    let InstructionData::FieldSet {
        dest,
        composite,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*composite);
    let val = state.get(*value);

    // write field
    let result = match access::set_field(state, agg, *index, val) {
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
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*composite);
    let val = state.get(*value);

    // store the selected field through the runtime storage class
    let result = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::managed_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_field_managed(state, handle, field, *index, *field_count, val)
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

/// Step field store on composite values.
pub(crate) fn step_field_store_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*composite);
    let val = state.get(*value);

    let result = match (agg.tag(), *field) {
        (ValueTag::ManagedReference, Some(field)) => {
            let handle = match managed_composite(agg) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::managed_reference_with_meta(handle, *reference);
            if let Err(error) = check_reference_kind(state, *reference, pointer) {
                return Transfer::Error(error);
            }

            access::store_field_managed(state, handle, field, *index, *field_count, val)
        }
        (ValueTag::StackPointer, Some(field)) => {
            let pointer = match stack_composite(agg) {
                Ok(pointer) => Value::stack_pointer_with_meta(pointer, *reference),
                Err(error) => return Transfer::Error(error),
            };
            if let Err(error) = check_reference_kind(state, *reference, pointer) {
                return Transfer::Error(error);
            }

            access::store_field_stack(
                state,
                match stack_composite(agg) {
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
            return Transfer::Error(Error::TypeMismatch {
                expected: "composite".to_string(),
                actual: format!("{agg:?}"),
            });
        }
    };
    if let Err(error) = result {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step field store on managed references.
pub(crate) fn step_field_store_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*composite);
    if agg.tag() != ValueTag::ManagedReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = match managed_composite(agg) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    if let Err(error) = access::store_field_managed(state, handle, field, *index, *field_count, val)
    {
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
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(field) = *field else {
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::FieldStore {
        composite,
        index,
        value,
        reference,
        field_count,
        field: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    if let Err(error) = access::store_field_global(state, global_pointer, *index, *field_count, val)
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
    // decode instruction data
    let InstructionData::ElementGet { dest, array, index } = &block[pc].data else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let value = match access::get_element(state, arr, idx_val) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step index select.
pub(crate) fn step_index_select(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::IndexSelect {
        dest,
        index,
        elements,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load index and cases
    let index_value = match load_array_index(state, *index) {
        Ok(index_value) => index_value,
        Err(error) => return Transfer::Error(error),
    };
    let element_slice = state.argument_slice(*elements);
    let selected = match usize::try_from(index_value) {
        Ok(index) => element_slice.get(index).copied(),
        Err(_) => None,
    };
    let Some(selected) = selected else {
        return Transfer::Error(Error::InvalidArrayAccess {
            index: index_value,
            length: element_slice.len() as u64,
        });
    };

    // store result
    state.set(*dest, state.get(selected));

    // continue to next instruction
    next!(state, block, pc)
}

/// Step select by index.
pub(crate) fn step_select_by_index(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::SelectByIndex {
        dest,
        index,
        match_index,
        then_value,
        else_value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load index and choose the source value
    let index_value = match load_array_index(state, *index) {
        Ok(index_value) => index_value,
        Err(error) => return Transfer::Error(error),
    };
    let selected = if index_value == *match_index {
        *then_value
    } else {
        *else_value
    };

    // store result
    state.set(*dest, state.get(selected));

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element addr.
pub(crate) fn step_element_addr(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].data
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
        (ValueTag::ManagedReference, Some(element)) => {
            let handle = match managed_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::element_addr_managed(state, handle, element, idx_val, *array_length)
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

/// Step element addr on composite values.
pub(crate) fn step_element_addr_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if !matches!(
        arr.tag(),
        ValueTag::ManagedReference | ValueTag::StackPointer
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match arr.tag() {
        ValueTag::ManagedReference => {
            let handle = match managed_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            match access::element_addr_managed(state, handle, element, idx_val, *array_length) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        ValueTag::StackPointer => {
            let pointer = match stack_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
            match access::element_addr_stack(state, pointer, element, idx_val, *array_length) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            }
        }
        _ => unreachable!(),
    };

    let value = match build_reference_result(state, *reference, value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set(*dest, value);
    next!(state, block, pc)
}

/// Step element addr on managed references.
pub(crate) fn step_element_addr_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // compute element address
    let handle = match managed_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::element_addr_managed(state, handle, element, idx_val, *array_length) {
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
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
        element: _,
    } = &block[pc].data
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
    let value = match access::element_addr_global(state, pointer, idx_val, *array_length) {
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
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].data
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
        (ValueTag::ManagedReference, Some(element)) => {
            let handle = match managed_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            access::load_element_managed(state, handle, element, idx_val, *array_length)
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

/// Step element load on composite values.
pub(crate) fn step_element_load_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length: _,
        element: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if !matches!(
        arr.tag(),
        ValueTag::ManagedReference | ValueTag::StackPointer
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let value = match access::get_element(state, arr, idx_val) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element load on managed references.
pub(crate) fn step_element_load_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return Transfer::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // load element value
    let handle = match managed_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    let value = match access::load_element_managed(state, handle, element, idx_val, *array_length) {
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
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element,
    } = &block[pc].data
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
        element: _,
    } = &block[pc].data
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
    let value = match access::load_element_global(state, pointer, idx_val, *array_length) {
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
    // decode instruction data
    let InstructionData::ElementSet {
        dest,
        array,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // write element
    let result = match access::set_element(state, arr, idx_val, val) {
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
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // store the selected element through the runtime storage class
    let result = match (arr.tag(), *element) {
        (ValueTag::ManagedReference, Some(element)) => {
            let handle = match managed_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            };
            let pointer = Value::managed_reference_with_meta(handle, *reference);

            let validate = (|| -> Result<(), Error> {
                check_reference_kind(state, *reference, pointer)?;
                check_reference_mutability(state, *reference)?;
                Ok(())
            })();
            if let Err(error) = validate {
                return Transfer::Error(error);
            }

            access::store_element_managed(state, handle, element, idx_val, *array_length, val)
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

/// Step element store on composite values.
pub(crate) fn step_element_store_composite(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length: _,
        element: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if !matches!(
        arr.tag(),
        ValueTag::ManagedReference | ValueTag::StackPointer
    ) {
        return Transfer::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let val = state.get(*value);
    let idx_val = match load_array_index(state, *index) {
        Ok(idx_val) => idx_val,
        Err(error) => return Transfer::Error(error),
    };

    // validate reference semantics
    let pointer = match arr.tag() {
        ValueTag::ManagedReference => Value::managed_reference_with_meta(
            match managed_composite(arr) {
                Ok(handle) => handle,
                Err(error) => return Transfer::Error(error),
            },
            *reference,
        ),
        ValueTag::StackPointer => Value::stack_pointer_with_meta(
            match stack_composite(arr) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            },
            *reference,
        ),
        _ => unreachable!(),
    };
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    if let Err(error) = access::set_element(state, arr, idx_val, val) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Step element store on managed references.
pub(crate) fn step_element_store_managed(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
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
    let handle = match managed_composite(arr) {
        Ok(handle) => handle,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidManagedReference);
    };
    if let Err(error) =
        access::store_element_managed(state, handle, element, idx_val, *array_length, val)
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
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    let Some(element) = *element else {
        return Transfer::Error(Error::InvalidManagedReference);
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
    // decode instruction data
    let InstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
        element: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
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
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    // store value
    if let Err(error) =
        access::store_element_global(state, global_pointer, idx_val, *array_length, val)
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
    // decode instruction data
    let InstructionData::Composite { dest, elements } = &block[pc].data else {
        unreachable!()
    };

    // materialize the composite with the destination storage policy
    let element_slice = state.argument_slice(*elements).to_vec();
    let result = match super::value::materialize_composite_by_index(
        state,
        *dest,
        |state, index, _component_type| {
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
