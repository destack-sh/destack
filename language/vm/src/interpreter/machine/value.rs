use super::prelude::*;
use destack_heap::PackedValues;

/// Build a global id from a raw value.
#[inline]
pub(crate) fn global_id(raw: u32) -> mir::LocalNodeId<mir::Global> {
    mir::LocalNodeId::new(raw)
}

/// Build a type id from a raw value.
#[inline]
pub(crate) fn type_id(raw: u32) -> mir::LocalNodeId<mir::Type> {
    mir::LocalNodeId::new(raw)
}

/// Collect argument values into a smallvec.
#[inline]
pub(crate) fn collect_values(
    state: &mut StepState<'_, '_>,
    arguments: ArgumentRange,
) -> SmallVec<[Value; 16]> {
    // load argument slice
    let argument_slice = state.argument_slice(arguments);
    let mut args = SmallVec::with_capacity(argument_slice.len());

    // resolve argument values
    for arg in argument_slice {
        let value = state.get(*arg);
        args.push(value);
    }

    // return argument values
    args
}

/// Map a runtime value to an unsigned index.
pub(crate) fn value_to_u64(value: Value) -> Result<u64, Error> {
    // decode integer values
    match value.tag() {
        ValueTag::Int => {
            let raw = value.raw_data() as i64;
            if raw < 0 {
                return Err(Error::TypeMismatch {
                    expected: "non-negative integer".to_string(),
                    actual: format!("{value:?}"),
                });
            }

            Ok(raw as u64)
        }
        ValueTag::UInt => Ok(value.raw_data()),
        _ => Err(Error::TypeMismatch {
            expected: "integer".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Map a runtime value to a usize index.
pub(crate) fn value_to_usize(value: Value) -> Result<usize, Error> {
    // convert to u64 first
    let index = value_to_u64(value)?;

    usize::try_from(index).map_err(|_| Error::TypeMismatch {
        expected: "usize index".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Load aggregate slots for an aggregate value.
pub(crate) fn aggregate_slots<'a>(
    state: &'a StepState<'_, '_>,
    value: Value,
) -> Result<PackedValues<'a>, Error> {
    // require aggregate payload
    let handle = value
        .as_managed_reference()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{value:?}"),
        })?;

    // validate the managed allocation and decode packed values
    let heap = state.heap_ref();
    if !heap.is_managed_allocated(handle) {
        return Err(Error::InvalidManagedReference);
    }

    heap.packed_values(handle)
        .ok_or(Error::InvalidManagedReference)
}

/// Load aggregate slots and convert them into one owned slot list.
pub(crate) fn aggregate_slots_vec(
    state: &StepState<'_, '_>,
    value: Value,
) -> Result<Vec<Value>, Error> {
    // decode aggregate slots once
    let slots = aggregate_slots(state, value)?;

    Ok(slots.into_vec())
}
