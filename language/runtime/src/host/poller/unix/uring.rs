use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::Arc;

use io_uring::{IoUring, opcode, types};

use crate::diagnostic::{
    HostErrorContext, HostErrorContextKind, RuntimeError, RuntimeResult, io_error_code_from_errno,
};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::{HostError, ResourceId, core as host_core};

/// Default io_uring queue depth.
const DEFAULT_QUEUE_DEPTH: u32 = 256;
/// Reserved token for wake events.
const WAKE_TOKEN: PollerToken = PollerToken::WAKE;
/// Reserved token for timeout events.
const TIMEOUT_TOKEN: PollerToken = PollerToken::TIMEOUT;

/// io_uring backed poller for Linux targets.
pub(crate) struct IoUringPoller {
    /// io_uring instance.
    ring: IoUring,
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Token to resource mapping for event lookup.
    tokens: HashMap<PollerToken, ResourceId>,
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
    /// Shared wake handle used by out-of-band wakeups.
    wake_handle: Arc<IoUringWakeHandle>,
    /// Whether the wake entry is currently in flight.
    wake_pending: bool,
    /// Timeout timespec storage for in-flight requests.
    timeout_spec: types::Timespec,
    /// Whether a timeout entry is currently in flight.
    timeout_pending: bool,
}

impl std::fmt::Debug for IoUringPoller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IoUringPoller")
            .field("registrations", &self.registrations.len())
            .field("tokens", &self.tokens.len())
            .field("wake_fd", &self.wake_fd)
            .field("wake_pending", &self.wake_pending)
            .field("timeout_pending", &self.timeout_pending)
            .finish()
    }
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
    /// Whether a poll request is currently in flight.
    pending: bool,
}

/// Shared wake handle for one io_uring poller.
#[derive(Debug)]
struct IoUringWakeHandle {
    /// Wake eventfd descriptor.
    wake_fd: RawFd,
}

impl PollerWakeHandle for IoUringWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        wake_eventfd(self.wake_fd)
    }
}

impl IoUringPoller {
    /// Create a new io_uring poller instance.
    pub(crate) fn new() -> RuntimeResult<Self> {
        // initialize io_uring
        let ring = IoUring::new(DEFAULT_QUEUE_DEPTH).map_err(|error| {
            let errno = error.raw_os_error();
            let code = errno.and_then(io_error_code_from_errno);
            RuntimeError::from(HostError::io_with(
                code,
                None,
                errno,
                Some("poller.io_uring".to_string()),
                None,
                format!("io_uring init failed: {error}"),
            ))
            .boxed()
        })?;

        // create the wake eventfd
        let wake_fd = create_wake_eventfd()?;

        // assemble the poller state
        let mut poller = Self {
            ring,
            registrations: HashMap::new(),
            tokens: HashMap::new(),
            wake_fd,
            wake_handle: Arc::new(IoUringWakeHandle { wake_fd }),
            wake_pending: false,
            timeout_spec: types::Timespec::new(),
            timeout_pending: false,
        };

        // register the wake fd
        poller.submit_poll(
            WAKE_TOKEN,
            wake_fd,
            PollInterest::READABLE,
            HostPollerFlags::NONE,
        )?;
        poller.wake_pending = true;
        poller.submit()?;

        Ok(poller)
    }

    fn submit(&mut self) -> RuntimeResult<()> {
        let submitter = self.ring.submitter();
        submitter
            .submit()
            .map_err(|_| io_error("poller.submit", None))?;
        Ok(())
    }

    fn submit_and_wait(&mut self, wait_for: usize) -> RuntimeResult<()> {
        let submitter = self.ring.submitter();
        submitter
            .submit_and_wait(wait_for)
            .map_err(|_| io_error("poller.submit_and_wait", None))?;
        Ok(())
    }

    fn submit_poll(
        &mut self,
        token: PollerToken,
        fd: RawFd,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        let mask = poll_mask_for_interest(interests, flags);
        let entry = opcode::PollAdd::new(types::Fd(fd), mask)
            .build()
            .user_data(token.0);

        // SAFETY: the submission queue is only accessed while holding exclusive poller state
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(|_| io_error("poller.sq_full", Some(fd)))?;
        }

