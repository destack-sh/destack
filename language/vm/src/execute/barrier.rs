use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{BarrierWrite, Instruction, PointerClass, Transfer};

/// Execute managed barrier write.
pub(crate) fn execute_barrier_write(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let BarrierWrite {
        object,
        offset,
        byte_len,
        pointer_class,
    } = instruction.payload_as::<BarrierWrite>();

    // load barrier range
    let object = state.get(*object);
    let offset = state.get(*offset).as_uint() as usize;
    let byte_len = state.get(*byte_len).as_uint() as usize;

    // publish to the owning collector
    let result = match pointer_class {
        PointerClass::Heap => {
            state
                .heap_mut()
                .write_barrier(object.as_heap_reference(), offset, byte_len)
        }
        PointerClass::SharedHeap => {
            state
                .shared()
                .write_barrier(object.as_shared_heap_reference(), offset, byte_len)
        }
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "managed barrier reference".to_string(),
                actual: format!("{pointer_class:?}"),
            });
        }
    };

    // report invalid heap ranges
    if let Err(error) = result {
        return Transfer::Error(Error::from(error));
    }

    Transfer::Continue
}
