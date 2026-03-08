#[cfg(feature = "affinity")]
use crate::diagnostic::RuntimeResult;
#[cfg(feature = "affinity")]
use crate::host::core::observer::process_runtime_observers;

/// CoreFoundation string reference type.
type CFStringRef = *const libc::c_void;

/// CoreFoundation run-loop handled-source status code.
const KCF_RUN_LOOP_RUN_HANDLED_SOURCE: i32 = 4;
/// CoreFoundation run-loop timeout status code.
const KCF_RUN_LOOP_RUN_TIMED_OUT: i32 = 3;
/// CoreFoundation run-loop finished status code.
const KCF_RUN_LOOP_RUN_FINISHED: i32 = 1;
/// CoreFoundation run-loop stopped status code.
const KCF_RUN_LOOP_RUN_STOPPED: i32 = 2;
/// Slice duration for bounded Apple run-loop servicing.
#[cfg(feature = "affinity")]
#[cfg_attr(feature = "affinity", allow(dead_code))]
const APPLE_THREAD_MESSAGE_WAIT_SLICE_SECONDS: f64 = 0.001;

// link corefoundation run-loop symbols used by host adapter message pumping
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Run one CoreFoundation run-loop mode for one bounded interval.
    fn CFRunLoopRunInMode(
        mode: CFStringRef,
        seconds: f64,
        return_after_source_handled: bool,
    ) -> i32;
    /// Default run-loop mode used by platform thread sources.
    static kCFRunLoopDefaultMode: CFStringRef;
}

/// Return whether the current execution context is the process main context.
pub(crate) fn is_process_main_context() -> bool {
    unsafe { libc::pthread_main_np() == 1 }
}

/// Service immediately ready platform ingress without blocking.
pub(crate) fn process_ingress_ready(ignore_quit_message: bool) -> bool {
    // ignore quit-message policy on run-loop platforms with no quit packets
    let _ = ignore_quit_message;
    let mut dispatched_any = false;

    loop {
        // dispatch one immediately ready source when available
        let status = unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.0, true) };
        if status == KCF_RUN_LOOP_RUN_HANDLED_SOURCE {
            dispatched_any = true;
            continue;
        }

        // otherwise no immediate source remained, stop the pump loop
        if status == KCF_RUN_LOOP_RUN_TIMED_OUT
            || status == KCF_RUN_LOOP_RUN_FINISHED
            || status == KCF_RUN_LOOP_RUN_STOPPED
        {
            break;
        }

        break;
    }

    dispatched_any
}

/// Service Apple thread messages until one caller-provided stop condition becomes true.
#[cfg(feature = "affinity")]
#[cfg_attr(feature = "affinity", allow(dead_code))]
pub(crate) fn service_registered_runtimes_until(
    ignore_quit_message: bool,
    mut should_stop: impl FnMut() -> bool,
) -> RuntimeResult<()> {
    // ignore quit-message policy on run-loop platforms with no quit packets
    let _ = ignore_quit_message;

    // keep the current run loop alive until the stop condition is satisfied
    while !should_stop() {
        let status = unsafe {
            CFRunLoopRunInMode(
                kCFRunLoopDefaultMode,
                APPLE_THREAD_MESSAGE_WAIT_SLICE_SECONDS,
                true,
            )
        };

        // notify runtime observers after one handled source
        if status == KCF_RUN_LOOP_RUN_HANDLED_SOURCE {
            process_runtime_observers()?;
            continue;
        }

        // continue after benign wait statuses
        if status == KCF_RUN_LOOP_RUN_TIMED_OUT
            || status == KCF_RUN_LOOP_RUN_FINISHED
            || status == KCF_RUN_LOOP_RUN_STOPPED
        {
            continue;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::process_ingress_ready;

    #[test]
    fn test_process_ingress_ready_ignores_runtime_without_panicking() {
        let _ = process_ingress_ready(true);
    }
}
