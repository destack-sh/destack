use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::ClipboardBinaryFormat;
use crate::platform::os::tests::{HarnessValue, OsHarnessContext};
use crate::platform::{NativeSlice, NativeStringRef, VmSlice};
use crate::tests::platform::error_code_from_runtime_error;

/// Saved clipboard snapshot used to restore host state after one test.
pub(super) struct ClipboardSnapshot {
    /// Current text payload when one exists.
    pub(super) text: Option<String>,
    /// Current html payload when one exists.
    pub(super) html: Option<Vec<u8>>,
}

/// Build one harness string payload for the active binding lane.
pub(super) fn string_harness_value(
    context: &mut OsHarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let value = vm::StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm clipboard test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Build one harness byte-slice payload for the active binding lane.
pub(super) fn bytes_harness_value(
    context: &mut OsHarnessContext<'_>,
    value: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let value = VmSlice::from_bytes(vm_context, value)
                .expect("vm clipboard test bytes should encode");

            Ok(HarnessValue::Vm(value))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_slice(value.to_vec()),
        )),
    }
}

/// Decode one clipboard text harness value into one owned string.
pub(super) fn decode_clipboard_text(
    context: &mut OsHarnessContext<'_>,
    value: HarnessValue<NativeStringRef, vm::StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str() }?.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::ExternalCallContext<'_>)
            };
            let value = vm_context
                .string_ref(value)
                .map_err(|error| RuntimeError::from(error).boxed())?;

            Ok(value.as_str().to_string())
        }
    }
}

/// Decode one clipboard byte payload into one owned byte vector.
pub(super) fn decode_clipboard_bytes(
    context: &mut OsHarnessContext<'_>,
    value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
) -> RuntimeResult<Vec<u8>> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_slice() }?.to_vec()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::ExternalCallContext<'_>)
            };

            value.read_bytes(vm_context)
        }
    }
}

/// Snapshot the clipboard text and html payloads when supported.
pub(super) fn snapshot_clipboard(
    context: &mut OsHarnessContext<'_>,
) -> RuntimeResult<ClipboardSnapshot> {
    let text = if context.destack_os_clipboard_has_text()? {
        let value = context.destack_os_clipboard_read_text()?;
        Some(decode_clipboard_text(context, value)?)
    } else {
        None
    };

    let html = match context.destack_os_clipboard_read_bytes(ClipboardBinaryFormat::Html) {
        Ok(value) => Some(decode_clipboard_bytes(context, value)?),
        Err(error) => {
            let error_code = error_code_from_runtime_error(&error);
            if error_code == Some(PlatformErrorCode::NotSupported)
                || error_code == Some(PlatformErrorCode::IoNotFound)
            {
                None
            } else {
                return Err(error);
            }
        }
    };

    Ok(ClipboardSnapshot { text, html })
}

/// Restore one prior clipboard snapshot after one destructive test.
pub(super) fn restore_clipboard(
    context: &mut OsHarnessContext<'_>,
    snapshot: ClipboardSnapshot,
) -> RuntimeResult<()> {
    if let Some(html) = snapshot.html {
        let html = bytes_harness_value(context, &html)?;
        return context.destack_os_clipboard_write_bytes(ClipboardBinaryFormat::Html, html);
    }

    if let Some(text) = snapshot.text {
        let text = string_harness_value(context, &text);
        return context.destack_os_clipboard_write_text(text);
    }

    context.destack_os_clipboard_clear()
}
