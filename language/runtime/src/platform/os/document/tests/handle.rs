use crate::platform::core::file_uri_from_path;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::DocumentAccess;
use crate::tests::platform::error_code_from_runtime_error;

use super::common::{decode_document_bytes, string_harness_value, with_document_harness};

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
        let uri = string_harness_value(&mut context, &file_uri_from_path(&path));
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
            error_code_from_runtime_error(&error),
            Some(PlatformErrorCode::InvalidArgumentValue),
        );

        Ok(())
    });

    std::fs::remove_file(&path).expect("temporary document file should clean up");
}
