use super::{HostRequestRequirement, request_requirements};
use crate::host::core::HostRequest;
use crate::platform::os::abi_generated::{
    BackgroundConflictPolicyValue, BackgroundNetworkRequirementValue, BackgroundTaskOptionsValue,
    BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue, BackgroundTriggerKindValue,
    DocumentPickOptionsValue,
};
use destack_artifact::Platform;

/// Resolve no declaration requirements for document picker requests.
#[test]
fn test_request_requirements_leave_document_pick_declaration_free() {
    let request = HostRequest::OsDocumentPick {
        options: DocumentPickOptionsValue {
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

/// Resolve one declared task identifier requirement for iOS background registration.
#[test]
fn test_request_requirements_require_ios_background_task_identifier() {
    let request = HostRequest::OsBackgroundRegister {
        options: BackgroundTaskOptionsValue {
            identifier: "sync".to_string(),
            trigger: BackgroundTriggerKindValue::Processing,
            schedule: BackgroundTaskScheduleValue {
                kind: BackgroundTaskScheduleKindValue::Recurring,
                earliest_begin_unix_ns: None,
                repeat_interval_ns: Some(60_000_000_000),
            },
            network: BackgroundNetworkRequirementValue::Connected,
            requires_charging: false,
            requires_idle: false,
            conflict_policy: BackgroundConflictPolicyValue::Replace,
        },
    };

    // mobile background registration requires both execution support and a declared identifier
    let requirements = request_requirements(Platform::IOS, &request);

    assert_eq!(
        requirements,
        vec![
            HostRequestRequirement::BackgroundExecution,
            HostRequestRequirement::BackgroundTaskIdentifier("sync".to_string()),
        ]
    );
}
