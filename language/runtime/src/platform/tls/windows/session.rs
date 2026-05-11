use std::io::{Read, Write};

use windows_sys::Win32::Networking::WinSock::{SOCKET, SOCKET_ERROR, WSAGetLastError, recv, send};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::net::core as core_net;
use crate::platform::resource::ResourceKind;
use crate::platform::tls::{
    TlsHandshakeStatus, TlsRole, TlsSessionResumptionState, core as core_tls,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Socket transport adapter for TLS over winsock sockets.
struct WindowsSocketTransport {
    /// Backing socket descriptor.
    socket: SOCKET,
}

impl Read for WindowsSocketTransport {
    /// Read encrypted TLS records from one socket descriptor.
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        // convert buffer length into a winsock length
        let length = i32::try_from(buffer.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "buffer is too large")
        })?;

        // read one socket frame
        let result = unsafe { recv(self.socket, buffer.as_mut_ptr(), length, 0) };
        if result == SOCKET_ERROR {
            return Err(std::io::Error::from_raw_os_error(unsafe {
                WSAGetLastError()
            }));
        }

        Ok(result as usize)
    }
}

impl Write for WindowsSocketTransport {
    /// Write encrypted TLS records to one socket descriptor.
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        // convert buffer length into a winsock length
        let length = i32::try_from(buffer.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "buffer is too large")
        })?;

        // write one socket frame
        let result = unsafe { send(self.socket, buffer.as_ptr(), length, 0) };
        if result == SOCKET_ERROR {
            return Err(std::io::Error::from_raw_os_error(unsafe {
                WSAGetLastError()
            }));
        }

        Ok(result as usize)
    }

    /// Flush one socket writer.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Resolve one socket descriptor from a socket handle.
fn socket_descriptor(
    binding: &BindingCallContext,
    handle: resource::SocketHandle,
) -> RuntimeResult<SOCKET> {
    core_net::require_resource(binding, handle.0, ResourceKind::Socket, "socket", |entry| {
        entry
            .socket()
            .map(|socket| socket as SOCKET)
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "socket handle missing payload",
                ))
                .boxed()
            })
    })
}

/// Close one tls session object.
pub(crate) unsafe fn destack_tls_session_close(
    binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    core_tls::remove_session_resource(binding, handle)
}

/// Export keying material bytes for one tls session.
pub(crate) unsafe fn destack_tls_session_export_keying_material(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
    label: NativeStringRef,
    argument_context: NativeSlice<u8>,
    outputlength: u32,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode exporter arguments
    let label = core_tls::decode_native_string(label, "label")?;
    let argument_context = core_tls::decode_native_bytes(argument_context)?;

    // resolve and lock the session connection
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let connection = session.connection.lock();
    let output =
        core_tls::export_keying_material(&connection, &label, &argument_context, outputlength)?;

    // write the output bytes
    unsafe {
        *out = binding.store_slice(output);
    }

    Ok(())
}

/// Advance one tls handshake state machine.
pub(crate) unsafe fn destack_tls_session_handshake(
    binding: &BindingCallContext,
    out: *mut TlsHandshakeStatus,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the session and socket transport
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let socket = socket_descriptor(binding, session.socket)?;
    let mut transport = WindowsSocketTransport { socket };
    let mut connection = session.connection.lock();
    let status = core_tls::handshake_step(&mut connection, &mut transport)?;

    // write the handshake status
    unsafe {
        *out = status;
    }

    Ok(())
}

/// Return negotiated alpn protocol bytes.
pub(crate) unsafe fn destack_tls_session_negotiated_alpn(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one session and ALPN value
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let connection = session.connection.lock();
    let alpn = core_tls::negotiated_alpn(&connection)?;

    // write the ALPN bytes
    unsafe {
        *out = binding.store_slice(alpn);
    }

    Ok(())
}

/// Open one tls session over one connected socket.
pub(crate) unsafe fn destack_tls_session_open(
    binding: &BindingCallContext,
    out: *mut resource::TlsSessionHandle,
    argument_context: resource::TlsContextHandle,
    socket: resource::SocketHandle,
    servername: NativeStringRef,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve binding and socket inputs
    core_tls::require_socket_handle(binding, socket)?;
    let policy = core_tls::resolve_context_resource(binding, argument_context)?;
    let mut policy = policy.lock();

    // build one tls connection
    let connection = match policy.role {
        TlsRole::Client => {
            let server_name = core_tls::decode_native_string(servername, "serverName")?;
            if server_name.is_empty() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "serverName",
                    "serverName must be non-empty",
                ))
                .boxed());
            }

            core_tls::build_client_connection(&mut policy, &server_name)?
        }
        TlsRole::Server => core_tls::build_server_connection(&mut policy)?,
    };

    // insert and return one tls session handle
    let handle = core_tls::insert_session_resource(binding, socket, connection);
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Return the peer certificate chain bytes in pem encoding.
pub(crate) unsafe fn destack_tls_session_peer_certificates_pem(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one session and encode peer certificates
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let connection = session.connection.lock();
    let pem = core_tls::peer_certificates_pem(&connection)?;

    // write PEM bytes
    unsafe {
        *out = binding.store_slice(pem);
    }

    Ok(())
}

/// Read decrypted application bytes from one tls session.
pub(crate) unsafe fn destack_tls_session_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TlsSessionHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the mutable buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // resolve one session and socket transport
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let socket = socket_descriptor(binding, session.socket)?;
    let mut transport = WindowsSocketTransport { socket };
    let mut connection = session.connection.lock();
    let read = core_tls::read_plaintext(&mut connection, &mut transport, buffer)?;

    // write the read byte count
    unsafe {
        *out = read;
    }

    Ok(())
}

/// Return whether one session resumed from cached state or ticket.
pub(crate) unsafe fn destack_tls_session_resumption_state(
    binding: &BindingCallContext,
    out: *mut TlsSessionResumptionState,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one session and read resumption state
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let connection = session.connection.lock();
    let state = core_tls::resumption_state(&connection);

    // write the resumption state
    unsafe {
        *out = state;
    }

    Ok(())
}

/// Shutdown one tls session.
pub(crate) unsafe fn destack_tls_session_shutdown(
    binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // resolve one session and socket transport
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let socket = socket_descriptor(binding, session.socket)?;
    let mut transport = WindowsSocketTransport { socket };
    let mut connection = session.connection.lock();
    core_tls::shutdown(&mut connection, &mut transport)
}

/// Write plaintext application bytes to one tls session.
pub(crate) unsafe fn destack_tls_session_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TlsSessionHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the immutable buffer
    let buffer = unsafe { buffer.as_slice()? };

    // resolve one session and socket transport
    let session = core_tls::resolve_session_resource(binding, handle)?;
    let socket = socket_descriptor(binding, session.socket)?;
    let mut transport = WindowsSocketTransport { socket };
    let mut connection = session.connection.lock();
    let written = core_tls::write_plaintext(&mut connection, &mut transport, buffer)?;

    // write the written byte count
    unsafe {
        *out = written;
    }

    Ok(())
}
