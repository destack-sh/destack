use super::prelude::*;

/// Build one stack pointer value.
#[inline]
pub(crate) fn stack_pointer_value(pointer: StackPointer) -> Result<Word, Error> {
    Ok(Word::stack_pointer(pointer))
}

/// Build one frame pointer value.
#[inline]
pub(crate) fn frame_pointer_value(pointer: FramePointer) -> Result<Word, Error> {
    Ok(Word::frame_pointer(pointer))
}

/// Build one static pointer value.
#[inline]
pub(crate) fn static_pointer_value(
    pointer: StaticPointer,
    _byte_offset: usize,
) -> Result<Word, Error> {
    Ok(Word::static_pointer(pointer))
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
    state: &mut DispatchState<'_, '_>,
    arguments: ArgumentRange,
) -> SmallVec<[Word; 16]> {
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
pub(crate) fn value_to_u64(value: Word) -> Result<u64, Error> {
    let raw = value.bits() as i64;
    if raw < 0 {
        return Err(Error::TypeMismatch {
            expected: "non-negative integer".to_string(),
            actual: format!("{value:?}"),
        });
    }

    Ok(raw as u64)
}

/// Map a runtime value to a usize index.
pub(crate) fn value_to_usize(value: Word) -> Result<usize, Error> {
    // convert to u64 first
    let index = value_to_u64(value)?;

    usize::try_from(index).map_err(|_| Error::TypeMismatch {
        expected: "usize index".to_string(),
        actual: format!("{value:?}"),
    })
}
