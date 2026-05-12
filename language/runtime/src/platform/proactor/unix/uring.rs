use std::collections::{HashMap, VecDeque};
use std::os::unix::io::RawFd;

use io_uring::{IoUring, opcode, types};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::{
    PlatformErrorContext, PlatformErrorContextKind, io_error_code_from_errno,
};
use crate::platform::proactor::{
    Proactor, ProactorAddress, ProactorAddressStorage, ProactorBuffer, ProactorBufferVec,
    ProactorCompletion, ProactorCompletionData, ProactorOp, ProactorOpKind, ProactorRequest,
    ProactorShutdown,
};
use crate::platform::{PlatformError, PlatformErrorCode, ResourceId, core as core_platform};
use crate::runtime::poller::{PlatformHandle, PlatformInterest, PollerEventMask};

/// Default io_uring queue depth.
const DEFAULT_QUEUE_DEPTH: u32 = 256;
/// Reserved user data value for the wake eventfd.
const WAKE_USER_DATA: u64 = u64::MAX;

/// io_uring backed proactor for Linux targets.
pub struct IoUringProactor {
    /// io_uring instance.
    ring: IoUring,
    /// Inflight request metadata by token.
    inflight: HashMap<u64, InflightRequest>,
    /// Completion queue for synchronous fallbacks.
    completions: VecDeque<ProactorCompletion>,
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
}

impl std::fmt::Debug for IoUringProactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IoUringProactor")
            .field("inflight", &self.inflight.len())
            .field("completions", &self.completions.len())
            .field("wake_fd", &self.wake_fd)
            .finish()
    }
}

unsafe impl Send for IoUringProactor {}

/// Inflight request metadata for completion parsing.
#[derive(Debug)]
struct InflightRequest {
    /// Associated resource id.
    resource_id: ResourceId,
    /// Task kind for this request.
    op: ProactorOpKind,
    /// Additional inflight data.
    data: InflightData,
}

/// Inflight request data that must outlive the submission.
#[derive(Debug)]
#[allow(dead_code)]
enum InflightData {
    /// No additional inflight state.
    None,
    /// Scatter gather buffers.
    Iov(Vec<libc::iovec>),
    /// Message header for recvmsg or sendmsg.
    Msg {
        /// Stored msghdr.
        header: Box<libc::msghdr>,
        /// Optional address length pointer.
        addr_len: Option<*mut u32>,
    },
    /// Stored timeout timespec.
    Timeout(Box<types::Timespec>),
}

impl IoUringProactor {
    /// Create a new io_uring proactor instance.
    pub fn new() -> RuntimeResult<Self> {
        Self::with_entries(DEFAULT_QUEUE_DEPTH)
    }

    /// Create a new io_uring proactor instance with one explicit queue depth.
    pub fn with_entries(entries: u32) -> RuntimeResult<Self> {
        // initialize io_uring
        let ring = IoUring::new(entries).map_err(|error| {
            let errno = error.raw_os_error();
            let code = errno.and_then(io_error_code_from_errno);
            RuntimeError::from(PlatformError::io_with(
                code,
                None,
                errno,
                Some("proactor.io_uring".to_string()),
                None,
                format!("io_uring init failed: {error}"),
            ))
            .boxed()
        })?;

        // create wake eventfd
        let wake_fd = create_eventfd()?;

        // build the proactor state
        let mut proactor = Self {
            ring,
            inflight: HashMap::new(),
            completions: VecDeque::new(),
            wake_fd,
        };

        // submit the wake poll entry
        proactor.submit_wake_poll()?;

        Ok(proactor)
    }

    fn submit_entry(&mut self, entry: io_uring::squeue::Entry) -> RuntimeResult<()> {
        // safety: only called while holding mutable access
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(|_| io_error("proactor.sq_full", None))?;
        }

