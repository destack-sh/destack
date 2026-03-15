#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(any(target_os = "macos", windows))]
use std::sync::{Mutex, OnceLock};
#[cfg(any(target_os = "macos", windows))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(target_os = "macos", windows))]
use destack_vm as vm;

#[cfg(any(target_os = "macos", windows))]
use crate::diagnostic::RuntimeResult;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::NativeArray;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::VmArray;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::fs::abi_generated::OsPathValue;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::abi_generated::DocumentDescriptorValue;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::abi_generated::DocumentPickOptionsValue;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::document::core::document_descriptor_value_from_path;
#[cfg(target_os = "macos")]
use crate::platform::os::document::unix::set_test_pick_hook;
#[cfg(windows)]
use crate::platform::os::document::windows::set_test_pick_hook;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::tests::with_harness_context;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::tests::{HarnessValue, OsHarnessContext};
#[cfg(any(target_os = "macos", windows))]
use crate::platform::os::{DocumentPickOptions, DocumentPickOptionsVm};
#[cfg(any(target_os = "macos", windows))]
use crate::tests::platform::assert_not_supported_error;
#[cfg(any(target_os = "macos", windows))]
use crate::tests::platform::error_code_from_result;

/// Shared picker-test mutex that serializes the process-global picker hook.
#[cfg(any(target_os = "macos", windows))]
static TEST_DOCUMENT_PICK_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Shared captured picker options from the active test hook.
#[cfg(any(target_os = "macos", windows))]
static TEST_DOCUMENT_PICK_OPTIONS: OnceLock<Mutex<Option<DocumentPickOptionsValue>>> =
    OnceLock::new();

/// Reset the installed picker hook after one test case.
#[cfg(any(target_os = "macos", windows))]
struct DocumentPickHookGuard;

#[cfg(any(target_os = "macos", windows))]
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
#[cfg(any(target_os = "macos", windows))]
fn pick_options_harness_value(
    context: &mut OsHarnessContext<'_>,
    mime_types: &[&str],
    extensions: &[&str],
    multiple: bool,
    allow_directories: bool,
    copy_to_sandbox: bool,
) -> HarnessValue<DocumentPickOptions, DocumentPickOptionsVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let mime_types = mime_types
                .iter()
                .map(|value| {
                    vm::StringHandle::new(
                        vm_context
                            .intern_string(value)
                            .expect("vm document picker mime type should intern"),
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
            let mime_types = VmArray::from_values(vm_context, &mime_types)
                .expect("document picker mime type array should encode");
            let extensions = VmArray::from_values(vm_context, &extensions)
                .expect("document picker extension array should encode");

            HarnessValue::Vm(DocumentPickOptionsVm {
                mime_types,
                extensions,
                multiple,
                allow_directories,
                copy_to_sandbox,
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
            copy_to_sandbox,
        }),
    }
}

/// Report one successful picker result without opening UI.
#[cfg(any(target_os = "macos", windows))]
fn test_document_pick_success_hook(
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
#[cfg(any(target_os = "macos", windows))]
fn decode_document_descriptors(
    context: &mut OsHarnessContext<'_>,
    value: HarnessValue<
        NativeArray<crate::platform::os::DocumentDescriptor>,
        VmArray<crate::platform::os::DocumentDescriptorVm>,
    >,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    match value {
        HarnessValue::Native(value) => {
            let descriptors = unsafe { value.as_slice() }?
                .iter()
                .copied()
                .map(|value| unsafe {
                    <crate::platform::os::DocumentDescriptor as crate::platform::core::NativeAbiCodec>::into_value(value)
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
                    <crate::platform::os::DocumentDescriptorVm as crate::platform::core::VmAbiCodec>::into_value(
                        descriptor,
                        vm_context,
                    )?;
                values.push(descriptor);
            }

            Ok(values)
        }
    }
}

/// Verify document pick decodes picker options and descriptor results on both lanes.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_document_pick_returns_hooked_descriptors() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_harness_context(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], true, true, false);
        let descriptors = context.destack_os_document_pick(options)?;
        let descriptors = decode_document_descriptors(&mut context, descriptors)?;

        // exact picker output
        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].name, "fixture.txt");
        assert_eq!(descriptors[0].uri, "file:///tmp/fixture.txt");
        assert_eq!(descriptors[0].mime_type, None);
        assert_eq!(descriptors[0].size_bytes, Some(12));
        assert_eq!(descriptors[0].modified_unix_ns, Some(55));
        assert!(!descriptors[0].is_directory);
        assert!(descriptors[0].local_path.is_none());

        Ok(())
    });

    // exact forwarded picker options
    let options = TEST_DOCUMENT_PICK_OPTIONS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .expect("document picker hook should capture one options payload");
    assert_eq!(options.mime_types, Vec::<String>::new());
    assert_eq!(options.extensions, vec!["txt".to_string()]);
    assert!(options.multiple);
    assert!(options.allow_directories);
    assert!(!options.copy_to_sandbox);
}

