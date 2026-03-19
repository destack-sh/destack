use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::DocumentAccess;

use super::common::{
    DocumentPickHookGuard, TEST_DOCUMENT_PICK_MUTEX, bytes_harness_value,
    configure_document_app_storage_directory, decode_document_access_grants, decode_document_bytes,
    decode_document_descriptors, descriptor_array_harness_value, pick_options_harness_value,
    set_test_document_app_storage_directory, set_test_document_source_path, set_test_pick_hook,
    string_harness_value, test_document_pick_local_file_hook,
    unique_document_app_storage_directory, with_document_harness,
    with_document_harness_with_options,
};

/// Verify local document URIs open as real byte-stream document handles.
#[test]
fn test_document_open_reads_and_closes_local_file_uris() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time should be after the unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("destack-document-open-{unique_suffix}.txt"));

    // create one temporary file so the handle lane opens real host bytes
    std::fs::write(&path, b"fixture bytes").expect("temporary document file should write");

    with_document_harness(|mut context| {
        let uri = string_harness_value(
            &mut context,
            &crate::platform::core::file_uri_from_path(&path),
        );
        let handle = context.destack_os_document_open(uri, DocumentAccess::Read)?;
        let bytes = context.destack_os_document_read(handle, 64, 0)?;
        let bytes = decode_document_bytes(&mut context, bytes)?;

        // exact read payload
        assert_eq!(bytes, b"fixture bytes");

        context.destack_os_document_close(handle)?;

        // closed handles must fail loudly instead of reading stale payload
        let error = match context.destack_os_document_read(handle, 64, 0) {
            Ok(_) => panic!("documentRead should fail for one closed document handle"),
            Err(error) => error,
        };
        assert_eq!(
            crate::tests::platform::error_code_from_runtime_error(&error),
            Some(PlatformErrorCode::InvalidArgumentValue),
        );

        Ok(())
    });

    std::fs::remove_file(&path).expect("temporary document file should clean up");
}

/// Verify persisted document-access grants reopen external files for later write access.
#[test]
fn test_document_access_open_reopens_persisted_local_grants() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time should be after the unix epoch")
        .as_nanos();
    let source_path =
        std::env::temp_dir().join(format!("destack-document-access-open-{unique_suffix}.txt"));
    let app_storage_directory = unique_document_app_storage_directory();

    // create one real local file so the persisted grant reopens host bytes later
    std::fs::write(&source_path, b"alpha").expect("temporary document source file should write");
    set_test_pick_hook(Some(test_document_pick_local_file_hook));
    set_test_document_app_storage_directory(app_storage_directory.clone());
    set_test_document_source_path(source_path.clone());
    let _guard = DocumentPickHookGuard;

    with_document_harness_with_options(configure_document_app_storage_directory, |mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], false, false);
        let descriptors = context.destack_os_document_pick(options)?;
        let descriptors = decode_document_descriptors(&mut context, descriptors)?;
        let documents = descriptor_array_harness_value(&mut context, descriptors)?;
        let grants =
            context.destack_os_document_access_persist(documents, DocumentAccess::ReadWrite)?;
        let grants = decode_document_access_grants(&mut context, grants)?;

        assert_eq!(grants.len(), 1);

        let id = string_harness_value(&mut context, &grants[0].id);
        let handle = context.destack_os_document_access_open(id, DocumentAccess::ReadWrite)?;
        let payload = bytes_harness_value(&mut context, b"beta")?;
        let written = context.destack_os_document_write(handle, payload, 0)?;

        assert_eq!(written, 4);

        context.destack_os_document_flush(handle)?;
        context.destack_os_document_close(handle)?;

        let uri = string_harness_value(&mut context, &grants[0].document.uri);
        let handle = context.destack_os_document_open(uri, DocumentAccess::Read)?;
        let bytes = context.destack_os_document_read(handle, 64, 0)?;
        let bytes = decode_document_bytes(&mut context, bytes)?;

        assert_eq!(bytes, b"betaa");

        context.destack_os_document_close(handle)?;

        Ok(())
    });

    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_dir_all(&app_storage_directory);
}
