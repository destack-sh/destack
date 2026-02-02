use crate::diagnostic::RuntimeError;
use crate::platform::time::bindings_generated as bindings;
use crate::platform::{PlatformError, RuntimeStatus};
use crate::runtime::with_runtime_call_context;

/// Return wall clock time in nanoseconds for native code.
#[unsafe(export_name = "destack.time.wallNs")]
pub unsafe extern "C" fn destack_time_wall_ns(out: *mut u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::WALL_NS)?;

            if out.is_null() {
                return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
            }

            let value = context.runtime().time.wall_nanos();
            unsafe {
                *out = value;
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

/// Return monotonic time in nanoseconds for native code.
#[unsafe(export_name = "destack.time.monoNs")]
pub unsafe extern "C" fn destack_time_mono_ns(out: *mut u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::MONO_NS)?;

            if out.is_null() {
                return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
            }

            let value = context.runtime().time.mono_nanos();
            unsafe {
                *out = value;
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

/// Sleep for the given duration for native code.
#[unsafe(export_name = "destack.time.sleepNs")]
pub unsafe extern "C" fn destack_time_sleep_ns(duration_nanos: u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::SLEEP_NS)?;
            context.runtime().time.sleep_nanos(duration_nanos);
            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}
