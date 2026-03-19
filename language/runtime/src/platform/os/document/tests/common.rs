use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_vm as vm;
use destack_workspace::RuntimeOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::app::document::pick::document_descriptor_value_from_path;
#[cfg(target_os = "macos")]
use crate::host::macos::set_macos_document_test_pick_hook;
#[cfg(windows)]
use crate::host::windows::set_windows_document_test_pick_hook;
use crate::platform::core::{NativeAbiCodec, VmAbiCodec};
#[cfg(not(any(unix, windows)))]
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
#[cfg(unix)]
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
#[cfg(windows)]
use crate::platform::fs::abi_generated::{OsPathUtf16Value, OsPathValue, PathUtf16Value};
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::tests::{
    HarnessContext, HarnessValue, with_configured_harness_context, with_harness_context,
};
use crate::platform::os::{
    DocumentAccessGrant, DocumentAccessGrantVm, DocumentDescriptor, DocumentDescriptorVm,
    DocumentPickOptions, DocumentPickOptionsVm,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, VmArray, VmSlice};

/// Shared picker-test mutex that serializes the process-global picker hook.
pub(super) static TEST_DOCUMENT_PICK_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Shared captured picker options from the active test hook.
static TEST_DOCUMENT_PICK_OPTIONS: OnceLock<Mutex<Option<DocumentPickOptionsValue>>> =
    OnceLock::new();
/// Shared app-storage directory override for picker tests.
static TEST_DOCUMENT_DATA_DIRECTORY: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
/// Shared source file override for app-storage import picker tests.
static TEST_DOCUMENT_SOURCE_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

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

        let mut directory = TEST_DOCUMENT_DATA_DIRECTORY
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *directory = None;

        let mut source_path = TEST_DOCUMENT_SOURCE_PATH
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *source_path = None;
    }
}

/// Build one pick-options payload for the active lane.
pub(super) fn pick_options_harness_value(
    context: &mut HarnessContext<'_>,
    content_types: &[&str],
    extensions: &[&str],
    multiple: bool,
    allow_directories: bool,
) -> HarnessValue<DocumentPickOptions, DocumentPickOptionsVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let content_types = content_types
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
            let content_types = VmArray::from_values(vm_context, &content_types)
                .expect("document picker content type array should encode");
            let extensions = VmArray::from_values(vm_context, &extensions)
                .expect("document picker extension array should encode");

            HarnessValue::Vm(DocumentPickOptionsVm {
                content_types,
                extensions,
                multiple,
                allow_directories,
            })
        }
        None => HarnessValue::Native(DocumentPickOptions {
            content_types: context.call_context.store_array(
                content_types
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
        content_type: None,
        size_bytes: Some(12),
        modified_unix_ns: Some(55),
        is_directory: false,
        local_path: None,
    }])
}

/// Report one successful picker result backed by one real local file.
pub(super) fn test_document_pick_local_file_hook(
    options: DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let mut slot = TEST_DOCUMENT_PICK_OPTIONS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *slot = Some(options);

    let source_path = TEST_DOCUMENT_SOURCE_PATH
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .expect("app-storage import picker hook should receive one source file path");

    Ok(vec![document_descriptor_value_from_path(&source_path)?])
}

/// Encode one local host path into one runtime path value.
pub(super) fn os_path_value_from_path(path: &Path) -> OsPathValue {
    #[cfg(unix)]
    {
        let bytes = path.as_os_str().as_encoded_bytes().to_vec();

        OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(bytes),
        })
    }

    #[cfg(windows)]
    {
        let utf16 = path.as_os_str().encode_wide().collect::<Vec<_>>();

        return OsPathValue::OsPathUtf16(OsPathUtf16Value {
            kind: "utf16".to_string(),
            utf16: PathUtf16Value(utf16),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = path.to_string_lossy().into_owned().into_bytes();

        OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(bytes),
        })
    }
}

/// Decode one runtime OS path value into one host path buffer.
pub(super) fn path_buf_from_os_path_value(path: &OsPathValue) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        match path {
            OsPathValue::OsPathBytes(value) => {
                PathBuf::from(OsString::from_vec(value.bytes.0.clone()))
            }
            OsPathValue::OsPathUtf16(_) => {
                panic!("unix picker app-storage path should not be utf16")
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;

        match path {
            OsPathValue::OsPathUtf16(value) => PathBuf::from(OsString::from_wide(&value.utf16.0)),
            OsPathValue::OsPathBytes(_) => {
                panic!("windows picker app-storage path should not be bytes")
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        match path {
            OsPathValue::OsPathBytes(value) => {
                PathBuf::from(String::from_utf8_lossy(&value.bytes.0).to_string())
            }
            OsPathValue::OsPathUtf16(value) => {
                PathBuf::from(String::from_utf16_lossy(&value.utf16.0))
            }
        }
    }
}

/// Install one app-storage directory override for picker tests.
pub(super) fn configure_document_app_storage_directory(options: &mut RuntimeOptions) {
    let directory = TEST_DOCUMENT_DATA_DIRECTORY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();

    options.os.data_directory = directory;
}

/// Return one unique picker app-storage directory for the current test case.
pub(super) fn unique_document_app_storage_directory() -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after the unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("destack-document-app-storage-{unique_suffix}"))
}

