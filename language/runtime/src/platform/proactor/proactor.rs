use super::super::ResourceId;
use super::super::diagnostic::PlatformErrorCode;
use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::{PlatformHandle, PlatformInterest, PollerEventMask};

/// Task kind for asynchronous I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProactorOpKind {
    /// Read from a handle.
    Read = 0,
    /// Write to a handle.
    Write = 1,
    /// Read using a vector of buffers.
    Readv = 2,
    /// Write using a vector of buffers.
    Writev = 3,
    /// Receive a datagram or stream payload.
    Recv = 4,
    /// Send a datagram or stream payload.
    Send = 5,
    /// Receive a datagram with address metadata.
    RecvFrom = 6,
    /// Send a datagram with address metadata.
    SendTo = 7,
    /// Copy data between two file descriptors.
    CopyFileRange = 8,
    /// Splice data between two file descriptors.
    Splice = 9,
    /// Send file data to a socket.
    SendFile = 10,
    /// Flush file data to storage.
    Fsync = 11,
    /// Flush file data without metadata.
    Fdatasync = 12,
    /// Preallocate space in a file.
    Fallocate = 13,
    /// Accept an incoming connection.
    Accept = 14,
    /// Connect to a remote address.
    Connect = 15,
    /// Poll for readiness on a handle.
    Poll = 16,
    /// Shutdown a handle.
    Shutdown = 17,
    /// Close a handle.
    Close = 18,
    /// Cancel an inflight operation.
    Cancel = 19,
    /// Timeout operation.
    Timeout = 20,
}

/// Buffer payload for asynchronous I/O.
#[derive(Debug, Clone, Copy)]
pub struct ProactorBuffer {
    /// Pointer to the buffer data.
    pub data: *mut u8,
    /// Length of the buffer.
    pub len: u32,
}

/// Safety: buffers are caller-managed and must remain valid until completion.
unsafe impl Send for ProactorBuffer {}

/// Scatter/gather buffer list payload.
#[derive(Debug, Clone, Copy)]
pub struct ProactorBufferVec {
    /// Pointer to the buffer list.
    pub data: *mut ProactorBuffer,
    /// Length of the buffer list.
    pub len: u32,
}

/// Safety: buffer lists are caller-managed and must remain valid until completion.
unsafe impl Send for ProactorBufferVec {}

/// Socket address payload for asynchronous I/O.
#[derive(Debug, Clone, Copy)]
pub struct ProactorAddress {
    /// Pointer to the socket address.
    pub data: *const u8,
    /// Length of the socket address.
    pub len: u32,
}

/// Safety: address pointers are caller-managed and must remain valid until completion.
unsafe impl Send for ProactorAddress {}

/// Socket address storage payload for asynchronous accept/recv.
#[derive(Debug, Clone, Copy)]
pub struct ProactorAddressStorage {
    /// Pointer to the socket address buffer.
    pub data: *mut u8,
    /// Pointer to the socket address length.
    pub len: *mut u32,
}

/// Safety: address storage pointers are caller-managed and must remain valid until completion.
unsafe impl Send for ProactorAddressStorage {}

/// Shutdown mode for a handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactorShutdown {
    /// Shut down reads.
    Read,
    /// Shut down writes.
    Write,
    /// Shut down reads and writes.
    Both,
}

/// Asynchronous I/O operation request.
#[derive(Debug, Clone, Copy)]
pub struct ProactorRequest {
    /// Resource identifier associated with the request.
    pub resource_id: ResourceId,
    /// Opaque request token.
    pub token: u64,
    /// Task payload.
    pub op: ProactorOp,
}

/// Safety: request payload pointers are caller-managed and must remain valid until completion.
unsafe impl Send for ProactorRequest {}

