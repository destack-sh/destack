use std::collections::HashMap;
use std::os::unix::io::RawFd;

use libc::{c_int, c_short, kevent as kevent_sys, timespec};

use super::{
    PlatformEvent, PlatformEventFlags, PlatformEventMask, PlatformEventPayload,
    PlatformEventSource, PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::{PlatformError, ResourceId};

/// Kqueue backed poller for BSD targets.
#[derive(Debug)]
pub struct KqueuePoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Kqueue file descriptor.
    kqueue_fd: RawFd,
}

/// Kqueue registration state for a resource.
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

impl KqueuePoller {
    /// Create a new kqueue poller instance.
    pub fn new() -> RuntimeResult<Self> {
        // open the kqueue descriptor
        let kqueue_fd = unsafe { libc::kqueue() };
        if kqueue_fd < 0 {
            return Err(io_error("poller.kqueue", None));
        }

        // register the wake user event
        let event = make_user_event(WAKE_IDENT, libc::EV_ADD | libc::EV_CLEAR, libc::NOTE_FFNOP);
        let result = unsafe {
            kevent_sys(
                kqueue_fd,
                &event,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if result < 0 {
            unsafe {
                libc::close(kqueue_fd);
            }
            return Err(io_error("poller.kevent", None));
        }

        Ok(Self {
            registrations: HashMap::new(),
            kqueue_fd,
        })
    }
}

impl Drop for KqueuePoller {
    fn drop(&mut self) {
        // close kqueue descriptors
        unsafe {
            libc::close(self.kqueue_fd);
        }
    }
}

impl PlatformPoller for KqueuePoller {
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

        // add the kqueue registration
        let fd = handle.as_raw_fd();
        let changes = build_filter_changes(resource_id, fd, interests, flags, false)?;
        let result = unsafe {
            kevent_sys(
                self.kqueue_fd,
                changes.as_ptr(),
                changes.len() as c_int,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if result < 0 {
            return Err(io_error("poller.kevent", Some(fd)));
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

        // apply filter changes in kqueue
        let changes = build_update_changes(resource_id, entry, interests, flags)?;
        if !changes.is_empty() {
            let result = unsafe {
                kevent_sys(
                    self.kqueue_fd,
                    changes.as_ptr(),
                    changes.len() as c_int,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            if result < 0 {
                return Err(io_error("poller.kevent", Some(entry.fd)));
            }
        }

        // update cached fields
        entry.interests = interests;
        entry.flags = flags;
        entry.token = token;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        // drop the registration entry
        let Some(entry) = self.registrations.remove(&resource_id) else {
            return Ok(());
        };

        // remove all registered filters
        let changes =
            build_filter_changes(resource_id, entry.fd, entry.interests, entry.flags, true)?;
        let result = unsafe {
            kevent_sys(
                self.kqueue_fd,
                changes.as_ptr(),
                changes.len() as c_int,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if result < 0 {
            return Err(io_error("poller.kevent", Some(entry.fd)));
        }

        Ok(())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        // trigger the user wake event
        let event = make_user_event(
            WAKE_IDENT,
            libc::EV_ADD | libc::EV_CLEAR,
            libc::NOTE_TRIGGER,
        );
        let result = unsafe {
            kevent_sys(
                self.kqueue_fd,
                &event,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        if result < 0 {
            return Err(io_error("poller.wake", None));
        }

        Ok(())
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>> {
        // allocate the event buffer
        let max_events = self.registrations.len() + 1;
        let mut events = vec![unsafe { std::mem::zeroed() }; max_events];

        // prepare the timeout struct
        let timeout = timeout_nanos.map(nanos_to_timespec);
        let timeout_ptr = timeout
            .as_ref()
            .map(|time| time as *const timespec)
            .unwrap_or(std::ptr::null());

        // call into kqueue
        let result = loop {
            let result = unsafe {
                kevent_sys(
                    self.kqueue_fd,
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    max_events as c_int,
                    timeout_ptr,
                )
            };
            if result >= 0 {
                break result;
            }

            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return Err(io_error("poller.kevent", None));
            }
        };

        // collect emitted events
        let mut output = Vec::new();
        let mut oneshot = Vec::new();
        for event in events.iter().take(result as usize) {
            if event.filter == libc::EVFILT_USER && event.ident == WAKE_IDENT {
                continue;
            }

            let Some(resource_id) = resource_id_from_udata(event.udata) else {
                continue;
            };
            let Some(registration) = self.registrations.get(&resource_id) else {
                continue;
            };

            let mask = event_mask_from_kevent(event);
            if mask.is_empty() {
                continue;
            }

            let flags = event_flags_from_registration(registration.flags);
            let data = if event.data < 0 { 0 } else { event.data as u64 };
            output.push(PlatformEvent {
                resource_id,
                source: PlatformEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PlatformEventPayload::Io { data },
            });

            if registration.flags.contains(PlatformPollerFlags::ONESHOT) {
                oneshot.push(resource_id);
            }
        }

        // drop any oneshot registrations
        if !oneshot.is_empty() {
            for resource_id in oneshot {
                if let Some(entry) = self.registrations.remove(&resource_id) {
                    let changes = build_filter_changes(
                        resource_id,
                        entry.fd,
                        entry.interests,
                        entry.flags,
                        true,
                    )?;
                    let result = unsafe {
                        kevent_sys(
                            self.kqueue_fd,
                            changes.as_ptr(),
                            changes.len() as c_int,
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null(),
                        )
                    };
                    if result < 0 {
                        return Err(io_error("poller.kevent", Some(entry.fd)));
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Convert registration flags into kqueue flags.
fn kevent_flags(flags: PlatformPollerFlags) -> u16 {
    // map registration flags to kqueue flags
    let mut out = libc::EV_ADD | libc::EV_ENABLE;
    if flags.contains(PlatformPollerFlags::EDGE) {
        out |= libc::EV_CLEAR;
    }
    if flags.contains(PlatformPollerFlags::ONESHOT) {
        out |= libc::EV_ONESHOT;
    }

    out
}

/// Build filter changes for a new registration.
fn build_filter_changes(
    resource_id: ResourceId,
    fd: RawFd,
    interests: PlatformInterest,
    flags: PlatformPollerFlags,
    deleting: bool,
) -> RuntimeResult<Vec<libc::kevent>> {
    // build the kevent change list
    let mut changes = Vec::new();
    let mut event_flags = kevent_flags(flags);
    if deleting {
        event_flags = libc::EV_DELETE;
    }

    if interests.contains(PlatformInterest::READABLE) {
        changes.push(make_kevent(
            fd,
            libc::EVFILT_READ,
            event_flags,
            resource_udata(resource_id),
        ));
    }
    if interests.contains(PlatformInterest::WRITABLE) {
        changes.push(make_kevent(
            fd,
            libc::EVFILT_WRITE,
            event_flags,
            resource_udata(resource_id),
        ));
    }

    Ok(changes)
}

/// Build filter changes for updates.
fn build_update_changes(
    resource_id: ResourceId,
    entry: &PollRegistration,
    interests: PlatformInterest,
    flags: PlatformPollerFlags,
) -> RuntimeResult<Vec<libc::kevent>> {
    // collect filter changes for removed interests
    let mut changes = Vec::new();
    if entry.interests.contains(PlatformInterest::READABLE)
        && !interests.contains(PlatformInterest::READABLE)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_READ,
            libc::EV_DELETE,
            std::ptr::null_mut(),
        ));
    }
    if entry.interests.contains(PlatformInterest::WRITABLE)
        && !interests.contains(PlatformInterest::WRITABLE)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_WRITE,
            libc::EV_DELETE,
            std::ptr::null_mut(),
        ));
    }

    // collect filter changes for added interests
    let event_flags = kevent_flags(flags);
    if interests.contains(PlatformInterest::READABLE)
        && !entry.interests.contains(PlatformInterest::READABLE)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_READ,
            event_flags,
            resource_udata(resource_id),
        ));
    }
    if interests.contains(PlatformInterest::WRITABLE)
        && !entry.interests.contains(PlatformInterest::WRITABLE)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_WRITE,
            event_flags,
            resource_udata(resource_id),
        ));
    }

    // collect filter changes for modified flags
    if entry.interests.contains(PlatformInterest::READABLE)
        && interests.contains(PlatformInterest::READABLE)
        && entry.flags != flags
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_READ,
            event_flags,
            resource_udata(resource_id),
        ));
    }
    if entry.interests.contains(PlatformInterest::WRITABLE)
        && interests.contains(PlatformInterest::WRITABLE)
        && entry.flags != flags
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_WRITE,
            event_flags,
            resource_udata(resource_id),
        ));
    }

    Ok(changes)
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

