use super::prelude::*;

/// Build an error for one pointer that cannot fit in the packed value format.
fn pointer_encoding_error(kind: &str) -> Error {
    Error::InvariantViolation {
        context: format!("{kind} pointer exceeds packed value representation"),
    }
}

/// Build one stack pointer value.
#[inline]
pub(crate) fn stack_pointer_value(pointer: StackPointer) -> Result<Value, Error> {
    Value::stack_pointer(pointer).ok_or_else(|| pointer_encoding_error("stack"))
}

/// Build one stack pointer value with reference metadata.
#[inline]
pub(crate) fn stack_pointer_value_with_meta(
    pointer: StackPointer,
    meta: ReferenceMeta,
) -> Result<Value, Error> {
    Value::stack_pointer_with_meta(pointer, meta).ok_or_else(|| pointer_encoding_error("stack"))
}

/// Build one frame pointer value.
#[inline]
pub(crate) fn frame_pointer_value(pointer: FramePointer) -> Result<Value, Error> {
    Value::frame_pointer(pointer).ok_or_else(|| pointer_encoding_error("frame"))
}

/// Build one frame pointer value with reference metadata.
#[inline]
pub(crate) fn frame_pointer_value_with_meta(
    pointer: FramePointer,
    meta: ReferenceMeta,
) -> Result<Value, Error> {
    Value::frame_pointer_with_meta(pointer, meta).ok_or_else(|| pointer_encoding_error("frame"))
}

/// Build one global pointer value.
#[inline]
pub(crate) fn global_pointer_value(
    pointer: mir::LocalNodeId<mir::Global>,
    byte_offset: usize,
) -> Result<Value, Error> {
    Value::global_pointer_with_offset(pointer, byte_offset)
        .ok_or_else(|| pointer_encoding_error("global"))
}

/// Build one global pointer value with reference metadata.
#[inline]
pub(crate) fn global_pointer_value_with_meta(
    pointer: mir::LocalNodeId<mir::Global>,
    byte_offset: usize,
    meta: ReferenceMeta,
) -> Result<Value, Error> {
    Value::global_pointer_with_meta(pointer, byte_offset, meta)
        .ok_or_else(|| pointer_encoding_error("global"))
}

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

/// Materialize one composite result for the destination slot type by value index.
pub(crate) fn materialize_composite_by_index<F>(
    state: &mut StepState<'_, '_>,
    destination: mir::Value,
    mut value_at_index: F,
) -> Result<Value, Error>
where
    F: FnMut(&mut StepState<'_, '_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Value, Error>,
{
    let composite_type = state.value_type(destination)?;
    let repr_composite_type = crate::module::repr_type(state.tree(), composite_type);

    // callable stays boxed in the vm so nested payload only carries one heap reference
    if matches!(
        state.tree().get(repr_composite_type),
        mir::Type::Callable { .. }
    ) {
        let function = value_at_index(state, 0, composite_type)?;
        let environment = value_at_index(state, 1, composite_type)?;

        return access::allocate_callable(state, composite_type, function, environment);
    }

    access::allocate_heap_value_by_index(state, composite_type, value_at_index)
}
