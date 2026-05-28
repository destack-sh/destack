use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::time::Instant;

use libc::{c_int, epoll_create1, epoll_ctl, epoll_event, epoll_wait};

use crate::diagnostic::{
    HostErrorContext, HostErrorContextKind, RuntimeError, RuntimeResult, io_error_code_from_errno,
};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::{HostError, ResourceId, core as host_core};

/// Epoll backed poller for Linux targets.
#[derive(Debug)]
pub(crate) struct EpollPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Token to resource mapping for event lookup.
    tokens: HashMap<PollerToken, ResourceId>,
    /// Epoll file descriptor.
    epoll_fd: RawFd,
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
    /// Shared wake handle used by out-of-band wakeups.
    wake_handle: Arc<EpollWakeHandle>,
    /// Event buffer reused across polls.
    events: Vec<epoll_event>,
}

/// Shared wake handle for one epoll poller.
#[derive(Debug)]
struct EpollWakeHandle {
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
}

impl PollerWakeHandle for EpollWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        wake_eventfd(self.wake_fd)
    }
}

/// Epoll registration state for a resource.
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

impl EpollPoller {
    /// Create a new epoll poller instance.
    pub(crate) fn new() -> RuntimeResult<Self> {
        let epoll_fd = create_epoll_fd()?;

        // allocate the wake eventfd used to interrupt polls
        let wake_fd = create_wake_eventfd()?;

        // register the wake pipe for readability
        let mut event = epoll_event {
            events: libc::EPOLLIN as u32,
            u64: PollerToken::WAKE.0,
        };
        let result = epoll_control(epoll_fd, libc::EPOLL_CTL_ADD, wake_fd, &mut event);
        if result < 0 {
            close_fd(epoll_fd);
            return Err(io_error("poller.epoll_ctl", Some(wake_fd)));
        }

        Ok(Self {
            registrations: HashMap::new(),
            tokens: HashMap::new(),
            epoll_fd,
            wake_fd,
            wake_handle: Arc::new(EpollWakeHandle { wake_fd }),
            events: Vec::new(),
        })
    }
}

impl Drop for EpollPoller {
    fn drop(&mut self) {
        // close wake and epoll descriptors
        close_fd(self.wake_fd);
        close_fd(self.epoll_fd);
    }
}

impl HostPoller for EpollPoller {
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

        // update existing registrations in place
        if self.registrations.contains_key(&resource_id) {
            return self.update(resource_id, token, interests, flags);
        }

        // reject tokens already in use
        if self.tokens.contains_key(&token) {
            return Err(RuntimeError::from(HostError::invalid_argument_value(
                "token",
                "token already registered",
            ))
            .boxed());
        }

        // add the epoll registration
        let fd = handle.as_raw_fd();
        let mut event = epoll_event {
            events: epoll_events_for_interest(interests, flags),
            u64: token.0,
        };
        let result = epoll_control(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event);
        if result < 0 {
            return Err(io_error("poller.epoll_ctl", Some(fd)));
        }

        // store the registration entry
        self.registrations.insert(
            resource_id,
            PollRegistration {
                fd,
                token,
                interests,
                flags,
            },
        );
        self.tokens.insert(token, resource_id);

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

        // resolve the existing registration
        let entry = self
            .registrations
            .get_mut(&resource_id)
            .ok_or_else(|| RuntimeError::resource_not_found(resource_id.local_id, None).boxed())?;

        // update the registration in epoll
        let mut event = epoll_event {
            events: epoll_events_for_interest(interests, flags),
            u64: token.0,
        };
        let result = epoll_control(self.epoll_fd, libc::EPOLL_CTL_MOD, entry.fd, &mut event);
        if result < 0 {
            return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
        }

        // update cached fields
        if entry.token != token {
            self.tokens.remove(&entry.token);
            if self.tokens.contains_key(&token) {
                return Err(RuntimeError::from(HostError::invalid_argument_value(
                    "token",
                    "token already registered",
                ))
                .boxed());
            }
            self.tokens.insert(token, resource_id);
            entry.token = token;
        }
        entry.interests = interests;
        entry.flags = flags;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        // drop the registration entry
        let Some(entry) = self.registrations.remove(&resource_id) else {
            return Ok(());
        };
        self.tokens.remove(&entry.token);

