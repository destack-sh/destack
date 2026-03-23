use destack_artifact::Platform;
use destack_workspace::RuntimeAppPermission;

use crate::host::core::HostRequest;

use super::core::{runtime_app_permission, url_scheme};

/// Declaration requirement for one outbound host request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostRequestRequirement {
    /// Require declared background execution support.
    BackgroundExecution,
    /// Require one declared app permission.
    Permission(RuntimeAppPermission),
    /// Require one declared notification authorization lane.
    NotificationAuthorization,
    /// Require one declared outbound URL query scheme.
    IntentQueryScheme(String),
    /// Require declared outbound file sharing support.
    IntentShareFiles,
}

/// Resolve declaration requirements for one outbound host request.
///
/// A request is declaration-bearing only when established host prior art
/// requires app-package metadata before the operation is legal or meaningful.
pub(crate) fn request_requirements(
    platform: Platform,
    request: &HostRequest,
) -> Vec<HostRequestRequirement> {
    match request {
        HostRequest::OsBackgroundStatus
        | HostRequest::OsBackgroundList
        | HostRequest::OsBackgroundRegister { .. }
        | HostRequest::OsBackgroundUnregister { .. }
        | HostRequest::OsBackgroundTriggerTest { .. }
        | HostRequest::OsBackgroundComplete { .. } => {
            vec![HostRequestRequirement::BackgroundExecution]
        }
        HostRequest::OsCalendarList
        | HostRequest::OsCalendarEventList { .. }
        | HostRequest::OsCalendarEventRead { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::CalendarRead,
            )]
        }
        HostRequest::OsCalendarEventCreate { .. }
        | HostRequest::OsCalendarEventUpdate { .. }
        | HostRequest::OsCalendarEventDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::CalendarWrite,
            )]
        }
        HostRequest::OsContactList { .. }
        | HostRequest::OsContactSearch { .. }
        | HostRequest::OsContactRead { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::ContactsRead,
            )]
        }
        HostRequest::OsContactCreate { .. }
        | HostRequest::OsContactUpdate { .. }
        | HostRequest::OsContactDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::ContactsWrite,
            )]
        }
        HostRequest::OsIntentCanOpenUrl { url } => {
            let Some(scheme) = url_scheme(url) else {
                return Vec::new();
            };

            if !matches!(platform, Platform::Android | Platform::IOS) {
                return Vec::new();
            }

            vec![HostRequestRequirement::IntentQueryScheme(scheme)]
        }
        HostRequest::OsPermissionRequest { permission } => {
            vec![HostRequestRequirement::Permission(runtime_app_permission(
                *permission,
            ))]
        }
        HostRequest::OsPermissionRequestMany { permissions } => permissions
            .iter()
            .map(|permission| {
                HostRequestRequirement::Permission(runtime_app_permission(*permission))
            })
            .collect(),
        HostRequest::OsNotificationRequestPermission
        | HostRequest::OsNotificationCancel { .. }
        | HostRequest::OsNotificationCancelAll
        | HostRequest::OsNotificationCategoryList
        | HostRequest::OsNotificationCategorySet { .. }
        | HostRequest::OsNotificationPendingList
        | HostRequest::OsNotificationPendingCancel { .. }
        | HostRequest::OsNotificationPendingCancelAll
        | HostRequest::OsNotificationPost { .. }
        | HostRequest::OsNotificationSchedule { .. } => {
            vec![HostRequestRequirement::NotificationAuthorization]
        }
        HostRequest::OsIntentSharePaths { .. } if matches!(platform, Platform::Android) => {
            vec![HostRequestRequirement::IntentShareFiles]
        }
        HostRequest::OsIntentOpenUrl { .. }
        | HostRequest::OsIntentOpenPath { .. }
        | HostRequest::OsIntentShareText { .. }
        | HostRequest::OsIntentSharePaths { .. }
        | HostRequest::OsDocumentPick { .. }
        | HostRequest::OsPermissionOpenSettings => Vec::new(),
        HostRequest::OsLocationServicesEnabled
        | HostRequest::OsLocationLastKnown
        | HostRequest::OsLocationWatchOpen { .. }
        | HostRequest::OsLocationWatchClose { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::Location,
            )]
        }
        HostRequest::OsMediaList { .. } | HostRequest::OsMediaRead { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::MediaRead,
            )]
        }
        HostRequest::OsMediaImportPath { .. } | HostRequest::OsMediaDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::MediaWrite,
            )]
        }
    }
}
