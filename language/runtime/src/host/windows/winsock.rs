use std::sync::OnceLock;

use windows_sys::Win32::Networking::WinSock::{WSADATA, WSAStartup};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostError;

/// Winsock 2.2 startup version.
const WINSOCK_VERSION: u16 = 0x0202;

/// Initialize Winsock for the process.
pub(crate) fn initialize_winsock() -> RuntimeResult<()> {
    static INIT: OnceLock<i32> = OnceLock::new();

    // initialize Winsock once per process
    let rc = *INIT.get_or_init(|| {
        let mut data = unsafe { std::mem::zeroed::<WSADATA>() };
        unsafe { WSAStartup(WINSOCK_VERSION, &mut data) }
    });
    if rc != 0 {
        let message = format!("WSAStartup failed: {rc}");
        return Err(RuntimeError::from(HostError::net_with(
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
