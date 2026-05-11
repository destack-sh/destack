use std::collections::{HashMap, HashSet, VecDeque};
use std::mem::size_of;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GetLastError, HANDLE, WAIT_TIMEOUT,
};
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AcceptEx, GetAcceptExSockaddrs, INVALID_SOCKET, LPFN_CONNECTEX, POLLERR,
    POLLHUP, POLLIN, POLLOUT, POLLPRI, SIO_GET_EXTENSION_FUNCTION_POINTER,
    SO_UPDATE_ACCEPT_CONTEXT, SO_UPDATE_CONNECT_CONTEXT, SOCK_STREAM, SOCKADDR, SOCKADDR_IN,
    SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET, SOL_SOCKET, TRANSMIT_FILE_BUFFERS, TransmitFile,
    WSA_IO_PENDING, WSABUF, WSAEADDRINUSE, WSAEINVAL, WSAENOTSOCK, WSAGetLastError,
    WSAID_CONNECTEX, WSAIoctl, WSAPOLLFD, WSAPoll, WSARecv, WSARecvFrom, WSASend, WSASendTo,
    accept, bind, closesocket, connect, getsockname, recv, recvfrom, send, sendto, setsockopt,
    shutdown, socket,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALLOCATION_INFO, FILE_STANDARD_INFO, FileAllocationInfo, FileStandardInfo,
    FlushFileBuffers, GetFileInformationByHandleEx, ReadFile, SetFileInformationByHandle,
    WriteFile,
};
use windows_sys::Win32::System::IO::{
    CancelIoEx, CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED,
    PostQueuedCompletionStatus,
};
use windows_sys::Win32::System::Threading::INFINITE;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::proactor::{
    Proactor, ProactorAddress, ProactorAddressStorage, ProactorBufferVec, ProactorCompletion,
    ProactorCompletionData, ProactorOp, ProactorOpKind, ProactorRequest, ProactorShutdown,
};
use crate::platform::{PlatformError, PlatformErrorCode, ResourceId, core as core_platform};
use crate::runtime::poller::{PlatformHandle, PlatformInterest, PollerEventMask};
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

/// Completion key reserved for wake notifications.
const WAKE_COMPLETION_KEY: usize = usize::MAX;
/// Completion key reserved for boxed completion payloads.
const MESSAGE_COMPLETION_KEY: usize = usize::MAX - 1;
/// Completion key reserved for native overlapped operations.
const OPERATION_COMPLETION_KEY: usize = usize::MAX - 2;
/// Additional address bytes required by AcceptEx.
const ACCEPT_EX_ADDRESS_PADDING_BYTES: usize = 16;
/// Address storage bytes used by AcceptEx for one endpoint.
const ACCEPT_EX_ADDRESS_BYTES: usize =
    size_of::<SOCKADDR_STORAGE>() + ACCEPT_EX_ADDRESS_PADDING_BYTES;
/// Copy chunk size used by copy_file_range emulation.
const COPY_FILE_RANGE_CHUNK_BYTES: u32 = 64 * 1024;
/// Default wait slice used while pending readiness polls are active.
const DEFAULT_PENDING_POLL_SLICE_NS: u64 = 10_000_000;

/// Message payload posted into the IOCP queue.
#[derive(Debug, Clone, Copy)]
struct IocpMessage {
    /// Completion payload delivered by poll.
    completion: ProactorCompletion,
}

/// One pending poll registration tracked by the IOCP proactor.
#[derive(Debug, Clone, Copy)]
struct PendingPoll {
    /// Original request metadata.
    request: ProactorRequest,
    /// Handle to poll.
    handle: PlatformHandle,
    /// Interested events to observe.
    interests: PlatformInterest,
}

/// One in-flight overlapped operation tracked for cancellation.
#[derive(Debug, Clone, Copy)]
struct InflightTask {
    /// Handle associated with the overlapped request.
    handle: HANDLE,
    /// Overlapped state pointer owned by the operation box.
    overlapped: usize,
}

/// Native overlapped operation state.
#[repr(C)]
struct IocpTask {
    /// Overlapped header passed to Win32 APIs.
    overlapped: OVERLAPPED,
    /// Request metadata used to build completion payloads.
    request: ProactorRequest,
    /// Socket buffers that must stay alive until completion.
    buffers: Vec<WSABUF>,
    /// Receive flags output for recv variants.
    recv_flags: u32,
    /// Socket address length for recvfrom.
    recvfrom_len: i32,
    /// Optional caller output pointer for recvfrom length.
    recvfrom_len_out: Option<*mut u32>,
    /// Accepted socket created for AcceptEx.
    accept_socket: Option<SOCKET>,
    /// Listening socket used by AcceptEx.
    accept_listen_socket: Option<SOCKET>,
    /// AcceptEx output buffer.
    accept_output: Vec<u8>,
    /// AcceptEx local address length.
    accept_local_len: u32,
    /// AcceptEx remote address length.
    accept_remote_len: u32,
    /// Optional caller output for the accepted remote address.
    accept_address_out: Option<ProactorAddressStorage>,
    /// ConnectEx target address copy.
    connect_address: Vec<u8>,
}

impl IocpTask {
    /// Create a new overlapped operation state.
    fn new(request: ProactorRequest) -> Self {
        // safety: zeroed OVERLAPPED is valid initialization
        let overlapped = unsafe { std::mem::zeroed() };

        Self {
            overlapped,
            request,
            buffers: Vec::new(),
            recv_flags: 0,
            recvfrom_len: 0,
            recvfrom_len_out: None,
            accept_socket: None,
            accept_listen_socket: None,
            accept_output: Vec::new(),
            accept_local_len: 0,
            accept_remote_len: 0,
            accept_address_out: None,
            connect_address: Vec::new(),
        }
    }
}

/// IOCP-backed proactor for Windows targets.
#[derive(Debug)]
pub struct IocpProactor {
    /// Completion port handle.
    port: HANDLE,
    /// Handles already associated with this completion port.
    associated_handles: HashSet<u64>,
    /// Tokens marked for cancellation.
    canceled_tokens: Arc<Mutex<HashSet<u64>>>,
    /// Pending timeout tokens keyed by resource id.
    pending_timeouts: Arc<Mutex<HashMap<u64, ResourceId>>>,
    /// Buffered completions ready to return.
    completions: VecDeque<ProactorCompletion>,
    /// Poll registrations checked from the frontend loop.
    pending_polls: Vec<PendingPoll>,
    /// Native overlapped operations keyed by request token.
    inflight_operations: HashMap<u64, InflightTask>,
    /// Wait slice used while pending poll registrations are active.
    pending_poll_slice_ns: u64,
}

impl IocpProactor {
    /// Create a new IOCP proactor.
    pub fn new() -> RuntimeResult<Self> {
        Self::with_pending_poll_slice_ns(DEFAULT_PENDING_POLL_SLICE_NS)
    }

    /// Create a new IOCP proactor with one explicit pending-poll wait slice.
    pub fn with_pending_poll_slice_ns(pending_poll_slice_ns: u64) -> RuntimeResult<Self> {
        // create one standalone completion port
        let port = unsafe { CreateIoCompletionPort(-1isize as HANDLE, 0, 0, 1) };
        if port == 0 {
            return Err(core_platform::io_error("CreateIoCompletionPort"));
        }

        Ok(Self {
            port,
            associated_handles: HashSet::new(),
            canceled_tokens: Arc::new(Mutex::new(HashSet::new())),
            pending_timeouts: Arc::new(Mutex::new(HashMap::new())),
            completions: VecDeque::new(),
            pending_polls: Vec::new(),
            inflight_operations: HashMap::new(),
            pending_poll_slice_ns: pending_poll_slice_ns.max(1),
        })
    }

    /// Submit one timeout request handled by a helper thread.
    fn submit_timeout(
        &mut self,
        resource_id: ResourceId,
        token: u64,
        timeout_nanos: u64,
    ) -> RuntimeResult<()> {
        // record timeout token as pending
        self.pending_timeouts.lock().insert(token, resource_id);

        // spawn one helper that sleeps and posts completion
        let pending_timeouts = Arc::clone(&self.pending_timeouts);
        let canceled_tokens = Arc::clone(&self.canceled_tokens);
        let port = self.port;
        start_with_policy(
            "destack-windows-timeout",
            "proactor.windows.submitTimeout",
            ExecutionPolicy::task(ExecutionMode::Blocking),
            move || {
                thread::sleep(Duration::from_nanos(timeout_nanos));

                // clear pending timeout state
                let Some(resource_id) = pending_timeouts.lock().remove(&token) else {
                    return;
                };

                // report cancellation when token was canceled
                let was_canceled = canceled_tokens.lock().remove(&token);
                let completion = if was_canceled {
                    canceled_completion(resource_id, token, ProactorOpKind::Timeout)
                } else {
                    ProactorCompletion {
                        resource_id,
                        token,
                        op: ProactorOpKind::Timeout,
                        result: 0,
                        error_code: None,
                        error_errno: None,
                        data: ProactorCompletionData::Timeout,
                    }
                };

                let _ = post_completion(port, completion);
            },
        )
        .map_err(|error| {
            RuntimeError::from(PlatformError::process_with(
                Some(PlatformErrorCode::ProcessSpawnFailed),
                None,
                None,
                None,
                None,
                error.to_string(),
            ))
            .boxed()
        })?;

        Ok(())
    }