/// Asynchronous I/O operation payload.
#[derive(Debug, Clone, Copy)]
pub enum ProactorOp {
    /// Read from a handle.
    Read {
        /// Handle to read from.
        handle: PlatformHandle,
        /// Buffer to fill.
        buffer: ProactorBuffer,
        /// Optional file offset.
        offset: Option<u64>,
    },
    /// Write to a handle.
    Write {
        /// Handle to write to.
        handle: PlatformHandle,
        /// Buffer to write.
        buffer: ProactorBuffer,
        /// Optional file offset.
        offset: Option<u64>,
    },
    /// Read into a vector of buffers.
    Readv {
        /// Handle to read from.
        handle: PlatformHandle,
        /// Buffer list to fill.
        buffers: ProactorBufferVec,
        /// Optional file offset.
        offset: Option<u64>,
    },
    /// Write from a vector of buffers.
    Writev {
        /// Handle to write to.
        handle: PlatformHandle,
        /// Buffer list to write.
        buffers: ProactorBufferVec,
        /// Optional file offset.
        offset: Option<u64>,
    },
    /// Receive a datagram or stream payload.
    Recv {
        /// Socket handle.
        handle: PlatformHandle,
        /// Buffer to fill.
        buffer: ProactorBuffer,
        /// Receive flags.
        flags: u32,
    },
    /// Send a datagram or stream payload.
    Send {
        /// Socket handle.
        handle: PlatformHandle,
        /// Buffer to send.
        buffer: ProactorBuffer,
        /// Send flags.
        flags: u32,
    },
    /// Receive a datagram with address metadata.
    RecvFrom {
        /// Socket handle.
        handle: PlatformHandle,
        /// Buffer to fill.
        buffer: ProactorBuffer,
        /// Receive flags.
        flags: u32,
        /// Address storage for the sender address.
        address: ProactorAddressStorage,
    },
    /// Send a datagram with address metadata.
    SendTo {
        /// Socket handle.
        handle: PlatformHandle,
        /// Buffer to send.
        buffer: ProactorBuffer,
        /// Send flags.
        flags: u32,
        /// Destination address.
        address: ProactorAddress,
    },
    /// Copy data between two file descriptors.
    CopyFileRange {
        /// Input handle.
        input: PlatformHandle,
        /// Output handle.
        output: PlatformHandle,
        /// Optional input offset.
        input_offset: Option<u64>,
        /// Optional output offset.
        output_offset: Option<u64>,
        /// Maximum bytes to copy.
        len: u64,
        /// Copy flags.
        flags: u32,
    },
    /// Splice data between two file descriptors.
    Splice {
        /// Input handle.
        input: PlatformHandle,
        /// Output handle.
        output: PlatformHandle,
        /// Optional input offset.
        input_offset: Option<u64>,
        /// Optional output offset.
        output_offset: Option<u64>,
        /// Maximum bytes to splice.
        len: u64,
        /// Splice flags.
        flags: u32,
    },
    /// Send file data to a socket.
    SendFile {
        /// Output handle.
        output: PlatformHandle,
        /// Input handle.
        input: PlatformHandle,
        /// Optional input offset.
        input_offset: Option<u64>,
        /// Maximum bytes to send.
        len: u64,
    },
    /// Flush file data to storage.
    Fsync {
        /// Handle to sync.
        handle: PlatformHandle,
    },
    /// Flush file data without metadata.
    Fdatasync {
        /// Handle to sync.
        handle: PlatformHandle,
    },
    /// Preallocate space in a file.
    Fallocate {
        /// Handle to allocate.
        handle: PlatformHandle,
        /// Allocation offset.
        offset: u64,
        /// Allocation length.
        len: u64,
        /// Allocation mode flags.
        mode: u32,
    },
    /// Accept a new connection.
    Accept {
        /// Listening handle.
        handle: PlatformHandle,
        /// Optional address storage.
        address: Option<ProactorAddressStorage>,
    },
    /// Connect to a remote address.
    Connect {
        /// Socket handle.
        handle: PlatformHandle,
        /// Remote address.
        address: ProactorAddress,
    },
    /// Poll for readiness on a handle.
    Poll {
        /// Handle to poll.
        handle: PlatformHandle,
        /// Interested readiness mask.
        interests: PlatformInterest,
    },
    /// Shutdown a handle.
    Shutdown {
        /// Handle to shut down.
        handle: PlatformHandle,
        /// Shutdown mode.
        how: ProactorShutdown,
    },
    /// Close a handle.
    Close {
        /// Handle to close.
        handle: PlatformHandle,
    },
    /// Cancel an inflight operation by token.
    Cancel {
        /// Token to cancel.
        token: u64,
    },
    /// Timeout operation.
    Timeout {
        /// Timeout in nanoseconds.
        timeout_ns: u64,
    },
}

