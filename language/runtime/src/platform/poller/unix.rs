use std::collections::HashMap;
use std::os::unix::io::RawFd;

use libc::{c_int, c_short, poll, pollfd};

use super::{
    PlatformEvent, PlatformEventFlags, PlatformEventKind, PlatformHandle, PlatformInterest,
    PlatformPoller, PlatformPollerFlags,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::{PlatformError, ResourceId};

/// Poll-based platform poller for Unix.
#[derive(Debug)]
pub struct UnixPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, UnixPollRegistration>,
    /// Read end of the wake pipe.
    wake_read: RawFd,
    /// Write end of the wake pipe.
    wake_write: RawFd,
}

#[derive(Debug, Clone, Copy)]
struct UnixPollRegistration {
    /// Raw file descriptor to poll.
    fd: RawFd,
    /// Interest mask for readiness.
    interests: PlatformInterest,
    /// Poller configuration flags.
    flags: PlatformPollerFlags,
}

impl UnixPoller {
    /// Create a new Unix poller.
    pub fn new() -> RuntimeResult<Self> {
        let (wake_read, wake_write) = create_wake_pipe()?;
        Ok(Self {
            registrations: HashMap::new(),
            wake_read,
            wake_write,
        })
    }
}

impl Drop for UnixPoller {
    fn drop(&mut self) {
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
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        let entry = UnixPollRegistration {
            fd: handle.as_raw_fd(),
            interests,
            flags,
        };
        self.registrations.insert(resource_id, entry);
        Ok(())
    }

    fn update(
        &mut self,
        resource_id: ResourceId,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        let entry = self
            .registrations
            .get_mut(&resource_id)
            .ok_or_else(|| RuntimeError::resource_not_found(resource_id.0, None).boxed())?;
        entry.interests = interests;
        entry.flags = flags;
        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.registrations.remove(&resource_id);
        Ok(())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        let byte = [1u8];
        let result = unsafe { libc::write(self.wake_write, byte.as_ptr() as *const _, byte.len()) };
        if result < 0 {
            return Err(io_error("poller.wake", None));
        }
        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>> {
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
            entries.push(Some(*resource_id));
        }

        let timeout_ms = timeout_nanos.map(nanos_to_timeout_ms).unwrap_or(-1);
        let result = unsafe {
            poll(
                pollfds.as_mut_ptr(),
                pollfds.len() as libc::nfds_t,
                timeout_ms,
            )
        };
        if result < 0 {
            return Err(io_error("poller.poll", None));
        }

        let mut events = Vec::new();
        for (index, pollfd) in pollfds.iter().enumerate() {
            if pollfd.revents == 0 {
                continue;
            }

            if index == 0 {
                drain_wake(self.wake_read);
                continue;
            }

            let Some(resource_id) = entries[index] else {
                continue;
            };
            let flags = event_flags_from_revents(pollfd.revents);
            emit_events(resource_id, pollfd.revents, flags, &mut events);
        }

        Ok(events)
    }
}

fn poll_events_for_interest(interests: PlatformInterest, flags: PlatformPollerFlags) -> c_short {
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

fn event_flags_from_revents(revents: c_short) -> PlatformEventFlags {
    let mut flags = PlatformEventFlags::NONE;
    if (revents & libc::POLLPRI) != 0 {
        flags |= PlatformEventFlags::PRIORITY;
    }
    flags
}

fn emit_events(
    resource_id: ResourceId,
    revents: c_short,
    flags: PlatformEventFlags,
    output: &mut Vec<PlatformEvent>,
) {
    if (revents & (libc::POLLERR | libc::POLLNVAL)) != 0 {
        output.push(PlatformEvent {
            resource_id,
            kind: PlatformEventKind::Error,
            flags,
            data: 0,
        });
    }
    if (revents & libc::POLLHUP) != 0 {
        output.push(PlatformEvent {
            resource_id,
            kind: PlatformEventKind::Closed,
            flags,
            data: 0,
        });
    }
    if (revents & libc::POLLIN) != 0 {
        output.push(PlatformEvent {
            resource_id,
            kind: PlatformEventKind::Readable,
            flags,
            data: 0,
        });
    }
    if (revents & libc::POLLOUT) != 0 {
        output.push(PlatformEvent {
            resource_id,
            kind: PlatformEventKind::Writable,
            flags,
            data: 0,
        });
    }
}

fn create_wake_pipe() -> RuntimeResult<(RawFd, RawFd)> {
    let mut fds = [0; 2];
    let result = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if result < 0 {
        return Err(io_error("poller.pipe", None));
    }
    set_nonblocking(fds[0])?;
    set_nonblocking(fds[1])?;
    Ok((fds[0], fds[1]))
}

fn set_nonblocking(fd: RawFd) -> RuntimeResult<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if result < 0 {
        return Err(io_error("poller.fcntl", Some(fd)));
    }
    Ok(())
}

fn drain_wake(fd: RawFd) {
    let mut buffer = [0u8; 64];
    loop {
        let read_bytes = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, buffer.len()) };
        if read_bytes <= 0 {
            break;
        }
    }
}

fn nanos_to_timeout_ms(nanos: u64) -> c_int {
    if nanos == 0 {
        return 0;
    }
    let ms = (nanos + 999_999) / 1_000_000;
    if ms > i32::MAX as u64 {
        i32::MAX
    } else {
        ms as i32
    }
}

fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    let err = std::io::Error::last_os_error();
    let errno = err.raw_os_error();
    let message = format!("{context} failed: {err}");
    let code = errno.and_then(io_error_code_from_errno);
    let error = PlatformError::io_with(code, None, errno, Some(context.to_string()), None, message);
    let mut error = error;
    if let Some(fd) = fd {
        error.fd = Some(fd);
    }
    RuntimeError::platform(error).boxed()
}
