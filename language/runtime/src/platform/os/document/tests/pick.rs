use std::sync::Mutex;

use crate::host::app::document::pick::document_descriptor_value_from_path;
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathValue, PathBytesValue};
#[cfg(windows)]
use crate::platform::fs::abi_generated::{OsPathUtf16Value, PathUtf16Value};
#[cfg(windows)]
use crate::tests::platform::assert_not_supported_error;
use crate::tests::platform::error_code_from_result;

use super::common::{
    DocumentPickHookGuard, TEST_DOCUMENT_PICK_MUTEX, captured_document_pick_options,
    decode_document_descriptors, pick_options_harness_value, set_test_pick_hook,
    test_document_pick_success_hook, with_document_harness,
};

/// Verify document pick decodes picker options and descriptor results on both lanes.
#[test]
fn test_document_pick_returns_hooked_descriptors() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_document_harness(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], true, true);
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
    let options = captured_document_pick_options();
    assert_eq!(options.mime_types, Vec::<String>::new());
    assert_eq!(options.extensions, vec!["txt".to_string()]);
    assert!(options.multiple);
    assert!(options.allow_directories);
}

/// Verify picker accepts content-type filters and forwards them unchanged.
#[test]
fn test_document_pick_accepts_mime_type_filters() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(Some(test_document_pick_success_hook));
    let _guard = DocumentPickHookGuard;

    with_document_harness(|mut context| {
        let options = pick_options_harness_value(&mut context, &["text/plain"], &[], false, false);
        let descriptors = context.destack_os_document_pick(options)?;
        let descriptors = decode_document_descriptors(&mut context, descriptors)?;

        assert_eq!(descriptors.len(), 1);
        Ok(())
    });

    let options = captured_document_pick_options();
    assert_eq!(options.mime_types, vec!["text/plain".to_string()]);
    assert_eq!(options.extensions, Vec::<String>::new());
}

/// Verify document pick tests fail closed without one picker hook.
#[test]
fn test_document_pick_requires_test_hook() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_test_pick_hook(None);

    with_document_harness(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &[], false, false);
        let result = context.destack_os_document_pick(options);

        // picker tests must fail closed instead of opening live host ui
        let error_code = error_code_from_result(result)
            .expect("missing picker hook should decode one platform error");

        assert_eq!(
            error_code,
            crate::platform::diagnostic::PlatformErrorCode::Generic
        );

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

    with_document_harness(|mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], false, true);
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
#[test]
fn test_document_descriptor_from_path_reports_local_path() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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
        Some(OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(path.as_os_str().as_encoded_bytes().to_vec()),
        },)),
    );

    #[cfg(windows)]
    assert_eq!(
        descriptor.local_path,
        Some(OsPathValue::OsPathUtf16(OsPathUtf16Value {
            kind: "utf16".to_string(),
            utf16: PathUtf16Value(path.as_os_str().encode_wide().collect::<Vec<_>>()),
        },)),
    );

    std::fs::remove_file(&path).expect("temporary document file should clean up");
}
