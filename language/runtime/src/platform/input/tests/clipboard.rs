use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::VmSlice;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::{NativeAbiCodec, VmAbiCodec};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::tests::{HarnessValue, InputHarnessContext as HarnessContext};
use crate::platform::input::{
    ClipboardItem, ClipboardItemDescriptor, ClipboardItemDescriptorVm, ClipboardItemRepresentation,
    ClipboardItemRepresentationKind, ClipboardItemRepresentationValue, ClipboardItemValue,
    ClipboardItemVm, ClipboardPresentationStyle,
};
use crate::tests::platform::error_code_from_runtime_error;

/// Saved clipboard snapshot used to restore host state after one test.
pub(super) struct ClipboardSnapshot {
    /// Current text payload when one exists.
    pub(super) text: Option<String>,
    /// Current html payload when one exists.
    pub(super) html: Option<Vec<u8>>,
}

/// One decoded clipboard item descriptor for tests.
pub(super) struct ClipboardItemDescriptorRecord {
    /// The item presentation style.
    pub(super) presentation_style: ClipboardPresentationStyle,
    /// The item representation descriptors.
    pub(super) representations: Vec<ClipboardItemRepresentationRecord>,
}

/// One decoded clipboard item representation descriptor for tests.
pub(super) struct ClipboardItemRepresentationRecord {
    /// The representation mime type.
    pub(super) mime_type: String,
    /// The representation kind.
    pub(super) kind: ClipboardItemRepresentationKind,
}

/// Build one harness string payload for the active binding lane.
pub(super) fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
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

/// Decode one clipboard text harness value into one owned string.
pub(super) fn decode_clipboard_text(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeStringRef, vm::StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str() }?.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::BindingContext<'_>)
            };
            let value = vm_context
                .string_value(value.value())
                .map_err(|error| RuntimeError::from(error).boxed())?;

            Ok(value)
        }
    }
}

/// Decode one clipboard byte payload into one owned byte vector.
pub(super) fn decode_clipboard_bytes(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
) -> RuntimeResult<Vec<u8>> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_slice() }?.to_vec()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::BindingContext<'_>)
            };

            value.read_bytes(&vm_context.read())
        }
    }
}

/// Decode one clipboard descriptor list into one owned record list.
pub(super) fn decode_clipboard_item_descriptors(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeSlice<ClipboardItemDescriptor>, VmSlice<ClipboardItemDescriptorVm>>,
) -> RuntimeResult<Vec<ClipboardItemDescriptorRecord>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            let mut records = Vec::with_capacity(values.len());

            // decode each native descriptor into one owned record
            for value in values {
                let value =
                    unsafe { <ClipboardItemDescriptor as NativeAbiCodec>::into_value(*value)? };
                records.push(ClipboardItemDescriptorRecord {
                    presentation_style: value.presentation_style,
                    representations: value
                        .representations
                        .into_iter()
                        .map(|representation| ClipboardItemRepresentationRecord {
                            mime_type: representation.mime_type,
                            kind: representation.kind,
                        })
                        .collect(),
                });
            }

            Ok(records)
        }
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::BindingContext<'_>)
            };
            let values = value.read_values(&vm_context.read())?;
            let mut records = Vec::with_capacity(values.len());

            // decode each vm descriptor into one owned record
            for value in values {
                let value = <ClipboardItemDescriptorVm as VmAbiCodec>::into_value(
                    value,
                    &vm_context.read(),
                )?;
                records.push(ClipboardItemDescriptorRecord {
                    presentation_style: value.presentation_style,
                    representations: value
                        .representations
                        .into_iter()
                        .map(|representation| ClipboardItemRepresentationRecord {
                            mime_type: representation.mime_type,
                            kind: representation.kind,
                        })
                        .collect(),
                });
            }

            Ok(records)
        }
    }
}