        Ok(())
    }

    fn submit_entries(&mut self) -> RuntimeResult<()> {
        // submit pending entries
        self.ring
            .submitter()
            .submit()
            .map_err(|_| io_error("proactor.submit", None))?;

        Ok(())
    }

    fn submit_and_wait(&mut self, wait_for: usize) -> RuntimeResult<()> {
        // submit pending entries and wait for completions
        self.ring
            .submitter()
            .submit_and_wait(wait_for)
            .map_err(|_| io_error("proactor.submit_and_wait", None))?;

        Ok(())
    }

    fn submit_with_timeout(&mut self, timeout_nanos: u64) -> RuntimeResult<()> {
        // build the submission timeout timespec
        let seconds = timeout_nanos / 1_000_000_000;
        let nanos = (timeout_nanos % 1_000_000_000) as u32;
        let mut timespec = types::Timespec::new();
        timespec = timespec.sec(seconds).nsec(nanos);

        // submit with timeout arguments
        let args = types::SubmitArgs::new().timespec(&timespec);
        self.ring
            .submitter()
            .submit_with_args(1, &args)
            .map_err(|_| io_error("proactor.submit_with_timeout", None))?;

        Ok(())
    }

    fn submit_cancel(&mut self, target: u64, token: u64) -> RuntimeResult<()> {
        // build a cancellation request
        let entry = opcode::AsyncCancel::new(target).build().user_data(token);
        self.submit_entry(entry)
    }

    fn submit_timeout(
        &mut self,
        timeout_ns: u64,
        token: u64,
    ) -> RuntimeResult<Box<types::Timespec>> {
        // build a timeout submission entry
        let seconds = timeout_ns / 1_000_000_000;
        let nanos = (timeout_ns % 1_000_000_000) as u32;
        let mut timespec = Box::new(types::Timespec::new());
        *timespec = timespec.sec(seconds).nsec(nanos);
        let entry = opcode::Timeout::new(&*timespec).build().user_data(token);
        self.submit_entry(entry)?;

        Ok(timespec)
    }

    fn submit_wake_poll(&mut self) -> RuntimeResult<()> {
        // submit a poll request for the wake eventfd
        let entry = opcode::PollAdd::new(types::Fd(self.wake_fd), libc::POLLIN as _)
            .build()
            .user_data(WAKE_USER_DATA);
        self.submit_entry(entry)?;

        // flush the submission queue
        self.submit_entries()?;

        Ok(())
    }

    fn build_iovecs(buffers: ProactorBufferVec) -> RuntimeResult<Vec<libc::iovec>> {
        // validate the buffer list pointer
        if buffers.data.is_null() && buffers.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("buffers.data")).boxed());
        }

        // safety: caller guarantees the buffer list is valid for the duration
        let list = unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
        let mut iovecs = Vec::with_capacity(list.len());
        for buffer in list {
            iovecs.push(to_iovec(*buffer));
        }

        Ok(iovecs)
    }

    fn build_msghdr(
        buffer: ProactorBuffer,
        address: ProactorAddress,
    ) -> RuntimeResult<Box<libc::msghdr>> {
        // validate the address pointer
        if address.data.is_null() && address.len != 0 {
            return Err(RuntimeError::from(PlatformError::null_pointer("address.data")).boxed());
        }

        // allocate the message header
        let mut iovec = to_iovec(buffer);
        let mut header: libc::msghdr = unsafe { std::mem::zeroed() };
        header.msg_name = address.data as *mut _;
        header.msg_namelen = address.len as _;
        header.msg_iov = &mut iovec as *mut _;
        header.msg_iovlen = 1;
        header.msg_control = std::ptr::null_mut();
        header.msg_controllen = 0;
        header.msg_flags = 0;

        Ok(Box::new(header))
    }

    fn build_recv_msg(
        buffer: ProactorBuffer,
        address: ProactorAddressStorage,
    ) -> RuntimeResult<(Box<libc::msghdr>, Option<*mut u32>)> {
        // validate the address buffer pointer
        if address.data.is_null() && !address.len.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("address.data")).boxed());
        }

        // validate the address length pointer
        if address.len.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("address.len")).boxed());
        }

        // allocate the message header
        let mut iovec = to_iovec(buffer);
        let mut header: libc::msghdr = unsafe { std::mem::zeroed() };
        header.msg_name = address.data as *mut _;
        header.msg_namelen = unsafe { *address.len as _ };
        header.msg_iov = &mut iovec as *mut _;
        header.msg_iovlen = 1;
        header.msg_control = std::ptr::null_mut();
        header.msg_controllen = 0;
        header.msg_flags = 0;

        Ok((Box::new(header), Some(address.len)))
    }

    fn build_accept_addr(
        address: ProactorAddressStorage,
    ) -> RuntimeResult<(
        Option<*mut libc::sockaddr>,
        Option<*mut libc::socklen_t>,
        Option<*mut u32>,
    )> {
        // short-circuit when no address storage is provided
        if address.data.is_null() || address.len.is_null() {
            return Ok((None, None, None));
        }

        // cast address storage into libc types
        let addr_ptr = address.data as *mut libc::sockaddr;
        let len_ptr = address.len as *mut libc::socklen_t;
        Ok((Some(addr_ptr), Some(len_ptr), Some(address.len)))
    }

    fn push_completion(&mut self, completion: ProactorCompletion) {
        // queue the completion for drain
        self.completions.push_back(completion);
    }

    fn complete_immediate(
        &mut self,
        resource_id: ResourceId,
        token: u64,
        op: ProactorOpKind,
        result: i32,
        error_code: Option<PlatformErrorCode>,
        error_errno: Option<i32>,
        data: ProactorCompletionData,
    ) {
        // push the completion to the queue
        self.push_completion(ProactorCompletion {
            resource_id,
            token,
            op,
            result,
            error_code,
            error_errno,
            data,
        });
    }
}

