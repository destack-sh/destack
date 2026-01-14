use destack_vm::Error;

use crate::native_binding_set;
use crate::platform::bindings::{BindingDescriptor, NativeBinding};
use crate::platform::host::{HostStatus, HostStringSlice, with_host_call_context};

native_binding_set!(
    pub PROCESS_NATIVE_BINDINGS,
    "process",
    [NativeBinding::new(
        BindingDescriptor::external_recordable("destack.process.args"),
        "destack.process.args"
    )]
);

/// Return the process args for native code.
#[unsafe(export_name = "destack.process.args")]
pub unsafe extern "C" fn destack_process_args(out: *mut HostStringSlice) -> HostStatus {
    // resolve policy and host context
    let result = with_host_call_context(|context| {
        context.check_policy(BindingDescriptor::external_recordable(
            "destack.process.args",
        ))?;

        // reject null output pointers
        if out.is_null() {
            return Err(Error::NullPointerDereference);
        }

        // write the output slice
        let slice = HostStringSlice::from_slice(context.host().args_refs());
        unsafe {
            *out = slice;
        }

        Ok(())
    });

    HostStatus::from_result(result)
}
