use std::sync::{Mutex, OnceLock};

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::host::os::macos::request::document::set_macos_document_test_pick_hook;
#[cfg(windows)]
use crate::host::os::windows::request::document::set_windows_document_test_pick_hook;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_harness_context};
use crate::platform::os::{
    DocumentDescriptor, DocumentDescriptorVm, DocumentPickOptions, DocumentPickOptionsVm,
};
use crate::platform::{NativeAbiCodec, NativeArray, VmAbiCodec, VmArray, VmSlice};

/// Shared picker-test mutex that serializes the process-global picker hook.
pub(super) static TEST_DOCUMENT_PICK_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Shared captured picker options from the active test hook.
static TEST_DOCUMENT_PICK_OPTIONS: OnceLock<Mutex<Option<DocumentPickOptionsValue>>> =
    OnceLock::new();
/// Install one test picker hook for the active host lane.
pub(super) fn set_test_pick_hook(
    hook: Option<fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>>,
) {
    #[cfg(target_os = "macos")]
    {
        set_macos_document_test_pick_hook(hook);
    }

    #[cfg(windows)]
    {
        set_windows_document_test_pick_hook(hook);
    }
}

/// Reset the installed picker hook after one test case.
pub(super) struct DocumentPickHookGuard;

impl Drop for DocumentPickHookGuard {
    /// Clear the active picker hook for the current backend.
    fn drop(&mut self) {
        set_test_pick_hook(None);

        let mut slot = TEST_DOCUMENT_PICK_OPTIONS
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *slot = None;
    }
}

/// Build one pick-options payload for the active lane.
pub(super) fn pick_options_harness_value(
    context: &mut HarnessContext<'_>,
    mime_types: &[&str],
    extensions: &[&str],
    multiple: bool,
    allow_directories: bool,
) -> HarnessValue<DocumentPickOptions, DocumentPickOptionsVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let mime_types = mime_types
                .iter()
                .map(|value| {
                    vm::StringHandle::new(
                        vm_context
                            .intern_string(value)
                            .expect("vm document picker content type should intern"),
                    )
                })
                .collect::<Vec<_>>();
            let extensions = extensions
                .iter()
                .map(|value| {
                    vm::StringHandle::new(
                        vm_context
                            .intern_string(value)
                            .expect("vm document picker extension should intern"),
                    )
                })
                .collect::<Vec<_>>();
            let mime_types = VmArray::from_values(&mut vm_context.write(), &mime_types)
                .expect("document picker mime type array should encode");
            let extensions = VmArray::from_values(&mut vm_context.write(), &extensions)
                .expect("document picker extension array should encode");

            HarnessValue::Vm(DocumentPickOptionsVm {
                mime_types,
                extensions,
                multiple,
                allow_directories,
                copy_to_sandbox: false,
            })
        }
        None => HarnessValue::Native(DocumentPickOptions {
            mime_types: context.call_context.store_array(
                mime_types
                    .iter()
                    .map(|value| context.call_context.store_string(value))
                    .collect::<Vec<_>>(),
            ),
            extensions: context.call_context.store_array(
                extensions
                    .iter()
                    .map(|value| context.call_context.store_string(value))
                    .collect::<Vec<_>>(),
            ),
            multiple,
            allow_directories,
            copy_to_sandbox: false,
        }),
    }
}

/// Report one successful picker result without opening UI.
pub(super) fn test_document_pick_success_hook(
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let mut slot = TEST_DOCUMENT_PICK_OPTIONS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *slot = Some(options);

    Ok(vec![DocumentDescriptorValue {
        uri: "file:///tmp/fixture.txt".to_string(),
        name: "fixture.txt".to_string(),
        mime_type: None,
        size_bytes: Some(12),
        modified_unix_ns: Some(55),
        is_directory: false,
        local_path: None,
    }])
}

/// Decode one picked document descriptor array.
pub(super) fn decode_document_descriptors(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeArray<DocumentDescriptor>, VmArray<DocumentDescriptorVm>>,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    match value {
        HarnessValue::Native(value) => {
            let descriptors = unsafe { value.as_slice() }?
                .iter()
                .copied()
                .map(|value| unsafe { <DocumentDescriptor as NativeAbiCodec>::into_value(value) })
                .collect::<RuntimeResult<Vec<_>>>()?;

            Ok(descriptors)
        }
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::BindingContext<'_>)
            };
            let descriptors = value.read_values(&vm_context.read())?;
            let mut values = Vec::with_capacity(descriptors.len());

            // decode one vm descriptor per picker result
            for descriptor in descriptors {
                let descriptor = <DocumentDescriptorVm as VmAbiCodec>::into_value(
                    descriptor,
                    &vm_context.read(),
                )?;
                values.push(descriptor);
            }

            Ok(values)
        }
    }
}

/// Build one harness string payload for the active document lane.
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
                    .expect("vm document test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Decode one document byte payload into one owned byte vector.
pub(super) fn decode_document_bytes(
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

/// Run one document harness callback on both native and VM lanes.
pub(super) fn with_document_harness(
    callback: impl FnMut(HarnessContext<'_>) -> RuntimeResult<()> + Copy,
) {
    with_harness_context(callback);
}

/// Return the last picker options payload captured by the active test hook.
pub(super) fn captured_document_pick_options() -> DocumentPickOptionsValue {
    TEST_DOCUMENT_PICK_OPTIONS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .expect("document picker hook should capture one options payload")
}
