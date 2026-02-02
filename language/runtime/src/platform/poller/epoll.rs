use std::collections::HashMap;
use std::os::unix::io::RawFd;

use libc::{c_int, epoll_create1, epoll_ctl, epoll_event, epoll_wait};

use super::{
    PlatformEvent, PlatformEventFlags, PlatformEventMask, PlatformEventPayload,
    PlatformEventSource, PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::{PlatformError, ResourceId};

/// Epoll backed poller for Linux targets.
#[derive(Debug)]
pub struct EpollPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Epoll file descriptor.
    epoll_fd: RawFd,
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
}

/// Epoll registration state for a resource.
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

impl EpollPoller {
    /// Create a new epoll poller instance.
    pub fn new() -> RuntimeResult<Self> {
        // open the epoll descriptor
        let epoll_fd = unsafe { epoll_create1(libc::EPOLL_CLOEXEC) };
        if epoll_fd < 0 {
            return Err(io_error("poller.epoll_create1", None));
        }

        // allocate the wake eventfd used to interrupt polls
        let wake_fd = create_wake_eventfd()?;

        // register the wake pipe for readability
        let mut event = epoll_event {
            events: libc::EPOLLIN as u32,
            u64: WAKE_RESOURCE_ID,
        };
        let result = unsafe { epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, wake_fd, &mut event) };
        if result < 0 {
            unsafe {
                libc::close(epoll_fd);
            }
            return Err(io_error("poller.epoll_ctl", Some(wake_fd)));
        }

        Ok(Self {
            registrations: HashMap::new(),
            epoll_fd,
            wake_fd,
        })
    }
}

impl Drop for EpollPoller {
    fn drop(&mut self) {
        // close wake and epoll descriptors
        unsafe {
            libc::close(self.wake_fd);
            libc::close(self.epoll_fd);
        }
    }
}

impl PlatformPoller for EpollPoller {
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: PlatformHandle,
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        // update existing registrations in place
        if self.registrations.contains_key(&resource_id) {
            return self.update(resource_id, token, interests, flags);
        }

        // add the epoll registration
        let fd = handle.as_raw_fd();
        let mut event = epoll_event {
            events: epoll_events_for_interest(interests, flags),
            u64: resource_id.0,
        };
        let result = unsafe { epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event) };
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

        Ok(())
    }

    fn update(
        &mut self,
        resource_id: ResourceId,
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        // resolve the existing registration
        let entry = self
            .registrations
            .get_mut(&resource_id)
            .ok_or_else(|| RuntimeError::resource_not_found(resource_id.0, None).boxed())?;

        // update the registration in epoll
        let mut event = epoll_event {
            events: epoll_events_for_interest(interests, flags),
            u64: resource_id.0,
        };
        let result = unsafe { epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_MOD, entry.fd, &mut event) };
        if result < 0 {
            return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
        }

        // update cached fields
        entry.token = token;
        entry.interests = interests;
        entry.flags = flags;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        // drop the registration entry
        let Some(entry) = self.registrations.remove(&resource_id) else {
            return Ok(());
        };

        // remove the epoll registration
        let result = unsafe {
            epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_DEL,
                entry.fd,
                std::ptr::null_mut(),
            )
        };
        if result < 0 {
            return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
        }

        Ok(())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        // write a value to the wake eventfd
        let value: u64 = 1;
        let result = unsafe {
            libc::write(
                self.wake_fd,
                &value as *const u64 as *const _,
                std::mem::size_of::<u64>(),
            )
        };
        if result < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::WouldBlock {
                return Err(io_error("poller.wake", None));
            }
        }

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>> {
        // allocate the event buffer
        let max_events = self.registrations.len() + 1;
        let mut events = vec![epoll_event { events: 0, u64: 0 }; max_events];

        // resolve the timeout in milliseconds
        let timeout_ms = timeout_nanos.map(nanos_to_timeout_ms).unwrap_or(-1);

        // call into epoll
        let result = loop {
            let result = unsafe {
                epoll_wait(
                    self.epoll_fd,
                    events.as_mut_ptr(),
                    max_events as c_int,
                    timeout_ms,
                )
            };
            if result >= 0 {
                break result;
            }

            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return Err(io_error("poller.epoll_wait", None));
            }
        };

        // collect emitted events
        let mut output = Vec::new();
        let mut oneshot = Vec::new();
        for event in events.iter().take(result as usize) {
            if event.u64 == WAKE_RESOURCE_ID {
                drain_wake(self.wake_fd);
                continue;
            }

            let resource_id = ResourceId(event.u64);
            let Some(registration) = self.registrations.get(&resource_id) else {
                continue;
            };

            let mask = event_mask_from_epoll(event.events);
            if mask.is_empty() {
                continue;
            }

            let flags = event_flags_from_registration(registration.flags);
            output.push(PlatformEvent {
                resource_id,
                source: PlatformEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PlatformEventPayload::Io {
                    data: event.events as u64,
                },
            });

            if registration.flags.contains(PlatformPollerFlags::ONESHOT) {
                oneshot.push(resource_id);
            }
        }

        // drop any oneshot registrations
        if !oneshot.is_empty() {
            for resource_id in oneshot {
                if let Some(entry) = self.registrations.remove(&resource_id) {
                    let result = unsafe {
                        epoll_ctl(
                            self.epoll_fd,
                            libc::EPOLL_CTL_DEL,
                            entry.fd,
                            std::ptr::null_mut(),
                        )
                    };
                    if result < 0 {
                        return Err(io_error("poller.epoll_ctl", Some(entry.fd)));
                    }
                }
            }
        }

        Ok(output)
    }
}

