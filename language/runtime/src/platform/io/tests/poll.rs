use super::{
    IoHarnessContext, assert_not_supported_result, assert_platform_error_code, with_harness_context,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(windows)]
use crate::platform::core as core_platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::io::{PollBackend, PollEvent, PollInterest};
use crate::platform::resource::{
    PollHandle, ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind,
};

#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, IN_ADDR, IN_ADDR_0, INVALID_SOCKET, IPPROTO_UDP, SOCK_DGRAM, SOCKADDR, SOCKADDR_IN,
    SOCKADDR_STORAGE, SOCKET, bind, closesocket, connect, getsockname, send, socket,
};

/// Readable event bit in `PollInterest`.
const POLL_READY_READABLE: u32 = 1 << 0;

#[cfg(unix)]
#[derive(Debug)]
struct UnixFdFinalizer {
    /// File descriptor to close during resource cleanup.
    descriptor: libc::c_int,
}

#[cfg(unix)]
impl ResourceFinalizer for UnixFdFinalizer {
    /// Close one owned descriptor during runtime resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

#[cfg(windows)]
#[derive(Debug)]
struct WindowsSocketFinalizer {
    /// Socket to close during resource cleanup.
    socket: SOCKET,
}

#[cfg(windows)]
impl ResourceFinalizer for WindowsSocketFinalizer {
    /// Close one owned socket during runtime resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            closesocket(self.socket);
        }
    }
}

/// Write endpoint used to trigger readiness on Unix.
#[cfg(unix)]
struct PollWriteEndpoint {
    /// Unix write descriptor.
    descriptor: libc::c_int,
}

/// Write endpoint used to trigger readiness on Windows.
#[cfg(windows)]
struct PollWriteEndpoint {
    /// Windows sender socket.
    socket: SOCKET,
}

/// Poll target handles and wake writer for readiness tests.
struct PollTargets {
    /// Runtime resource id for the read target.
    read_target: ResourceId,
    /// Runtime resource id for the write target.
    write_target: ResourceId,
    /// Host write endpoint used to trigger readiness.
    write_endpoint: PollWriteEndpoint,
}

/// Build one runtime io error used by poll test helpers.
fn poll_test_error(message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io(message)).boxed()
}

/// Build one read and write poll target pair for readiness tests.
fn create_poll_targets(context: &IoHarnessContext<'_>) -> RuntimeResult<PollTargets> {
    #[cfg(unix)]
    {
        // create one anonymous pipe pair
        let mut descriptors = [0; 2];
        let status = unsafe { libc::pipe(descriptors.as_mut_ptr()) };
        if status < 0 {
            return Err(poll_test_error("failed to create poll pipe targets"));
        }

        // register the read and write descriptors as pollable resources
        let read_entry = ResourceEntry::new(ResourceKind::Pipe)
            .with_fd(descriptors[0])
            .with_finalizer(UnixFdFinalizer {
                descriptor: descriptors[0],
            });
        let write_entry = ResourceEntry::new(ResourceKind::Pipe)
            .with_fd(descriptors[1])
            .with_finalizer(UnixFdFinalizer {
                descriptor: descriptors[1],
            });

        let read_target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            read_entry,
            Some(context.call_context.engine()),
        );
        let write_target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            write_entry,
            Some(context.call_context.engine()),
        );

        Ok(PollTargets {
            read_target,
            write_target,
            write_endpoint: PollWriteEndpoint {
                descriptor: descriptors[1],
            },
        })
    }

    #[cfg(windows)]
    {
        // ensure winsock is initialized before creating sockets
        core_platform::ensure_winsock()?;

        // create one receiver socket bound to loopback with an ephemeral port
        let receiver = unsafe { socket(AF_INET.into(), SOCK_DGRAM, IPPROTO_UDP) };
        if receiver == INVALID_SOCKET {
            return Err(poll_test_error("failed to create receiver socket"));
        }

        let mut address = SOCKADDR_IN {
            sin_family: AF_INET,
            sin_port: 0,
            sin_addr: IN_ADDR {
                S_un: IN_ADDR_0 { S_addr: 0 },
            },
            sin_zero: [0; 8],
        };
        let status = unsafe {
            bind(
                receiver,
                &mut address as *mut _ as *mut SOCKADDR,
                std::mem::size_of::<SOCKADDR_IN>() as i32,
            )
        };
        if status != 0 {
            unsafe {
                closesocket(receiver);
            }
            return Err(poll_test_error("failed to bind receiver socket"));
        }

        // resolve the receiver endpoint and connect one sender socket to it
        let mut storage = std::mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
        let mut length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
        let status =
            unsafe { getsockname(receiver, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
        if status != 0 {
            unsafe {
                closesocket(receiver);
            }
            return Err(poll_test_error("failed to resolve receiver socket address"));
        }
        let storage = unsafe { storage.assume_init() };

        let sender = unsafe { socket(AF_INET.into(), SOCK_DGRAM, IPPROTO_UDP) };
        if sender == INVALID_SOCKET {
            unsafe {
                closesocket(receiver);
            }
            return Err(poll_test_error("failed to create sender socket"));
        }

        let status = unsafe { connect(sender, &storage as *const _ as *const SOCKADDR, length) };
        if status != 0 {
            unsafe {
                closesocket(sender);
                closesocket(receiver);
            }
            return Err(poll_test_error("failed to connect sender socket"));
        }

        // register the socket pair as pollable resources
        let read_entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(receiver as _)
            .with_finalizer(WindowsSocketFinalizer { socket: receiver });
        let write_entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(sender as _)
            .with_finalizer(WindowsSocketFinalizer { socket: sender });

        let read_target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            read_entry,
            Some(context.call_context.engine()),
        );
        let write_target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            write_entry,
            Some(context.call_context.engine()),
        );

        Ok(PollTargets {
            read_target,
            write_target,
            write_endpoint: PollWriteEndpoint { socket: sender },
        })
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = context;
        Err(poll_test_error("poll tests require unix or windows"))
    }
}

