use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Once;

use windows_sys::Win32::Networking::WinSock::{
    AF_INET, INVALID_SOCKET, IPPROTO_UDP, POLLERR, POLLHUP, POLLIN, POLLOUT, POLLPRI, SOCK_DGRAM,
    SOCKADDR, SOCKADDR_IN, SOCKADDR_STORAGE, SOCKET, WSAPOLLFD, WSAPoll, bind, closesocket,
    connect, getsockname, recv, send, socket,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::poller::{
    PlatformEvent, PlatformEventFlags, PlatformEventMask, PlatformEventPayload,
    PlatformEventSource, PlatformHandle, PlatformInterest, PlatformPoller, PlatformPollerFlags,
};
use crate::platform::{PlatformError, ResourceId};

/// Initialise Winsock using the standard library.
fn ensure_winsock() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = UdpSocket::bind("127.0.0.1:0");
    });
}

/// Build a runtime error from the last socket error.
fn last_net_error(syscall: &str) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error();
    let message = format!("{syscall} failed: {error}");
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        errno,
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
    token: u64,
    /// Interest mask for readiness.
    interests: PlatformInterest,
    /// Poller configuration flags.
    flags: PlatformPollerFlags,
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
        ensure_winsock();

        let receiver = unsafe { socket(AF_INET.into(), SOCK_DGRAM, IPPROTO_UDP) };
        if receiver == INVALID_SOCKET {
            return Err(last_net_error("socket"));
        }

        let mut addr = SOCKADDR_IN {
            sin_family: AF_INET as u16,
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
        let buffer = [0u8; 1];
        let rc = unsafe { send(self.sender, buffer.as_ptr() as *const _, 1, 0) };
        if rc < 0 {
            return Err(last_net_error("send"));
        }
        Ok(())
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

/// WSAPoll-backed poller for Windows.
#[derive(Debug)]
pub struct WindowsPoller {
    /// Registered resource entries.
    registrations: HashMap<ResourceId, PollRegistration>,
    /// Wake sockets used to interrupt polling.
    wake: WakeSockets,
}

impl WindowsPoller {
    /// Create a new Windows poller instance.
    pub fn new() -> RuntimeResult<Self> {
        Ok(Self {
            registrations: HashMap::new(),
            wake: WakeSockets::new()?,
        })
    }
}

impl PlatformPoller for WindowsPoller {
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
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()> {
        let Some(entry) = self.registrations.get_mut(&resource_id) else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
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

    fn wake(&mut self) -> RuntimeResult<()> {
        self.wake.wake()
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>> {
        let timeout = timeout_nanos
            .map(|value| (value / 1_000_000) as i32)
            .unwrap_or(-1);

        let mut pollfds = Vec::with_capacity(self.registrations.len() + 1);
        let mut entries = Vec::with_capacity(self.registrations.len() + 1);

        // wake socket
        pollfds.push(WSAPOLLFD {
            fd: self.wake.receiver,
            events: POLLIN,
            revents: 0,
        });
        entries.push(None);

        for (resource_id, entry) in &self.registrations {
            let mut events = 0;
            if entry.interests.contains(PlatformInterest::READABLE) {
                events |= POLLIN;
            }
            if entry.interests.contains(PlatformInterest::WRITABLE) {
                events |= POLLOUT;
            }
            if entry.flags.contains(PlatformPollerFlags::PRIORITY) {
                events |= POLLPRI;
            }
            pollfds.push(WSAPOLLFD {
                fd: entry.socket,
                events: events as i16,
                revents: 0,
            });
            entries.push(Some((*resource_id, *entry)));
        }

        let result = unsafe { WSAPoll(pollfds.as_mut_ptr(), pollfds.len() as u32, timeout) };
        if result < 0 {
            return Err(last_net_error("WSAPoll"));
        }

        let mut output = Vec::new();
        for (index, pollfd) in pollfds.iter().enumerate() {
            if pollfd.revents == 0 {
                continue;
            }
            let entry = entries[index];
            if entry.is_none() {
                self.wake.drain();
                continue;
            }
            let (resource_id, registration) = entry.unwrap();
            let mask = event_mask_from_revents(pollfd.revents);
            let flags = event_flags_from_registration(registration.flags);
            output.push(PlatformEvent {
                resource_id,
                source: PlatformEventSource::Io,
                mask,
                flags,
                token: registration.token,
                payload: PlatformEventPayload::Io {
                    data: pollfd.revents as u64,
                },
            });
        }

        Ok(output)
    }
}

/// Convert WSAPoll revents to a platform mask.
fn event_mask_from_revents(revents: i16) -> PlatformEventMask {
    let mut out = PlatformEventMask::NONE;
    if revents & POLLIN != 0 {
        out |= PlatformEventMask::READABLE;
    }
    if revents & POLLOUT != 0 {
        out |= PlatformEventMask::WRITABLE;
    }
    if revents & POLLERR != 0 {
        out |= PlatformEventMask::ERROR;
    }
    if revents & POLLHUP != 0 {
        out |= PlatformEventMask::HANGUP;
    }
    if revents & POLLPRI != 0 {
        out |= PlatformEventMask::PRIORITY;
    }
    out
}

/// Build event flags from registration flags.
fn event_flags_from_registration(flags: PlatformPollerFlags) -> PlatformEventFlags {
    let mut out = PlatformEventFlags::NONE;
    if flags.contains(PlatformPollerFlags::EDGE) {
        out |= PlatformEventFlags::EDGE;
    }
    if flags.contains(PlatformPollerFlags::ONESHOT) {
        out |= PlatformEventFlags::ONESHOT;
    }
    out
}