        Ok(())
    }

    fn submit_poll_remove(&mut self, token: PollerToken) -> RuntimeResult<()> {
        let entry = opcode::PollRemove::new(token.0).build().user_data(token.0);

        // SAFETY: the submission queue is only accessed while holding exclusive poller state
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(|_| io_error("poller.sq_full", None))?;
        }

        Ok(())
    }

    fn submit_timeout_remove(&mut self) -> RuntimeResult<()> {
        let entry = opcode::TimeoutRemove::new(TIMEOUT_TOKEN.0)
            .build()
            .user_data(TIMEOUT_TOKEN.0);

        // SAFETY: the submission queue is only accessed while holding exclusive poller state
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(|_| io_error("poller.sq_full", None))?;
        }

        Ok(())
    }

    fn submit_timeout(&mut self, timeout_nanos: u64) -> RuntimeResult<()> {
        if self.timeout_pending {
            return Ok(());
        }

        let seconds = timeout_nanos / 1_000_000_000;
        let nanos = (timeout_nanos % 1_000_000_000) as u32;
        self.timeout_spec = types::Timespec::new().sec(seconds).nsec(nanos);
        let entry = opcode::Timeout::new(&self.timeout_spec)
            .build()
            .user_data(TIMEOUT_TOKEN.0);

        // SAFETY: timeout_spec is stored on self and lives until the submission queue is flushed
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(|_| io_error("poller.sq_full", None))?;
        }

        self.timeout_pending = true;

        Ok(())
    }

    fn resubmit_if_needed(&mut self, resource_id: ResourceId) {
        let (token, fd, interests, flags, pending) = match self.registrations.get(&resource_id) {
            Some(entry) => (
                entry.token,
                entry.fd,
                entry.interests,
                entry.flags,
                entry.pending,
            ),
            None => return,
        };

        if flags.contains(HostPollerFlags::ONESHOT) || pending {
            return;
        }

        if self.submit_poll(token, fd, interests, flags).is_ok()
            && let Some(entry) = self.registrations.get_mut(&resource_id)
        {
            entry.pending = true;
        }
    }
}

impl Drop for IoUringPoller {
    fn drop(&mut self) {
        // SAFETY: wake_fd is owned by this poller
        unsafe {
            libc::close(self.wake_fd);
        }
    }
}

impl HostPoller for IoUringPoller {
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

        if flags.contains(HostPollerFlags::EDGE) {
            return Err(RuntimeError::from(HostError::not_supported(
                "io_uring poller does not support edge-triggered registrations yet",
            ))
            .boxed());
        }

        if self.tokens.contains_key(&token) {
            return Err(RuntimeError::from(HostError::invalid_argument_value(
                "token",
                "token already registered",
            ))
            .boxed());
        }

        let entry = PollRegistration {
            fd: handle.as_raw_fd(),
            token,
            interests,
            flags,
            pending: false,
        };
        self.registrations.insert(resource_id, entry);
        self.tokens.insert(token, resource_id);
        self.submit_poll(token, handle.as_raw_fd(), interests, flags)?;
        if let Some(registration) = self.registrations.get_mut(&resource_id) {
            registration.pending = true;
        }
        self.submit()?;

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

        if flags.contains(HostPollerFlags::EDGE) {
            return Err(RuntimeError::from(HostError::not_supported(
                "io_uring poller does not support edge-triggered registrations yet",
            ))
            .boxed());
        }

        let (existing_token, fd, was_pending) = self
            .registrations
            .get(&resource_id)
            .map(|entry| (entry.token, entry.fd, entry.pending))
            .ok_or_else(|| {
                RuntimeError::ResourceNotFound {
                    resource_id: resource_id.local_id,
                    resource_kind: None,
                }
                .boxed()
            })?;

        if was_pending {
            self.submit_poll_remove(existing_token)?;
        }

        if existing_token != token {
            if self.tokens.contains_key(&token) {
                return Err(RuntimeError::from(HostError::invalid_argument_value(
                    "token",
                    "token already registered",
                ))
                .boxed());
            }
            self.tokens.remove(&existing_token);
            self.tokens.insert(token, resource_id);
        }

        if let Some(entry) = self.registrations.get_mut(&resource_id) {
            entry.token = token;
            entry.interests = interests;
            entry.flags = flags;
            entry.pending = false;
        }

        self.submit_poll(token, fd, interests, flags)?;
        if let Some(entry) = self.registrations.get_mut(&resource_id) {
            entry.pending = true;
        }
        self.submit()?;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        let Some(entry) = self.registrations.remove(&resource_id) else {
            return Ok(());
        };
        self.tokens.remove(&entry.token);

