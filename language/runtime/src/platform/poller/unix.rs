use std::collections::HashMap;
use std::os::unix::io::RawFd;

use libc::{c_int, c_short, poll, pollfd};

use super::{
    PlatformEvent, PlatformEventFlags, PlatformEventMask, PlatformEventPayload,
    PlatformEventSource, PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::{PlatformError, ResourceId};

/// Poll based platform poller for Unix systems.
#[derive(Debug)]
pub struct UnixPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Read end of the wake pipe.
    wake_read: RawFd,
    /// Write end of the wake pipe.
    wake_write: RawFd,
}

/// Poll registration state for a resource.
#[derive(Debug, Clone, Copy)]
struct PollRegistration {
    /// Raw file descriptor to poll.
    fd: RawFd,
    /// Opaque token associated with the registration.
    token: u64,
    /// Interest mask for readiness.
    interests: PlatformInterest,
    /// Poller configuration flags.
    flags: PlatformPollerFlags,
}

impl UnixPoller {
    /// Create a new Unix poller instance.
    pub fn new() -> RuntimeResult<Self> {
        // wake pipe used to interrupt blocking polls
        let (wake_read, wake_write) = create_wake_pipe()?;

        // assemble the poller state
        Ok(Self {
            registrations: HashMap::new(),
            wake_read,
            wake_write,
        })
    }
}

impl Drop for UnixPoller {
    fn drop(&mut self) {
        // close wake pipe descriptors
        unsafe {
            libc::close(self.wake_read);
            libc::close(self.wake_write);
        }
    }
}

