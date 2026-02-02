use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

use crate::platform::process::core::process_args;

/// Return the process arguments as a VM slice.
pub fn destack_process_args(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    build_process_args(context, process_args(runtime.platform()))
}

/// Build a process args slice value for the VM.
fn build_process_args(
    context: &mut vm::RuntimeContext<'_>,
    args: &[String],
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    // collect argument values
    let length = args.len() as u32;
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(context.intern_string(arg));
    }

    // allocate the payload buffer
    let data_ptr = if values.is_empty() {
        vm::RawPointer::NULL
    } else {
        context.allocate_raw_values(values)
    };

    // build the slice representation
    Ok(VmSlice {
        data: data_ptr,
        len: length,
        _marker: std::marker::PhantomData,
    })
}
