use super::{HostRequestRequirement, request_requirements};
use crate::host::core::HostRequest;
use destack_artifact::Platform;

/// Resolve no declaration requirements for document picker requests.
#[test]
fn test_request_requirements_leave_document_pick_declaration_free() {
    let request = HostRequest::OsDocumentPick {
        options: crate::platform::os::abi_generated::DocumentPickOptionsValue {
            mime_types: Vec::new(),
            extensions: Vec::new(),
            multiple: false,
            allow_directories: false,
            copy_to_sandbox: false,
        },
    };

    // document picker import is declaration-free
    let requirements = request_requirements(Platform::MacOS, &request);

    assert_eq!(requirements, Vec::new());
}

/// Resolve one query-scheme declaration for iOS can-open-url requests.
#[test]
fn test_request_requirements_require_ios_query_scheme_for_can_open_url() {
    let request = HostRequest::OsIntentCanOpenUrl {
        url: "mailto:test@example.com".to_string(),
    };

    // mobile query declaration
    let requirements = request_requirements(Platform::IOS, &request);

    assert_eq!(
        requirements,
        vec![HostRequestRequirement::IntentQueryScheme(
            "mailto".to_string()
        )]
    );
}

/// Resolve one declaration requirement for Android file sharing.
#[test]
fn test_request_requirements_require_android_file_share_declaration() {
    let request = HostRequest::OsIntentSharePaths {
        paths: Vec::new(),
        content_type: Some("text/plain".to_string()),
    };

    // android file sharing requires declared app integration
    let requirements = request_requirements(Platform::Android, &request);

    assert_eq!(requirements, vec![HostRequestRequirement::IntentShareFiles]);
}

/// Resolve no declaration requirements for non-Android file sharing.
#[test]
fn test_request_requirements_leave_non_android_file_share_declaration_free() {
    let request = HostRequest::OsIntentSharePaths {
        paths: Vec::new(),
        content_type: Some("text/plain".to_string()),
    };

    // desktop file sharing stays declaration-free for now
    let requirements = request_requirements(Platform::Windows, &request);

    assert_eq!(requirements, Vec::new());
}
