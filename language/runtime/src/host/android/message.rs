use std::ptr::null_mut;

/// Android looper wake status code.
const ALOOPER_POLL_WAKE: i32 = -1;
/// Android looper callback status code.
const ALOOPER_POLL_CALLBACK: i32 = -2;
/// Android looper timeout status code.
const ALOOPER_POLL_TIMEOUT: i32 = -3;
/// Android looper error status code.
const ALOOPER_POLL_ERROR: i32 = -4;

// link android looper symbols used by host adapter message pumping
#[link(name = "android")]
unsafe extern "C" {
    /// Poll one android looper for one message or callback event.
    fn ALooper_pollOnce(
        timeout_millis: i32,
        out_file_descriptor: *mut i32,
        out_events: *mut i32,
        out_data: *mut *mut libc::c_void,
    ) -> i32;
}

/// Drain pending platform thread messages without blocking.
pub(super) fn pump_pending_thread_messages(ignore_quit_message: bool) -> bool {
    // ignore quit-message policy for android looper polling
    let _ = ignore_quit_message;
    let mut dispatched_any = false;

    loop {
        // poll one ready looper item without blocking
        let status =
            unsafe { ALooper_pollOnce(0, null_mut(), null_mut(), null_mut::<*mut libc::c_void>()) };
        if status == ALOOPER_POLL_CALLBACK || status >= 0 {
            dispatched_any = true;
            continue;
        }

        // stop when no immediate callback or fd event remains
        if status == ALOOPER_POLL_WAKE
            || status == ALOOPER_POLL_TIMEOUT
            || status == ALOOPER_POLL_ERROR
        {
            break;
        }

        break;
    }

    dispatched_any
}

/// Run one blocking platform thread message loop.
pub(super) fn run_blocking_thread_message_loop() {
    // block until one callback or one looper event is dispatched
    let status =
        unsafe { ALooper_pollOnce(-1, null_mut(), null_mut(), null_mut::<*mut libc::c_void>()) };
    let _ = status;
}
