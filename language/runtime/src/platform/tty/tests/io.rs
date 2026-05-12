#[cfg(unix)]
use super::{HarnessValue, close_tty_worker_resource, decode_harness_value, pty_descriptor};
use super::{assert_platform_error_codes, with_harness_context};
#[cfg(unix)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, TtyHandle};

/// Convert one platform tty flag into `u64` for test bit arithmetic.
#[cfg(unix)]
#[allow(clippy::useless_conversion)]
fn tty_flag_u64(flag: libc::tcflag_t) -> u64 {
    flag.into()
}

/// Write all bytes to one unix descriptor.
#[cfg(unix)]
fn write_all(descriptor: libc::c_int, bytes: &[u8]) -> RuntimeResult<()> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let status = unsafe {
            libc::write(
                descriptor,
                bytes[offset..].as_ptr().cast::<libc::c_void>(),
                bytes.len() - offset,
            )
        };
        if status < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::EINTR {
                continue;
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "write failed for pty descriptor: errno {errno}",
            )))
            .boxed());
        }

        offset += status as usize;
    }

    Ok(())
}

/// Read an exact number of bytes from one unix descriptor.
#[cfg(unix)]
fn read_exact(descriptor: libc::c_int, length: usize) -> RuntimeResult<Vec<u8>> {
    let mut output = vec![0u8; length];
    let mut offset = 0usize;
    while offset < length {
        let status = unsafe {
            libc::read(
                descriptor,
                output[offset..].as_mut_ptr().cast::<libc::c_void>(),
                length - offset,
            )
        };
        if status < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::EINTR {
                continue;
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "read failed for pty descriptor: errno {errno}",
            )))
            .boxed());
        }
        if status == 0 {
            return Err(RuntimeError::from(PlatformError::io(
                "unexpected EOF while reading from pty descriptor".to_string(),
            ))
            .boxed());
        }

        offset += status as usize;
    }

    Ok(output)
}

/// Exchange bytes between tty worker bindings and pty controller descriptor.
#[cfg(unix)]
#[test]
fn test_tty_io_roundtrip_through_pty_pair() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // set one raw-like mode to avoid canonical buffering and echo behavior
        let mode = context.destack_tty_get_mode(pair.worker)?;
        let mut mode = decode_harness_value(mode);
        mode.local_flags &= !tty_flag_u64(libc::ICANON);
        mode.local_flags &= !tty_flag_u64(libc::ECHO);
        let mode_value = context.tty_mode_value(mode);
        context.destack_tty_set_mode(pair.worker, mode_value)?;

        // resolve the controller descriptor from the resource table
        let controller_descriptor = pty_descriptor(context.call_context, pair.controller)?;

        // write through tty binding and read from controller descriptor
        let outbound = b"destack-tty-outbound";
        let outbound_value = context.bytes_value(outbound)?;
        let written = context.destack_tty_write(pair.worker, outbound_value)?;
        assert_eq!(written as usize, outbound.len());
        let observed = read_exact(controller_descriptor, outbound.len())?;
        assert_eq!(observed, outbound);

        // write through controller descriptor and read through tty binding
        let inbound = b"destack-tty-inbound";
        write_all(controller_descriptor, inbound)?;
        let mut buffer = vec![0u8; inbound.len()];
        let inbound_value = context.mutable_bytes_value(&mut buffer)?;
        match inbound_value {
            HarnessValue::Native(native_buffer) => {
                let read =
                    context.destack_tty_read(pair.worker, HarnessValue::Native(native_buffer))?;
                assert_eq!(read as usize, inbound.len());
                assert_eq!(buffer, inbound);
            }
            HarnessValue::Vm(vm_buffer) => {
                let read = context.destack_tty_read(pair.worker, HarnessValue::Vm(vm_buffer))?;
                assert_eq!(read as usize, inbound.len());
                let observed = context.bytes_from_value(HarnessValue::Vm(vm_buffer))?;
                assert_eq!(observed, inbound);
            }
        }

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject unknown tty handles for io operations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_io_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId::local(0));

        let mut read_buffer = [0u8; 8];
        let read_buffer = context.mutable_bytes_value(&mut read_buffer)?;
        let read_result = context.destack_tty_read(unknown, read_buffer);
        assert_platform_error_codes(read_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let write_buffer = context.bytes_value(b"destack")?;
        let write_result = context.destack_tty_write(unknown, write_buffer);
        assert_platform_error_codes(write_result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}
