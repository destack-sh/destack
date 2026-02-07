use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use std::sync::OnceLock;
use windows_sys::Win32::Networking::WinSock::{WSADATA, WSAStartup};

/// Ensure Winsock is initialized for the process.
pub(crate) fn ensure_winsock() -> RuntimeResult<()> {
    static INIT: OnceLock<i32> = OnceLock::new();

    // initialize Winsock once per process
    let rc = *INIT.get_or_init(|| {
        let mut data = unsafe { std::mem::zeroed::<WSADATA>() };
        unsafe { WSAStartup(makeword(2, 2), &mut data) }
    });
    if rc != 0 {
        let message = format!("WSAStartup failed: {rc}");
        return Err(RuntimeError::from(PlatformError::net_with(
            None,
            None,
            Some(rc),
            Some("WSAStartup".to_string()),
            None,
            None,
            message,
        ))
        .boxed());
    }

    Ok(())
}

/// Build a WORD value for WSAStartup.
fn makeword(low: u8, high: u8) -> u16 {
    (low as u16) | ((high as u16) << 8)
}
