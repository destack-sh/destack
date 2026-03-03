use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

/// Drain pending thread messages without blocking.
pub(super) fn pump_pending_thread_messages(ignore_quit_message: bool) -> bool {
    // drain pending messages from the current thread queue
    let mut dispatched_any = false;
    loop {
        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        let has_message = unsafe { PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) } != 0;
        if !has_message {
            break;
        }

        // optionally swallow quit messages for caller-managed lifecycles
        if ignore_quit_message && message.message == WM_QUIT {
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

/// Run the blocking thread message loop until quit or failure.
pub(super) fn run_blocking_thread_message_loop() {
    // block on get message and dispatch until quit or error
    let mut message = unsafe { std::mem::zeroed::<MSG>() };
    loop {
        let status = unsafe { GetMessageW(&mut message, 0, 0, 0) };
        if status <= 0 {
            break;
        }

        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