/// Remove poll target resources from the runtime table.
fn remove_poll_targets(context: &IoHarnessContext<'_>, targets: PollTargets) {
    context.call_context.worker().resources.remove_and_finalize(
        context.call_context.world(),
        targets.read_target,
        Some(context.call_context.engine()),
    );
    context.call_context.worker().resources.remove_and_finalize(
        context.call_context.world(),
        targets.write_target,
        Some(context.call_context.engine()),
    );
}

/// Write one wake byte to one poll write endpoint.
fn write_poll_wake(endpoint: &PollWriteEndpoint) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let byte = [0x41u8];
        let status = unsafe { libc::write(endpoint.descriptor, byte.as_ptr() as *const _, 1) };
        if status != 1 {
            return Err(poll_test_error("failed to write wake byte"));
        }

        Ok(())
    }

    #[cfg(windows)]
    {
        let byte = [0x41u8];
        let status = unsafe { send(endpoint.socket, byte.as_ptr() as *const _, 1, 0) };
        if status != 1 {
            return Err(poll_test_error("failed to send wake byte"));
        }

        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = endpoint;
        Err(poll_test_error(
            "poll wake is not supported on this platform",
        ))
    }
}

/// Return whether one poll event carries the readable bit.
fn is_readable_event(event: PollEvent) -> bool {
    (event.ready.0 & POLL_READY_READABLE) != 0
}

/// Return whether one backend should be supported on the active host.
fn poll_backend_is_supported(backend: PollBackend) -> bool {
    match backend {
        PollBackend::Auto => true,
        PollBackend::Epoll => cfg!(target_os = "linux"),
        PollBackend::Kqueue => cfg!(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        )),
        PollBackend::Poll => cfg!(any(unix, windows)),
    }
}

/// Open and close one poll handle on both harness engines.
#[test]
fn test_io_poll_open_close_roundtrip() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;
        context.destack_io_poll_close(handle)?;

        assert_platform_error_code(
            context.destack_io_poll_close(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject forged poll handles that point to non-poll resources.
#[test]
fn test_io_poll_close_rejects_non_poll_handle() {
    with_harness_context(|mut context| {
        let foreign = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
        let forged = PollHandle(foreign);
        assert_platform_error_code(
            context.destack_io_poll_close(forged),
            PlatformErrorCode::IoNotFound,
        )?;
        assert!(context.call_context.worker().resources.contains(foreign));
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            foreign,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Reject waits that request zero max events.
#[test]
fn test_io_poll_wait_rejects_zero_maxevents() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        assert_platform_error_code(
            context.destack_io_poll_wait(handle, 0, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_io_poll_close(handle)?;

        Ok(())
    });
}

/// Return an empty result when no targets are registered.
#[test]
fn test_io_poll_wait_empty_returns_no_events() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        let events = context.poll_wait_events(handle, 0, 16)?;
        assert!(events.is_empty());

        context.destack_io_poll_close(handle)?;

        Ok(())
    });
}

/// Reject operations that use unknown poll handles.
#[test]
fn test_io_poll_rejects_unknown_poll_handle() {
    with_harness_context(|mut context| {
        let unknown = PollHandle(ResourceId::local(9999));

        assert_platform_error_code(
            context.destack_io_poll_close(unknown),
            PlatformErrorCode::IoNotFound,
        )?;

        assert_platform_error_code(
            context.destack_io_poll_deregister(unknown, ResourceId::local(1)),
            PlatformErrorCode::IoNotFound,
        )?;

        assert_platform_error_code(
            context.destack_io_poll_wait(unknown, 0, 8),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject registrations with unsupported interest bits.
#[test]
fn test_io_poll_rejects_invalid_interest_bits() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        assert_platform_error_code(
            context.destack_io_poll_register(
                handle,
                ResourceId::local(9999),
                1,
                PollInterest(1 << 31),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_io_poll_close(handle)?;

        Ok(())
    });
}

/// Reject registrations with an empty interest mask.
#[test]
fn test_io_poll_rejects_empty_interest() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        assert_platform_error_code(
            context.destack_io_poll_register(handle, ResourceId::local(9999), 1, PollInterest(0)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_io_poll_close(handle)?;

        Ok(())
    });
}

/// Reject registrations that target unknown resources.
#[test]
fn test_io_poll_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        assert_platform_error_code(
            context.destack_io_poll_register(handle, ResourceId::local(9999), 1, PollInterest(1)),
            PlatformErrorCode::IoNotFound,
        )?;

        context.destack_io_poll_close(handle)?;

        Ok(())
    });
}

