use std::sync::Mutex;

use crate::platform::os::DocumentAccess;

use super::common::{
    DocumentPickHookGuard, TEST_DOCUMENT_PICK_MUTEX, configure_document_app_storage_directory,
    decode_document_access_grants, decode_document_descriptors, descriptor_array_harness_value,
    pick_options_harness_value, set_test_document_app_storage_directory,
    set_test_document_source_path, string_array_harness_value, test_document_pick_local_file_hook,
    unique_document_app_storage_directory, with_document_harness_with_options,
};
#[cfg(target_os = "macos")]
use crate::platform::os::document::unix::set_test_pick_hook;
#[cfg(windows)]
use crate::platform::os::document::windows::set_test_pick_hook;

/// Verify persisted access grants are listed and revoked independently from picker selection.
#[test]
fn test_document_access_persist_lists_and_revokes_grants() {
    let _guard = TEST_DOCUMENT_PICK_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time should be after the unix epoch")
        .as_nanos();
    let source_path =
        std::env::temp_dir().join(format!("destack-document-grant-source-{unique_suffix}.txt"));
    let app_storage_directory = unique_document_app_storage_directory();

    // create one real source file so the picker returns one local descriptor
    std::fs::write(&source_path, b"fixture").expect("temporary document source file should write");
    set_test_pick_hook(Some(test_document_pick_local_file_hook));
    set_test_document_app_storage_directory(app_storage_directory.clone());
    set_test_document_source_path(source_path.clone());
    let _guard = DocumentPickHookGuard;

    with_document_harness_with_options(configure_document_app_storage_directory, |mut context| {
        let options = pick_options_harness_value(&mut context, &[], &["txt"], false, false);
        let descriptors = context.destack_os_document_pick(options)?;
        let descriptors = decode_document_descriptors(&mut context, descriptors)?;
        let documents = descriptor_array_harness_value(&mut context, descriptors.clone())?;
        let grants = context.destack_os_document_access_persist(documents, DocumentAccess::Read)?;
        let grants = decode_document_access_grants(&mut context, grants)?;

        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0].document.uri,
            crate::platform::core::file_uri_from_path(&source_path)
        );
        assert_eq!(grants[0].access, DocumentAccess::Read);

        let listed = context.destack_os_document_access_list()?;
        let listed = decode_document_access_grants(&mut context, listed)?;

        assert_eq!(listed, grants);

        let ids = string_array_harness_value(
            &mut context,
            &grants
                .iter()
                .map(|grant| grant.id.clone())
                .collect::<Vec<_>>(),
        )?;
        let revoked = context.destack_os_document_access_revoke(ids)?;

        assert_eq!(revoked, 1);

        let listed = context.destack_os_document_access_list()?;
        let listed = decode_document_access_grants(&mut context, listed)?;

        assert!(listed.is_empty());

        Ok(())
    });

    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_dir_all(&app_storage_directory);
}
