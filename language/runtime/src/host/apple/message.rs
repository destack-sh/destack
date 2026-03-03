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

// link corefoundation run-loop symbols used by host adapter message pumping
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Run one CoreFoundation run-loop mode for one bounded interval.
    fn CFRunLoopRunInMode(
        mode: CFStringRef,
        seconds: f64,
        return_after_source_handled: bool,
    ) -> i32;
    /// Run one CoreFoundation run loop until explicit stop.
    fn CFRunLoopRun();
    /// Default run-loop mode used by platform thread sources.
    static kCFRunLoopDefaultMode: CFStringRef;
}

/// Drain pending platform thread messages without blocking.
pub(crate) fn pump_pending_thread_messages(ignore_quit_message: bool) -> bool {
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

/// Run one blocking platform thread message loop.
pub(crate) fn run_blocking_thread_message_loop() {
    // run until the current run loop is explicitly stopped
    unsafe {
        CFRunLoopRun();
    }
}
