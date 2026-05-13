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

// link CoreFoundation run-loop symbols used by host event pumping
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Run one CoreFoundation run-loop mode for one bounded interval.
    fn CFRunLoopRunInMode(
        mode: CFStringRef,
        seconds: f64,
        return_after_source_handled: bool,
    ) -> i32;

    /// Default run-loop mode used by host thread sources.
    static kCFRunLoopDefaultMode: CFStringRef;
}

/// Return whether the current execution context is the process main context.
pub(crate) fn is_process_main_context() -> bool {
    unsafe { libc::pthread_main_np() == 1 }
}

/// Drain immediately ready run-loop events without blocking.
pub(crate) fn drain_ready_events() -> bool {
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
