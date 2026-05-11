use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::proactor::{
    Proactor, ProactorBufferVec, ProactorCompletion, ProactorCompletionData, ProactorOp,
    ProactorOpKind, ProactorRequest, ProactorShutdown,
};
use crate::platform::{PlatformError, PlatformErrorCode, ResourceId, core as core_platform};
use crate::runtime::poller::{PlatformHandle, PlatformInterest, PollerEventMask};
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

/// Sentinel token emitted by wake notifications.
const WAKE_TOKEN: u64 = u64::MAX;
/// Reserved resource id emitted by internal control completions.
const INTERNAL_RESOURCE_ID: ResourceId = ResourceId(0);

/// Command sent from the proactor frontend to the worker.
#[derive(Debug, Clone, Copy)]
enum WorkerCommand {
    /// Submit a new operation request.
    Submit(QueuedRequest),
    /// Cancel one token before execution.
    Cancel(u64),
    /// Stop the worker thread.
    Shutdown,
}

/// Queued request payload crossing into the worker thread.
#[derive(Debug, Clone, Copy)]
struct QueuedRequest(
    /// Request payload forwarded to the worker.
    ProactorRequest,
);

/// Safety: request pointers are treated as caller-owned buffers that must remain valid.
unsafe impl Send for QueuedRequest {}