/// Reject registrations that target resources without pollable handles.
#[test]
fn test_io_poll_rejects_non_pollable_target() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;

        let target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
        assert_platform_error_code(
            context.destack_io_poll_register(handle, target, 1, PollInterest(1)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_io_poll_close(handle)?;
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            target,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Validate explicit backend selection contracts on the active host.
#[test]
fn test_io_poll_backend_selection_contract() {
    with_harness_context(|mut context| {
        let auto = context.destack_io_poll_open(PollBackend::Auto)?;
        context.destack_io_poll_close(auto)?;

        for backend in [PollBackend::Epoll, PollBackend::Kqueue, PollBackend::Poll] {
            let result = context.destack_io_poll_open(backend);
            if poll_backend_is_supported(backend) {
                let handle = result?;
                context.destack_io_poll_close(handle)?;
            } else {
                assert_not_supported_result(result)?;
            }
        }

        Ok(())
    });
}

/// Capture one readable readiness event from one registered target.
#[test]
fn test_io_poll_register_wait_readable_event() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;
        let targets = create_poll_targets(&context)?;

        context.destack_io_poll_register(handle, targets.read_target, 41, PollInterest(1))?;
        write_poll_wake(&targets.write_endpoint)?;

        let events = context.poll_wait_events(handle, 1_000_000, 8)?;
        let event = events
            .iter()
            .copied()
            .find(|event| event.key == 41 && is_readable_event(*event))
            .expect("missing readable poll event for key 41");
        assert_ne!(event.data, 0);

        context.destack_io_poll_close(handle)?;
        remove_poll_targets(&context, targets);

        Ok(())
    });
}

/// Apply one update and emit events with the updated key.
#[test]
fn test_io_poll_update_replaces_event_key() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;
        let targets = create_poll_targets(&context)?;

        context.destack_io_poll_register(handle, targets.read_target, 11, PollInterest(1))?;
        context.destack_io_poll_update(handle, targets.read_target, 27, PollInterest(1))?;
        write_poll_wake(&targets.write_endpoint)?;

        let mut all_events = Vec::new();
        let mut saw_updated_key = false;
        for _ in 0..8 {
            let events = context.poll_wait_events(handle, 5_000_000, 8)?;
            saw_updated_key |= events
                .iter()
                .any(|event| event.key == 27 && is_readable_event(*event));
            all_events.extend(events);
            if saw_updated_key {
                break;
            }
        }
        assert!(
            saw_updated_key,
            "missing updated-key poll event: {all_events:?}"
        );

        context.destack_io_poll_close(handle)?;
        remove_poll_targets(&context, targets);

        Ok(())
    });
}

/// Stop receiving readiness notifications after target deregistration.
#[test]
fn test_io_poll_deregister_stops_events() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;
        let targets = create_poll_targets(&context)?;

        context.destack_io_poll_register(handle, targets.read_target, 31, PollInterest(1))?;
        context.destack_io_poll_deregister(handle, targets.read_target)?;
        write_poll_wake(&targets.write_endpoint)?;

        let events = context.poll_wait_events(handle, 0, 8)?;
        assert!(events.is_empty());

        context.destack_io_poll_close(handle)?;
        remove_poll_targets(&context, targets);

        Ok(())
    });
}

/// Reject reserved poll key values reserved for backend internals.
#[test]
fn test_io_poll_rejects_reserved_keys() {
    with_harness_context(|mut context| {
        let handle = context.destack_io_poll_open(PollBackend::Auto)?;
        let targets = create_poll_targets(&context)?;

        assert_platform_error_code(
            context.destack_io_poll_register(
                handle,
                targets.read_target,
                u64::MAX,
                PollInterest(1),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_io_poll_close(handle)?;
        remove_poll_targets(&context, targets);

        Ok(())
    });
}
