use crate::{DestackError, DestackStatus, owned_array, read_bytes, read_string, return_status};

/// C ABI local workspace server handle.
#[repr(C)]
#[derive(Debug)]
pub struct DestackLocalWorkspaceServer {
    /// Rust workspace server.
    server: destack::LocalWorkspaceServer,
}

/// One owned protocol message payload.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProtocolPayload {
    /// Payload bytes.
    pub ptr: *mut u8,
    /// Payload byte length.
    pub len: usize,
}

/// One owned protocol message payload array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProtocolPayloadArray {
    /// Payload pointer.
    pub ptr: *mut DestackProtocolPayload,
    /// Payload count.
    pub len: usize,
}

/// Open an in-process workspace protocol server.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_local_workspace_server_open(
    home: *const std::ffi::c_char,
    out: *mut *mut DestackLocalWorkspaceServer,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let home = read_string(home)?;
        let server =
            destack::LocalWorkspaceServer::open(home).map_err(|error| error.to_string())?;
        let server = Box::into_raw(Box::new(DestackLocalWorkspaceServer { server }));

        crate::write_out(out, server, "local workspace server output pointer is null")
    })
}

/// Destroy an in-process workspace protocol server.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_local_workspace_server_destroy(
    server: *mut DestackLocalWorkspaceServer,
) {
    if server.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(server));
    }
}

/// Dispatch one encoded protocol message payload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_local_workspace_server_dispatch(
    server: *const DestackLocalWorkspaceServer,
    bytes: *const u8,
    len: usize,
    out: *mut DestackProtocolPayloadArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        if server.is_null() {
            return Err("local workspace server pointer is null".to_string());
        }

        let payload = read_bytes(bytes, len)?;
        let payloads = unsafe { &(*server).server }
            .dispatch(&payload)
            .map_err(|error| error.to_string())?;
        let payloads = payloads
            .into_iter()
            .map(DestackProtocolPayload::from_bytes)
            .collect();
        let payloads = DestackProtocolPayloadArray::from_payloads(payloads);

        crate::write_out(
            out,
            payloads,
            "protocol payload array output pointer is null",
        )
    })
}

/// Destroy one protocol payload array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_protocol_payload_array_destroy(
    array: DestackProtocolPayloadArray,
) {
    unsafe {
        crate::destroy_array(array.ptr, array.len, |frame| {
            frame.destroy();
        });
    }
}

impl DestackProtocolPayload {
    /// Create one owned payload from bytes.
    fn from_bytes(bytes: Vec<u8>) -> Self {
        let (ptr, len) = owned_array(bytes);

        Self { ptr, len }
    }

    /// Destroy this payload.
    fn destroy(&mut self) {
        unsafe {
            crate::destroy_array(self.ptr, self.len, |_byte| {});
        }
        self.ptr = std::ptr::null_mut();
        self.len = 0;
    }
}

impl DestackProtocolPayloadArray {
    /// Create one payload array.
    fn from_payloads(payloads: Vec<DestackProtocolPayload>) -> Self {
        let (ptr, len) = owned_array(payloads);

        Self { ptr, len }
    }
}