/// Build one single-item clipboard payload for one string representation.
pub(super) fn string_clipboard_items_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<NativeSlice<ClipboardItem>, VmSlice<ClipboardItemVm>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let item = ClipboardItemValue {
                presentation_style: ClipboardPresentationStyle::Unspecified,
                representations: vec![ClipboardItemRepresentationValue {
                    mime_type: "text/plain".to_string(),
                    kind: ClipboardItemRepresentationKind::String,
                    name: None,
                    text: Some(value.to_string()),
                    path: None,
                    bytes: None,
                }],
            };
            let item = ClipboardItemVm::from_value(&mut vm_context.write(), item)?;
            let items = VmSlice::from_values(&mut vm_context.write(), &[item])?;

            Ok(HarnessValue::Vm(items))
        }
        None => {
            let representation = ClipboardItemRepresentation {
                mime_type: context.call_context.store_string("text/plain"),
                kind: ClipboardItemRepresentationKind::String,
                name: None,
                text: Some(context.call_context.store_string(value)),
                path: None,
                bytes: None,
            };
            let item = ClipboardItem {
                presentation_style: ClipboardPresentationStyle::Unspecified,
                representations: context.call_context.store_slice(vec![representation]),
            };
            let items = context.call_context.store_slice(vec![item]);

            Ok(HarnessValue::Native(items))
        }
    }
}

/// Build one single-item clipboard payload for one html representation.
pub(super) fn html_clipboard_items_value(
    context: &mut HarnessContext<'_>,
    value: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<ClipboardItem>, VmSlice<ClipboardItemVm>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let item = ClipboardItemValue {
                presentation_style: ClipboardPresentationStyle::Unspecified,
                representations: vec![ClipboardItemRepresentationValue {
                    mime_type: "text/html".to_string(),
                    kind: ClipboardItemRepresentationKind::Binary,
                    name: None,
                    text: None,
                    path: None,
                    bytes: Some(value.to_vec()),
                }],
            };
            let item = ClipboardItemVm::from_value(&mut vm_context.write(), item)?;
            let items = VmSlice::from_values(&mut vm_context.write(), &[item])?;

            Ok(HarnessValue::Vm(items))
        }
        None => {
            let representation = ClipboardItemRepresentation {
                mime_type: context.call_context.store_string("text/html"),
                kind: ClipboardItemRepresentationKind::Binary,
                name: None,
                text: None,
                path: None,
                bytes: Some(context.call_context.store_slice(value.to_vec())),
            };
            let item = ClipboardItem {
                presentation_style: ClipboardPresentationStyle::Unspecified,
                representations: context.call_context.store_slice(vec![representation]),
            };
            let items = context.call_context.store_slice(vec![item]);

            Ok(HarnessValue::Native(items))
        }
    }
}

/// Return the first html representation index from one decoded item list.
fn html_representation_index(items: &[ClipboardItemDescriptorRecord]) -> Option<u32> {
    let item = items.first()?;

    item.representations
        .iter()
        .enumerate()
        .find(|(_, representation)| {
            representation.kind == ClipboardItemRepresentationKind::Binary
                && representation.mime_type == "text/html"
        })
        .map(|(index, _)| index as u32)
}

/// Snapshot the clipboard text and html payloads when supported.
pub(super) fn snapshot_clipboard(
    context: &mut HarnessContext<'_>,
) -> RuntimeResult<ClipboardSnapshot> {
    let text = if context.destack_input_clipboard_has_text()? {
        let value = context.destack_input_clipboard_read_text()?;
        Some(decode_clipboard_text(context, value)?)
    } else {
        None
    };

    let html = match context.destack_input_clipboard_list_items() {
        Ok(value) => {
            let items = decode_clipboard_item_descriptors(context, value)?;

            if let Some(index) = html_representation_index(&items) {
                let value = context.destack_input_clipboard_read_item_bytes(0, index)?;
                Some(decode_clipboard_bytes(context, value)?)
            } else {
                None
            }
        }
        Err(error) => {
            let error_code = error_code_from_runtime_error(&error);
            if error_code == Some(PlatformErrorCode::NotSupported)
                || error_code == Some(PlatformErrorCode::IoNotFound)
            {
                None
            } else {
                return Err(error.boxed());
            }
        }
    };

    Ok(ClipboardSnapshot { text, html })
}

/// Restore one prior clipboard snapshot after one destructive test.
pub(super) fn restore_clipboard(
    context: &mut HarnessContext<'_>,
    snapshot: ClipboardSnapshot,
) -> RuntimeResult<()> {
    // prefer restoring html when it was present in the original snapshot
    if let Some(html) = snapshot.html {
        let html = html_clipboard_items_value(context, &html)?;
        return context.destack_input_clipboard_write_items(html);
    }

    // otherwise restore text when it was present
    if let Some(text) = snapshot.text {
        let text = string_clipboard_items_value(context, &text)?;
        return context.destack_input_clipboard_write_items(text);
    }

    context.destack_input_clipboard_clear()
}
