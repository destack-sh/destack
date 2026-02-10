#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{NetHarnessKind, native_slice, native_slice_mut, with_harness_context};
use crate::platform::net::{
    AcceptFlags, destack_net_accept, destack_net_close, destack_net_close_listener,
    destack_net_read, destack_net_write,
};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};

#[cfg(any(unix, windows))]
#[test]
fn test_net_invalid_handles() {
    with_harness_context(|mut context| {
        match context.kind() {
            NetHarnessKind::Native => {
                let status = unsafe { destack_net_close(SocketHandle(ResourceId(9999))) };
                context.status_err(status, "close invalid socket")?;

                let status =
                    unsafe { destack_net_close_listener(ListenerHandle(ResourceId(9999))) };
                context.status_err(status, "close invalid listener")?;

                let buffer = b"data".to_vec();
                let slice = native_slice(&buffer);
                let mut out = 0u64;
                let status =
                    unsafe { destack_net_write(&mut out, SocketHandle(ResourceId(9999)), slice) };
                context.status_err(status, "write invalid socket")?;

                let mut buffer = vec![0u8; 8];
                let slice = native_slice_mut(&mut buffer);
                let status =
                    unsafe { destack_net_read(&mut out, SocketHandle(ResourceId(9999)), slice) };
                context.status_err(status, "read invalid socket")?;
            }
            NetHarnessKind::Vm => {
                let result = context.close(SocketHandle(ResourceId(9999)));
                assert!(result.is_err(), "close invalid socket should error");

                let result = context.close_listener(ListenerHandle(ResourceId(9999)));
                assert!(result.is_err(), "close invalid listener should error");

                let result = context.write(SocketHandle(ResourceId(9999)), b"data");
                assert!(result.is_err(), "write invalid socket should error");

                let mut buffer = vec![0u8; 8];
                let result = context.read(SocketHandle(ResourceId(9999)), &mut buffer);
                assert!(result.is_err(), "read invalid socket should error");
            }
        }

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_net_accept_after_close() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 8)?;

        // close the listener before accepting
        context.close_listener(listener)?;

        match context.kind() {
            NetHarnessKind::Native => {
                let mut handle = SocketHandle(ResourceId(0));
                let status = unsafe { destack_net_accept(&mut handle, listener, AcceptFlags(0)) };
                context.status_err(status, "accept after close")?;
            }
            NetHarnessKind::Vm => {
                let result = context.accept(listener);
                assert!(result.is_err(), "accept after close should error");
            }
        }

        Ok(())
    });
}
