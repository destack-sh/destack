#[cfg(unix)]
use std::os::fd::RawFd;

#[cfg(unix)]
use crate::diagnostic::RuntimeError;
#[cfg(unix)]
use crate::platform::core as core_platform;
#[cfg(unix)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
#[cfg(unix)]
use crate::platform::{PlatformError, resource};

#[cfg(unix)]
use super::core::assert_runtime_error_code;
#[cfg(unix)]
use super::with_harness_context;

/// Finalizer that closes one unix descriptor used in IPC tests.
#[cfg(unix)]
#[derive(Debug)]
struct UnixDescriptorFinalizer {
    /// Descriptor to close.
    descriptor: RawFd,
}

#[cfg(unix)]
impl ResourceFinalizer for UnixDescriptorFinalizer {
    /// Close the descriptor during resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Build one runtime error for unix test setup failures.
#[cfg(unix)]
fn unix_test_error(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(core_platform::get_errno()),
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Verify unix ancillary send and receive transfer descriptors.
#[cfg(unix)]
#[test]
fn test_unix_ancillary_send_receive_roundtrip() {
    with_harness_context(|mut context| {
        // create one unix datagram socket pair
        let mut sockets = [0; 2];
        let status =
            unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_DGRAM, 0, sockets.as_mut_ptr()) };
        if status != 0 {
            return Err(unix_test_error(
                "socketpair",
                "failed to create unix test socketpair",
            ));
        }

        // register socket endpoints in the runtime resource table
        let left_entry = ResourceEntry::new(ResourceKind::Socket)
            .with_fd(sockets[0])
            .with_finalizer(UnixDescriptorFinalizer {
                descriptor: sockets[0],
            });
        let right_entry = ResourceEntry::new(ResourceKind::Socket)
            .with_fd(sockets[1])
            .with_finalizer(UnixDescriptorFinalizer {
                descriptor: sockets[1],
            });
        let left = resource::SocketHandle(context.call_context.worker().resources.insert(
            context.call_context.world(),
            left_entry,
            Some(context.call_context.engine()),
        ));
        let right = resource::SocketHandle(context.call_context.worker().resources.insert(
            context.call_context.world(),
            right_entry,
            Some(context.call_context.engine()),
        ));

        // register one transferable descriptor
        let duplicated = unsafe { libc::dup(sockets[0]) };
        if duplicated < 0 {
            context.call_context.worker().resources.remove_and_finalize(
                context.call_context.world(),
                left.0,
                Some(context.call_context.engine()),
            );
            context.call_context.worker().resources.remove_and_finalize(
                context.call_context.world(),
                right.0,
                Some(context.call_context.engine()),
            );
            return Err(unix_test_error(
                "dup",
                "failed to duplicate one test descriptor for transfer",
            ));
        }
        let transferred_entry = ResourceEntry::new(ResourceKind::Transferred)
            .with_fd(duplicated)
            .with_finalizer(UnixDescriptorFinalizer {
                descriptor: duplicated,
            });
        let transferred =
            resource::TransferredHandle(context.call_context.worker().resources.insert(
                context.call_context.world(),
                transferred_entry,
                Some(context.call_context.engine()),
            ));

        // send one payload and descriptor over unix ancillary bindings
        let payload = context.bytes_value(b"ancillary-payload")?;
        let handles = context.transferred_handles_value(&[transferred])?;
        let sent = context.destack_ipc_unix_send(left, payload, handles)?;
        assert_eq!(sent, 17);

        // receive one payload and descriptor through unix ancillary bindings
        let receive = context.destack_ipc_unix_receive(right, 4)?;
        let (bytes, handles, _credentials) = context.unix_receive_value(receive)?;
        assert_eq!(bytes, 17);
        assert_eq!(handles.len(), 1);

        // cleanup all registered resources
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            left.0,
            Some(context.call_context.engine()),
        );
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            right.0,
            Some(context.call_context.engine()),
        );
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            transferred.0,
            Some(context.call_context.engine()),
        );
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            handles[0].0,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Verify unix ancillary operations reject unknown socket handles.
#[cfg(unix)]
#[test]
fn test_unix_ancillary_rejects_unknown_socket_handle() {
    with_harness_context(|mut context| {
        // build one unknown socket handle
        let unknown = resource::SocketHandle(resource::ResourceId::local(0));

        // receive should fail with invalid-argument for unknown socket handles
        let receive_error = context.destack_ipc_unix_receive(unknown, 0);
        let receive_error = match receive_error {
            Ok(_) => panic!("expected unixReceive to fail for unknown socket handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&receive_error, PlatformErrorCode::InvalidArgumentValue);

        // send should fail with invalid-argument for unknown socket handles
        let payload = context.bytes_value(b"payload")?;
        let handles = context.transferred_handles_value(&[])?;
        let send_error = context.destack_ipc_unix_send(unknown, payload, handles);
        let send_error = match send_error {
            Ok(_) => panic!("expected unixSend to fail for unknown socket handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&send_error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
