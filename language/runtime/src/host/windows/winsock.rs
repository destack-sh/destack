use std::mem::MaybeUninit;
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
        let mut data = MaybeUninit::<WSADATA>::uninit();

        // SAFETY: the requested version is constant and the out pointer is valid
        unsafe { WSAStartup(WINSOCK_VERSION, data.as_mut_ptr()) }
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
