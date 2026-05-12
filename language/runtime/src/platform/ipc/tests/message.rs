use crate::platform::abi::NativeSlice;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{VmSlice, resource};

use super::core::{assert_runtime_error_code, unique_ipc_name};
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

/// Verify message queues can open, send, receive, close, and unlink.
#[test]
fn test_message_queue_roundtrip() {
    with_harness_context(|mut context| {
        // create one uniquely named message queue
        let name = unique_ipc_name("ipc_mqueue_roundtrip");
        let flags = (libc::O_CREAT | libc::O_EXCL | libc::O_RDWR) as u32;
        let name_value = context.string_value(&name)?;
        let handle = context.destack_ipc_message_queue_open(name_value, flags, 0o600, 8, 256)?;

        // send one payload through the message queue
        let payload = b"destack-ipc-mqueue";
        let payload_value = context.bytes_value(payload)?;
        context.destack_ipc_message_queue_send(handle, 7, 1_000_000_000, payload_value)?;

        // receive one payload from the message queue
        let mut receive_buffer = vec![0u8; 256];
        let receive_value = context.mutable_bytes_value(&mut receive_buffer)?;
        let receive_value_copy = clone_bytes_value(&context, &receive_value);
        let received_payload =
            context.destack_ipc_message_queue_receive(handle, 1_000_000_000, receive_value)?;
        let receive = context.message_queue_receive_value(received_payload);
        assert_eq!(receive.bytes as usize, payload.len());
        assert_eq!(receive.priority, 7);

        // verify payload integrity after queue receive
        let received_bytes = context.bytes_from_value(receive_value_copy)?;
        assert_eq!(&received_bytes[..payload.len()], payload);

        // close and unlink the queue once the roundtrip is complete
        context.destack_ipc_message_queue_close(handle)?;
        let unlink_name = context.string_value(&name)?;
        context.destack_ipc_message_queue_unlink(unlink_name)?;

        Ok(())
    });
}

/// Verify message queue open rejects unsupported flag bits.
#[test]
fn test_message_queue_open_rejects_unsupported_flags() {
    with_harness_context(|mut context| {
        let name = unique_ipc_name("ipc_mqueue_flags");
        let name_value = context.string_value(&name)?;

        let error = context
            .destack_ipc_message_queue_open(name_value, u32::MAX, 0o600, 8, 256)
            .expect_err("expected queueOpen to reject unsupported flags");
        assert_runtime_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify message queue operations reject unknown handles.
#[test]
fn test_message_queue_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        // build one unknown message queue handle
        let unknown = resource::MessageQueueHandle(resource::ResourceId::local(0));

        // close should fail with invalid-argument for unknown handle
        let close_error = context
            .destack_ipc_message_queue_close(unknown)
            .expect_err("expected queueClose to fail for unknown handle");
        assert_runtime_error_code(&close_error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