/// Build event mask from kevent data.
fn event_mask_from_kevent(event: &libc::kevent) -> PlatformEventMask {
    // translate kevent data into the runtime mask
    let mut mask = PlatformEventMask::NONE;

    if (event.flags & libc::EV_ERROR) != 0 {
        mask |= PlatformEventMask::ERROR;
    }
    if (event.flags & libc::EV_EOF) != 0 {
        mask |= PlatformEventMask::HANGUP;
    }

    if event.filter == libc::EVFILT_READ {
        mask |= PlatformEventMask::READABLE;
    }
    if event.filter == libc::EVFILT_WRITE {
        mask |= PlatformEventMask::WRITABLE;
    }

    mask
}

const WAKE_IDENT: libc::uintptr_t = 1;

/// Convert a nanosecond timeout to a timespec.
fn nanos_to_timespec(nanos: u64) -> timespec {
    // split into seconds and remaining nanoseconds
    let secs = nanos / 1_000_000_000;
    let nsec = nanos % 1_000_000_000;
    timespec {
        tv_sec: secs as libc::time_t,
        tv_nsec: nsec as libc::c_long,
    }
}

/// Create a kevent entry for a filter.
fn make_kevent(fd: RawFd, filter: c_short, flags: u16, udata: *mut libc::c_void) -> libc::kevent {
    libc::kevent {
        ident: fd as libc::uintptr_t,
        filter,
        flags,
        fflags: 0,
        data: 0,
        udata,
    }
}

/// Create a kevent entry for the user wake event.
fn make_user_event(ident: libc::uintptr_t, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident,
        filter: libc::EVFILT_USER,
        flags,
        fflags,
        data: 0,
        udata: std::ptr::null_mut(),
    }
}

/// Convert a resource id into a udata pointer.
fn resource_udata(resource_id: ResourceId) -> *mut libc::c_void {
    resource_id.0 as usize as *mut libc::c_void
}

/// Convert a udata pointer into a resource id.
fn resource_id_from_udata(udata: *mut libc::c_void) -> Option<ResourceId> {
    let value = udata as usize;
    if value == 0 {
        return None;
    }
    Some(ResourceId(value as u64))
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
    use super::KqueuePoller;
    use crate::platform::{
        PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags, ResourceId,
    };

    #[test]
    fn test_poll_readable_event() {
        let mut poller = KqueuePoller::new().expect("poller should initialize");

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