const WAKE_RESOURCE_ID: u64 = 0;

/// Convert interests and flags into epoll events.
fn epoll_events_for_interest(interests: PlatformInterest, flags: PlatformPollerFlags) -> u32 {
    // build the epoll event mask
    let mut events: u32 = 0;
    if interests.contains(PlatformInterest::READABLE) {
        events |= libc::EPOLLIN as u32;
    }
    if interests.contains(PlatformInterest::WRITABLE) {
        events |= libc::EPOLLOUT as u32;
    }
    if flags.contains(PlatformPollerFlags::PRIORITY) {
        events |= libc::EPOLLPRI as u32;
    }
    if flags.contains(PlatformPollerFlags::EDGE) {
        events |= libc::EPOLLET as u32;
    }
    if flags.contains(PlatformPollerFlags::ONESHOT) {
        events |= libc::EPOLLONESHOT as u32;
    }

    events
}

/// Build event flags from epoll events.
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

/// Build event mask from epoll events.
fn event_mask_from_epoll(events: u32) -> PlatformEventMask {
    // translate epoll event bits into the runtime mask
    let mut mask = PlatformEventMask::NONE;

    if (events & libc::EPOLLIN as u32) != 0 {
        mask |= PlatformEventMask::READABLE;
    }
    if (events & libc::EPOLLOUT as u32) != 0 {
        mask |= PlatformEventMask::WRITABLE;
    }
    if (events & libc::EPOLLERR as u32) != 0 {
        mask |= PlatformEventMask::ERROR;
    }
    if (events & libc::EPOLLHUP as u32) != 0 || (events & libc::EPOLLRDHUP as u32) != 0 {
        mask |= PlatformEventMask::HANGUP;
    }
    if (events & libc::EPOLLPRI as u32) != 0 {
        mask |= PlatformEventMask::PRIORITY;
    }

    mask
}

/// Create the wake eventfd used to interrupt polls.
fn create_wake_eventfd() -> RuntimeResult<RawFd> {
    // allocate an eventfd for wakeup signaling
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
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
        let read_bytes = unsafe {
            libc::read(
                fd,
                &mut buffer as *mut u64 as *mut _,
                std::mem::size_of::<u64>(),
            )
        };
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

/// Convert a nanosecond timeout to milliseconds for epoll.
fn nanos_to_timeout_ms(nanos: u64) -> c_int {
    // map zero directly to immediate polls
    if nanos == 0 {
        return 0;
    }

    // round up to the nearest millisecond
    let ms = (nanos + 999_999) / 1_000_000;
    if ms > i32::MAX as u64 {
        i32::MAX
    } else {
        ms as c_int
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

    RuntimeError::platform(error).boxed()
}

#[cfg(test)]
mod tests {
    use super::EpollPoller;
    use crate::platform::{
        PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags, ResourceId,
    };

    #[test]
    fn test_poll_readable_event() {
        let mut poller = EpollPoller::new().expect("poller should initialize");

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