impl PlatformPoller for UnixPoller {
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: PlatformHandle,
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        // reject unsupported edge-triggered registrations
        if flags.contains(PlatformPollerFlags::EDGE) {
            return Err(RuntimeError::from(PlatformError::not_supported(
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
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        // reject unsupported edge-triggered registrations
        if flags.contains(PlatformPollerFlags::EDGE) {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "poller.edge is not supported by poll",
            ))
            .boxed());
        }

        // resolve the existing registration
        let entry = self.registrations.get_mut(&resource_id).ok_or_else(|| {
            RuntimeError::ResourceNotFound {
                resource_id: resource_id.0,
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

    fn wake(&mut self) -> RuntimeResult<()> {
        // write a byte to the wake pipe
        let byte = [1u8];
        let result = unsafe { libc::write(self.wake_write, byte.as_ptr() as *const _, byte.len()) };
        if result < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::WouldBlock {
                return Err(io_error("poller.wake", None));
            }
        }

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>> {
        // build pollfd array including wake pipe
        let mut pollfds = Vec::with_capacity(self.registrations.len() + 1);
        let mut entries = Vec::with_capacity(self.registrations.len() + 1);

        pollfds.push(pollfd {
            fd: self.wake_read,
            events: libc::POLLIN,
            revents: 0,
        });
        entries.push(None);

        for (resource_id, entry) in &self.registrations {
            let events = poll_events_for_interest(entry.interests, entry.flags);
            pollfds.push(pollfd {
                fd: entry.fd,
                events,
                revents: 0,
            });
            entries.push(Some((*resource_id, *entry)));
        }

        // resolve the timeout in milliseconds
        let timeout_ms = timeout_nanos.map(nanos_to_timeout_ms).unwrap_or(-1);

        // call poll and surface errors
        let _ = loop {
            let result = unsafe {
                poll(
                    pollfds.as_mut_ptr(),
                    pollfds.len() as libc::nfds_t,
                    timeout_ms,
                )
            };
            if result >= 0 {
                break result;
            }

            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return Err(io_error("poller.poll", None));
            }
        };

        // collect emitted events
        let mut events = Vec::new();
        let mut oneshot = Vec::new();
        for (index, pollfd) in pollfds.iter().enumerate() {
            if pollfd.revents == 0 {
                continue;
            }

            if index == 0 {
                drain_wake(self.wake_read);
                continue;
            }

            let Some((resource_id, registration)) = entries[index] else {
                continue;
            };

            let mask = event_mask_from_revents(pollfd.revents);
            if mask.is_empty() {
                continue;
            }

            let flags = event_flags_from_registration(registration.flags);
            events.push(PlatformEvent {
                resource_id,
                source: PlatformEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PlatformEventPayload::Io {
                    data: pollfd.revents as u64,
                },
            });

            if registration.flags.contains(PlatformPollerFlags::ONESHOT) {
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

/// Convert interests and flags into poll events.
fn poll_events_for_interest(interests: PlatformInterest, flags: PlatformPollerFlags) -> c_short {
    // build the poll event mask
    let mut events: c_short = 0;
    if interests.contains(PlatformInterest::READABLE) {
        events |= libc::POLLIN;
    }
    if interests.contains(PlatformInterest::WRITABLE) {
        events |= libc::POLLOUT;
    }
    if flags.contains(PlatformPollerFlags::PRIORITY) {
        events |= libc::POLLPRI;
    }

    events
}

/// Build event flags from registration flags.
fn event_flags_from_registration(flags: PlatformPollerFlags) -> PlatformEventFlags {
    // expose edge and oneshot flags to consumers
    let mut out = PlatformEventFlags::NONE;
    if flags.contains(PlatformPollerFlags::EDGE) {
        out |= PlatformEventFlags::EDGE;
    }
    if flags.contains(PlatformPollerFlags::ONESHOT) {
        out |= PlatformEventFlags::ONESHOT;
    }

    out
}

/// Build event mask from poll revents.
fn event_mask_from_revents(revents: c_short) -> PlatformEventMask {
    // translate poll events into the runtime mask
    let mut mask = PlatformEventMask::NONE;

    if (revents & libc::POLLIN) != 0 {
        mask |= PlatformEventMask::READABLE;
    }
    if (revents & libc::POLLOUT) != 0 {
        mask |= PlatformEventMask::WRITABLE;
    }
    if (revents & (libc::POLLERR | libc::POLLNVAL)) != 0 {
        mask |= PlatformEventMask::ERROR;
    }
    if (revents & libc::POLLHUP) != 0 {
        mask |= PlatformEventMask::HANGUP;
    }
    if (revents & libc::POLLPRI) != 0 {
        mask |= PlatformEventMask::PRIORITY;
    }

    mask
}

/// Create the wake pipe used to interrupt polls.
fn create_wake_pipe() -> RuntimeResult<(RawFd, RawFd)> {
    // allocate a pipe for wakeup signaling
    let mut fds = [0; 2];
    let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
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
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    // update the descriptor flags
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if result < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    Ok(())
}

/// Mark a file descriptor as close-on-exec.
fn set_close_on_exec(fd: RawFd) -> RuntimeResult<()> {
    // read the current descriptor flags
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }

    // update the descriptor flags
    let result = unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
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
        let read_bytes = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, buffer.len()) };
        if read_bytes > 0 {
            continue;
        }
        if read_bytes == 0 {
            break;
        }

        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::WouldBlock {
            break;
        }
        break;
    }
}

/// Convert a nanosecond timeout to milliseconds for poll.
fn nanos_to_timeout_ms(nanos: u64) -> c_int {
    // map zero directly to immediate polls
    if nanos == 0 {
        return 0;
    }

    // round up to the nearest millisecond
    let ms = nanos.div_ceil(1_000_000);
    if ms > i32::MAX as u64 {
        i32::MAX
    } else {
        ms as i32
    }
}

/// Convert the last OS error into a runtime error.
fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    // capture the last OS error
    let err = std::io::Error::last_os_error();
    let errno = err.raw_os_error();
    let message = format!("{context} failed: {err}");

    // map the error into platform diagnostics
    let code = errno.and_then(io_error_code_from_errno);
    let error = PlatformError::io_with(code, None, errno, Some(context.to_string()), None, message);
    let mut error = error;
    if let Some(fd) = fd {
        error.fd = Some(fd);
    }

    RuntimeError::from(error).boxed()
}

#[cfg(test)]
mod tests {
    use super::UnixPoller;
    use crate::platform::{
        PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags, ResourceId,
    };

    /// Ensures poll emits a readable event when data is available.
    #[test]
    fn test_poll_readable_event() {
        let mut poller = UnixPoller::new().expect("poller should initialize");

        let mut fds = [0; 2];
        let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert!(result == 0);

        let read_fd = fds[0];
        let write_fd = fds[1];

        let handle = PlatformHandle::from_raw_fd(read_fd);
        poller
            .register(
                ResourceId(1),
                handle,
                1,
                PlatformInterest::READABLE,
                PlatformPollerFlags::NONE,
            )
            .expect("register should succeed");

        let payload = [1u8];
        let wrote = unsafe { libc::write(write_fd, payload.as_ptr() as *const _, payload.len()) };
        assert!(wrote >= 0);

        let events = poller.poll(Some(0)).expect("poll should return events");
        assert!(!events.is_empty());

        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
    }
}