/// Install one temporary picker app-storage directory override.
pub(super) fn set_test_document_app_storage_directory(directory: PathBuf) {
    let mut slot = TEST_DOCUMENT_DATA_DIRECTORY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = Some(directory);
}

/// Install one temporary picker source-file override.
pub(super) fn set_test_document_source_path(path: PathBuf) {
    let mut slot = TEST_DOCUMENT_SOURCE_PATH
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = Some(path);
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
                .map(|value| unsafe {
                    <DocumentDescriptor as crate::platform::core::NativeAbiCodec>::into_value(value)
                })
                .collect::<RuntimeResult<Vec<_>>>()?;

            Ok(descriptors)
        }
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::ExternalCallContext<'_>)
            };
            let descriptors = value.read_values(vm_context)?;
            let mut values = Vec::with_capacity(descriptors.len());

            // decode one vm descriptor per picker result
            for descriptor in descriptors {
                let descriptor =
                    <DocumentDescriptorVm as crate::platform::core::VmAbiCodec>::into_value(
                        descriptor, vm_context,
                    )?;
                values.push(descriptor);
            }

            Ok(values)
        }
    }
}

/// Decode one persisted document-access grant array.
pub(super) fn decode_document_access_grants(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeArray<DocumentAccessGrant>, VmArray<DocumentAccessGrantVm>>,
) -> RuntimeResult<Vec<crate::platform::os::abi_generated::DocumentAccessGrantValue>> {
    match value {
        HarnessValue::Native(value) => {
            let grants = unsafe { value.as_slice() }?
                .iter()
                .copied()
                .map(|value| unsafe {
                    <DocumentAccessGrant as crate::platform::core::NativeAbiCodec>::into_value(
                        value,
                    )
                })
                .collect::<RuntimeResult<Vec<_>>>()?;

            Ok(grants)
        }
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::ExternalCallContext<'_>)
            };
            let grants = value.read_values(vm_context)?;
            let mut values = Vec::with_capacity(grants.len());

            // decode one vm grant per persisted access result
            for grant in grants {
                let grant =
                    <DocumentAccessGrantVm as crate::platform::core::VmAbiCodec>::into_value(
                        grant, vm_context,
                    )?;
                values.push(grant);
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
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

/// Build one harness byte payload for the active document lane.
pub(super) fn bytes_harness_value(
    context: &mut HarnessContext<'_>,
    value: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let value =
                VmSlice::from_bytes(vm_context, value).expect("vm document bytes should encode");

            Ok(HarnessValue::Vm(value))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_slice(value.to_vec()),
        )),
    }
}

/// Build one descriptor-array payload for the active document lane.
pub(super) fn descriptor_array_harness_value(
    context: &mut HarnessContext<'_>,
    descriptors: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<HarnessValue<NativeArray<DocumentDescriptor>, VmArray<DocumentDescriptorVm>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let mut values = Vec::with_capacity(descriptors.len());

            // encode one vm descriptor per document descriptor value
            for descriptor in descriptors {
                values.push(DocumentDescriptorVm::from_value(vm_context, descriptor)?);
            }

            Ok(HarnessValue::Vm(VmArray::from_values(vm_context, &values)?))
        }
        None => {
            let values = descriptors
                .into_iter()
                .map(|value| DocumentDescriptor::from_value(&context.call_context, value))
                .collect::<Vec<_>>();

            Ok(HarnessValue::Native(
                context.call_context.store_array(values),
            ))
        }
    }
}

/// Build one string-array payload for the active document lane.
pub(super) fn string_array_harness_value(
    context: &mut HarnessContext<'_>,
    values: &[String],
) -> RuntimeResult<HarnessValue<NativeArray<NativeStringRef>, VmArray<vm::StringHandle>>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let handles = values
                .iter()
                .map(|value| {
                    vm::StringHandle::new(
                        vm_context
                            .intern_string(value)
                            .expect("document grant id should intern"),
                    )
                })
                .collect::<Vec<_>>();

            Ok(HarnessValue::Vm(VmArray::from_values(
                vm_context, &handles,
            )?))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_array(
                values
                    .iter()
                    .map(|value| context.call_context.store_string(value))
                    .collect::<Vec<_>>(),
            ),
        )),
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
                    as *mut vm::ExternalCallContext<'_>)
            };

            value.read_bytes(vm_context)
        }
    }
}

/// Run one document harness callback on both native and VM lanes.
pub(super) fn with_document_harness(
    callback: impl FnMut(HarnessContext<'_>) -> RuntimeResult<()> + Copy,
) {
    with_harness_context(callback);
}

/// Run one document harness callback with runtime options on both native and VM lanes.
pub(super) fn with_document_harness_with_options(
    configure: fn(&mut RuntimeOptions),
    callback: impl FnMut(HarnessContext<'_>) -> RuntimeResult<()> + Copy,
) {
    with_configured_harness_context(configure, callback);
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