/// Poll based fallback proactor for non-Linux Unix targets.
#[derive(Debug)]
pub struct UnixProactor {
    /// Command sender for the worker thread.
    command_sender: Sender<WorkerCommand>,
    /// Completion sender used by wake notifications.
    completion_sender: Sender<ProactorCompletion>,
    /// Completion receiver drained by poll.
    completion_receiver: Receiver<ProactorCompletion>,
    /// Buffered completions ready to return.
    completions: VecDeque<ProactorCompletion>,
    /// Worker thread handle.
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl UnixProactor {
    /// Create a new Unix fallback proactor.
    pub fn new() -> RuntimeResult<Self> {
        // create command and completion channels
        let (command_sender, command_receiver) = mpsc::channel();
        let (completion_sender, completion_receiver) = mpsc::channel();

        // start the worker loop
        let worker_completion_sender = completion_sender.clone();
        let worker_handle = start_with_policy(
            "destack-unix-proactor",
            "proactor.unix.open",
            ExecutionPolicy::resource(ExecutionMode::Thread),
            move || worker_main(command_receiver, worker_completion_sender),
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

        Ok(Self {
            command_sender,
            completion_sender,
            completion_receiver,
            completions: VecDeque::new(),
            worker_handle: Some(worker_handle),
        })
    }
}

impl Drop for UnixProactor {
    fn drop(&mut self) {
        // request worker shutdown
        let _ = self.command_sender.send(WorkerCommand::Shutdown);

        // join the worker thread
        if let Some(worker_handle) = self.worker_handle.take() {
            let _ = worker_handle.join();
        }
    }
}

impl Proactor for UnixProactor {
    fn submit(&mut self, request: ProactorRequest) -> RuntimeResult<()> {
        // forward request to worker
        self.command_sender
            .send(WorkerCommand::Submit(QueuedRequest(request)))
            .map_err(|_| worker_channel_closed("proactor.unix.submit"))?;

        Ok(())
    }

    fn cancel(&mut self, token: u64) -> RuntimeResult<()> {
        // forward cancellation to worker
        self.command_sender
            .send(WorkerCommand::Cancel(token))
            .map_err(|_| worker_channel_closed("proactor.unix.cancel"))?;

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<ProactorCompletion>> {
        // first drain any ready completions
        drain_completion_channel(&self.completion_receiver, &mut self.completions)?;

        // if no completion is ready, wait for one when timeout permits
        if self.completions.is_empty() {
            match timeout_nanos {
                // wait indefinitely
                None => {
                    let completion = self
                        .completion_receiver
                        .recv()
                        .map_err(|_| worker_channel_closed("proactor.unix.poll"))?;
                    self.completions.push_back(completion);
                }
                // return immediately on zero timeout
                Some(0) => {}
                // wait for the configured timeout
                Some(timeout_nanos) => {
                    let timeout = Duration::from_nanos(timeout_nanos);
                    match self.completion_receiver.recv_timeout(timeout) {
                        Ok(completion) => {
                            self.completions.push_back(completion);
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            return Err(worker_channel_closed("proactor.unix.poll"));
                        }
                    }
                }
            }
        }

        // drain completions that arrived after the first one
        drain_completion_channel(&self.completion_receiver, &mut self.completions)?;

        // build the output list while filtering wake completions
        let mut output = Vec::with_capacity(self.completions.len());
        while let Some(completion) = self.completions.pop_front() {
            // drop internal wake notifications
            if completion.token == WAKE_TOKEN {
                continue;
            }

            output.push(completion);
        }

        Ok(output)
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        // enqueue an internal wake completion
        self.completion_sender
            .send(wake_completion())
            .map_err(|_| worker_channel_closed("proactor.unix.wake"))?;

        Ok(())
    }
}

/// Run the worker loop for the fallback Unix proactor.
fn worker_main(
    command_receiver: Receiver<WorkerCommand>,
    completion_sender: Sender<ProactorCompletion>,
) {
    // track canceled tokens before execution starts
    let mut canceled_tokens = HashSet::new();

    // process commands until shutdown
    while let Ok(command) = command_receiver.recv() {
        // process one command
        match command {
            // submit one operation
            WorkerCommand::Submit(request) => {
                let request = request.0;
                // skip canceled requests before execution
                if canceled_tokens.remove(&request.token) {
                    let completion =
                        canceled_completion(request.resource_id, request.token, request.op.kind());
                    let _ = completion_sender.send(completion);
                    continue;
                }

                // execute the operation concurrently and emit one completion
                let completion_sender = completion_sender.clone();
                let completion_sender_for_spawn = completion_sender.clone();
                let queued_request = QueuedRequest(request);
                let spawn_result = start_with_policy(
                    "destack-unix-proactor-op",
                    "proactor.unix.submit",
                    ExecutionPolicy::task(ExecutionMode::Blocking),
                    move || {
                        let completion = execute_request(queued_request.0);
                        let _ = completion_sender_for_spawn.send(completion);
                    },
                );
                if spawn_result.is_err() {
                    let completion = thread_spawn_failed_completion(request);
                    let _ = completion_sender.send(completion);
                }
            }
            // mark a token as canceled
            WorkerCommand::Cancel(token) => {
                canceled_tokens.insert(token);
            }
            // stop the worker loop
            WorkerCommand::Shutdown => {
                break;
            }
        }
    }
}

/// Drain available completions from the receiver.
fn drain_completion_channel(
    receiver: &Receiver<ProactorCompletion>,
    completions: &mut VecDeque<ProactorCompletion>,
) -> RuntimeResult<()> {
    // pull all currently queued completions
    loop {
        match receiver.try_recv() {
            Ok(completion) => {
                completions.push_back(completion);
            }
            Err(TryRecvError::Empty) => {
                break;
            }
            Err(TryRecvError::Disconnected) => {
                return Err(worker_channel_closed("proactor.unix.poll"));
            }
        }
    }

    Ok(())
}

/// Execute one request and return its completion record.
fn execute_request(request: ProactorRequest) -> ProactorCompletion {
    // execute operation by variant
    match request.op {
        // read bytes from the handle
        ProactorOp::Read {
            handle,
            buffer,
            offset,
        } => {
            let fd = handle.as_raw_fd();
            let result = if let Some(offset) = offset {
                unsafe {
                    libc::pread(
                        fd,
                        buffer.data.cast::<libc::c_void>(),
                        buffer.len as usize,
                        offset as libc::off_t,
                    )
                }
            } else {
                unsafe { libc::read(fd, buffer.data.cast::<libc::c_void>(), buffer.len as usize) }
            };
            syscall_completion(request, result as i32)
        }
        // write bytes to the handle
        ProactorOp::Write {
            handle,
            buffer,
            offset,
        } => {
            let fd = handle.as_raw_fd();
            let result = if let Some(offset) = offset {
                unsafe {
                    libc::pwrite(
                        fd,
                        buffer.data.cast::<libc::c_void>(),
                        buffer.len as usize,
                        offset as libc::off_t,
                    )
                }
            } else {
                unsafe { libc::write(fd, buffer.data.cast::<libc::c_void>(), buffer.len as usize) }
            };
            syscall_completion(request, result as i32)
        }
        // read into scatter buffers
        ProactorOp::Readv {
            handle,
            buffers,
            offset,
        } => {
            let fd = handle.as_raw_fd();
            let result = match iovecs_from_buffers(request, buffers) {
                Ok(mut iovecs) => {
                    if let Some(offset) = offset {
                        unsafe {
                            libc::preadv(
                                fd,
                                iovecs.as_mut_ptr(),
                                iovecs.len() as i32,
                                offset as libc::off_t,
                            )
                        }
                    } else {
                        unsafe { libc::readv(fd, iovecs.as_mut_ptr(), iovecs.len() as i32) }
                    }
                }
                Err(completion) => {
                    return completion;
                }
            };

            syscall_completion(request, result as i32)
        }
        // write from scatter buffers
        ProactorOp::Writev {
            handle,
            buffers,
            offset,
        } => {
            let fd = handle.as_raw_fd();
            let result = match iovecs_from_buffers(request, buffers) {
                Ok(mut iovecs) => {
                    if let Some(offset) = offset {
                        unsafe {
                            libc::pwritev(
                                fd,
                                iovecs.as_mut_ptr(),
                                iovecs.len() as i32,
                                offset as libc::off_t,
                            )
                        }
                    } else {
                        unsafe { libc::writev(fd, iovecs.as_mut_ptr(), iovecs.len() as i32) }
                    }
                }
                Err(completion) => {
                    return completion;
                }
            };

            syscall_completion(request, result as i32)
        }
        // receive payload from socket
        ProactorOp::Recv {
            handle,
            buffer,
            flags,
        } => {
            let fd = handle.as_raw_fd();
            let result = unsafe {
                libc::recv(
                    fd,
                    buffer.data.cast::<libc::c_void>(),
                    buffer.len as usize,
                    flags as i32,
                )
            };
            let mut completion = syscall_completion(request, result as i32);
            if completion.result >= 0 {
                completion.data = ProactorCompletionData::Recv {
                    addr_len: 0,
                    flags: 0,
                };
            }
            completion
        }
        // send payload to socket
        ProactorOp::Send {
            handle,
            buffer,
            flags,
        } => {
            let fd = handle.as_raw_fd();
            let result = unsafe {
                libc::send(
                    fd,
                    buffer.data.cast::<libc::c_void>(),
                    buffer.len as usize,
                    flags as i32,
                )
            };
            syscall_completion(request, result as i32)
        }
        // receive datagram and source address
        ProactorOp::RecvFrom {
            handle,
            buffer,
            flags,
            address,
        } => {
            let fd = handle.as_raw_fd();

            // validate address storage pointers
            if address.len.is_null() {
                return invalid_argument_completion(request, "address.len");
            }
            if address.data.is_null() {
                return invalid_argument_completion(request, "address.data");
            }

            // perform recvfrom and capture address length
            let mut address_len = unsafe { *address.len as libc::socklen_t };
            let result = unsafe {
                libc::recvfrom(
                    fd,
                    buffer.data.cast::<libc::c_void>(),
                    buffer.len as usize,
                    flags as i32,
                    address.data.cast::<libc::sockaddr>(),
                    &mut address_len,
                )
            };

            if result < 0 {
                return syscall_completion(request, -1);
            }

            unsafe {
                *address.len = address_len;
            }

            let mut completion = syscall_completion(request, result as i32);
            completion.data = ProactorCompletionData::Recv {
                addr_len: address_len,
                flags: 0,
            };
            completion
        }
        // send datagram to destination address
        ProactorOp::SendTo {
            handle,
            buffer,
            flags,
            address,
        } => {
            let fd = handle.as_raw_fd();

            // validate destination address pointer
            if address.data.is_null() && address.len != 0 {
                return invalid_argument_completion(request, "address.data");
            }

            let result = unsafe {
                libc::sendto(
                    fd,
                    buffer.data.cast::<libc::c_void>(),
                    buffer.len as usize,
                    flags as i32,
                    address.data.cast::<libc::sockaddr>(),
                    address.len as libc::socklen_t,
                )
            };

            syscall_completion(request, result as i32)
        }
        // copy file range is Linux-specific and unsupported here
        ProactorOp::CopyFileRange { .. } => {
            not_supported_completion(request, "destack.io.copyFileRange")
        }
        // splice is Linux-specific and unsupported here
        ProactorOp::Splice { .. } => not_supported_completion(request, "destack.io.splice"),
        // sendfile support differs across Unix and is deferred for this backend
        ProactorOp::SendFile { .. } => not_supported_completion(request, "destack.io.sendFile"),
        // fsync file data and metadata
        ProactorOp::Fsync { handle } => {
            let fd = handle.as_raw_fd();
            let result = unsafe { libc::fsync(fd) };
            syscall_completion(request, result)
        }
        // fdatasync file data only
        ProactorOp::Fdatasync { handle } => {
            let fd = handle.as_raw_fd();
            let result = unsafe { libc::fsync(fd) };
            syscall_completion(request, result)
        }
        // fallocate semantics vary by platform and are deferred here
        ProactorOp::Fallocate { .. } => not_supported_completion(request, "destack.io.fallocate"),
        // accept one connection
        ProactorOp::Accept { handle, address } => {
            let fd = handle.as_raw_fd();

            // prepare optional address output
            let (address_ptr, address_len_ptr) = match address {
                Some(address) => {
                    if address.len.is_null() {
                        return invalid_argument_completion(request, "address.len");
                    }
                    if address.data.is_null() {
                        return invalid_argument_completion(request, "address.data");
                    }

                    let address_len = unsafe { *address.len as libc::socklen_t };
                    (
                        address.data.cast::<libc::sockaddr>(),
                        Some((address.len, address_len)),
                    )
                }
                None => (std::ptr::null_mut(), None),
            };

            // call accept
            let mut address_len = address_len_ptr
                .map(|(_, length)| length)
                .unwrap_or_default();
            let result = unsafe { libc::accept(fd, address_ptr, &mut address_len) };
            if result < 0 {
                return syscall_completion(request, -1);
            }

            // write address length back when requested
            if let Some((address_len_out, _)) = address_len_ptr {
                unsafe {
                    *address_len_out = address_len;
                }
            }

            let mut completion = syscall_completion(request, result);
            completion.data = ProactorCompletionData::Accept {
                handle: PlatformHandle::from_raw_fd(result),
                addr_len: address_len,
            };
            completion
        }
        // connect socket to remote address
        ProactorOp::Connect { handle, address } => {
            let fd = handle.as_raw_fd();

            // validate destination address pointer
            if address.data.is_null() && address.len != 0 {
                return invalid_argument_completion(request, "address.data");
            }

            let result = unsafe {
                libc::connect(
                    fd,
                    address.data.cast::<libc::sockaddr>(),
                    address.len as libc::socklen_t,
                )
            };
            syscall_completion(request, result)
        }
        // wait for readiness on one handle
        ProactorOp::Poll { handle, interests } => {
            let fd = handle.as_raw_fd();

            // build poll events from interests
            let mut events: libc::c_short = 0;
            if interests.contains(PlatformInterest::READABLE) {
                events |= libc::POLLIN;
            }
            if interests.contains(PlatformInterest::WRITABLE) {
                events |= libc::POLLOUT;
            }

            let mut pollfd = libc::pollfd {
                fd,
                events,
                revents: 0,
            };
            let result = unsafe { libc::poll(&mut pollfd, 1, -1) };
            if result < 0 {
                return syscall_completion(request, -1);
            }

            let mut mask = PollerEventMask::NONE;
            if (pollfd.revents & libc::POLLIN) != 0 {
                mask |= PollerEventMask::READABLE;
            }
            if (pollfd.revents & libc::POLLOUT) != 0 {
                mask |= PollerEventMask::WRITABLE;
            }
            if (pollfd.revents & libc::POLLERR) != 0 {
                mask |= PollerEventMask::ERROR;
            }
            if (pollfd.revents & libc::POLLHUP) != 0 {
                mask |= PollerEventMask::HANGUP;
            }
            if (pollfd.revents & libc::POLLPRI) != 0 {
                mask |= PollerEventMask::PRIORITY;
            }

            let mut completion = syscall_completion(request, result);
            completion.data = ProactorCompletionData::Poll { mask };
            completion
        }
        // shutdown socket direction
        ProactorOp::Shutdown { handle, how } => {
            let fd = handle.as_raw_fd();
            let shutdown_how = match how {
                ProactorShutdown::Read => libc::SHUT_RD,
                ProactorShutdown::Write => libc::SHUT_WR,
                ProactorShutdown::Both => libc::SHUT_RDWR,
            };
            let result = unsafe { libc::shutdown(fd, shutdown_how) };
            syscall_completion(request, result)
        }
        // close file descriptor
        ProactorOp::Close { handle } => {
            let fd = handle.as_raw_fd();
            let result = unsafe { libc::close(fd) };
            syscall_completion(request, result)
        }
        // cancel operation is handled by cancel(), not submit
        ProactorOp::Cancel { token } => {
            let _ = token;
            not_supported_completion(request, "destack.io.cancel")
        }
        // timeout completion
        ProactorOp::Timeout { timeout_ns } => {
            thread::sleep(Duration::from_nanos(timeout_ns));
            let mut completion = success_completion(request, 0);
            completion.data = ProactorCompletionData::Timeout;
            completion
        }
    }
}

/// Convert one buffer vector into iovec entries.
fn iovecs_from_buffers(
    request: ProactorRequest,
    buffers: ProactorBufferVec,
) -> Result<Vec<libc::iovec>, ProactorCompletion> {
    // validate the buffer vector pointer
    if buffers.data.is_null() && buffers.len != 0 {
        return Err(invalid_argument_completion(request, "buffers.data"));
    }

    // convert each buffer into iovec
    let list = unsafe { std::slice::from_raw_parts(buffers.data, buffers.len as usize) };
    let mut output = Vec::with_capacity(list.len());
    for buffer in list {
        output.push(libc::iovec {
            iov_base: buffer.data.cast::<libc::c_void>(),
            iov_len: buffer.len as usize,
        });
    }

    Ok(output)
}

/// Build a successful completion.
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

/// Build a completion from one syscall result.
fn syscall_completion(request: ProactorRequest, result: i32) -> ProactorCompletion {
    // return success when syscall did not fail
    if result >= 0 {
        return success_completion(request, result);
    }

    // otherwise map errno into error fields
    let errno = core_platform::get_errno();
    let code = io_error_code_from_errno(errno).or(Some(PlatformErrorCode::Io));
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: code,
        error_errno: Some(errno),
        data: ProactorCompletionData::None,
    }
}

/// Build a completion for invalid argument failures.
fn invalid_argument_completion(
    request: ProactorRequest,
    _argument: &'static str,
) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(PlatformErrorCode::InvalidArgumentValue),
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Build a completion for unsupported operation failures.
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

/// Build one completion for a canceled request token.
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
        error_errno: Some(libc::ECANCELED),
        data: ProactorCompletionData::None,
    }
}