    /// Submit one operation through helper threading.
    fn submit_threaded_operation(&mut self, request: ProactorRequest) -> RuntimeResult<()> {
        // spawn one helper that executes and posts completion
        let canceled_tokens = Arc::clone(&self.canceled_tokens);
        let port = self.port;
        start_with_policy(
            "destack-windows-proactor-op",
            "proactor.windows.submit",
            ExecutionPolicy::task(ExecutionMode::Blocking),
            move || {
                // skip execution when already canceled
                if canceled_tokens.lock().remove(&request.token) {
                    let completion =
                        canceled_completion(request.resource_id, request.token, request.op.kind());
                    let _ = post_completion(port, completion);
                    return;
                }

                // execute one operation
                let completion = execute_request(request);

                // replace completion with cancellation when canceled during execution
                let completion = if canceled_tokens.lock().remove(&request.token) {
                    canceled_completion(request.resource_id, request.token, request.op.kind())
                } else {
                    completion
                };

                let _ = post_completion(port, completion);
            },
        )
        .map_err(|error| {
            RuntimeError::from(PlatformError::process_with(
                Some(PlatformErrorCode::ProcessSpawnFailed),
                None,
                None,
                None,
                None,
                error.to_string(),
            ))
            .boxed()
        })?;

        Ok(())
    }

    /// Associate one handle with this completion port.
    fn associate_handle(&mut self, handle: PlatformHandle) -> RuntimeResult<()> {
        // skip handles already associated
        if self.associated_handles.contains(&handle.0) {
            return Ok(());
        }

        // associate handle with completion port
        let associated = unsafe {
            CreateIoCompletionPort(handle.0 as HANDLE, self.port, OPERATION_COMPLETION_KEY, 0)
        };
        if associated == 0 {
            return Err(core_platform::io_error("CreateIoCompletionPort"));
        }

        // remember association
        self.associated_handles.insert(handle.0);

        Ok(())
    }

