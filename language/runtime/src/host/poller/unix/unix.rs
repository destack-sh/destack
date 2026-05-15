use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::time::Instant;

use libc::{c_int, c_short, poll, pollfd};

use crate::diagnostic::{
    HostErrorContext, HostErrorContextKind, RuntimeError, RuntimeResult, io_error_code_from_errno,
};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::{HostError, ResourceId, core as host_core};

/// Poll based host poller for Unix systems.
#[derive(Debug)]
pub(crate) struct UnixPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Read end of the wake pipe.
    wake_read: RawFd,
    /// Write end of the wake pipe.
    wake_write: RawFd,
    /// Shared wake handle used by out-of-band wakeups.
    wake_handle: Arc<UnixPollWakeHandle>,
    /// Pollfd buffer reused across polls.
    pollfds: Vec<pollfd>,
    /// Entry buffer reused across polls.
    entries: Vec<Option<(ResourceId, PollRegistration)>>,
}

/// Poll registration state for a resource.
#[derive(Debug, Clone, Copy)]
struct PollRegistration {
    /// Raw file descriptor to poll.
    fd: RawFd,
    /// Opaque token associated with the registration.
    token: PollerToken,
    /// Interest mask for readiness.
    interests: PollInterest,
    /// Poller configuration flags.
    flags: HostPollerFlags,
}

/// Shared wake handle for one unix poll poller.
#[derive(Debug)]
struct UnixPollWakeHandle {
    /// Write end of the wake pipe.
    wake_write: RawFd,
}

impl PollerWakeHandle for UnixPollWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        wake_pipe(self.wake_write)
    }
}

impl UnixPoller {
    /// Create a new Unix poller instance.
    pub(crate) fn new() -> RuntimeResult<Self> {
        // wake pipe used to interrupt blocking polls
        let (wake_read, wake_write) = create_wake_pipe()?;

        // assemble the poller state
        Ok(Self {
            registrations: HashMap::new(),
            wake_read,
            wake_write,
            wake_handle: Arc::new(UnixPollWakeHandle { wake_write }),
            pollfds: Vec::new(),
            entries: Vec::new(),
        })
    }
}

impl Drop for UnixPoller {
    fn drop(&mut self) {
        // close wake pipe descriptors
        close_fd(self.wake_read);
        close_fd(self.wake_write);
    }
}

impl HostPoller for UnixPoller {
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: HostHandle,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        // reject reserved tokens
        if token.is_reserved() {
            return Err(RuntimeError::from(HostError::invalid_argument_value(
                "token",
                "token reserved for poller internals",
            ))
            .boxed());
        }

        // reject unsupported edge-triggered registrations
        if flags.contains(HostPollerFlags::EDGE) {
            return Err(RuntimeError::from(HostError::not_supported(
                "poller.edge is not supported by poll",
            ))
            .boxed());
        }

        // store the registration entry
        let entry = PollRegistration {
            fd: handle.as_raw_fd(),
            token,
            interests,
            flags,
        };
        self.registrations.insert(resource_id, entry);