/// Build a completion for one runtime helper-thread spawn failure.
fn thread_spawn_failed_completion(request: ProactorRequest) -> ProactorCompletion {
    ProactorCompletion {
        resource_id: request.resource_id,
        token: request.token,
        op: request.op.kind(),
        result: -1,
        error_code: Some(PlatformErrorCode::ProcessSpawnFailed),
        error_errno: None,
        data: ProactorCompletionData::None,
    }
}

/// Build the sentinel wake completion.
fn wake_completion() -> ProactorCompletion {
    ProactorCompletion {
        resource_id: INTERNAL_RESOURCE_ID,
        token: WAKE_TOKEN,
        op: ProactorOpKind::Timeout,
        result: 0,
        error_code: None,
        error_errno: None,
        data: ProactorCompletionData::Timeout,
    }
}

/// Build a runtime error for a closed worker channel.
fn worker_channel_closed(context: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInterrupted),
        None,
        Some(libc::EPIPE),
        Some(context.to_string()),
        None,
        "proactor worker channel closed".to_string(),
    ))
    .boxed()
}

#[cfg(test)]
mod tests {
    use std::os::fd::FromRawFd;
    use std::os::unix::io::RawFd;

    use super::UnixProactor;
    use crate::platform::ResourceId;
    use crate::platform::proactor::{
        Proactor, ProactorBuffer, ProactorCompletionData, ProactorOp, ProactorOpKind,
        ProactorRequest,
    };
    use crate::runtime::poller::PlatformHandle;

