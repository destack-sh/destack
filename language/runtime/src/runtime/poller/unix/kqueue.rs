use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::time::Instant;

use libc::{c_int, c_short, kevent as kevent_sys, timespec};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::{
    PlatformErrorContext, PlatformErrorContextKind, io_error_code_from_errno,
};
use crate::platform::{PlatformError, ResourceId, core as core_platform};
use crate::runtime::poller::{
    HostPoller, HostPollerFlags, PlatformHandle, PlatformInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};

/// Kqueue backed poller for BSD targets.
#[derive(Debug)]
pub(crate) struct KqueuePoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Token to resource mapping for event lookup.
    tokens: HashMap<PollerToken, ResourceId>,
    /// Kqueue file descriptor.
    kqueue_fd: RawFd,
    /// Shared wake handle used by out-of-band wakeups.
    wake_handle: Arc<KqueueWakeHandle>,
    /// Event buffer reused across polls.
    events: Vec<libc::kevent>,
}

// safety: the poller only stores owned kevent buffers and raw fds
unsafe impl Send for KqueuePoller {}

/// Kqueue registration state for a resource.
#[derive(Debug, Clone, Copy)]
struct PollRegistration {
    /// Raw file descriptor to poll.
    fd: RawFd,
    /// Opaque token associated with the registration.
    token: PollerToken,
    /// Interest mask for readiness.
    interests: PlatformInterest,
    /// Poller configuration flags.
    flags: HostPollerFlags,
}

/// Shared wake handle for one kqueue poller.
#[derive(Debug)]
struct KqueueWakeHandle {
    /// Kqueue descriptor used for user wake events.
    kqueue_fd: RawFd,
}

impl PollerWakeHandle for KqueueWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        wake_kqueue(self.kqueue_fd)
    }
}

impl KqueuePoller {
    /// Create a new kqueue poller instance.
    pub(crate) fn new() -> RuntimeResult<Self> {
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
            tokens: HashMap::new(),
            kqueue_fd,
            wake_handle: Arc::new(KqueueWakeHandle { kqueue_fd }),
            events: Vec::new(),
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

impl HostPoller for KqueuePoller {
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: PlatformHandle,
        token: PollerToken,
        interests: PlatformInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        // reject reserved tokens
        if token.is_reserved() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "token",
                "token reserved for poller internals",
            ))
            .boxed());
        }

        // update existing registrations in place
        if self.registrations.contains_key(&resource_id) {
            return self.update(resource_id, token, interests, flags);
        }

        if self.tokens.contains_key(&token) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "token",
                "token already registered",
            ))
            .boxed());
        }

        // add the kqueue registration
        let fd = handle.as_raw_fd();
        let changes = build_filter_changes(fd, interests, flags, token, false)?;
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
        self.tokens.insert(token, resource_id);

        Ok(())
    }

    fn update(
        &mut self,
        resource_id: ResourceId,
        token: PollerToken,
        interests: PlatformInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        // reject reserved tokens
        if token.is_reserved() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "token",
                "token reserved for poller internals",
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

        // apply filter changes in kqueue
        let changes = build_update_changes(entry, interests, flags, token)?;
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
        if entry.token != token {
            self.tokens.remove(&entry.token);
            if self.tokens.contains_key(&token) {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "token",
                    "token already registered",
                ))
                .boxed());
            }
            self.tokens.insert(token, resource_id);
            entry.token = token;
        }

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        // drop the registration entry
        let Some(entry) = self.registrations.remove(&resource_id) else {
            return Ok(());
        };
        self.tokens.remove(&entry.token);

        // remove all registered filters
        let changes =
            build_filter_changes(entry.fd, entry.interests, entry.flags, entry.token, true)?;
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
                .resize_with(max_events, || unsafe { std::mem::zeroed() });
        }

        // resolve the poll deadline once so EINTR does not reset the timeout budget
        let deadline = timeout_nanos.and_then(core_platform::timeout_deadline);

        // call into kqueue
        let result = loop {
            let timeout = timespec_from_deadline(deadline);
            let timeout_ptr = timeout
                .as_ref()
                .map(|time| time as *const timespec)
                .unwrap_or(std::ptr::null());
            let result = unsafe {
                kevent_sys(
                    self.kqueue_fd,
                    std::ptr::null(),
                    0,
                    self.events.as_mut_ptr(),
                    max_events as c_int,
                    timeout_ptr,
                )
            };
            if result >= 0 {
                break result;
            }

            let errno = core_platform::get_errno();
            if errno != libc::EINTR {
                return Err(io_error("poller.kevent", None));
            }
        };

        // collect emitted events
        let mut output = Vec::with_capacity(result as usize);
        let mut oneshot = Vec::new();
        for event in self.events.iter().take(result as usize) {
            if event.filter == libc::EVFILT_USER && event.ident == WAKE_IDENT {
                continue;
            }

            let Some(token) = token_from_udata(event.udata) else {
                continue;
            };
            let Some(resource_id) = self.tokens.get(&token).copied() else {
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
            output.push(PollerEvent {
                resource_id,
                source: PollerEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PollerEventPayload::Io { data },
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
                }
            }
        }

        Ok(output)
    }
}