        Ok(())
    }

    fn update(
        &mut self,
        resource_id: ResourceId,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        // reject reserved tokens
        if token.is_reserved() {
            return Err(RuntimeError::from(HostError::invalid_argument_value(
                "token",
                "token reserved for poller internals",
            ))
            .boxed());
        }

        // reject unsupported edge-triggered registrations
        if flags.contains(HostPollerFlags::EDGE) {
            return Err(RuntimeError::from(HostError::not_supported(
                "poller.edge is not supported by poll",
            ))
            .boxed());
        }

        // resolve the existing registration
        let entry = self.registrations.get_mut(&resource_id).ok_or_else(|| {
            RuntimeError::ResourceNotFound {
                resource_id: resource_id.local_id,
                resource_kind: None,
            }
            .boxed()
        })?;

        // update interest and flags
        entry.interests = interests;
        entry.flags = flags;
        entry.token = token;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        // remove the registration when present
        self.registrations.remove(&resource_id);

        Ok(())
    }

    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        Some(self.wake_handle.clone())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        self.wake_handle.wake()
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        // build pollfd array including wake pipe
        self.pollfds.clear();
        self.entries.clear();
        self.pollfds.reserve(self.registrations.len() + 1);
        self.entries.reserve(self.registrations.len() + 1);

        self.pollfds.push(pollfd {
            fd: self.wake_read,
            events: libc::POLLIN,
            revents: 0,
        });
        self.entries.push(None);

        for (resource_id, entry) in &self.registrations {
            let events = poll_events_for_interest(entry.interests, entry.flags);
            self.pollfds.push(pollfd {
                fd: entry.fd,
                events,
                revents: 0,
            });
            self.entries.push(Some((*resource_id, *entry)));
        }

        // resolve the poll deadline once so EINTR does not reset the timeout budget
        let deadline = timeout_nanos.and_then(host_core::timeout_deadline);

        // call poll and surface errors
        let _ = loop {
            let timeout_ms = timeout_ms_from_deadline(deadline);
            let result = poll_ready(
                self.pollfds.as_mut_ptr(),
                self.pollfds.len() as libc::nfds_t,
                timeout_ms,
            );
            if result >= 0 {
                break result;
            }

            let errno = host_core::get_errno();
            if errno != libc::EINTR {
                return Err(io_error("poller.poll", None));
            }
        };

        // collect emitted events
        let mut events = Vec::with_capacity(self.registrations.len());
        let mut oneshot = Vec::new();
        for (index, pollfd) in self.pollfds.iter().enumerate() {
            if pollfd.revents == 0 {
                continue;
            }

            if index == 0 {
                drain_wake(self.wake_read);
                continue;
            }

            let Some((resource_id, registration)) = self.entries[index] else {
                continue;
            };

            let mask = event_mask_from_revents(pollfd.revents);
            if mask.is_empty() {
                continue;
            }

            let flags = event_flags_from_registration(registration.flags);
            events.push(PollerEvent {
                resource_id,
                source: PollerEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PollerEventPayload::Io {
                    data: pollfd.revents as u64,
                },
            });

            if registration.flags.contains(HostPollerFlags::ONESHOT) {
                oneshot.push(resource_id);
            }
        }

        // drop any oneshot registrations
        if !oneshot.is_empty() {
            for resource_id in oneshot {
                self.registrations.remove(&resource_id);
            }
        }

        Ok(events)
    }
}

/// Close one file descriptor.
fn close_fd(fd: RawFd) {
    let _ = unsafe { libc::close(fd) };
}

/// Read bytes from one descriptor.
fn read_fd(fd: RawFd, buffer: &mut [u8]) -> isize {
    unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, buffer.len()) }
}

/// Write bytes to one descriptor.
fn write_fd(fd: RawFd, bytes: &[u8]) -> isize {
    unsafe { libc::write(fd, bytes.as_ptr() as *const _, bytes.len()) }
}

/// Create one pipe.
fn pipe_fds(fds: &mut [RawFd; 2]) -> c_int {
    unsafe { libc::pipe(fds.as_mut_ptr()) }
}

/// Read one descriptor control value.
fn fcntl_get(fd: RawFd, command: c_int) -> c_int {
    unsafe { libc::fcntl(fd, command) }
}

/// Set one descriptor control value.
fn fcntl_set(fd: RawFd, command: c_int, value: c_int) -> c_int {
    unsafe { libc::fcntl(fd, command, value) }
}

/// Wait for poll readiness.
fn poll_ready(fds: *mut pollfd, len: libc::nfds_t, timeout_ms: c_int) -> c_int {
    unsafe { poll(fds, len, timeout_ms) }
}

/// Write one wake byte into one wake pipe.
fn wake_pipe(wake_write: RawFd) -> RuntimeResult<()> {
    let byte = [1u8];
    let result = write_fd(wake_write, &byte);
    if result < 0 {
        let errno = host_core::get_errno();
        if errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            return Err(io_error("poller.wake", Some(wake_write)));
        }
    }

    Ok(())
}

/// Convert interests and flags into poll events.
fn poll_events_for_interest(interests: PollInterest, flags: HostPollerFlags) -> c_short {
    // build the poll event mask
    let mut events: c_short = 0;
    if interests.contains(PollInterest::READABLE) {
        events |= libc::POLLIN;
    }
    if interests.contains(PollInterest::WRITABLE) {
        events |= libc::POLLOUT;
    }
    if flags.contains(HostPollerFlags::PRIORITY) {
        events |= libc::POLLPRI;
    }

    events
}

/// Build event flags from registration flags.
fn event_flags_from_registration(flags: HostPollerFlags) -> PollerEventFlags {
    // expose edge and oneshot flags to consumers
    let mut out = PollerEventFlags::NONE;
    if flags.contains(HostPollerFlags::EDGE) {
        out |= PollerEventFlags::EDGE;
    }
    if flags.contains(HostPollerFlags::ONESHOT) {
        out |= PollerEventFlags::ONESHOT;
    }

    out
}