impl ProactorOp {
    /// Return the operation kind.
    pub const fn kind(self) -> ProactorOpKind {
        // map the op variant to the kind
        match self {
            ProactorOp::Read { .. } => ProactorOpKind::Read,
            ProactorOp::Write { .. } => ProactorOpKind::Write,
            ProactorOp::Readv { .. } => ProactorOpKind::Readv,
            ProactorOp::Writev { .. } => ProactorOpKind::Writev,
            ProactorOp::Recv { .. } => ProactorOpKind::Recv,
            ProactorOp::Send { .. } => ProactorOpKind::Send,
            ProactorOp::RecvFrom { .. } => ProactorOpKind::RecvFrom,
            ProactorOp::SendTo { .. } => ProactorOpKind::SendTo,
            ProactorOp::CopyFileRange { .. } => ProactorOpKind::CopyFileRange,
            ProactorOp::Splice { .. } => ProactorOpKind::Splice,
            ProactorOp::SendFile { .. } => ProactorOpKind::SendFile,
            ProactorOp::Fsync { .. } => ProactorOpKind::Fsync,
            ProactorOp::Fdatasync { .. } => ProactorOpKind::Fdatasync,
            ProactorOp::Fallocate { .. } => ProactorOpKind::Fallocate,
            ProactorOp::Accept { .. } => ProactorOpKind::Accept,
            ProactorOp::Connect { .. } => ProactorOpKind::Connect,
            ProactorOp::Poll { .. } => ProactorOpKind::Poll,
            ProactorOp::Shutdown { .. } => ProactorOpKind::Shutdown,
            ProactorOp::Close { .. } => ProactorOpKind::Close,
            ProactorOp::Cancel { .. } => ProactorOpKind::Cancel,
            ProactorOp::Timeout { .. } => ProactorOpKind::Timeout,
        }
    }
}

/// Completion payload data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactorCompletionData {
    /// No additional data.
    None,
    /// Accepted connection payload.
    Accept {
        /// Accepted handle.
        handle: PlatformHandle,
        /// Length of the address written.
        addr_len: u32,
    },
    /// Receive payload.
    Recv {
        /// Length of the address written.
        addr_len: u32,
        /// Receive flags.
        flags: u32,
    },
    /// Poll payload.
    Poll {
        /// Readiness mask.
        mask: PollerEventMask,
    },
    /// Timeout payload.
    Timeout,
}

/// Completion record for an asynchronous I/O request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProactorCompletion {
    /// Resource identifier associated with the request.
    pub resource_id: ResourceId,
    /// Opaque request token.
    pub token: u64,
    /// Task kind.
    pub op: ProactorOpKind,
    /// Result code or byte count.
    pub result: i32,
    /// Optional error code.
    pub error_code: Option<PlatformErrorCode>,
    /// Optional errno value.
    pub error_errno: Option<i32>,
    /// Additional completion data.
    pub data: ProactorCompletionData,
}

/// Completion-based I/O interface.
pub trait Proactor: Send {
    /// Submit an asynchronous I/O request.
    fn submit(&mut self, request: ProactorRequest) -> RuntimeResult<()>;
    /// Cancel a pending request by token.
    fn cancel(&mut self, token: u64) -> RuntimeResult<()>;
    /// Poll for I/O completions.
    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<ProactorCompletion>>;
    /// Wake the proactor if it is blocked.
    fn wake(&mut self) -> RuntimeResult<()>;
}