        if entry.pending {
            self.submit_poll_remove(entry.token)?;
            self.submit()?;
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
        if let Some(timeout) = timeout_nanos
            && timeout > 0
        {
            self.submit_timeout(timeout)?;
        }

        let wait_for = if matches!(timeout_nanos, Some(0)) {
            0
        } else {
            1
        };
        if wait_for > 0 {
            self.submit_and_wait(wait_for)?;
        } else {
            self.submit()?;
        }

        let mut output = Vec::with_capacity(self.registrations.len());
        let mut to_resubmit = Vec::with_capacity(self.registrations.len());
        let mut saw_timeout = false;

        for cqe in self.ring.completion() {
            let user_data = cqe.user_data();
            if user_data == TIMEOUT_TOKEN.0 {
                self.timeout_pending = false;
                saw_timeout = true;
                continue;
            }
            if user_data == WAKE_TOKEN.0 {
                drain_eventfd(self.wake_fd);
                self.wake_pending = false;
                continue;
            }

            let token = PollerToken(user_data);
            let Some(resource_id) = self.tokens.get(&token).copied() else {
                continue;
            };
            let Some(registration) = self.registrations.get_mut(&resource_id) else {
                continue;
            };
            registration.pending = false;

            let result = cqe.result();
            if result < 0 {
                let errno = -result;
                if errno == libc::ECANCELED {
                    continue;
                }
                return Err(io_error_with_errno(
                    "poller.io_uring",
                    Some(registration.fd),
                    errno,
                ));
            }

            let mask = event_mask_from_revents(result as u32);
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
                    data: result as u64,
                },
            });

            to_resubmit.push(resource_id);
        }

        for resource_id in to_resubmit {
            self.resubmit_if_needed(resource_id);
        }

        if self.timeout_pending && !saw_timeout {
            self.submit_timeout_remove()?;
            self.timeout_pending = false;
        }

        if !self.wake_pending {
            self.submit_poll(
                WAKE_TOKEN,
                self.wake_fd,
                PollInterest::READABLE,
                HostPollerFlags::NONE,
            )?;
            self.wake_pending = true;
        }

        self.submit()?;

        Ok(output)
    }
}

/// Write one wake value into one eventfd.
fn wake_eventfd(fd: RawFd) -> RuntimeResult<()> {
    let value: u64 = 1;

    // SAFETY: value is a valid readable u64 buffer for the requested byte count
    let result = unsafe {
        libc::write(
            fd,
            &value as *const u64 as *const _,
            std::mem::size_of::<u64>(),
        )
    };
    if result < 0 {
        let errno = host_core::get_errno();
        if errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            return Err(io_error("poller.wake", Some(fd)));
        }
    }

    Ok(())
}

fn poll_mask_for_interest(interests: PollInterest, flags: HostPollerFlags) -> u32 {
    let mut mask = 0;
    if interests.contains(PollInterest::READABLE) {
        mask |= libc::POLLIN as u32;
    }
    if interests.contains(PollInterest::WRITABLE) {
        mask |= libc::POLLOUT as u32;
    }
    if flags.contains(HostPollerFlags::PRIORITY) {
        mask |= libc::POLLPRI as u32;
    }

    mask
}

fn event_mask_from_revents(revents: u32) -> PollerEventMask {
    let mut mask = PollerEventMask::NONE;

    if (revents & (libc::POLLIN as u32)) != 0 {
        mask |= PollerEventMask::READABLE;
    }
    if (revents & (libc::POLLOUT as u32)) != 0 {
        mask |= PollerEventMask::WRITABLE;
    }
    if (revents & (libc::POLLERR as u32)) != 0 {
        mask |= PollerEventMask::ERROR;
    }
    if (revents & (libc::POLLHUP as u32)) != 0 {
        mask |= PollerEventMask::HANGUP;
    }
    if (revents & (libc::POLLPRI as u32)) != 0 {
        mask |= PollerEventMask::PRIORITY;
    }

    mask
}

fn event_flags_from_registration(flags: HostPollerFlags) -> PollerEventFlags {
    let mut out = PollerEventFlags::NONE;
    if flags.contains(HostPollerFlags::EDGE) {
        out |= PollerEventFlags::EDGE;
    }
    if flags.contains(HostPollerFlags::ONESHOT) {
        out |= PollerEventFlags::ONESHOT;
    }

    out
}

fn create_wake_eventfd() -> RuntimeResult<RawFd> {
    // SAFETY: eventfd has no pointer arguments and returns either an fd or errno
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    if fd < 0 {
        return Err(io_error("poller.eventfd", None));
    }
    Ok(fd)
}

fn drain_eventfd(fd: RawFd) {
    let mut value = 0u64;
    loop {
        // SAFETY: value is a valid writable u64 buffer for the requested byte count
        let result = unsafe {
            libc::read(
                fd,
                &mut value as *mut u64 as *mut _,
                std::mem::size_of::<u64>(),
            )
        };
        if result < 0 {
            let errno = host_core::get_errno();
            if errno == libc::EWOULDBLOCK || errno == libc::EAGAIN {
                break;
            }
        } else {
            break;
        }
    }
}

fn io_error(context: &str, fd: Option<RawFd>) -> Box<RuntimeError> {
    let errno = host_core::get_errno();
    io_error_with_errno(context, fd, errno)
}

fn io_error_with_errno(context: &str, fd: Option<RawFd>, errno: i32) -> Box<RuntimeError> {
    let message = format!("{context} failed: errno {errno}");
    let code = io_error_code_from_errno(errno);
    let mut error = HostError::io_with(
        code,
        None,
        Some(errno),
        Some(context.to_string()),
        None,
        message,
    );
    if let Some(fd) = fd {
        let mut context = error
            .context
            .unwrap_or_else(|| HostErrorContext::with_kind(HostErrorContextKind::Io));
        context.fd = Some(fd);
        error.context = Some(context);
    }
    RuntimeError::from(error).boxed()
}