/// Trigger one user wake event on one kqueue descriptor.
fn wake_kqueue(kqueue_fd: RawFd) -> RuntimeResult<()> {
    let event = make_user_event(
        WAKE_IDENT,
        libc::EV_ADD | libc::EV_CLEAR,
        libc::NOTE_TRIGGER,
    );
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
        return Err(io_error("poller.wake", Some(kqueue_fd)));
    }

    Ok(())
}

/// Convert registration flags into kqueue flags.
fn kevent_flags(flags: HostPollerFlags) -> u16 {
    // map registration flags to kqueue flags
    let mut out = libc::EV_ADD | libc::EV_ENABLE;
    if flags.contains(HostPollerFlags::EDGE) {
        out |= libc::EV_CLEAR;
    }
    if flags.contains(HostPollerFlags::ONESHOT) {
        out |= libc::EV_ONESHOT;
    }

    out
}

/// Build filter changes for a new registration.
fn build_filter_changes(
    fd: RawFd,
    interests: PlatformInterest,
    flags: HostPollerFlags,
    token: PollerToken,
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
            token_udata(token),
        ));
    }
    if interests.contains(PlatformInterest::WRITABLE) {
        changes.push(make_kevent(
            fd,
            libc::EVFILT_WRITE,
            event_flags,
            token_udata(token),
        ));
    }

    Ok(changes)
}

/// Build filter changes for updates.
fn build_update_changes(
    entry: &PollRegistration,
    interests: PlatformInterest,
    flags: HostPollerFlags,
    token: PollerToken,
) -> RuntimeResult<Vec<libc::kevent>> {
    // detect token updates that require reprogramming retained filters
    let token_changed = entry.token != token;

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
            token_udata(token),
        ));
    }
    if interests.contains(PlatformInterest::WRITABLE)
        && !entry.interests.contains(PlatformInterest::WRITABLE)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_WRITE,
            event_flags,
            token_udata(token),
        ));
    }

    // collect filter changes for modified flags
    if entry.interests.contains(PlatformInterest::READABLE)
        && interests.contains(PlatformInterest::READABLE)
        && (entry.flags != flags || token_changed)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_READ,
            event_flags,
            token_udata(token),
        ));
    }
    if entry.interests.contains(PlatformInterest::WRITABLE)
        && interests.contains(PlatformInterest::WRITABLE)
        && (entry.flags != flags || token_changed)
    {
        changes.push(make_kevent(
            entry.fd,
            libc::EVFILT_WRITE,
            event_flags,
            token_udata(token),
        ));
    }

    Ok(changes)
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

/// Build event mask from kevent data.
fn event_mask_from_kevent(event: &libc::kevent) -> PollerEventMask {
    // translate kevent data into the runtime mask
    let mut mask = PollerEventMask::NONE;

    if (event.flags & libc::EV_ERROR) != 0 {
        mask |= PollerEventMask::ERROR;
    }
    if (event.flags & libc::EV_EOF) != 0 {
        mask |= PollerEventMask::HANGUP;
    }

    if event.filter == libc::EVFILT_READ {
        mask |= PollerEventMask::READABLE;
    }
    if event.filter == libc::EVFILT_WRITE {
        mask |= PollerEventMask::WRITABLE;
    }

    mask
}

/// Identifier for the user wake event.
const WAKE_IDENT: libc::uintptr_t = 1;

/// Convert one absolute deadline into one relative timespec.
fn timespec_from_deadline(deadline: Option<Instant>) -> Option<timespec> {
    let deadline = deadline?;

    let now = Instant::now();
    let remaining = deadline.saturating_duration_since(now);

    // split into seconds and remaining nanoseconds
    let secs = remaining.as_secs();
    let nsec = remaining.subsec_nanos();

    Some(timespec {
        tv_sec: secs as libc::time_t,
        tv_nsec: nsec as libc::c_long,
    })
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
fn token_udata(token: PollerToken) -> *mut libc::c_void {
    token.0 as *mut libc::c_void
}

/// Convert a udata pointer into a resource id.
fn token_from_udata(udata: *mut libc::c_void) -> Option<PollerToken> {
    let value = udata as usize;
    if value == 0 {
        return None;
    }
    Some(PollerToken(value as u64))
}

/// Convert the last OS error into a runtime error.
fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    // capture the last OS error
    let errno = core_platform::get_errno();
    let message = format!("{context} failed: errno {errno}");

    // map the error into platform diagnostics
    let code = io_error_code_from_errno(errno);
    let error = PlatformError::io_with(
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
            .unwrap_or_else(|| PlatformErrorContext::with_kind(PlatformErrorContextKind::Io));
        context.fd = Some(fd);
        error.context = Some(context);
    }

    RuntimeError::from(error).boxed()
}

#[cfg(test)]
mod tests {
    use super::{
        HostPoller, HostPollerFlags, KqueuePoller, PlatformHandle, PlatformInterest, PollerToken,
        ResourceId,
    };
    use crate::runtime::WorkerId;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    /// Ensures kqueue emits a readable event when data is available.
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
                ResourceId::new(TEST_WORKER_ID, 1),
                handle,
                PollerToken(1),
                PlatformInterest::READABLE,
                HostPollerFlags::NONE,
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