impl Proactor for IoUringProactor {
    fn submit(&mut self, request: ProactorRequest) -> RuntimeResult<()> {
        // reject duplicate tokens
        if self.inflight.contains_key(&request.token) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "token",
                "token already in use",
            ))
            .boxed());
        }

        // reject the wake token
        if request.token == WAKE_USER_DATA {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "token",
                "token reserved for wake",
            ))
            .boxed());
        }

        // capture request metadata
        let op = request.op;
        let kind = op.kind();
        let token = request.token;

        // build the submission entry and inflight state
        let inflight = match op {
            ProactorOp::Read {
                handle,
                buffer,
                offset,
            } => {
                // submit a read request
                let entry = if let Some(offset) = offset {
                    opcode::Read::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .offset(offset as _)
                        .build()
                } else {
                    opcode::Read::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .build()
                };
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Write {
                handle,
                buffer,
                offset,
            } => {
                // submit a write request
                let entry = if let Some(offset) = offset {
                    opcode::Write::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .offset(offset as _)
                        .build()
                } else {
                    opcode::Write::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .build()
                };
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Readv {
                handle,
                buffers,
                offset,
            } => {
                // submit a readv request with owned iovecs
                let iovecs = Self::build_iovecs(buffers)?;
                let entry = if let Some(offset) = offset {
                    opcode::Readv::new(
                        types::Fd(handle.as_raw_fd()),
                        iovecs.as_ptr(),
                        iovecs.len() as _,
                    )
                    .offset(offset as _)
                    .build()
                } else {
                    opcode::Readv::new(
                        types::Fd(handle.as_raw_fd()),
                        iovecs.as_ptr(),
                        iovecs.len() as _,
                    )
                    .build()
                };
                self.submit_entry(entry.user_data(token))?;
                InflightData::Iov(iovecs)
            }
            ProactorOp::Writev {
                handle,
                buffers,
                offset,
            } => {
                // submit a writev request with owned iovecs
                let iovecs = Self::build_iovecs(buffers)?;
                let entry = if let Some(offset) = offset {
                    opcode::Writev::new(
                        types::Fd(handle.as_raw_fd()),
                        iovecs.as_ptr(),
                        iovecs.len() as _,
                    )
                    .offset(offset as _)
                    .build()
                } else {
                    opcode::Writev::new(
                        types::Fd(handle.as_raw_fd()),
                        iovecs.as_ptr(),
                        iovecs.len() as _,
                    )
                    .build()
                };
                self.submit_entry(entry.user_data(token))?;
                InflightData::Iov(iovecs)
            }
            ProactorOp::Recv {
                handle,
                buffer,
                flags,
            } => {
                // submit a recv request
                let entry =
                    opcode::Recv::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .flags(flags as _)
                        .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Send {
                handle,
                buffer,
                flags,
            } => {
                // submit a send request
                let entry =
                    opcode::Send::new(types::Fd(handle.as_raw_fd()), buffer.data, buffer.len)
                        .flags(flags as _)
                        .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::RecvFrom {
                handle,
                buffer,
                flags,
                address,
            } => {
                // submit a recvmsg request with address storage
                let (mut header, addr_len) = Self::build_recv_msg(buffer, address)?;
                let entry = opcode::RecvMsg::new(types::Fd(handle.as_raw_fd()), header.as_mut())
                    .flags(flags as _)
                    .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::Msg { header, addr_len }
            }
            ProactorOp::SendTo {
                handle,
                buffer,
                flags,
                address,
            } => {
                // submit a sendmsg request with address metadata
                let header = Self::build_msghdr(buffer, address)?;
                let entry = opcode::SendMsg::new(types::Fd(handle.as_raw_fd()), &*header)
                    .flags(flags as _)
                    .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::Msg {
                    header,
                    addr_len: None,
                }
            }
            ProactorOp::CopyFileRange {
                input,
                output,
                input_offset,
                output_offset,
                len,
                flags,
            } => {
                // perform a synchronous copy_file_range syscall
                let mut in_offset = input_offset.map(|value| value as libc::off_t);
                let mut out_offset = output_offset.map(|value| value as libc::off_t);
                let in_ptr = in_offset
                    .as_mut()
                    .map(|value| value as *mut _)
                    .unwrap_or(std::ptr::null_mut());
                let out_ptr = out_offset
                    .as_mut()
                    .map(|value| value as *mut _)
                    .unwrap_or(std::ptr::null_mut());

                // invoke the syscall
                let result = unsafe {
                    libc::copy_file_range(
                        input.as_raw_fd(),
                        in_ptr,
                        output.as_raw_fd(),
                        out_ptr,
                        len as usize,
                        flags as _,
                    )
                };

                // report the synchronous completion
                if result < 0 {
                    let errno = core_platform::get_errno();
                    let error_code = io_error_code_from_errno(errno);
                    self.complete_immediate(
                        request.resource_id,
                        token,
                        kind,
                        result as i32,
                        error_code,
                        Some(errno),
                        ProactorCompletionData::None,
                    );
                } else {
                    self.complete_immediate(
                        request.resource_id,
                        token,
                        kind,
                        result as i32,
                        None,
                        None,
                        ProactorCompletionData::None,
                    );
                }

                return Ok(());
            }
            ProactorOp::Splice {
                input,
                output,
                input_offset,
                output_offset,
                len,
                flags,
            } => {
                // submit a splice request
                let off_in = input_offset.map(|value| value as i64).unwrap_or(-1);
                let off_out = output_offset.map(|value| value as i64).unwrap_or(-1);
                let entry = opcode::Splice::new(
                    types::Fd(output.as_raw_fd()),
                    off_out,
                    types::Fd(input.as_raw_fd()),
                    off_in,
                    len as u32,
                )
                .flags(flags)
                .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::SendFile {
                output,
                input,
                input_offset,
                len,
            } => {
                // perform a synchronous sendfile syscall
                let mut offset = input_offset.map(|value| value as libc::off_t);
                let offset_ptr = offset
                    .as_mut()
                    .map(|value| value as *mut _)
                    .unwrap_or(std::ptr::null_mut());

                // invoke the syscall
                let result = unsafe {
                    libc::sendfile(
                        output.as_raw_fd(),
                        input.as_raw_fd(),
                        offset_ptr,
                        len as usize,
                    )
                };

                // report the synchronous completion
                if result < 0 {
                    let errno = core_platform::get_errno();
                    let error_code = io_error_code_from_errno(errno);
                    self.complete_immediate(
                        request.resource_id,
                        token,
                        kind,
                        result as i32,
                        error_code,
                        Some(errno),
                        ProactorCompletionData::None,
                    );
                } else {
                    self.complete_immediate(
                        request.resource_id,
                        token,
                        kind,
                        result as i32,
                        None,
                        None,
                        ProactorCompletionData::None,
                    );
                }

                return Ok(());
            }
            ProactorOp::Fsync { handle } => {
                // submit an fsync request
                let entry = opcode::Fsync::new(types::Fd(handle.as_raw_fd())).build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Fdatasync { handle } => {
                // submit a fdatasync request
                let entry = opcode::Fsync::new(types::Fd(handle.as_raw_fd()))
                    .flags(types::FsyncFlags::DATASYNC)
                    .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Fallocate {
                handle,
                offset,
                len,
                mode,
            } => {
                // submit a fallocate request
                let entry = opcode::Fallocate::new(types::Fd(handle.as_raw_fd()), len)
                    .offset(offset)
                    .mode(mode as i32)
                    .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Accept { handle, address } => {
                // submit an accept request
                let (addr_ptr, addr_len_ptr, raw_len) = address
                    .map(Self::build_accept_addr)
                    .transpose()?
                    .unwrap_or((None, None, None));
                let entry = opcode::Accept::new(
                    types::Fd(handle.as_raw_fd()),
                    addr_ptr.unwrap_or(std::ptr::null_mut()),
                    addr_len_ptr.unwrap_or(std::ptr::null_mut()),
                )
                .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::Msg {
                    header: Box::new(unsafe { std::mem::zeroed() }),
                    addr_len: raw_len,
                }
            }
            ProactorOp::Connect { handle, address } => {
                // submit a connect request
                let entry = opcode::Connect::new(
                    types::Fd(handle.as_raw_fd()),
                    address.data as *const libc::sockaddr,
                    address.len as _,
                )
                .build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Poll { handle, interests } => {
                // submit a poll request
                let mask = poll_mask_for_interest(interests);
                let entry = opcode::PollAdd::new(types::Fd(handle.as_raw_fd()), mask).build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Shutdown { handle, how } => {
                // submit a shutdown request
                let how = match how {
                    ProactorShutdown::Read => libc::SHUT_RD,
                    ProactorShutdown::Write => libc::SHUT_WR,
                    ProactorShutdown::Both => libc::SHUT_RDWR,
                };
                let entry = opcode::Shutdown::new(types::Fd(handle.as_raw_fd()), how).build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Close { handle } => {
                // submit a close request
                let entry = opcode::Close::new(types::Fd(handle.as_raw_fd())).build();
                self.submit_entry(entry.user_data(token))?;
                InflightData::None
            }
            ProactorOp::Cancel { token: target } => {
                // submit a cancellation request
                self.submit_cancel(target, token)?;
                InflightData::None
            }
            ProactorOp::Timeout { timeout_ns } => {
                // submit a timeout request
                let timespec = self.submit_timeout(timeout_ns, token)?;
                InflightData::Timeout(timespec)
            }
        };

        // track inflight request state
        self.inflight.insert(
            token,
            InflightRequest {
                resource_id: request.resource_id,
                op: kind,
                data: inflight,
            },
        );

        // flush submissions to the kernel
        self.submit_entries()?;

        Ok(())
    }

    fn cancel(&mut self, token: u64) -> RuntimeResult<()> {
        // submit a cancel request
        self.submit_cancel(token, token)?;

        // flush submissions to the kernel
        self.submit_entries()?;

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<ProactorCompletion>> {
        // drain immediate completions first
        if !self.completions.is_empty() {
            return Ok(self.completions.drain(..).collect());
        }

        // submit work and wait for completions when requested
        match timeout_nanos {
            Some(0) => {
                self.submit_entries()?;
            }
            Some(timeout_nanos) => {
                self.submit_with_timeout(timeout_nanos)?;
            }
            None => {
                self.submit_and_wait(1)?;
            }
        }

        // drain completions from the ring
        let mut output = Vec::new();
        let mut resubmit_wake = false;
        for cqe in self.ring.completion() {
            // resolve the completion token
            let user_data = cqe.user_data();
            if user_data == WAKE_USER_DATA {
                // drain and resubmit the wake poll entry
                drain_eventfd(self.wake_fd);
                resubmit_wake = true;
                continue;
            }

            // look up inflight request state
            let Some(inflight) = self.inflight.remove(&user_data) else {
                continue;
            };

            // translate the completion result into error data
            let result = cqe.result();
            let (error_code, error_errno) = if result < 0 {
                let errno = -result;
                (io_error_code_from_errno(errno), Some(errno))
            } else {
                (None, None)
            };

            // build completion payload metadata
            let data = match inflight.op {
                ProactorOpKind::Accept => {
                    let handle = if result >= 0 {
                        PlatformHandle::from_raw_fd(result)
                    } else {
                        PlatformHandle::from_raw_fd(-1)
                    };
                    let addr_len = match inflight.data {
                        InflightData::Msg { addr_len, .. } => addr_len
                            .and_then(|ptr| unsafe { ptr.as_ref() })
                            .copied()
                            .unwrap_or(0),
                        _ => 0,
                    };
                    ProactorCompletionData::Accept { handle, addr_len }
                }
                ProactorOpKind::RecvFrom => {
                    let (addr_len, flags) = match inflight.data {
                        InflightData::Msg { addr_len, header } => {
                            let len = addr_len
                                .and_then(|ptr| unsafe { ptr.as_ref() })
                                .copied()
                                .unwrap_or(0);
                            let flags = header.msg_flags as u32;
                            (len, flags)
                        }
                        _ => (0, 0),
                    };
                    ProactorCompletionData::Recv { addr_len, flags }
                }
                ProactorOpKind::Recv => ProactorCompletionData::Recv {
                    addr_len: 0,
                    flags: 0,
                },
                ProactorOpKind::Poll => {
                    let mask = if result >= 0 {
                        event_mask_from_revents(result as u32)
                    } else {
                        PollerEventMask::NONE
                    };
                    ProactorCompletionData::Poll { mask }
                }
                ProactorOpKind::Timeout => ProactorCompletionData::Timeout,
                _ => ProactorCompletionData::None,
            };

            // record the completion
            output.push(ProactorCompletion {
                resource_id: inflight.resource_id,
                token: user_data,
                op: inflight.op,
                result,
                error_code,
                error_errno,
                data,
            });
        }

        // resubmit the wake poll entry after draining completions
        if resubmit_wake {
            self.submit_wake_poll()?;
        }

        Ok(output)
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        // notify the wake eventfd
        let value: u64 = 1;
        let result = unsafe {
            libc::write(
                self.wake_fd,
                &value as *const u64 as *const _,
                std::mem::size_of::<u64>(),
            )
        };

        // tolerate eventfd contention
        if result < 0 {
            let errno = core_platform::get_errno();
            if errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
                return Err(io_error("proactor.wake", None));
            }
        }

        Ok(())
    }
}

fn to_iovec(buffer: ProactorBuffer) -> libc::iovec {
    // map the buffer into an iovec
    libc::iovec {
        iov_base: buffer.data as *mut _,
        iov_len: buffer.len as _,
    }
}

fn poll_mask_for_interest(interests: PlatformInterest) -> u32 {
    // start with an empty poll mask
    let mut mask = 0;

    // include readable interest
    if interests.contains(PlatformInterest::READABLE) {
        mask |= libc::POLLIN as u32;
    }

    // include writable interest
    if interests.contains(PlatformInterest::WRITABLE) {
        mask |= libc::POLLOUT as u32;
    }

    mask
}

fn event_mask_from_revents(revents: u32) -> PollerEventMask {
    // start with an empty event mask
    let mut mask = PollerEventMask::NONE;

    // map readable readiness
    if (revents & libc::POLLIN as u32) != 0 {
        mask |= PollerEventMask::READABLE;
    }

    // map writable readiness
    if (revents & libc::POLLOUT as u32) != 0 {
        mask |= PollerEventMask::WRITABLE;
    }

    // map error readiness
    if (revents & libc::POLLERR as u32) != 0 {
        mask |= PollerEventMask::ERROR;
    }

    // map hangup readiness
    if (revents & libc::POLLHUP as u32) != 0 {
        mask |= PollerEventMask::HANGUP;
    }

    // map priority readiness
    if (revents & libc::POLLPRI as u32) != 0 {
        mask |= PollerEventMask::PRIORITY;
    }

    mask
}

fn create_eventfd() -> RuntimeResult<RawFd> {
    // create the non-blocking eventfd
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };

    // validate the eventfd creation
    if fd < 0 {
        return Err(io_error("proactor.eventfd", None));
    }

    Ok(fd)
}

fn drain_eventfd(fd: RawFd) {
    // drain the eventfd counter
    let mut value = 0u64;
    loop {
        // read the eventfd value
        let result = unsafe {
            libc::read(
                fd,
                &mut value as *mut u64 as *mut _,
                std::mem::size_of::<u64>(),
            )
        };

        // break once the fd is drained
        if result < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EWOULDBLOCK || errno == libc::EAGAIN {
                break;
            }
        } else {
            break;
        }
    }
}

fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    // capture the current errno
    let errno = core_platform::get_errno();
    io_error_with_errno(context, fd, errno)
}

fn io_error_with_errno(context: &str, fd: Option<RawFd>, errno: i32) -> Box<RuntimeError> {
    // format the error message
    let message = format!("{context} failed: errno {errno}");
    let code = io_error_code_from_errno(errno);
    let mut error = PlatformError::io_with(
        code,
        None,
        Some(errno),
        Some(context.to_string()),
        None,
        message,
    );

    // attach the file descriptor when available
    if let Some(fd) = fd {
        let mut context = error
            .context
            .unwrap_or_else(|| PlatformErrorContext::with_kind(PlatformErrorContextKind::Io));
        context.fd = Some(fd);
        error.context = Some(context);
    }

    RuntimeError::from(error).boxed()
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, UdpSocket};
    use std::os::unix::io::AsRawFd;

    use super::IoUringProactor;
    use crate::platform::ResourceId;
    use crate::platform::proactor::{
        Proactor, ProactorAddress, ProactorAddressStorage, ProactorBuffer, ProactorCompletion,
        ProactorCompletionData, ProactorOp, ProactorOpKind, ProactorRequest,
    };
    use crate::runtime::poller::{PlatformHandle, PlatformInterest};

    /// Build a sockaddr_in for a loopback socket.
    fn socket_addr_v4(addr: SocketAddrV4) -> libc::sockaddr_in {
        libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: addr.port().to_be(),
            sin_addr: libc::in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()).to_be(),
            },
            sin_zero: [0; 8],
        }
    }

    /// Return the size of a sockaddr_in.
    fn socket_addr_len() -> u32 {
        std::mem::size_of::<libc::sockaddr_in>() as u32
    }

    /// Poll until the expected completion count is reached.
    fn poll_until(proactor: &mut IoUringProactor, expected: usize) -> Vec<ProactorCompletion> {
        // collect completions until the expected count is reached
        let mut output = Vec::new();
        for _ in 0..8 {
            // poll for new completions
            let mut completions = proactor.poll(Some(5_000_000)).expect("poll should work");
            output.append(&mut completions);

            // stop once the expected count is reached
            if output.len() >= expected {
                break;
            }
        }

        output
    }

    /// Ensures io_uring can complete a read and write.
    #[test]
    fn test_proactor_read_write() {
        // create a pipe for read and write testing
        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(result, 0);

        // create the proactor and handles
        let mut proactor = IoUringProactor::new().expect("io_uring should initialize");
        let write_handle = PlatformHandle::from_raw_fd(fds[1]);
        let read_handle = PlatformHandle::from_raw_fd(fds[0]);

        // allocate the buffers
        let mut read_buffer = vec![0u8; 4];
        let write_buffer = b"pong";

        // submit the read request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(1),
                token: 1,
                op: ProactorOp::Read {
                    handle: read_handle,
                    buffer: ProactorBuffer {
                        data: read_buffer.as_mut_ptr(),
                        len: read_buffer.len() as u32,
                    },
                    offset: None,
                },
            })
            .expect("read should submit");

        // submit the write request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(2),
                token: 2,
                op: ProactorOp::Write {
                    handle: write_handle,
                    buffer: ProactorBuffer {
                        data: write_buffer.as_ptr() as *mut u8,
                        len: write_buffer.len() as u32,
                    },
                    offset: None,
                },
            })
            .expect("write should submit");

        // collect completions
        let completions = poll_until(&mut proactor, 2);
        assert_eq!(completions.len(), 2);

        // verify the data payload
        read_buffer.sort();
        let mut expected = write_buffer.to_vec();
        expected.sort();
        assert_eq!(read_buffer, expected);

        // close the pipe handles
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    /// Ensures poll operations yield readiness masks.
    #[test]
    fn test_proactor_poll_readable() {
        // create a pipe for poll testing
        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(result, 0);

        // create the proactor and read handle
        let mut proactor = IoUringProactor::new().expect("io_uring should initialize");
        let read_handle = PlatformHandle::from_raw_fd(fds[0]);

        // submit the poll request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(3),
                token: 3,
                op: ProactorOp::Poll {
                    handle: read_handle,
                    interests: PlatformInterest::READABLE,
                },
            })
            .expect("poll submit should work");

        // write a byte to trigger readiness
        let byte = [1u8];
        let wrote = unsafe { libc::write(fds[1], byte.as_ptr() as *const _, byte.len()) };
        assert_eq!(wrote, 1);

        // collect completions
        let completions = poll_until(&mut proactor, 1);
        assert_eq!(completions.len(), 1);
        assert!(matches!(
            completions[0].data,
            ProactorCompletionData::Poll { .. }
        ));

        // close the pipe handles
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    /// Ensures recvfrom and sendto flow through io_uring.
    #[test]
    fn test_proactor_udp_send_recv() {
        // bind a UDP server socket
        let server = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("udp bind should work");
        let server_addr = server.local_addr().expect("udp addr should exist");

        // bind a UDP client socket
        let client = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("udp bind should work");

        // create the proactor and handles
        let mut proactor = IoUringProactor::new().expect("io_uring should initialize");
        let server_handle = PlatformHandle::from_raw_fd(server.as_raw_fd());
        let client_handle = PlatformHandle::from_raw_fd(client.as_raw_fd());

        // allocate the recv buffer and address storage
        let mut recv_buffer = [0u8; 4];
        // safety: sockaddr_storage is plain old data
        let mut addr_storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut addr_len = std::mem::size_of::<libc::sockaddr_storage>() as u32;

        // submit the recvfrom request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(10),
                token: 10,
                op: ProactorOp::RecvFrom {
                    handle: server_handle,
                    buffer: ProactorBuffer {
                        data: recv_buffer.as_mut_ptr(),
                        len: recv_buffer.len() as u32,
                    },
                    flags: 0,
                    address: ProactorAddressStorage {
                        data: &mut addr_storage as *mut _ as *mut u8,
                        len: &mut addr_len as *mut u32,
                    },
                },
            })
            .expect("recvfrom submit should work");

        // submit the sendto request
        let server_addr = match server_addr {
            std::net::SocketAddr::V4(addr) => addr,
            _ => panic!("expected ipv4 addr"),
        };
        let sockaddr = socket_addr_v4(server_addr);
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(11),
                token: 11,
                op: ProactorOp::SendTo {
                    handle: client_handle,
                    buffer: ProactorBuffer {
                        data: b"ping".as_ptr() as *mut u8,
                        len: 4,
                    },
                    flags: 0,
                    address: ProactorAddress {
                        data: &sockaddr as *const _ as *const u8,
                        len: socket_addr_len(),
                    },
                },
            })
            .expect("sendto submit should work");

        // collect completions
        let completions = poll_until(&mut proactor, 2);
        assert_eq!(completions.len(), 2);

        // verify recv completion metadata
        let recv_completion = completions
            .iter()
            .find(|completion| completion.op == ProactorOpKind::RecvFrom)
            .expect("recvfrom completion should exist");
        assert!(matches!(
            recv_completion.data,
            ProactorCompletionData::Recv { .. }
        ));
        assert_eq!(&recv_buffer, b"ping");
    }

    /// Ensures accept and connect flow through io_uring.
    #[test]
    fn test_proactor_accept_connect() {
        // bind a TCP listener
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("tcp listen should work");
        let listener_addr = listener.local_addr().expect("listener addr should exist");

        // create a raw client socket
        let client_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
        assert!(client_fd >= 0);

        // create the proactor and handles
        let mut proactor = IoUringProactor::new().expect("io_uring should initialize");
        let listen_handle = PlatformHandle::from_raw_fd(listener.as_raw_fd());
        let client_handle = PlatformHandle::from_raw_fd(client_fd);

        // submit the accept request
        // safety: sockaddr_storage is plain old data
        let mut addr_storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut addr_len = std::mem::size_of::<libc::sockaddr_storage>() as u32;
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(20),
                token: 20,
                op: ProactorOp::Accept {
                    handle: listen_handle,
                    address: Some(ProactorAddressStorage {
                        data: &mut addr_storage as *mut _ as *mut u8,
                        len: &mut addr_len as *mut u32,
                    }),
                },
            })
            .expect("accept submit should work");

        // submit the connect request
        let listener_addr = match listener_addr {
            std::net::SocketAddr::V4(addr) => addr,
            _ => panic!("expected ipv4 addr"),
        };
        let sockaddr = socket_addr_v4(listener_addr);
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(21),
                token: 21,
                op: ProactorOp::Connect {
                    handle: client_handle,
                    address: ProactorAddress {
                        data: &sockaddr as *const _ as *const u8,
                        len: socket_addr_len(),
                    },
                },
            })
            .expect("connect submit should work");

        // collect completions
        let completions = poll_until(&mut proactor, 2);
        assert_eq!(completions.len(), 2);

        // validate the accept completion
        let accept_completion = completions
            .iter()
            .find(|completion| completion.op == ProactorOpKind::Accept)
            .expect("accept completion should exist");
        let accepted_handle = match accept_completion.data {
            ProactorCompletionData::Accept { handle, .. } => handle,
            _ => PlatformHandle::from_raw_fd(-1),
        };
        assert!(accepted_handle.as_raw_fd() >= 0);

        // close the client and accepted sockets
        unsafe {
            libc::close(client_fd);
            libc::close(accepted_handle.as_raw_fd());
        }
    }

    /// Ensures timeout operations yield completions.
    #[test]
    fn test_proactor_timeout() {
        // create the proactor
        let mut proactor = IoUringProactor::new().expect("io_uring should initialize");

        // submit the timeout request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId::local(30),
                token: 30,
                op: ProactorOp::Timeout {
                    timeout_ns: 1_000_000,
                },
            })
            .expect("timeout submit should work");

        // collect the completion
        let completions = poll_until(&mut proactor, 1);
        assert_eq!(completions.len(), 1);
        assert!(matches!(
            completions[0].data,
            ProactorCompletionData::Timeout
        ));
    }
}