/// Verify picker rejects unsupported MIME-type filters.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_document_pick_rejects_unsupported_mime_filters() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_harness_context(|mut context| {
        let options =
            pick_options_harness_value(&mut context, &["text/plain"], &[], false, false, false);
        let error = match context.destack_os_document_pick(options) {
            Ok(_) => panic!("documentPick should report notSupported for MIME filters"),
            Err(error) => error,
        };

        assert_not_supported_error(&error);
        Ok(())
    });
}

/// Verify picker rejects unsupported sandbox-copy requests.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_document_pick_rejects_unsupported_sandbox_copy() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_harness_context(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], false, false, true);
        let error = match context.destack_os_document_pick(options) {
            Ok(_) => panic!("documentPick should report notSupported for sandbox copies"),
            Err(error) => error,
        };

        assert_not_supported_error(&error);
        Ok(())
    });
}

/// Verify document pick tests fail closed without one picker hook.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_document_pick_requires_test_hook() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(None);

    with_harness_context(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &[], false, false, false);
        let result = context.destack_os_document_pick(options);

        // picker tests must fail closed instead of opening live host ui
        let error_code = error_code_from_result(result)
            .expect("missing picker hook should decode one platform error");

        assert_eq!(error_code, PlatformErrorCode::Generic);

        Ok(())
    });
}

/// Verify the Windows picker rejects mixed directory and extension filters.
#[cfg(windows)]
#[test]
fn test_document_pick_rejects_directory_mode_with_extension_filters() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_harness_context(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], false, true, false);
        let error = match context.destack_os_document_pick(options) {
            Ok(_) => {
                panic!("documentPick should report notSupported for mixed folder and file filters")
            }
            Err(error) => error,
        };

        assert_not_supported_error(&error);
        Ok(())
    });
}

/// Verify document descriptors retain one direct local-path payload for file-backed picks.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_document_descriptor_from_path_reports_local_path() {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after the unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("destack-document-{unique_suffix}.txt"));

    // create one temporary file for descriptor extraction
    std::fs::write(&path, b"fixture").expect("temporary document file should write");

    let descriptor = document_descriptor_value_from_path(&path)
        .expect("document descriptor should build from one real path");

    // keep the descriptor exact for current direct-path picker backends
    assert_eq!(descriptor.name, path.file_name().unwrap().to_string_lossy());
    assert!(!descriptor.is_directory);
    assert_eq!(descriptor.size_bytes, Some(7));

    #[cfg(unix)]
    assert_eq!(
        descriptor.local_path,
        Some(OsPathValue::OsPathBytes(
            crate::platform::fs::abi_generated::OsPathBytesValue {
                kind: "bytes".to_string(),
                bytes: crate::platform::fs::abi_generated::PathBytesValue(
                    path.as_os_str().as_encoded_bytes().to_vec(),
                ),
            },
        )),
    );

    #[cfg(windows)]
    assert_eq!(
        descriptor.local_path,
        Some(OsPathValue::OsPathUtf16(
            crate::platform::fs::abi_generated::OsPathUtf16Value {
                kind: "utf16".to_string(),
                utf16: crate::platform::fs::abi_generated::PathUtf16Value(
                    path.as_os_str().encode_wide().collect::<Vec<_>>(),
                ),
            },
        )),
    );

    std::fs::remove_file(&path).expect("temporary document file should clean up");
}
