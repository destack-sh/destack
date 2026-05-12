use crate::platform::abi::NativeSlice;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{VmSlice, resource};

use super::core::{assert_runtime_error_code, decode_pipe_pair_value};
use super::{HarnessValue, IpcHarnessContext, with_harness_context};

/// Clone one byte-slice harness wrapper by copying the contained slice descriptor.
fn clone_bytes_value(
    context: &IpcHarnessContext<'_>,
    value: &HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
) -> HarnessValue<NativeSlice<u8>, VmSlice<u8>> {
    match value {
        HarnessValue::Native(value) => context.harness_value(*value),
        HarnessValue::Vm(value) => context.harness_value_vm(*value),
    }
}

/// Verify pipe open, write, read, and close work across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_pipe_roundtrip_read_write_close() {
    with_harness_context(|mut context| {
        // open one unnamed pipe pair
        let pair = decode_pipe_pair_value(context.destack_ipc_pipe_open(0)?);

        // write one payload through the write endpoint
        let payload = b"destack-ipc-pipe";
        let write_value = context.bytes_value(payload)?;
        let written = context.destack_ipc_pipe_write(pair.write, write_value)?;
        assert_eq!(written as usize, payload.len());

        // read one payload from the read endpoint
        let mut read_buffer = vec![0u8; payload.len()];
        let read_value = context.mutable_bytes_value(&mut read_buffer)?;
        let read_value_copy = clone_bytes_value(&context, &read_value);
        let read = context.destack_ipc_pipe_read(pair.read, read_value)?;
        assert_eq!(read as usize, payload.len());

        // verify payload integrity after roundtrip
        let read_bytes = context.bytes_from_value(read_value_copy)?;
        assert_eq!(&read_bytes[..payload.len()], payload);

        // close both endpoints after use
        context.destack_ipc_pipe_close(pair.read)?;
        context.destack_ipc_pipe_close(pair.write)?;

        Ok(())
    });
}

/// Verify pipe open rejects unsupported non-zero flag words.
#[cfg(any(unix, windows))]
#[test]
fn test_pipe_open_rejects_unsupported_flags() {
    with_harness_context(|mut context| {
        let error = context.destack_ipc_pipe_open(1);
        let error = match error {
            Ok(_) => panic!("expected invalid flags to fail"),
            Err(error) => error,
        };
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify pipe operations reject unknown pipe handles.
#[cfg(any(unix, windows))]
#[test]
fn test_pipe_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        // build one unknown pipe handle and one tiny read buffer
        let unknown = resource::PipeHandle(resource::ResourceId::local(0));
        let mut buffer = [0u8; 1];

        // read should fail with invalid-argument for unknown handle
        let read_value = context.mutable_bytes_value(&mut buffer)?;
        let read_error = context.destack_ipc_pipe_read(unknown, read_value);
        let read_error = match read_error {
            Ok(_) => panic!("expected pipeRead to fail for unknown handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&read_error, PlatformErrorCode::InvalidArgumentValue);

        // write should fail with invalid-argument for unknown handle
        let write_value = context.bytes_value(&buffer)?;
        let write_error = context.destack_ipc_pipe_write(unknown, write_value);
        let write_error = match write_error {
            Ok(_) => panic!("expected pipeWrite to fail for unknown handle"),
            Err(error) => error,
        };
        assert_runtime_error_code(&write_error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