        // remove the epoll registration
        let result = epoll_control(
            self.epoll_fd,
            libc::EPOLL_CTL_DEL,
            entry.fd,
            std::ptr::null_mut(),
        );
        if result < 0 {
            return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
        }

        Ok(())
    }

    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        Some(self.wake_handle.clone())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        self.wake_handle.wake()
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        // ensure the event buffer can hold all registrations
        let max_events = self.registrations.len() + 1;
        if self.events.len() < max_events {
            self.events
                .resize_with(max_events, || epoll_event { events: 0, u64: 0 });
        }

        // resolve the poll deadline once so EINTR does not reset the timeout budget
        let deadline = timeout_nanos.and_then(host_core::timeout_deadline);

        // call into epoll
        let result = loop {
            let timeout_ms = timeout_ms_from_deadline(deadline);
            let result = epoll_wait_ready(
                self.epoll_fd,
                self.events.as_mut_ptr(),
                max_events as c_int,
                timeout_ms,
            );
            if result >= 0 {
                break result;
            }

            let errno = host_core::get_errno();
            if errno != libc::EINTR {
                return Err(io_error("poller.epoll_wait", None));
            }
        };

        // collect emitted events
        let mut output = Vec::with_capacity(result as usize);
        let mut oneshot = Vec::new();
        for event in self.events.iter().take(result as usize) {
            if event.u64 == PollerToken::WAKE.0 {
                drain_wake(self.wake_fd);
                continue;
            }

            let token = PollerToken(event.u64);
            let Some(resource_id) = self.tokens.get(&token).copied() else {
                continue;
            };
            let Some(registration) = self.registrations.get(&resource_id) else {
                continue;
            };

            let mask = event_mask_from_epoll(event.events);
            if mask.is_empty() {
                continue;
            }

            let flags = event_flags_from_registration(registration.flags);
            output.push(PollerEvent {
                resource_id,
                source: PollerEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PollerEventPayload::Io {
                    data: event.events as u64,
                },
            });

            if registration.flags.contains(HostPollerFlags::ONESHOT) {
                oneshot.push(resource_id);
            }
        }

        // drop any oneshot registrations
        if !oneshot.is_empty() {
            for resource_id in oneshot {
                if let Some(entry) = self.registrations.remove(&resource_id) {
                    self.tokens.remove(&entry.token);
                    let result = epoll_control(
                        self.epoll_fd,
                        libc::EPOLL_CTL_DEL,
                        entry.fd,
                        std::ptr::null_mut(),
                    );
                    if result < 0 {
                        return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Create one epoll descriptor.
fn create_epoll_fd() -> RuntimeResult<RawFd> {
    // SAFETY: epoll_create1 has no pointer arguments and returns either an fd or errno
    let fd = unsafe { epoll_create1(libc::EPOLL_CLOEXEC) };
    if fd < 0 {
        return Err(io_error("poller.epoll_create1", None));
    }

    Ok(fd)
}

/// Create one nonblocking event descriptor.
fn create_event_fd() -> RawFd {
    // SAFETY: eventfd has no pointer arguments and returns either an fd or errno
    unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) }
}

/// Close one file descriptor.
fn close_fd(fd: RawFd) {
    // SAFETY: callers pass file descriptors owned by this poller or setup path
    let _ = unsafe { libc::close(fd) };
}

/// Control one epoll registration.
fn epoll_control(epoll_fd: RawFd, operation: c_int, fd: RawFd, event: *mut epoll_event) -> c_int {
    // SAFETY: caller passes either a valid epoll_event pointer or null for EPOLL_CTL_DEL
    unsafe { epoll_ctl(epoll_fd, operation, fd, event) }
}

/// Wait for epoll readiness.
fn epoll_wait_ready(
    epoll_fd: RawFd,
    events: *mut epoll_event,
    max_events: c_int,
    timeout_ms: c_int,
) -> c_int {
    // SAFETY: events points to a buffer with at least max_events epoll_event entries
    unsafe { epoll_wait(epoll_fd, events, max_events, timeout_ms) }
}

/// Read one u64 from a descriptor.
fn read_u64(fd: RawFd, value: &mut u64) -> isize {
    // SAFETY: value is a valid writable u64 buffer for the requested byte count
    unsafe { libc::read(fd, value as *mut u64 as *mut _, std::mem::size_of::<u64>()) }
}

/// Write one u64 to a descriptor.
fn write_u64(fd: RawFd, value: &u64) -> isize {
    // SAFETY: value is a valid readable u64 buffer for the requested byte count
    unsafe {
        libc::write(
            fd,
            value as *const u64 as *const _,
            std::mem::size_of::<u64>(),
        )
    }
}

/// Write one wake value into one eventfd.
fn wake_eventfd(fd: RawFd) -> RuntimeResult<()> {
    let value: u64 = 1;
    let result = write_u64(fd, &value);
    if result < 0 {
        let errno = host_core::get_errno();
        if errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            return Err(io_error("poller.wake", Some(fd)));
        }
    }

    Ok(())
}

/// Convert interests and flags into epoll events.
fn epoll_events_for_interest(interests: PollInterest, flags: HostPollerFlags) -> u32 {
    // build the epoll event mask
    let mut events: u32 = 0;
    if interests.contains(PollInterest::READABLE) {
        events |= libc::EPOLLIN as u32;
    }
    if interests.contains(PollInterest::WRITABLE) {
        events |= libc::EPOLLOUT as u32;
    }
    if flags.contains(HostPollerFlags::PRIORITY) {
        events |= libc::EPOLLPRI as u32;
    }
    if flags.contains(HostPollerFlags::EDGE) {
        events |= libc::EPOLLET as u32;
    }
    if flags.contains(HostPollerFlags::ONESHOT) {
        events |= libc::EPOLLONESHOT as u32;
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

/// Build event mask from epoll events.
fn event_mask_from_epoll(events: u32) -> PollerEventMask {
    // translate epoll event bits into the runtime mask
    let mut mask = PollerEventMask::NONE;

    if (events & libc::EPOLLIN as u32) != 0 {
        mask |= PollerEventMask::READABLE;
    }
    if (events & libc::EPOLLOUT as u32) != 0 {
        mask |= PollerEventMask::WRITABLE;
    }
    if (events & libc::EPOLLERR as u32) != 0 {
        mask |= PollerEventMask::ERROR;
    }
    if (events & libc::EPOLLHUP as u32) != 0 || (events & libc::EPOLLRDHUP as u32) != 0 {
        mask |= PollerEventMask::HANGUP;
    }
    if (events & libc::EPOLLPRI as u32) != 0 {
        mask |= PollerEventMask::PRIORITY;
    }

    mask
}

/// Create the wake eventfd used to interrupt polls.
fn create_wake_eventfd() -> RuntimeResult<RawFd> {
    let fd = create_event_fd();
    if fd < 0 {
        return Err(io_error("poller.eventfd", None));
    }

    Ok(fd)
}

/// Drain the wake eventfd so it stops signaling readiness.
fn drain_wake(fd: RawFd) {
    // read until the eventfd is drained
    let mut buffer = 0u64;
    loop {
        let read_bytes = read_u64(fd, &mut buffer);
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

/// Convert one absolute deadline into one epoll timeout in milliseconds.
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
        ms as c_int
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
    use super::{
        EpollPoller, HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerToken, ResourceId,
    };
    use crate::runtime::WorkerId;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    /// Ensures epoll emits a readable event when data is available.
    #[test]
    fn test_poll_readable_event() {
        let mut poller = EpollPoller::new().expect("poller should initialize");

        let mut fds = [0; 2];

        // SAFETY: fds points to two writable file descriptor slots
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert!(result == 0);

        let read_fd = fds[0];
        let write_fd = fds[1];

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

        // SAFETY: payload points to a readable byte buffer for the requested length
        let wrote = unsafe { libc::write(write_fd, payload.as_ptr() as *const _, payload.len()) };
        assert!(wrote >= 0);

        let events = poller.poll(Some(0)).expect("poll should return events");
        assert!(!events.is_empty());

        // SAFETY: closing test-owned descriptors at the end of the test
        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }
}