    /// Ensure timeout requests complete through the worker loop.
    #[test]
    fn test_unix_proactor_timeout() {
        // create the fallback proactor
        let mut proactor = UnixProactor::new().expect("proactor should initialize");

        // submit one timeout request
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId(1),
                token: 99,
                op: ProactorOp::Timeout {
                    timeout_ns: 1_000_000,
                },
            })
            .expect("submit should work");

        // wait for one completion
        let completions = proactor.poll(Some(100_000_000)).expect("poll should work");
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].op, ProactorOpKind::Timeout);
        assert_eq!(completions[0].token, 99);
        assert_eq!(completions[0].data, ProactorCompletionData::Timeout);
    }

    /// Ensure read and write operations complete with byte counts.
    #[test]
    fn test_unix_proactor_read_write() {
        // create the fallback proactor
        let mut proactor = UnixProactor::new().expect("proactor should initialize");

        // create one pipe pair
        let (read_fd, write_fd) = pipe_pair().expect("pipe should initialize");

        // submit read first so it can block
        let mut read_buffer = vec![0u8; 4];
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId(10),
                token: 1,
                op: ProactorOp::Read {
                    handle: PlatformHandle::from_raw_fd(read_fd),
                    buffer: ProactorBuffer {
                        data: read_buffer.as_mut_ptr(),
                        len: read_buffer.len() as u32,
                    },
                    offset: None,
                },
            })
            .expect("submit read should work");

        // submit write to unblock read
        let payload = b"pong".to_vec();
        proactor
            .submit(ProactorRequest {
                resource_id: ResourceId(11),
                token: 2,
                op: ProactorOp::Write {
                    handle: PlatformHandle::from_raw_fd(write_fd),
                    buffer: ProactorBuffer {
                        data: payload.as_ptr() as *mut u8,
                        len: payload.len() as u32,
                    },
                    offset: None,
                },
            })
            .expect("submit write should work");

        // collect both completions
        let mut completions = Vec::new();
        while completions.len() < 2 {
            let mut chunk = proactor.poll(Some(100_000_000)).expect("poll should work");
            completions.append(&mut chunk);
        }

        // validate completion outcomes
        completions.sort_by_key(|completion| completion.token);
        assert_eq!(completions[0].token, 1);
        assert_eq!(completions[0].result, 4);
        assert_eq!(completions[1].token, 2);
        assert_eq!(completions[1].result, 4);
        assert_eq!(read_buffer, b"pong");

        // close pipe descriptors
        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }

    /// Create one pipe pair for read and write tests.
    fn pipe_pair() -> Result<(RawFd, RawFd), std::io::Error> {
        // create the raw pipe descriptors
        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }

        // take ownership of both descriptors
        let read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        let write = unsafe { std::fs::File::from_raw_fd(fds[1]) };

        // transfer ownership back to raw descriptors
        Ok((
            std::os::fd::IntoRawFd::into_raw_fd(read),
            std::os::fd::IntoRawFd::into_raw_fd(write),
        ))
    }
}
