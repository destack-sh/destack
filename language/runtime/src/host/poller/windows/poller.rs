use std::collections::HashMap;
use std::sync::Arc;

use windows_sys::Win32::Networking::WinSock::{
    AF_INET, INVALID_SOCKET, IPPROTO_UDP, POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLPRI,
    SOCK_DGRAM, SOCKADDR, SOCKADDR_IN, SOCKADDR_STORAGE, SOCKET, WSAPOLLFD, WSAPoll, bind,
    closesocket, connect, getsockname, recv, send, socket,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::poller::{
    HostHandle, HostPoller, HostPollerFlags, PollInterest, PollerEvent, PollerEventFlags,
    PollerEventMask, PollerEventPayload, PollerEventSource, PollerToken, PollerWakeHandle,
};
use crate::host::{HostError, ResourceId, windows as host_windows};

/// Build a runtime error from the last socket error.
fn last_net_error(syscall: &str) -> Box<RuntimeError> {
    let errno = host_windows::last_wsa_error_code();
    let message = host_windows::error_message(syscall, errno);
    RuntimeError::from(HostError::io_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Poller registration state for a resource.
#[derive(Debug, Clone, Copy)]
struct PollRegistration {
    /// Raw socket to poll.
    socket: SOCKET,
    /// Opaque token associated with the registration.
    token: PollerToken,
    /// Interest mask for readiness.
    interests: PollInterest,
    /// Poller configuration flags.
    flags: HostPollerFlags,
}

/// Wake socket pair for interrupting polls.
#[derive(Debug)]
struct WakeSockets {
    /// Receiver socket used in WSAPoll.
    receiver: SOCKET,
    /// Sender socket used to trigger wake.
    sender: SOCKET,
}

impl WakeSockets {
    /// Create a new wake socket pair.
    fn new() -> RuntimeResult<Self> {
        // ensure Winsock is initialized before creating sockets
        host_windows::initialize_winsock()?;

        let receiver = unsafe { socket(AF_INET.into(), SOCK_DGRAM, IPPROTO_UDP) };
        if receiver == INVALID_SOCKET {
            return Err(last_net_error("socket"));
        }

        let mut addr = SOCKADDR_IN {
            sin_family: AF_INET,
            sin_port: 0,
            sin_addr: windows_sys::Win32::Networking::WinSock::IN_ADDR {
                S_un: windows_sys::Win32::Networking::WinSock::IN_ADDR_0 { S_addr: 0 },
            },
            sin_zero: [0; 8],
        };
        let rc = unsafe {
            bind(
                receiver,
                &mut addr as *mut _ as *mut SOCKADDR,
                std::mem::size_of::<SOCKADDR_IN>() as i32,
            )
        };
        if rc != 0 {
            unsafe {
                closesocket(receiver);
            }
            return Err(last_net_error("bind"));
        }

        let mut storage = std::mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
        let mut length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
        let rc =
            unsafe { getsockname(receiver, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
        if rc != 0 {
            unsafe {
                closesocket(receiver);
            }
            return Err(last_net_error("getsockname"));
        }
        let storage = unsafe { storage.assume_init() };

        let sender = unsafe { socket(AF_INET.into(), SOCK_DGRAM, IPPROTO_UDP) };
        if sender == INVALID_SOCKET {
            unsafe {
                closesocket(receiver);
            }
            return Err(last_net_error("socket"));
        }
        let rc = unsafe { connect(sender, &storage as *const _ as *const SOCKADDR, length) };
        if rc != 0 {
            unsafe {
                closesocket(sender);
                closesocket(receiver);
            }
            return Err(last_net_error("connect"));
        }

        Ok(Self { receiver, sender })
    }

    /// Wake the poller.
    fn wake(&self) -> RuntimeResult<()> {
        wake_socket(self.sender)
    }

    /// Drain pending wake bytes.
    fn drain(&self) {
        let mut buffer = [0u8; 32];
        loop {
            let rc = unsafe {
                recv(
                    self.receiver,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as i32,
                    0,
                )
            };
            if rc <= 0 {
                break;
            }
            if rc < buffer.len() as i32 {
                break;
            }
        }
    }
}

impl Drop for WakeSockets {
    fn drop(&mut self) {
        unsafe {
            closesocket(self.sender);
            closesocket(self.receiver);
        }
    }
}

/// Shared wake handle for one windows poller.
#[derive(Debug)]
struct WindowsWakeHandle {
    /// Sender socket used to trigger wake.
    sender: SOCKET,
}

impl PollerWakeHandle for WindowsWakeHandle {
    fn wake(&self) -> RuntimeResult<()> {
        wake_socket(self.sender)
    }
}

/// WSAPoll-backed poller for Windows.
pub(crate) struct WindowsPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Wake sockets used to interrupt polling.
    wake: WakeSockets,
    /// Shared wake handle used by out-of-band wakeups.
    wake_handle: Arc<WindowsWakeHandle>,
    /// Pollfd buffer reused across polls.
    pollfds: Vec<WSAPOLLFD>,
    /// Entry buffer reused across polls.
    entries: Vec<Option<(ResourceId, PollRegistration)>>,
}

impl std::fmt::Debug for WindowsPoller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsPoller")
            .field("registrations", &self.registrations.len())
            .field("pollfds", &self.pollfds.len())
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl WindowsPoller {
    /// Create a new Windows poller instance.
    pub(crate) fn new() -> RuntimeResult<Self> {
        let wake = WakeSockets::new()?;

        Ok(Self {
            registrations: HashMap::new(),
            wake_handle: Arc::new(WindowsWakeHandle {
                sender: wake.sender,
            }),
            wake,
            pollfds: Vec::new(),
            entries: Vec::new(),
        })
    }
}

impl HostPoller for WindowsPoller {
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
                "WSAPoll does not support edge-triggered registrations",
            ))
            .boxed());
        }

        // update existing registrations in place
        if self.registrations.contains_key(&resource_id) {
            return self.update(resource_id, token, interests, flags);
        }

        // store the registration entry
        self.registrations.insert(
            resource_id,
            PollRegistration {
                socket: handle.as_raw_socket() as SOCKET,
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
                "WSAPoll does not support edge-triggered registrations",
            ))
            .boxed());
        }

        let Some(entry) = self.registrations.get_mut(&resource_id) else {
            return Err(RuntimeError::from(HostError::invalid_argument_value(
                "resource_id",
                "unknown resource id",
            ))
            .boxed());
        };

        entry.token = token;
        entry.interests = interests;
        entry.flags = flags;

        Ok(())
    }

    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.registrations.remove(&resource_id);
        Ok(())
    }

    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        Some(self.wake_handle.clone())
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        self.wake.wake()
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        let timeout = timeout_nanos.map(nanos_to_timeout_ms).unwrap_or(-1);

        self.pollfds.clear();
        self.entries.clear();
        self.pollfds.reserve(self.registrations.len() + 1);
        self.entries.reserve(self.registrations.len() + 1);

        // wake socket
        self.pollfds.push(WSAPOLLFD {
            fd: self.wake.receiver,
            events: POLLIN,
            revents: 0,
        });
        self.entries.push(None);

        for (resource_id, entry) in &self.registrations {
            let mut events = 0;
            if entry.interests.contains(PollInterest::READABLE) {
                events |= POLLIN;
            }
            if entry.interests.contains(PollInterest::WRITABLE) {
                events |= POLLOUT;
            }
            if entry.flags.contains(HostPollerFlags::PRIORITY) {
                events |= POLLPRI;
            }
            self.pollfds.push(WSAPOLLFD {
                fd: entry.socket,
                events,
                revents: 0,
            });
            self.entries.push(Some((*resource_id, *entry)));
        }

        let result = unsafe {
            WSAPoll(
                self.pollfds.as_mut_ptr(),
                self.pollfds.len() as u32,
                timeout,
            )
        };

        if result < 0 {
            return Err(last_net_error("WSAPoll"));
        }

        let mut output = Vec::with_capacity(result as usize);
        let mut oneshot = Vec::new();
        for (index, pollfd) in self.pollfds.iter().enumerate() {
            if pollfd.revents == 0 {
                continue;
            }
            let Some((resource_id, registration)) = self.entries[index] else {
                self.wake.drain();
                continue;
            };
            let mask = event_mask_from_revents(pollfd.revents);
            let flags = event_flags_from_registration(registration.flags);
            output.push(PollerEvent {
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

        Ok(output)
    }
}

/// Convert WSAPoll revents to a host mask.
fn event_mask_from_revents(revents: i16) -> PollerEventMask {
    let mut out = PollerEventMask::NONE;
    if revents & POLLIN != 0 {
        out |= PollerEventMask::READABLE;
    }
    if revents & POLLOUT != 0 {
        out |= PollerEventMask::WRITABLE;
    }
    if revents & POLLERR != 0 || revents & POLLNVAL != 0 {
        out |= PollerEventMask::ERROR;
    }
    if revents & POLLHUP != 0 {
        out |= PollerEventMask::HANGUP;
    }
    if revents & POLLPRI != 0 {
        out |= PollerEventMask::PRIORITY;
    }
    out
}

/// Convert a nanosecond timeout to milliseconds for WSAPoll.
fn nanos_to_timeout_ms(nanos: u64) -> i32 {
    if nanos == 0 {
        return 0;
    }

    let ms = nanos.div_ceil(1_000_000);
    if ms > i32::MAX as u64 {
        i32::MAX
    } else {
        ms as i32
    }
}

/// Build event flags from registration flags.
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

/// Send one wake byte through one wake socket.
fn wake_socket(sender: SOCKET) -> RuntimeResult<()> {
    let buffer = [0u8; 1];
    let rc = unsafe { send(sender, buffer.as_ptr() as *const _, 1, 0) };
    if rc < 0 {
        return Err(last_net_error("send"));
    }

    Ok(())
}