    /// Submit one socket recv/send style operation via overlapped IOCP.
    fn submit_socket_overlapped(&mut self, request: ProactorRequest) -> RuntimeResult<bool> {
        // decode the operation payload
        let mut socket_buffers: Vec<WSABUF> = Vec::new();
        let mut recv_flags = 0u32;
        let mut is_receive = false;
        let mut address_storage = None;
        let mut address_target = None;
        let handle = match request.op {
            ProactorOp::Recv {
                handle,
                buffer,
                flags,
            } => {
                is_receive = true;
                recv_flags = flags;
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::Send {
                handle,
                buffer,
                flags,
            } => {
                recv_flags = flags;
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::RecvFrom {
                handle,
                buffer,
                flags,
                address,
            } => {
                is_receive = true;
                recv_flags = flags;
                address_storage = Some(address);
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::SendTo {
                handle,
                buffer,
                flags,
                address,
            } => {
                recv_flags = flags;
                address_target = Some(address);
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::Read {
                handle,
                buffer,
                offset: None,
            } => {
                is_receive = true;
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::Write {
                handle,
                buffer,
                offset: None,
            } => {
                socket_buffers.push(WSABUF {
                    len: buffer.len,
                    buf: buffer.data,
                });
                handle
            }
            ProactorOp::Readv {
                handle,
                buffers,
                offset: None,
            } => {
                is_receive = true;
                if buffers.data.is_null() && buffers.len != 0 {
                    let completion =
                        invalid_argument_completion(request, PlatformErrorCode::NullPointer);
                    post_completion(self.port, completion)?;
                    return Ok(true);
                }
                if buffers.len != 0 {
                    let buffer_slice =
                        unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
                    socket_buffers.reserve(buffer_slice.len());
                    for buffer in buffer_slice {
                        socket_buffers.push(WSABUF {
                            len: buffer.len,
                            buf: buffer.data,
                        });
                    }
                }
                handle
            }
            ProactorOp::Writev {
                handle,
                buffers,
                offset: None,
            } => {
                if buffers.data.is_null() && buffers.len != 0 {
                    let completion =
                        invalid_argument_completion(request, PlatformErrorCode::NullPointer);
                    post_completion(self.port, completion)?;
                    return Ok(true);
                }
                if buffers.len != 0 {
                    let buffer_slice =
                        unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
                    socket_buffers.reserve(buffer_slice.len());
                    for buffer in buffer_slice {
                        socket_buffers.push(WSABUF {
                            len: buffer.len,
                            buf: buffer.data,
                        });
                    }
                }
                handle
            }
            _ => return Ok(false),
        };

        // validate per-buffer pointer payloads
        for buffer in &socket_buffers {
            if buffer.buf.is_null() && buffer.len != 0 {
                let completion =
                    invalid_argument_completion(request, PlatformErrorCode::NullPointer);
                post_completion(self.port, completion)?;
                return Ok(true);
            }
        }

        // ensure handle is associated with this completion port
        self.associate_handle(handle)?;

        // allocate operation state
        let mut operation = Box::new(IocpTask::new(request));
        operation.recv_flags = recv_flags;
        operation.buffers = socket_buffers;
        let token = operation.request.token;
        let tracking_handle = handle.0 as HANDLE;
        let operation_ptr = Box::into_raw(operation);
        self.register_inflight_operation(token, tracking_handle, operation_ptr.cast());

        // submit by operation shape
        let socket = handle.0 as SOCKET;
        let submit_result = unsafe {
            if let Some(address) = address_storage {
                if address.data.is_null() || address.len.is_null() {
                    let operation = Box::from_raw(operation_ptr);
                    self.remove_inflight_operation(operation.request.token);
                    let completion = invalid_argument_completion(
                        operation.request,
                        PlatformErrorCode::NullPointer,
                    );
                    post_completion(self.port, completion)?;
                    return Ok(true);
                }

                (*operation_ptr).recvfrom_len = *address.len as i32;
                (*operation_ptr).recvfrom_len_out = Some(address.len);
                WSARecvFrom(
                    socket,
                    (*operation_ptr).buffers.as_mut_ptr(),
                    (*operation_ptr).buffers.len() as u32,
                    std::ptr::null_mut(),
                    &mut (*operation_ptr).recv_flags,
                    address.data.cast(),
                    &mut (*operation_ptr).recvfrom_len,
                    &mut (*operation_ptr).overlapped,
                    None,
                )
            } else if let Some(address) = address_target {
                if address.data.is_null() && address.len != 0 {
                    let operation = Box::from_raw(operation_ptr);
                    self.remove_inflight_operation(operation.request.token);
                    let completion = invalid_argument_completion(
                        operation.request,
                        PlatformErrorCode::NullPointer,
                    );
                    post_completion(self.port, completion)?;
                    return Ok(true);
                }

                WSASendTo(
                    socket,
                    (*operation_ptr).buffers.as_ptr().cast_mut(),
                    (*operation_ptr).buffers.len() as u32,
                    std::ptr::null_mut(),
                    (*operation_ptr).recv_flags,
                    address.data.cast(),
                    address.len as i32,
                    &mut (*operation_ptr).overlapped,
                    None,
                )
            } else if is_receive {
                WSARecv(
                    socket,
                    (*operation_ptr).buffers.as_mut_ptr(),
                    (*operation_ptr).buffers.len() as u32,
                    std::ptr::null_mut(),
                    &mut (*operation_ptr).recv_flags,
                    &mut (*operation_ptr).overlapped,
                    None,
                )
            } else {
                WSASend(
                    socket,
                    (*operation_ptr).buffers.as_ptr().cast_mut(),
                    (*operation_ptr).buffers.len() as u32,
                    std::ptr::null_mut(),
                    (*operation_ptr).recv_flags,
                    &mut (*operation_ptr).overlapped,
                    None,
                )
            }
        };

        // completion will arrive asynchronously or operation submitted immediately
        if submit_result == 0 {
            return Ok(true);
        }

        // pending means completion will still arrive asynchronously
        let error = unsafe { WSAGetLastError() };
        if error == WSA_IO_PENDING {
            return Ok(true);
        }

        // treat non-socket read and write variants as not handled here
        let should_fallback = error == WSAENOTSOCK
            && matches!(
                request.op,
                ProactorOp::Read { .. }
                    | ProactorOp::Write { .. }
                    | ProactorOp::Readv { .. }
                    | ProactorOp::Writev { .. }
            );
        if should_fallback {
            unsafe {
                let operation = Box::from_raw(operation_ptr);
                self.remove_inflight_operation(operation.request.token);
            }

            return Ok(false);
        }

        // reclaim operation and report failure synchronously
        unsafe {
            let operation = Box::from_raw(operation_ptr);
            self.remove_inflight_operation(operation.request.token);
            let completion = net_error_completion(operation.request, error);
            post_completion(self.port, completion)?;
        }

        Ok(true)
    }

    /// Submit one file read or write operation via overlapped IOCP.
    fn submit_file_overlapped(&mut self, request: ProactorRequest) -> RuntimeResult<bool> {
        // decode the operation payload
        let (handle, buffer_ptr, buffer_len, offset, is_read) = match request.op {
            ProactorOp::Read {
                handle,
                buffer,
                offset,
            } => (handle, buffer.data.cast::<u8>(), buffer.len, offset, true),
            ProactorOp::Write {
                handle,
                buffer,
                offset,
            } => (handle, buffer.data.cast::<u8>(), buffer.len, offset, false),
            _ => return Ok(false),
        };

        // validate read and write pointers
        if buffer_ptr.is_null() && buffer_len != 0 {
            let completion = invalid_argument_completion(request, PlatformErrorCode::NullPointer);
            post_completion(self.port, completion)?;
            return Ok(true);
        }

        // require explicit offsets to avoid implicit shared file pointer races
        let Some(offset) = offset else {
            return Ok(false);
        };

        // ensure the handle is associated with this completion port
        self.associate_handle(handle)?;

        // allocate operation state
        let mut operation = Box::new(IocpTask::new(request));
        operation.overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
        operation.overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        let token = operation.request.token;
        let tracking_handle = handle.0 as HANDLE;
        let operation_ptr = Box::into_raw(operation);
        self.register_inflight_operation(token, tracking_handle, operation_ptr.cast());

        // submit the operation through ReadFile or WriteFile
        let handle = handle.0 as HANDLE;
        let submit_result = unsafe {
            if is_read {
                ReadFile(
                    handle,
                    buffer_ptr.cast(),
                    buffer_len,
                    std::ptr::null_mut(),
                    &mut (*operation_ptr).overlapped,
                )
            } else {
                WriteFile(
                    handle,
                    buffer_ptr.cast(),
                    buffer_len,
                    std::ptr::null_mut(),
                    &mut (*operation_ptr).overlapped,
                )
            }
        };

        // immediate success still delivers one completion packet
        if submit_result != 0 {
            return Ok(true);
        }

        // pending means completion will arrive asynchronously
        let error = unsafe { GetLastError() };
        if error == ERROR_IO_PENDING {
            return Ok(true);
        }

        // reclaim operation state and report synchronous failure
        unsafe {
            let operation = Box::from_raw(operation_ptr);
            self.remove_inflight_operation(operation.request.token);
            let completion = io_error_completion(operation.request, error as i32);
            post_completion(self.port, completion)?;
        }

        Ok(true)
    }

    /// Submit one accept operation through AcceptEx.
    fn submit_accept_overlapped(&mut self, request: ProactorRequest) -> RuntimeResult<bool> {
        // decode request payload
        let (listen_handle, address_out) = match request.op {
            ProactorOp::Accept { handle, address } => (handle, address),
            _ => return Ok(false),
        };

        // create one accepted socket with the listener family
        let listen_socket = listen_handle.0 as SOCKET;
        let accepted_socket = create_accept_socket(listen_socket).map_err(|error| {
            RuntimeError::from(PlatformError::net_with(
                Some(PlatformErrorCode::Net),
                None,
                Some(error),
                Some("AcceptEx".to_string()),
                None,
                None,
                "failed to create accepted socket".to_string(),
            ))
            .boxed()
        })?;

        // ensure both handles are associated with this completion port
        self.associate_handle(listen_handle)?;
        self.associate_handle(PlatformHandle(accepted_socket as u64))?;

        // allocate operation state
        let mut operation = Box::new(IocpTask::new(request));
        operation.accept_socket = Some(accepted_socket);
        operation.accept_listen_socket = Some(listen_socket);
        operation.accept_output = vec![0u8; ACCEPT_EX_ADDRESS_BYTES * 2];
        operation.accept_local_len = ACCEPT_EX_ADDRESS_BYTES as u32;
        operation.accept_remote_len = ACCEPT_EX_ADDRESS_BYTES as u32;
        operation.accept_address_out = address_out;
        let token = operation.request.token;
        let tracking_handle = listen_socket as HANDLE;
        let operation_ptr = Box::into_raw(operation);
        self.register_inflight_operation(token, tracking_handle, operation_ptr.cast());

        // submit one accept request
        let submit_result = unsafe {
            AcceptEx(
                listen_socket,
                accepted_socket,
                (*operation_ptr).accept_output.as_mut_ptr().cast(),
                0,
                (*operation_ptr).accept_local_len,
                (*operation_ptr).accept_remote_len,
                std::ptr::null_mut(),
                &mut (*operation_ptr).overlapped,
            )
        };

        // immediate success still delivers one completion packet
        if submit_result != 0 {
            return Ok(true);
        }

        // pending means completion will arrive asynchronously
        let error = unsafe { WSAGetLastError() };
        if error == WSA_IO_PENDING {
            return Ok(true);
        }

        // reclaim operation state and close accepted socket on submit failure
        unsafe {
            let operation = Box::from_raw(operation_ptr);
            self.remove_inflight_operation(operation.request.token);
            if let Some(socket) = operation.accept_socket {
                closesocket(socket);
            }

            let completion = net_error_completion(operation.request, error);
            post_completion(self.port, completion)?;
        }

        Ok(true)
    }

    /// Submit one connect operation through ConnectEx.
    fn submit_connect_overlapped(&mut self, request: ProactorRequest) -> RuntimeResult<bool> {
        // decode request payload
        let (handle, address) = match request.op {
            ProactorOp::Connect { handle, address } => (handle, address),
            _ => return Ok(false),
        };

        // validate address payload
        if address.data.is_null() && address.len != 0 {
            let completion = invalid_argument_completion(request, PlatformErrorCode::NullPointer);
            post_completion(self.port, completion)?;
            return Ok(true);
        }

        // ensure socket is associated with this completion port
        self.associate_handle(handle)?;

        // resolve one connectex function pointer
        let socket = handle.0 as SOCKET;
        let connect_ex = load_connect_ex(socket)?;

        // ensure the socket is bound before connectex
        let address_family = socket_address_family(address);
        bind_socket_for_connect(socket, address_family).map_err(|error| {
            RuntimeError::from(PlatformError::net_with(
                Some(PlatformErrorCode::Net),
                None,
                Some(error),
                Some("bind".to_string()),
                None,
                None,
                "failed to bind socket before connectex".to_string(),
            ))
            .boxed()
        })?;

        // allocate operation state and copy the target address
        let mut operation = Box::new(IocpTask::new(request));
        if address.len != 0 {
            let source = unsafe { std::slice::from_raw_parts(address.data, address.len as usize) };
            operation.connect_address.extend_from_slice(source);
        }
        let token = operation.request.token;
        let tracking_handle = socket as HANDLE;
        let operation_ptr = Box::into_raw(operation);
        self.register_inflight_operation(token, tracking_handle, operation_ptr.cast());

        // choose the address pointer for connectex
        let (address_ptr, address_len) = unsafe {
            if (*operation_ptr).connect_address.is_empty() {
                (std::ptr::null(), 0)
            } else {
                (
                    (*operation_ptr).connect_address.as_ptr().cast::<SOCKADDR>(),
                    (*operation_ptr).connect_address.len() as i32,
                )
            }
        };

        // submit one connect request
        let submit_result = unsafe {
            connect_ex(
                socket,
                address_ptr,
                address_len,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut (*operation_ptr).overlapped,
            )
        };

        // immediate success still delivers one completion packet
        if submit_result != 0 {
            return Ok(true);
        }

        // pending means completion will arrive asynchronously
        let error = unsafe { WSAGetLastError() };
        if error == WSA_IO_PENDING {
            return Ok(true);
        }

        // reclaim operation state and report synchronous failure
        unsafe {
            let operation = Box::from_raw(operation_ptr);
            self.remove_inflight_operation(operation.request.token);
            let completion = net_error_completion(operation.request, error);
            post_completion(self.port, completion)?;
        }

        Ok(true)
    }

    /// Submit one accept or connect operation through native overlapped APIs.
    fn submit_accept_connect_overlapped(
        &mut self,
        request: ProactorRequest,
    ) -> RuntimeResult<bool> {
        // route accept requests through acceptex
        if matches!(request.op, ProactorOp::Accept { .. }) {
            return self.submit_accept_overlapped(request);
        }

        // route connect requests through connectex
        if matches!(request.op, ProactorOp::Connect { .. }) {
            return self.submit_connect_overlapped(request);
        }

        Ok(false)
    }

    /// Submit one poll registration that is checked in poll().
    fn submit_poll_registration(
        &mut self,
        request: ProactorRequest,
        handle: PlatformHandle,
        interests: PlatformInterest,
    ) {
        // append one pending poll registration
        self.pending_polls.push(PendingPoll {
            request,
            handle,
            interests,
        });
    }

    /// Check pending poll registrations and enqueue ready completions.
    fn drain_pending_polls(&mut self) {
        // preserve registrations that are still pending
        let mut still_pending = Vec::with_capacity(self.pending_polls.len());
        for registration in self.pending_polls.drain(..) {
            // drop canceled registrations and emit canceled completion
            let was_canceled = self
                .canceled_tokens
                .lock()
                .remove(&registration.request.token);
            if was_canceled {
                self.completions.push_back(canceled_completion(
                    registration.request.resource_id,
                    registration.request.token,
                    registration.request.op.kind(),
                ));
                continue;
            }

            // check readiness using one nonblocking wsapoll probe
            let readiness = poll_socket_readiness(registration.handle, registration.interests);
            match readiness {
                Ok(None) => still_pending.push(registration),
                Ok(Some(mask)) => {
                    let mut completion = success_completion(registration.request, 1);
                    completion.data = ProactorCompletionData::Poll { mask };
                    self.completions.push_back(completion);
                }
                Err(error) => {
                    self.completions
                        .push_back(net_error_completion(registration.request, error));
                }
            }
        }

        self.pending_polls = still_pending;
    }

    /// Register one in-flight native overlapped operation by token.
    fn register_inflight_operation(
        &mut self,
        token: u64,
        handle: HANDLE,
        overlapped: *mut OVERLAPPED,
    ) {
        self.inflight_operations.insert(
            token,
            InflightTask {
                handle,
                overlapped: overlapped as usize,
            },
        );
    }

    /// Remove one in-flight native overlapped operation by token.
    fn remove_inflight_operation(&mut self, token: u64) -> Option<InflightTask> {
        self.inflight_operations.remove(&token)
    }

    /// Wait for one completion and optionally decode it.
    fn wait_one(&mut self, timeout_millis: u32) -> RuntimeResult<Option<ProactorCompletion>> {
        // wait for one completion packet
        let mut transferred: u32 = 0;
        let mut completion_key: usize = 0;
        let mut overlapped: *mut OVERLAPPED = std::ptr::null_mut();
        let status = unsafe {
            GetQueuedCompletionStatus(
                self.port,
                &mut transferred,
                &mut completion_key,
                &mut overlapped,
                timeout_millis,
            )
        };

        // handle timeout separately
        if status == 0 {
            let error = unsafe { GetLastError() };
            if error == WAIT_TIMEOUT {
                return Ok(None);
            }

            if overlapped.is_null() {
                return Err(core_platform::io_error_with_code(
                    "GetQueuedCompletionStatus",
                    error as i32,
                ));
            }
        }

        // drop wake notifications
        if completion_key == WAKE_COMPLETION_KEY && overlapped.is_null() {
            return Ok(None);
        }

        // decode native overlapped operation completions
        if completion_key == OPERATION_COMPLETION_KEY && !overlapped.is_null() {
            // reclaim operation state and convert completion result
            let operation = unsafe { *Box::from_raw(overlapped as *mut IocpTask) };
            self.remove_inflight_operation(operation.request.token);

            // surface cancellations consistently for native overlapped operations
            let was_canceled = self.canceled_tokens.lock().remove(&operation.request.token);
            if was_canceled {
                cleanup_canceled_operation(&operation);
                let completion = canceled_completion(
                    operation.request.resource_id,
                    operation.request.token,
                    operation.request.op.kind(),
                );
                return Ok(Some(completion));
            }

            let completion = completion_from_operation(operation, status != 0, transferred);

            return Ok(Some(completion));
        }

        // decode posted completion payloads
        if completion_key == MESSAGE_COMPLETION_KEY && !overlapped.is_null() {
            let message = unsafe { Box::from_raw(overlapped as *mut IocpMessage) };
            return Ok(Some(message.completion));
        }

        // reject unknown completion payloads
        Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("GetQueuedCompletionStatus".to_string()),
            None,
            "invalid iocp completion payload".to_string(),
        ))
        .boxed())
    }
}

impl Drop for IocpProactor {
    fn drop(&mut self) {
        // close completion port handle
        unsafe {
            CloseHandle(self.port);
        }
    }
}

impl Proactor for IocpProactor {
    fn submit(&mut self, request: ProactorRequest) -> RuntimeResult<()> {
        // dispatch by operation kind
        match request.op {
            // submit timeout requests via helper threads
            ProactorOp::Timeout { timeout_ns } => {
                self.submit_timeout(request.resource_id, request.token, timeout_ns)
            }

            // route explicit cancel requests through cancel()
            ProactorOp::Cancel { token } => self.cancel(token),

            // submit recv/send style operations through native overlapped IOCP
            ProactorOp::Recv { .. }
            | ProactorOp::Send { .. }
            | ProactorOp::RecvFrom { .. }
            | ProactorOp::SendTo { .. }
            | ProactorOp::Read { offset: None, .. }
            | ProactorOp::Write { offset: None, .. }
            | ProactorOp::Readv { offset: None, .. }
            | ProactorOp::Writev { offset: None, .. } => {
                let did_submit = self.submit_socket_overlapped(request)?;
                if did_submit {
                    return Ok(());
                }

                self.submit_threaded_operation(request)
            }

            // submit read and write requests through overlapped file io when possible
            ProactorOp::Read { .. } | ProactorOp::Write { .. } => {
                let did_submit = self.submit_file_overlapped(request)?;
                if did_submit {
                    return Ok(());
                }

                self.submit_threaded_operation(request)
            }

            // submit accept and connect through native socket extension apis
            ProactorOp::Accept { .. } | ProactorOp::Connect { .. } => {
                let did_submit = self.submit_accept_connect_overlapped(request)?;
                if did_submit {
                    return Ok(());
                }

                self.submit_threaded_operation(request)
            }

            // register poll operations in the frontend loop instead of helper threads
            ProactorOp::Poll { handle, interests } => {
                self.submit_poll_registration(request, handle, interests);
                Ok(())
            }

            // execute simple synchronous operations inline and enqueue completion
            ProactorOp::Fsync { .. }
            | ProactorOp::Fdatasync { .. }
            | ProactorOp::Shutdown { .. }
            | ProactorOp::Close { .. } => {
                let completion = execute_request(request);
                self.completions.push_back(completion);
                Ok(())
            }

            // submit all other operations via worker helpers
            _ => self.submit_threaded_operation(request),
        }
    }

    fn cancel(&mut self, token: u64) -> RuntimeResult<()> {
        // mark token as canceled for helpers
        self.canceled_tokens.lock().insert(token);

        // cancel in-flight native overlapped operations via win32 api
        if let Some(operation) = self.inflight_operations.get(&token).copied() {
            // cancel specific overlapped request for this handle
            unsafe {
                CancelIoEx(operation.handle, operation.overlapped as *const OVERLAPPED);
            }
        }

        // clear pending timeout state when present and emit cancellation
        if let Some(resource_id) = self.pending_timeouts.lock().remove(&token) {
            self.canceled_tokens.lock().remove(&token);
            self.completions.push_back(canceled_completion(
                resource_id,
                token,
                ProactorOpKind::Timeout,
            ));
        }

        // clear pending poll registrations and emit cancellation
        let mut still_pending = Vec::with_capacity(self.pending_polls.len());
        for registration in self.pending_polls.drain(..) {
            if registration.request.token == token {
                self.completions.push_back(canceled_completion(
                    registration.request.resource_id,
                    registration.request.token,
                    registration.request.op.kind(),
                ));
            } else {
                still_pending.push(registration);
            }
        }
        self.pending_polls = still_pending;

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<ProactorCompletion>> {
        // compute one optional deadline for this poll call
        let deadline = timeout_nanos.map(|timeout| Instant::now() + Duration::from_nanos(timeout));

        // drain any ready completions first
        while let Some(completion) = self.wait_one(0)? {
            self.completions.push_back(completion);
        }
        self.drain_pending_polls();

        // wait for one completion when queue is empty
        while self.completions.is_empty() {
            // choose one wait slice: short slices while polls are pending
            let wait_millis = if self.pending_polls.is_empty() {
                timeout_nanos
                    .map(timeout_millis_from_nanos)
                    .unwrap_or(INFINITE)
            } else {
                match deadline {
                    Some(deadline) => {
                        let now = Instant::now();
                        if now >= deadline {
                            break;
                        }
                        let remaining = deadline.duration_since(now);
                        let pending_poll_slice = Duration::from_nanos(self.pending_poll_slice_ns);
                        let slice = remaining.min(pending_poll_slice);
                        timeout_millis_from_nanos(slice.as_nanos() as u64)
                    }
                    None => timeout_millis_from_nanos(self.pending_poll_slice_ns),
                }
            };

            // wait for one iocp packet in this slice
            if let Some(completion) = self.wait_one(wait_millis)? {
                self.completions.push_back(completion);
            }

            // check pending poll registrations after each wait slice
            self.drain_pending_polls();

            // stop once one finite timeout window elapsed
            if let Some(deadline) = deadline
                && Instant::now() >= deadline
                && self.completions.is_empty()
            {
                break;
            }
        }

        // drain immediately available completions
        while let Some(completion) = self.wait_one(0)? {
            self.completions.push_back(completion);
        }
        self.drain_pending_polls();

        // flush buffered completions into output
        let mut output = Vec::with_capacity(self.completions.len());
        while let Some(completion) = self.completions.pop_front() {
            output.push(completion);
        }

        Ok(output)
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        // post one wake packet into the completion queue
        let status = unsafe {
            PostQueuedCompletionStatus(self.port, 0, WAKE_COMPLETION_KEY, std::ptr::null_mut())
        };
        if status == 0 {
            return Err(core_platform::io_error("PostQueuedCompletionStatus"));
        }

        Ok(())
    }
}

/// Execute one request operation and return one completion.
fn execute_request(request: ProactorRequest) -> ProactorCompletion {
    // execute one operation variant
    match request.op {
        // read bytes from socket or file handle
        ProactorOp::Read {
            handle,
            buffer,
            offset,
        } => execute_read(request, handle, buffer.data, buffer.len, offset),

        // write bytes to socket or file handle
        ProactorOp::Write {
            handle,
            buffer,
            offset,
        } => execute_write(
            request,
            handle,
            buffer.data.cast_const(),
            buffer.len,
            offset,
        ),

        // read bytes into multiple buffers
        ProactorOp::Readv {
            handle,
            buffers,
            offset,
        } => execute_readv(request, handle, buffers, offset),

        // write bytes from multiple buffers
        ProactorOp::Writev {
            handle,
            buffers,
            offset,
        } => execute_writev(request, handle, buffers, offset),

        // receive payload from socket
        ProactorOp::Recv {
            handle,
            buffer,
            flags,
        } => execute_recv(request, handle, buffer.data, buffer.len, flags),

        // send payload to socket
        ProactorOp::Send {
            handle,
            buffer,
            flags,
        } => execute_send(request, handle, buffer.data.cast_const(), buffer.len, flags),

        // receive datagram and source address
        ProactorOp::RecvFrom {
            handle,
            buffer,
            flags,
            address,
        } => execute_recvfrom(request, handle, buffer.data, buffer.len, flags, address),

        // send datagram to destination address
        ProactorOp::SendTo {
            handle,
            buffer,
            flags,
            address,
        } => execute_sendto(
            request,
            handle,
            buffer.data.cast_const(),
            buffer.len,
            flags,
            address,
        ),

        // copy one byte range between file handles
        ProactorOp::CopyFileRange {
            input,
            output,
            input_offset,
            output_offset,
            len,
            flags: _,
        } => execute_copy_file_range(request, input, output, input_offset, output_offset, len),

        // splice one byte range between handles
        ProactorOp::Splice {
            input,
            output,
            input_offset,
            output_offset,
            len,
            flags: _,
        } => execute_splice(request, input, output, input_offset, output_offset, len),

        // send file bytes through one socket
        ProactorOp::SendFile {
            output,
            input,
            input_offset,
            len,
        } => execute_sendfile(request, output, input, input_offset, len),

        // flush file data and metadata
        ProactorOp::Fsync { handle } => execute_fsync(request, handle),

        // flush file data and metadata
        ProactorOp::Fdatasync { handle } => execute_fsync(request, handle),

        // preallocate file storage where available
        ProactorOp::Fallocate {
            handle,
            offset,
            len,
            mode: _,
        } => execute_fallocate(request, handle, offset, len),

        // accept one incoming connection
        ProactorOp::Accept { handle, address } => execute_accept(request, handle, address),

        // connect one socket
        ProactorOp::Connect { handle, address } => execute_connect(request, handle, address),

        // poll one socket readiness
        ProactorOp::Poll { handle, interests } => execute_poll(request, handle, interests),

        // shutdown one socket
        ProactorOp::Shutdown { handle, how } => execute_shutdown(request, handle, how),

        // close one socket or handle
        ProactorOp::Close { handle } => execute_close(request, handle),

        // cancel is handled by cancel(), not submit()
        ProactorOp::Cancel { .. } => not_supported_completion(request, "proactor.iocp.cancel"),

        // timeout is handled by submit_timeout(), not execute_request()
        ProactorOp::Timeout { .. } => not_supported_completion(request, "proactor.iocp.timeout"),
    }
}

/// Execute one read operation.
fn execute_read(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *mut u8,
    buffer_len: u32,
    offset: Option<u64>,
) -> ProactorCompletion {
    // first try socket receive semantics for non-positional reads
    if offset.is_none() {
        let socket = handle.0 as SOCKET;
        let result = unsafe { recv(socket, buffer.cast(), buffer_len as i32, 0) };
        if result >= 0 {
            return success_completion(request, result);
        }

        // only fall back to file read when handle is not a socket
        let wsa_error = unsafe { WSAGetLastError() };
        if wsa_error != WSAENOTSOCK {
            return net_error_completion(request, wsa_error);
        }
    }

    // perform file read
    let mut bytes_read = 0u32;
    // safety: zeroed overlapped is valid initialization
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    let overlapped_ptr = if let Some(offset) = offset {
        overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
        overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        &mut overlapped as *mut OVERLAPPED
    } else {
        std::ptr::null_mut()
    };
    let status = unsafe {
        ReadFile(
            handle.0 as HANDLE,
            buffer.cast(),
            buffer_len,
            &mut bytes_read,
            overlapped_ptr,
        )
    };
    if status == 0 {
        let code = unsafe { GetLastError() };
        return io_error_completion(request, code as i32);
    }

    success_completion(request, bytes_read as i32)
}

/// Execute one write operation.
fn execute_write(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *const u8,
    buffer_len: u32,
    offset: Option<u64>,
) -> ProactorCompletion {
    // first try socket send semantics for non-positional writes
    if offset.is_none() {
        let socket = handle.0 as SOCKET;
        let result = unsafe { send(socket, buffer.cast(), buffer_len as i32, 0) };
        if result >= 0 {
            return success_completion(request, result);
        }

        // only fall back to file write when handle is not a socket
        let wsa_error = unsafe { WSAGetLastError() };
        if wsa_error != WSAENOTSOCK {
            return net_error_completion(request, wsa_error);
        }
    }

    // perform file write
    let mut bytes_written = 0u32;
    // safety: zeroed overlapped is valid initialization
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    let overlapped_ptr = if let Some(offset) = offset {
        overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
        overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        &mut overlapped as *mut OVERLAPPED
    } else {
        std::ptr::null_mut()
    };
    let status = unsafe {
        WriteFile(
            handle.0 as HANDLE,
            buffer.cast(),
            buffer_len,
            &mut bytes_written,
            overlapped_ptr,
        )
    };
    if status == 0 {
        let code = unsafe { GetLastError() };
        return io_error_completion(request, code as i32);
    }

    success_completion(request, bytes_written as i32)
}

/// Execute one readv operation.
fn execute_readv(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffers: ProactorBufferVec,
    offset: Option<u64>,
) -> ProactorCompletion {
    // validate buffer vector pointer
    if buffers.data.is_null() && buffers.len != 0 {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }

    // decode buffer list and execute reads sequentially
    let list = unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
    let mut current_offset = offset;
    let mut total = 0i32;
    for buffer in list {
        let completion = execute_read(request, handle, buffer.data, buffer.len, current_offset);
        if completion.result < 0 {
            return completion;
        }

        total = total.saturating_add(completion.result);
        if completion.result < buffer.len as i32 {
            break;
        }

        // advance file offset for positional vectored io
        if let Some(offset) = current_offset {
            current_offset = Some(offset.saturating_add(buffer.len as u64));
        }
    }

    success_completion(request, total)
}

/// Execute one writev operation.
fn execute_writev(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffers: ProactorBufferVec,
    offset: Option<u64>,
) -> ProactorCompletion {
    // validate buffer vector pointer
    if buffers.data.is_null() && buffers.len != 0 {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }

    // decode buffer list and execute writes sequentially
    let list = unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
    let mut current_offset = offset;
    let mut total = 0i32;
    for buffer in list {
        let completion = execute_write(
            request,
            handle,
            buffer.data.cast_const(),
            buffer.len,
            current_offset,
        );
        if completion.result < 0 {
            return completion;
        }

        total = total.saturating_add(completion.result);
        if completion.result < buffer.len as i32 {
            break;
        }

        // advance file offset for positional vectored io
        if let Some(offset) = current_offset {
            current_offset = Some(offset.saturating_add(buffer.len as u64));
        }
    }

    success_completion(request, total)
}

/// Execute one recv operation.
fn execute_recv(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *mut u8,
    buffer_len: u32,
    flags: u32,
) -> ProactorCompletion {
    let socket = handle.0 as SOCKET;
    let result = unsafe { recv(socket, buffer.cast(), buffer_len as i32, flags as i32) };
    if result < 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    let mut completion = success_completion(request, result);
    completion.data = ProactorCompletionData::Recv {
        addr_len: 0,
        flags: 0,
    };
    completion
}

/// Execute one send operation.
fn execute_send(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *const u8,
    buffer_len: u32,
    flags: u32,
) -> ProactorCompletion {
    let socket = handle.0 as SOCKET;
    let result = unsafe { send(socket, buffer.cast(), buffer_len as i32, flags as i32) };
    if result < 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    success_completion(request, result)
}

/// Execute one recvfrom operation.
fn execute_recvfrom(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *mut u8,
    buffer_len: u32,
    flags: u32,
    address: ProactorAddressStorage,
) -> ProactorCompletion {
    // validate address output pointers
    if address.data.is_null() {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }
    if address.len.is_null() {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }

    // decode current sockaddr capacity
    let mut address_len = unsafe { *address.len as i32 };
    let socket = handle.0 as SOCKET;
    let result = unsafe {
        recvfrom(
            socket,
            buffer.cast(),
            buffer_len as i32,
            flags as i32,
            address.data.cast(),
            &mut address_len,
        )
    };
    if result < 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    // write final sockaddr length
    unsafe {
        *address.len = address_len as u32;
    }

    let mut completion = success_completion(request, result);
    completion.data = ProactorCompletionData::Recv {
        addr_len: address_len as u32,
        flags: 0,
    };
    completion
}

/// Execute one sendto operation.
fn execute_sendto(
    request: ProactorRequest,
    handle: PlatformHandle,
    buffer: *const u8,
    buffer_len: u32,
    flags: u32,
    address: ProactorAddress,
) -> ProactorCompletion {
    // validate address input pointer
    if address.data.is_null() && address.len != 0 {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }

    let socket = handle.0 as SOCKET;
    let result = unsafe {
        sendto(
            socket,
            buffer.cast(),
            buffer_len as i32,
            flags as i32,
            address.data.cast(),
            address.len as i32,
        )
    };
    if result < 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    success_completion(request, result)
}

/// Execute one copy_file_range operation.
fn execute_copy_file_range(
    request: ProactorRequest,
    input: PlatformHandle,
    output: PlatformHandle,
    input_offset: Option<u64>,
    output_offset: Option<u64>,
    len: u64,
) -> ProactorCompletion {
    // complete immediately for zero-length copies
    if len == 0 {
        return success_completion(request, 0);
    }

    // process copy in fixed-size chunks
    let mut remaining = len;
    let mut read_offset = input_offset;
    let mut write_offset = output_offset;
    let mut copied_total = 0u64;
    let mut buffer = vec![0u8; COPY_FILE_RANGE_CHUNK_BYTES as usize];
    while remaining != 0 {
        // choose one chunk bounded by remaining bytes
        let chunk_len = remaining.min(COPY_FILE_RANGE_CHUNK_BYTES as u64) as u32;

        // read one chunk from the input handle
        let read_completion =
            execute_read(request, input, buffer.as_mut_ptr(), chunk_len, read_offset);
        if read_completion.result < 0 {
            return read_completion;
        }

        let bytes_read = read_completion.result as u32;
        if bytes_read == 0 {
            break;
        }

        // write the chunk to the output handle
        let write_completion =
            execute_write(request, output, buffer.as_ptr(), bytes_read, write_offset);
        if write_completion.result < 0 {
            return write_completion;
        }

        let bytes_written = write_completion.result as u32;
        copied_total = copied_total.saturating_add(bytes_written as u64);
        remaining = remaining.saturating_sub(bytes_written as u64);

        // advance optional positional offsets
        if let Some(offset) = read_offset {
            read_offset = Some(offset.saturating_add(bytes_written as u64));
        }
        if let Some(offset) = write_offset {
            write_offset = Some(offset.saturating_add(bytes_written as u64));
        }

        // stop on partial writes
        if bytes_written < bytes_read {
            break;
        }
    }

    success_completion(request, copied_total.min(i32::MAX as u64) as i32)
}

/// Execute one splice operation.
fn execute_splice(
    request: ProactorRequest,
    input: PlatformHandle,
    output: PlatformHandle,
    input_offset: Option<u64>,
    output_offset: Option<u64>,
    len: u64,
) -> ProactorCompletion {
    // complete immediately for zero-length splice operations
    if len == 0 {
        return success_completion(request, 0);
    }

    // transfer payload in fixed-size chunks
    let mut remaining = len;
    let mut read_offset = input_offset;
    let mut write_offset = output_offset;
    let mut moved_total = 0u64;
    let mut buffer = vec![0u8; COPY_FILE_RANGE_CHUNK_BYTES as usize];
    while remaining != 0 {
        // choose one transfer chunk
        let chunk_len = remaining.min(COPY_FILE_RANGE_CHUNK_BYTES as u64) as u32;

        // read one chunk from the input handle
        let read_completion =
            execute_read(request, input, buffer.as_mut_ptr(), chunk_len, read_offset);
        if read_completion.result < 0 {
            return read_completion;
        }

        let bytes_read = read_completion.result as u32;
        if bytes_read == 0 {
            break;
        }

        // write one chunk to the output handle
        let write_completion =
            execute_write(request, output, buffer.as_ptr(), bytes_read, write_offset);
        if write_completion.result < 0 {
            return write_completion;
        }

        let bytes_written = write_completion.result as u32;
        moved_total = moved_total.saturating_add(bytes_written as u64);
        remaining = remaining.saturating_sub(bytes_written as u64);

        // advance optional positional offsets
        if let Some(offset) = read_offset {
            read_offset = Some(offset.saturating_add(bytes_written as u64));
        }
        if let Some(offset) = write_offset {
            write_offset = Some(offset.saturating_add(bytes_written as u64));
        }

        // stop on partial writes
        if bytes_written < bytes_read {
            break;
        }
    }

    success_completion(request, moved_total.min(i32::MAX as u64) as i32)
}

/// Execute one sendfile operation.
fn execute_sendfile(
    request: ProactorRequest,
    output: PlatformHandle,
    input: PlatformHandle,
    input_offset: Option<u64>,
    len: u64,
) -> ProactorCompletion {
    // transmit all remaining file bytes when len is zero
    if len == 0 {
        // safety: zeroed overlapped is valid initialization
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        let overlapped_ptr = if let Some(offset) = input_offset {
            overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
            overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
            &mut overlapped as *mut OVERLAPPED
        } else {
            std::ptr::null_mut()
        };

        let status = unsafe {
            TransmitFile(
                output.0 as SOCKET,
                input.0 as HANDLE,
                0,
                0,
                overlapped_ptr,
                std::ptr::null::<TRANSMIT_FILE_BUFFERS>(),
                0,
            )
        };
        if status == 0 {
            return net_error_completion(request, unsafe { WSAGetLastError() });
        }

        return success_completion(request, 0);
    }

    // transmit in bounded chunks for large sends
    let mut remaining = len;
    let mut current_offset = input_offset;
    let mut sent_total = 0u64;
    while remaining != 0 {
        // choose one transmit chunk
        let chunk = remaining.min(u32::MAX as u64) as u32;

        // safety: zeroed overlapped is valid initialization
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        let overlapped_ptr = if let Some(offset) = current_offset {
            overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFF_FFFF) as u32;
            overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
            &mut overlapped as *mut OVERLAPPED
        } else {
            std::ptr::null_mut()
        };

        // transmit one chunk through the socket
        let status = unsafe {
            TransmitFile(
                output.0 as SOCKET,
                input.0 as HANDLE,
                chunk,
                0,
                overlapped_ptr,
                std::ptr::null::<TRANSMIT_FILE_BUFFERS>(),
                0,
            )
        };
        if status == 0 {
            return net_error_completion(request, unsafe { WSAGetLastError() });
        }

        sent_total = sent_total.saturating_add(chunk as u64);
        remaining = remaining.saturating_sub(chunk as u64);

        // advance positional offset when requested
        if let Some(offset) = current_offset {
            current_offset = Some(offset.saturating_add(chunk as u64));
        }
    }

    success_completion(request, sent_total.min(i32::MAX as u64) as i32)
}

/// Execute one fallocate operation.
fn execute_fallocate(
    request: ProactorRequest,
    handle: PlatformHandle,
    offset: u64,
    len: u64,
) -> ProactorCompletion {
    // complete immediately for zero-length allocations
    if len == 0 {
        return success_completion(request, 0);
    }

    // reject offset and length combinations that overflow
    let Some(target_size) = offset.checked_add(len) else {
        return invalid_argument_completion(request, PlatformErrorCode::IoInvalidData);
    };
    if target_size > i64::MAX as u64 {
        return invalid_argument_completion(request, PlatformErrorCode::IoInvalidData);
    }

    // query current file size to avoid shrinking allocation
    // safety: zeroed file info struct is valid before API write
    let mut standard_info: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
    let status = unsafe {
        GetFileInformationByHandleEx(
            handle.0 as HANDLE,
            FileStandardInfo,
            (&mut standard_info as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )
    };
    if status == 0 {
        return io_error_completion(request, unsafe { GetLastError() } as i32);
    }

    let current_size = if standard_info.EndOfFile < 0 {
        0u64
    } else {
        standard_info.EndOfFile as u64
    };
    let allocation_size = current_size.max(target_size) as i64;

    // update allocation size through file information api
    let allocation_info = FILE_ALLOCATION_INFO {
        AllocationSize: allocation_size,
    };
    let status = unsafe {
        SetFileInformationByHandle(
            handle.0 as HANDLE,
            FileAllocationInfo,
            (&allocation_info as *const FILE_ALLOCATION_INFO).cast(),
            size_of::<FILE_ALLOCATION_INFO>() as u32,
        )
    };
    if status == 0 {
        return io_error_completion(request, unsafe { GetLastError() } as i32);
    }

    success_completion(request, 0)
}

/// Execute one fsync operation.
fn execute_fsync(request: ProactorRequest, handle: PlatformHandle) -> ProactorCompletion {
    let status = unsafe { FlushFileBuffers(handle.0 as HANDLE) };
    if status == 0 {
        let code = unsafe { GetLastError() };
        return io_error_completion(request, code as i32);
    }

    success_completion(request, 0)
}

/// Execute one accept operation.
fn execute_accept(
    request: ProactorRequest,
    handle: PlatformHandle,
    address: Option<ProactorAddressStorage>,
) -> ProactorCompletion {
    // prepare optional address pointers
    let (address_ptr, address_len_ptr, output_len_ptr) = match address {
        Some(address) => {
            if address.data.is_null() || address.len.is_null() {
                return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
            }

            let address_len = unsafe { *address.len as i32 };
            (address.data.cast(), Some(address_len), Some(address.len))
        }
        None => (std::ptr::null_mut(), None, None),
    };

    // perform accept call
    let mut address_len = address_len_ptr.unwrap_or(0);
    let socket = handle.0 as SOCKET;
    let accepted = unsafe { accept(socket, address_ptr, &mut address_len) };
    if accepted == INVALID_SOCKET {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    // write output address length when requested
    if let Some(output_len_ptr) = output_len_ptr {
        unsafe {
            *output_len_ptr = address_len as u32;
        }
    }

    let mut completion = success_completion(request, accepted as i32);
    completion.data = ProactorCompletionData::Accept {
        handle: PlatformHandle(accepted as u64),
        addr_len: address_len as u32,
    };
    completion
}

/// Execute one connect operation.
fn execute_connect(
    request: ProactorRequest,
    handle: PlatformHandle,
    address: ProactorAddress,
) -> ProactorCompletion {
    // validate address pointer
    if address.data.is_null() && address.len != 0 {
        return invalid_argument_completion(request, PlatformErrorCode::NullPointer);
    }

    let socket = handle.0 as SOCKET;
    let result = unsafe { connect(socket, address.data.cast(), address.len as i32) };
    if result != 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    success_completion(request, 0)
}

/// Execute one poll operation.
fn execute_poll(
    request: ProactorRequest,
    handle: PlatformHandle,
    interests: PlatformInterest,
) -> ProactorCompletion {
    // check readiness using one zero-timeout wsapoll probe
    let readiness = poll_socket_readiness(handle, interests);
    let mask = match readiness {
        Ok(Some(mask)) => mask,
        Ok(None) => PollerEventMask::NONE,
        Err(error) => return net_error_completion(request, error),
    };

    let mut completion = success_completion(request, 1);
    completion.data = ProactorCompletionData::Poll { mask };
    completion
}

/// Check socket readiness with one nonblocking WSAPoll call.
fn poll_socket_readiness(
    handle: PlatformHandle,
    interests: PlatformInterest,
) -> Result<Option<PollerEventMask>, i32> {
    // build one wsapoll events mask from requested interests
    let mut events: i16 = 0;
    if interests.contains(PlatformInterest::READABLE) {
        events |= POLLIN;
    }
    if interests.contains(PlatformInterest::WRITABLE) {
        events |= POLLOUT;
    }

    // execute one zero-timeout readiness probe
    let mut pollfd = WSAPOLLFD {
        fd: handle.0 as SOCKET,
        events,
        revents: 0,
    };
    let result = unsafe { WSAPoll(&mut pollfd, 1, 0) };
    if result < 0 {
        return Err(unsafe { WSAGetLastError() });
    }
    if result == 0 {
        return Ok(None);
    }

    // decode ready event bits from wsapoll
    let mut mask = PollerEventMask::NONE;
    if (pollfd.revents & POLLIN) != 0 {
        mask |= PollerEventMask::READABLE;
    }
    if (pollfd.revents & POLLOUT) != 0 {
        mask |= PollerEventMask::WRITABLE;
    }
    if (pollfd.revents & POLLERR) != 0 {
        mask |= PollerEventMask::ERROR;
    }
    if (pollfd.revents & POLLHUP) != 0 {
        mask |= PollerEventMask::HANGUP;
    }
    if (pollfd.revents & POLLPRI) != 0 {
        mask |= PollerEventMask::PRIORITY;
    }

    Ok(Some(mask))
}

/// Execute one shutdown operation.
fn execute_shutdown(
    request: ProactorRequest,
    handle: PlatformHandle,
    how: ProactorShutdown,
) -> ProactorCompletion {
    let shutdown_how = match how {
        ProactorShutdown::Read => 0,
        ProactorShutdown::Write => 1,
        ProactorShutdown::Both => 2,
    };

    let socket = handle.0 as SOCKET;
    let result = unsafe { shutdown(socket, shutdown_how) };
    if result != 0 {
        return net_error_completion(request, unsafe { WSAGetLastError() });
    }

    success_completion(request, 0)
}

/// Execute one close operation.
fn execute_close(request: ProactorRequest, handle: PlatformHandle) -> ProactorCompletion {
    // first try socket close semantics
    let socket = handle.0 as SOCKET;
    let result = unsafe { closesocket(socket) };
    if result == 0 {
        return success_completion(request, 0);
    }

    // only fall back to handle close when value is not a socket
    let wsa_error = unsafe { WSAGetLastError() };
    if wsa_error != WSAENOTSOCK {
        return net_error_completion(request, wsa_error);
    }

    // close as a generic handle
    let status = unsafe { CloseHandle(handle.0 as HANDLE) };
    if status == 0 {
        let code = unsafe { GetLastError() };
        return io_error_completion(request, code as i32);
    }

    success_completion(request, 0)
}

/// Build one completion from native overlapped operation state.
fn completion_from_operation(
    mut operation: IocpTask,
    is_success: bool,
    transferred: u32,
) -> ProactorCompletion {
    // build base completion from status
    let mut completion = if is_success {
        success_completion(operation.request, transferred as i32)
    } else {
        let error = unsafe { GetLastError() } as i32;
        if matches!(
            operation.request.op.kind(),
            ProactorOpKind::Recv
                | ProactorOpKind::Send
                | ProactorOpKind::RecvFrom
                | ProactorOpKind::SendTo
        ) {
            net_error_completion(operation.request, error)
        } else {
            io_error_completion(operation.request, error)
        }
    };

    // attach operation-specific completion payload
    match operation.request.op.kind() {
        ProactorOpKind::Recv => {
            completion.data = ProactorCompletionData::Recv {
                addr_len: 0,
                flags: operation.recv_flags,
            };
        }
        ProactorOpKind::RecvFrom => {
            if let Some(address_len_out) = operation.recvfrom_len_out.take() {
                unsafe {
                    *address_len_out = operation.recvfrom_len as u32;
                }
            }

            completion.data = ProactorCompletionData::Recv {
                addr_len: operation.recvfrom_len as u32,
                flags: operation.recv_flags,
            };
        }

        ProactorOpKind::Accept if is_success => {
            // require accepted and listener sockets from operation state
            let Some(accepted_socket) = operation.accept_socket.take() else {
                return invalid_argument_completion(
                    operation.request,
                    PlatformErrorCode::IoInvalidData,
                );
            };
            let Some(listen_socket) = operation.accept_listen_socket else {
                unsafe {
                    closesocket(accepted_socket);
                }
                return invalid_argument_completion(
                    operation.request,
                    PlatformErrorCode::IoInvalidData,
                );
            };

            // attach accept context to the accepted socket
            let listen_socket_bytes = listen_socket.to_ne_bytes();
            let update_accept_context_status = unsafe {
                setsockopt(
                    accepted_socket,
                    SOL_SOCKET,
                    SO_UPDATE_ACCEPT_CONTEXT,
                    listen_socket_bytes.as_ptr(),
                    size_of::<SOCKET>() as i32,
                )
            };
            if update_accept_context_status != 0 {
                let error = unsafe { WSAGetLastError() };
                unsafe {
                    closesocket(accepted_socket);
                }
                return net_error_completion(operation.request, error);
            }

            // decode remote peer address from acceptex output
            let mut remote_addr_len = 0u32;
            if let Some(address_out) = operation.accept_address_out.take()
                && !address_out.data.is_null()
                && !address_out.len.is_null()
            {
                let decoded =
                    decode_acceptex_remote_address(&operation, address_out.data, address_out.len);
                match decoded {
                    Ok(addr_len) => {
                        remote_addr_len = addr_len;
                    }
                    Err(error) => {
                        unsafe {
                            closesocket(accepted_socket);
                        }
                        return net_error_completion(operation.request, error);
                    }
                }
            }

            completion.data = ProactorCompletionData::Accept {
                handle: PlatformHandle(accepted_socket as u64),
                addr_len: remote_addr_len,
            };
        }

        ProactorOpKind::Connect if is_success => {
            // extract socket handle from request payload
            let socket = match operation.request.op {
                ProactorOp::Connect { handle, .. } => handle.0 as SOCKET,
                _ => {
                    return invalid_argument_completion(
                        operation.request,
                        PlatformErrorCode::IoInvalidData,
                    );
                }
            };

            // attach connect context to the connected socket
            let update_connect_context_status = unsafe {
                setsockopt(
                    socket,
                    SOL_SOCKET,
                    SO_UPDATE_CONNECT_CONTEXT,
                    std::ptr::null(),
                    0,
                )
            };
            if update_connect_context_status != 0 {
                let error = unsafe { WSAGetLastError() };
                return net_error_completion(operation.request, error);
            }
        }

        ProactorOpKind::Accept if !is_success => {
            // close accepted socket when accept failed
            if let Some(socket) = operation.accept_socket.take() {
                unsafe {
                    closesocket(socket);
                }
            }
        }

        _ => {}
    }

    completion
}

/// Copy the remote peer address from one acceptex output buffer.
fn decode_acceptex_remote_address(
    operation: &IocpTask,
    address_buffer: *mut u8,
    address_len_out: *mut u32,
) -> Result<u32, i32> {
    // parse local and remote sockaddr ranges from acceptex output
    let mut local_sockaddr: *mut SOCKADDR = std::ptr::null_mut();
    let mut local_sockaddr_len = 0i32;
    let mut remote_sockaddr: *mut SOCKADDR = std::ptr::null_mut();
    let mut remote_sockaddr_len = 0i32;
    unsafe {
        GetAcceptExSockaddrs(
            operation.accept_output.as_ptr().cast(),
            0,
            operation.accept_local_len,
            operation.accept_remote_len,
            &mut local_sockaddr,
            &mut local_sockaddr_len,
            &mut remote_sockaddr,
            &mut remote_sockaddr_len,
        );
    }

    // reject invalid remote sockaddr payloads
    if remote_sockaddr.is_null() || remote_sockaddr_len < 0 {
        return Err(unsafe { WSAGetLastError() });
    }

    // copy the remote sockaddr into caller storage with truncation to capacity
    let requested_capacity = unsafe { *address_len_out as usize };
    let available_len = remote_sockaddr_len as usize;
    let copy_len = requested_capacity.min(available_len);
    unsafe {
        std::ptr::copy_nonoverlapping(remote_sockaddr.cast::<u8>(), address_buffer, copy_len);
        *address_len_out = available_len as u32;
    }

    Ok(available_len as u32)
}

/// Resolve one socket family from one sockaddr payload.
fn socket_address_family(address: ProactorAddress) -> Option<i32> {
    // reject empty or null address payloads
    if address.data.is_null() || address.len < 2 {
        return None;
    }

    // decode the two-byte address family in native endian format
    let family = unsafe { *(address.data.cast::<u16>()) };

    Some(family as i32)
}

/// Bind one socket before connectex using a wildcard local endpoint.
fn bind_socket_for_connect(socket_handle: SOCKET, family: Option<i32>) -> Result<(), i32> {
    // decode socket family from the target address
    let family = family.unwrap_or(AF_INET as i32);

    // bind an ipv4 wildcard endpoint
    if family == AF_INET as i32 {
        // safety: zeroed sockaddr is valid for wildcard bind
        let mut address: SOCKADDR_IN = unsafe { std::mem::zeroed() };
        address.sin_family = AF_INET;
        address.sin_port = 0;
        let status = unsafe {
            bind(
                socket_handle,
                (&address as *const SOCKADDR_IN).cast::<SOCKADDR>(),
                size_of::<SOCKADDR_IN>() as i32,
            )
        };
        if status == 0 {
            return Ok(());
        }

        let error = unsafe { WSAGetLastError() };
        if error == WSAEINVAL || error == WSAEADDRINUSE {
            return Ok(());
        }

        return Err(error);
    }

    // bind an ipv6 wildcard endpoint
    if family == AF_INET6 as i32 {
        // safety: zeroed sockaddr is valid for wildcard bind
        let mut address: SOCKADDR_IN6 = unsafe { std::mem::zeroed() };
        address.sin6_family = AF_INET6;
        address.sin6_port = 0;
        address.sin6_flowinfo = 0;
        address.Anonymous.sin6_scope_id = 0;
        let status = unsafe {
            bind(
                socket_handle,
                (&address as *const SOCKADDR_IN6).cast::<SOCKADDR>(),
                size_of::<SOCKADDR_IN6>() as i32,
            )
        };
        if status == 0 {
            return Ok(());
        }

        let error = unsafe { WSAGetLastError() };
        if error == WSAEINVAL || error == WSAEADDRINUSE {
            return Ok(());
        }

        return Err(error);
    }

    Ok(())
}

/// Resolve one acceptex-compatible socket for the listener family.
fn create_accept_socket(listen_socket: SOCKET) -> Result<SOCKET, i32> {
    // read listener address to determine family
    // safety: zeroed storage is valid before getsockname writes into it
    let mut address: SOCKADDR_STORAGE = unsafe { std::mem::zeroed() };
    let mut address_len = size_of::<SOCKADDR_STORAGE>() as i32;
    let status = unsafe {
        getsockname(
            listen_socket,
            (&mut address as *mut SOCKADDR_STORAGE).cast::<SOCKADDR>(),
            &mut address_len,
        )
    };
    if status != 0 {
        return Err(unsafe { WSAGetLastError() });
    }

    // create one stream socket in the same address family
    let family = address.ss_family as i32;
    let accepted = unsafe { socket(family, SOCK_STREAM, 0) };
    if accepted == INVALID_SOCKET {
        return Err(unsafe { WSAGetLastError() });
    }

    Ok(accepted)
}

/// Resolve one connectex function pointer for the socket provider.
fn load_connect_ex(
    socket: SOCKET,
) -> RuntimeResult<
    unsafe extern "system" fn(
        SOCKET,
        *const SOCKADDR,
        i32,
        *const core::ffi::c_void,
        u32,
        *mut u32,
        *mut OVERLAPPED,
    ) -> windows_sys::Win32::Foundation::BOOL,
> {
    // request connectex from winsock extension api
    let mut bytes_returned = 0u32;
    let mut function: LPFN_CONNECTEX = None;
    let status = unsafe {
        WSAIoctl(
            socket,
            SIO_GET_EXTENSION_FUNCTION_POINTER,
            (&WSAID_CONNECTEX as *const windows_sys::core::GUID).cast(),
            size_of::<windows_sys::core::GUID>() as u32,
            (&mut function as *mut LPFN_CONNECTEX).cast(),
            size_of::<LPFN_CONNECTEX>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
            None,
        )
    };
    if status != 0 {
        let error = unsafe { WSAGetLastError() };
        return Err(RuntimeError::from(PlatformError::net_with(
            Some(PlatformErrorCode::Net),
            None,
            Some(error),
            Some("WSAIoctl".to_string()),
            None,
            None,
            "failed to resolve connectex".to_string(),
        ))
        .boxed());
    }

    // require one function pointer from winsock
    let Some(function) = function else {
        return Err(RuntimeError::from(PlatformError::net_with(
            Some(PlatformErrorCode::Net),
            None,
            None,
            Some("WSAIoctl".to_string()),
            None,
            None,
            "connectex pointer was null".to_string(),
        ))
        .boxed());
    };

    Ok(function)
}

/// Release resources held by one canceled operation.
fn cleanup_canceled_operation(operation: &IocpTask) {
    // close accepted sockets created for canceled accepts
    if let Some(socket) = operation.accept_socket {
        unsafe {
            closesocket(socket);
        }
    }
}

/// Build one successful completion.
fn success_completion(request: ProactorRequest, result: i32) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result,
        error_code: None,
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Build one invalid-argument completion.
fn invalid_argument_completion(
    request: ProactorRequest,
    code: PlatformErrorCode,
) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(code),
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Build one unsupported-operation completion.
fn not_supported_completion(
    request: ProactorRequest,
    _feature: &'static str,
) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(PlatformErrorCode::NotSupported),
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Build one net error completion.
fn net_error_completion(request: ProactorRequest, system_error: i32) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(PlatformErrorCode::Net),
        error_errno: Some(system_error),
        data: ProactorCompletionData::None,
    }
}

/// Build one io error completion.
fn io_error_completion(request: ProactorRequest, system_error: i32) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(PlatformErrorCode::Io),
        error_errno: Some(system_error),
        data: ProactorCompletionData::None,
    }
}

