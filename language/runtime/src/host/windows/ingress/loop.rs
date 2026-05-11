use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

use crate::runtime::service::windows::{
    WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID, process_windows_loop_callbacks,
};

/// Service immediately ready thread messages without blocking.
pub(crate) fn drain_ready_ingress() -> bool {
    // drain pending messages from the current thread queue
    let mut dispatched_any = false;
    loop {
        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        let has_message = unsafe { PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) } != 0;
        if !has_message {
            break;
        }

        // swallow quit packets during nonblocking ingress draining
        if message.message == WM_QUIT {
            continue;
        }

        // service one queued platform host-loop callback
        if message.message == WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID {
            dispatched_any |= process_windows_loop_callbacks();
            continue;
        }

        // dispatch one translated message
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        dispatched_any = true;
    }

    dispatched_any
}

/// Run the blocking ingress loop until quit or failure.
pub(crate) fn run_ingress_loop() {
    // block on get message and dispatch until quit or error
    let mut message = unsafe { std::mem::zeroed::<MSG>() };
    loop {
        let status = unsafe { GetMessageW(&mut message, 0, 0, 0) };
        if status <= 0 {
            break;
        }

        // service one queued platform host-loop callback
        if message.message == WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID {
            process_windows_loop_callbacks();
            continue;
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
