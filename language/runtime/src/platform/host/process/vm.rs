use destack_vm::{ExternalContext, RawPointer, Value};

use crate::binding;
use crate::binding_set;
use crate::platform::bindings::BindingDescriptor;
use crate::platform::host::HostResult;

use super::core::process_args;

binding_set!(
    pub PROCESS_VM_BINDINGS,
    "process",
    |registry, isolate, host| {
        // capture host for the external handler
        let host = host.clone();
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.process.args"),
            move |context, _| build_process_args(context, process_args(&host))
        );
    }
);

/// Build a process args array value for the VM.
fn build_process_args(
    context: &mut ExternalContext<'_>,
    args: &[String],
) -> HostResult<Value> {
    // collect argument values
    let length = args.len() as u32;
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(context.intern_string(arg));
    }

    // allocate the payload buffer
    let data_ptr = if values.is_empty() {
        RawPointer::NULL
    } else {
        context.allocate_raw_values(values)
    };

    // build the array aggregate
    let length_value = Value::uint32(length);
    let capacity_value = Value::uint32(length);
    let data_value = Value::raw_pointer(data_ptr);
    let array_value = context.allocate_aggregate(vec![length_value, capacity_value, data_value]);

    Ok(array_value)
}
