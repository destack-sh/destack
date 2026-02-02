use crate::diagnostic::RuntimeError;
use crate::platform::process::bindings_generated as bindings;
use crate::platform::{PlatformError, PlatformStringSlice, RuntimeStatus};
use crate::runtime::with_runtime_call_context;

/// Return the process args for native code.
#[unsafe(export_name = "destack.process.args")]
pub unsafe extern "C" fn destack_process_args(out: *mut PlatformStringSlice) -> RuntimeStatus {
    // resolve policy and platform context
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::ARGS)?;

            // reject null output pointers
            if out.is_null() {
                return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
            }

            // write the output slice
            let slice = PlatformStringSlice::from_slice(context.platform().args_refs());
            unsafe {
                *out = slice;
            }

            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}