/// Build one cancellation completion.
fn canceled_completion(
    resource_id: ResourceId,
    token: u64,
    op: ProactorOpKind,
) -> ProactorCompletion {
    ProactorCompletion {
        resource_id,
        token,
        op,
        result: -1,
        error_code: Some(PlatformErrorCode::IoInterrupted),
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Post one completion payload into the completion queue.
fn post_completion(port: HANDLE, completion: ProactorCompletion) -> RuntimeResult<()> {
    // allocate one completion payload and pass ownership to IOCP
    let message = Box::new(IocpMessage { completion });
    let overlapped = Box::into_raw(message) as *mut OVERLAPPED;

    // post one completion packet
    let status = unsafe { PostQueuedCompletionStatus(port, 0, MESSAGE_COMPLETION_KEY, overlapped) };
    if status != 0 {
        return Ok(());
    }

    // reclaim payload on post failure
    unsafe {
        let _ = Box::from_raw(overlapped as *mut IocpMessage);
    }

    Err(core_platform::io_error("PostQueuedCompletionStatus"))
}

#[cfg(all(test, windows))]
mod tests {
    use super::{IocpProactor, execute_request, timeout_millis_from_nanos};
    use crate::platform::{
        PlatformErrorCode, Proactor, ProactorAddress, ProactorOp, ProactorRequest, ResourceId,
    };
    use crate::runtime::poller::PlatformHandle;

    /// Cancellation should surface one interrupted timeout completion.
    #[test]
    fn test_iocp_cancel_timeout_reports_interrupted() {
        // create one iocp proactor
        let mut proactor = IocpProactor::new().expect("iocp proactor should initialize");

        // submit and immediately cancel one timeout token
        let token = 71u64;
        let request = ProactorRequest {
            resource_id: ResourceId(11),
            token,
            op: ProactorOp::Timeout {
                timeout_ns: 50_000_000,
            },
        };
        proactor.submit(request).expect("submit should succeed");
        proactor.cancel(token).expect("cancel should succeed");

        // poll until one completion for the canceled token arrives
        let completions = proactor
            .poll(Some(300_000_000))
            .expect("poll should succeed");
        let completion = completions
            .iter()
            .find(|completion| completion.token == token)
            .copied()
            .expect("canceled timeout completion should be present");

        // verify cancellation shape
        assert_eq!(completion.result, -1);
        assert_eq!(
            completion.error_code,
            Some(PlatformErrorCode::IoInterrupted)
        );
    }

    /// Connect should reject null address pointers with invalid argument.
    #[test]
    fn test_iocp_connect_null_address_reports_invalid_argument() {
        // execute one connect request with one invalid address pointer
        let request = ProactorRequest {
            resource_id: ResourceId(21),
            token: 3,
            op: ProactorOp::Connect {
                handle: PlatformHandle(0),
                address: ProactorAddress {
                    data: std::ptr::null(),
                    len: 1,
                },
            },
        };
        let completion = execute_request(request);

        // verify argument validation failure
        assert_eq!(completion.result, -1);
        assert_eq!(completion.error_code, Some(PlatformErrorCode::NullPointer));
    }

    /// Accept should report a net error when the listener handle is invalid.
    #[test]
    fn test_iocp_accept_invalid_handle_reports_net_error() {
        // execute one accept request with one invalid listener handle
        let request = ProactorRequest {
            resource_id: ResourceId(31),
            token: 9,
            op: ProactorOp::Accept {
                handle: PlatformHandle(0),
                address: None,
            },
        };
        let completion = execute_request(request);

        // verify one net error completion
        assert_eq!(completion.result, -1);
        assert_eq!(completion.error_code, Some(PlatformErrorCode::Net));
    }

    /// SendFile should report a net error when handles are invalid.
    #[test]
    fn test_iocp_sendfile_invalid_handles_report_net_error() {
        // execute one sendfile request with invalid socket and file handles
        let request = ProactorRequest {
            resource_id: ResourceId(41),
            token: 17,
            op: ProactorOp::SendFile {
                output: PlatformHandle(0),
                input: PlatformHandle(0),
                input_offset: None,
                len: 1024,
            },
        };
        let completion = execute_request(request);

        // verify one net error completion
        assert_eq!(completion.result, -1);
        assert_eq!(completion.error_code, Some(PlatformErrorCode::Net));
    }

    /// CopyFileRange should complete immediately for zero-length requests.
    #[test]
    fn test_iocp_copy_file_range_zero_length_succeeds() {
        // execute one zero-length copy request
        let request = ProactorRequest {
            resource_id: ResourceId(51),
            token: 25,
            op: ProactorOp::CopyFileRange {
                input: PlatformHandle(0),
                output: PlatformHandle(0),
                input_offset: None,
                output_offset: None,
                len: 0,
                flags: 0,
            },
        };
        let completion = execute_request(request);

        // verify success for no-op copy
        assert_eq!(completion.result, 0);
        assert_eq!(completion.error_code, None);
    }

    /// Splice should complete immediately for zero-length requests.
    #[test]
    fn test_iocp_splice_zero_length_succeeds() {
        // execute one zero-length splice request
        let request = ProactorRequest {
            resource_id: ResourceId(61),
            token: 29,
            op: ProactorOp::Splice {
                input: PlatformHandle(0),
                output: PlatformHandle(0),
                input_offset: None,
                output_offset: None,
                len: 0,
                flags: 0,
            },
        };
        let completion = execute_request(request);

        // verify success for no-op splice
        assert_eq!(completion.result, 0);
        assert_eq!(completion.error_code, None);
    }

    /// Fallocate should complete immediately for zero-length requests.
    #[test]
    fn test_iocp_fallocate_zero_length_succeeds() {
        // execute one zero-length preallocation request
        let request = ProactorRequest {
            resource_id: ResourceId(71),
            token: 33,
            op: ProactorOp::Fallocate {
                handle: PlatformHandle(0),
                offset: 0,
                len: 0,
                mode: 0,
            },
        };
        let completion = execute_request(request);

        // verify success for no-op allocation
        assert_eq!(completion.result, 0);
        assert_eq!(completion.error_code, None);
    }

    /// Timeout conversion should round up and clamp to Win32 limits.
    #[test]
    fn test_timeout_millis_from_nanos_rounds_up_and_clamps() {
        // verify round-up behavior for sub-millisecond values
        assert_eq!(timeout_millis_from_nanos(1), 1);
        assert_eq!(timeout_millis_from_nanos(1_000_000), 1);
        assert_eq!(timeout_millis_from_nanos(1_000_001), 2);

        // verify clamping to u32 range
        assert_eq!(timeout_millis_from_nanos(u64::MAX), u32::MAX);
    }
}

/// Convert nanoseconds into IOCP timeout milliseconds.
fn timeout_millis_from_nanos(timeout_nanos: u64) -> u32 {
    // convert nanos to milliseconds with ceil semantics
    let millis = timeout_nanos.saturating_add(999_999) / 1_000_000;

    // clamp timeout to u32 range for Win32 APIs
    if millis > u32::MAX as u64 {
        return u32::MAX;
    }

    millis as u32
}