/// Build event mask from poll revents.
fn event_mask_from_revents(revents: c_short) -> PollerEventMask {
    // translate poll events into the runtime mask
    let mut mask = PollerEventMask::NONE;

    if (revents & libc::POLLIN) != 0 {
        mask |= PollerEventMask::READABLE;
    }
    if (revents & libc::POLLOUT) != 0 {
        mask |= PollerEventMask::WRITABLE;
    }
    if (revents & (libc::POLLERR | libc::POLLNVAL)) != 0 {
        mask |= PollerEventMask::ERROR;
    }
    if (revents & libc::POLLHUP) != 0 {
        mask |= PollerEventMask::HANGUP;
    }
    if (revents & libc::POLLPRI) != 0 {
        mask |= PollerEventMask::PRIORITY;
    }

    mask
}

/// Create the wake pipe used to interrupt polls.
fn create_wake_pipe() -> RuntimeResult<(RawFd, RawFd)> {
    // allocate a pipe for wakeup signaling
    let mut fds = [0; 2];
    let result = pipe_fds(&mut fds);
    if result < 0 {
        return Err(io_error("poller.pipe", None));
    }

    // mark both ends nonblocking
    set_nonblocking(fds[0])?;
    set_nonblocking(fds[1])?;

    // mark both ends close-on-exec
    set_close_on_exec(fds[0])?;
    set_close_on_exec(fds[1])?;

    Ok((fds[0], fds[1]))
}

/// Mark a file descriptor as nonblocking.
fn set_nonblocking(fd: RawFd) -> RuntimeResult<()> {
    // read the current descriptor flags
    let flags = fcntl_get(fd, libc::F_GETFL);
    if flags < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    // update the descriptor flags
    let result = fcntl_set(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    if result < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    Ok(())
}

/// Mark a file descriptor as close-on-exec.
fn set_close_on_exec(fd: RawFd) -> RuntimeResult<()> {
    // read the current descriptor flags
    let flags = fcntl_get(fd, libc::F_GETFD);
    if flags < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    // update the descriptor flags
    let result = fcntl_set(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
    if result < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    Ok(())
}

/// Drain the wake pipe so it stops signaling readiness.
fn drain_wake(fd: RawFd) {
    // read until the pipe is empty
    let mut buffer = [0u8; 64];
    loop {
        let read_bytes = read_fd(fd, &mut buffer);
        if read_bytes > 0 {
            continue;
        }
        if read_bytes == 0 {
            break;
        }

        let errno = host_core::get_errno();
        if errno == libc::EWOULDBLOCK || errno == libc::EAGAIN {
            break;
        }
        break;
    }
}

/// Convert one absolute deadline into one poll timeout in milliseconds.
fn timeout_ms_from_deadline(deadline: Option<Instant>) -> c_int {
    let Some(deadline) = deadline else {
        return -1;
    };

    // map elapsed deadlines directly to immediate polls
    let now = Instant::now();
    if now >= deadline {
        return 0;
    }

    // round up to the nearest millisecond
    let remaining = deadline.saturating_duration_since(now);
    let ms = remaining.as_nanos().div_ceil(1_000_000);
    if ms > i32::MAX as u128 {
        i32::MAX
    } else {
        ms as i32
    }
}

/// Convert the last OS error into a runtime error.
fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    // capture the last OS error
    let errno = host_core::get_errno();
    let message = format!("{context} failed: errno {errno}");

    // map the error into host diagnostics
    let code = io_error_code_from_errno(errno);
    let error = HostError::io_with(
        code,
        None,
        Some(errno),
        Some(context.to_string()),
        None,
        message,
    );
    let mut error = error;
    if let Some(fd) = fd {
        let mut context = error
            .context
            .unwrap_or_else(|| HostErrorContext::with_kind(HostErrorContextKind::Io));
        context.fd = Some(fd);
        error.context = Some(context);
    }

    RuntimeError::from(error).boxed()
}

#[cfg(test)]
mod tests {
    use super::{UnixPoller, close_fd, pipe_fds, write_fd};
    use crate::host::ResourceId;
    use crate::host::poller::{HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerToken};
    use crate::runtime::WorkerId;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    /// Ensures poll emits a readable event when data is available.
    #[test]
    fn test_poll_readable_event() {
        let mut poller = UnixPoller::new().expect("poller should initialize");

        let mut fds = [0; 2];
        let result = pipe_fds(&mut fds);
        assert!(result == 0);

        let read_fd = fds[0];
        let write_descriptor = fds[1];

        let handle = HostHandle::from_raw_fd(read_fd);
        poller
            .register(
                ResourceId::new(TEST_WORKER_ID, 1),
                handle,
                PollerToken(1),
                PollInterest::READABLE,
                HostPollerFlags::NONE,
            )
            .expect("register should succeed");

        let payload = [1u8];
        let wrote = write_fd(write_descriptor, &payload);
        assert!(wrote >= 0);

        let events = poller.poll(Some(0)).expect("poll should return events");
        assert!(!events.is_empty());

        close_fd(read_fd);
        close_fd(write_descriptor);
    }
}
