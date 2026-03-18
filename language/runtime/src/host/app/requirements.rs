use destack_workspace::{Platform, RuntimeAppDeclaration, RuntimeAppPermission};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::request::HostRequest;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::Permission;

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

/// Require that one host request is declared for the active target app.
pub(crate) fn require_declared_request(
    platform: Platform,
    app: &RuntimeAppDeclaration,
    request: &HostRequest,
) -> RuntimeResult<()> {
    // request requirements
    let requirements = request_requirements(platform, request);

    // enforce one resolved declaration requirement at a time
    for requirement in &requirements {
        require_request_requirement(app, request.operation_name(), requirement)?;
    }

    Ok(())
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
        // background scheduler operations
        HostRequest::OsBackgroundStatus
        | HostRequest::OsBackgroundList
        | HostRequest::OsBackgroundRegister { .. }
        | HostRequest::OsBackgroundUnregister { .. }
        | HostRequest::OsBackgroundTriggerTest { .. }
        | HostRequest::OsBackgroundComplete { .. } => {
            vec![HostRequestRequirement::BackgroundExecution]
        }

        // calendar read lanes
        HostRequest::OsCalendarList
        | HostRequest::OsCalendarEventList { .. }
        | HostRequest::OsCalendarEventRead { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::CalendarRead,
            )]
        }

        // calendar write lanes
        HostRequest::OsCalendarEventCreate { .. }
        | HostRequest::OsCalendarEventUpdate { .. }
        | HostRequest::OsCalendarEventDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::CalendarWrite,
            )]
        }

        // contact read lanes
        HostRequest::OsContactList { .. }
        | HostRequest::OsContactSearch { .. }
        | HostRequest::OsContactRead { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::ContactsRead,
            )]
        }

        // contact write lanes
        HostRequest::OsContactCreate { .. }
        | HostRequest::OsContactUpdate { .. }
        | HostRequest::OsContactDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::ContactsWrite,
            )]
        }

        // mobile route discovery
        HostRequest::OsIntentCanOpenUrl { url } => {
            let Some(scheme) = url_scheme(url) else {
                return Vec::new();
            };

            if !matches!(platform, Platform::Android | Platform::IOS) {
                return Vec::new();
            }

            vec![HostRequestRequirement::IntentQueryScheme(scheme)]
        }

        // runtime permission prompts
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

        // notification authorization
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

        // outbound local file sharing on Android
        HostRequest::OsIntentSharePaths { .. } if matches!(platform, Platform::Android) => {
            vec![HostRequestRequirement::IntentShareFiles]
        }

        // host routing requests
        HostRequest::OsIntentOpenUrl { .. } | HostRequest::OsIntentOpenPath { .. } => Vec::new(),

        // outbound text sharing
        HostRequest::OsIntentShareText { .. } => Vec::new(),

        // non-Android outbound file sharing
        HostRequest::OsIntentSharePaths { .. } => Vec::new(),

        // user-driven picker import
        HostRequest::OsDocumentPick { .. } => Vec::new(),

        // location lanes
        HostRequest::OsLocationServicesEnabled
        | HostRequest::OsLocationLastKnown
        | HostRequest::OsLocationWatchOpen { .. }
        | HostRequest::OsLocationWatchClose { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::Location,
            )]
        }

        // media read lanes
        HostRequest::OsMediaList { .. }
        | HostRequest::OsMediaDescribe { .. }
        | HostRequest::OsMediaWatchOpen { .. }
        | HostRequest::OsMediaWatchClose { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::MediaRead,
            )]
        }

        // media write lanes
        HostRequest::OsMediaImportPath { .. } | HostRequest::OsMediaDelete { .. } => {
            vec![HostRequestRequirement::Permission(
                RuntimeAppPermission::MediaWrite,
            )]
        }

        // host settings navigation
        HostRequest::OsPermissionOpenSettings => Vec::new(),
    }
}

/// Require that one resolved requirement is satisfied by the target app declaration.
fn require_request_requirement(
    app: &RuntimeAppDeclaration,
    operation: &'static str,
    requirement: &HostRequestRequirement,
) -> RuntimeResult<()> {
    let is_satisfied = requirement_satisfied(app, requirement);

    // explicit declaration failure
    if !is_satisfied {
        return Err(missing_declaration(
            operation,
            requirement_message(requirement),
        ));
    }

    Ok(())
}

/// Return whether one target app declaration satisfies one request requirement.
fn requirement_satisfied(
    app: &RuntimeAppDeclaration,
    requirement: &HostRequestRequirement,
) -> bool {
    match requirement {
        HostRequestRequirement::BackgroundExecution => !app.background.modes.is_empty(),
        HostRequestRequirement::Permission(permission) => app.permissions.contains(permission),
        HostRequestRequirement::NotificationAuthorization => {
            app.notifications.enabled
                || app
                    .permissions
                    .contains(&RuntimeAppPermission::Notifications)
        }
        HostRequestRequirement::IntentQueryScheme(scheme) => {
            app.intents.query_schemes.contains(scheme)
        }
        HostRequestRequirement::IntentShareFiles => app.intents.shares_files,
    }
}

/// Return the user-facing declaration guidance for one request requirement.
fn requirement_message(requirement: &HostRequestRequirement) -> String {
    match requirement {
        HostRequestRequirement::BackgroundExecution => {
            "declare one background.modes entry".to_string()
        }
        HostRequestRequirement::Permission(permission) => {
            format!("declare permissions.{}", permission_name(*permission))
        }
        HostRequestRequirement::NotificationAuthorization => {
            "declare notifications.enabled or permissions.notifications".to_string()
        }
        HostRequestRequirement::IntentQueryScheme(scheme) => {
            format!("declare intents.querySchemes for scheme `{scheme}`")
        }
        HostRequestRequirement::IntentShareFiles => "declare intents.sharesFiles".to_string(),
    }
}

/// Return the runtime app permission selector for one runtime permission.
fn runtime_app_permission(permission: Permission) -> RuntimeAppPermission {
    match permission {
        Permission::Location => RuntimeAppPermission::Location,
        Permission::LocationBackground => RuntimeAppPermission::LocationBackground,
        Permission::Camera => RuntimeAppPermission::Camera,
        Permission::Microphone => RuntimeAppPermission::Microphone,
        Permission::Bluetooth => RuntimeAppPermission::Bluetooth,
        Permission::Notifications => RuntimeAppPermission::Notifications,
        Permission::ContactsRead => RuntimeAppPermission::ContactsRead,
        Permission::ContactsWrite => RuntimeAppPermission::ContactsWrite,
        Permission::MediaRead => RuntimeAppPermission::MediaRead,
        Permission::MediaWrite => RuntimeAppPermission::MediaWrite,
        Permission::Motion => RuntimeAppPermission::Motion,
        Permission::ClipboardRead => RuntimeAppPermission::ClipboardRead,
        Permission::CalendarRead => RuntimeAppPermission::CalendarRead,
        Permission::CalendarWrite => RuntimeAppPermission::CalendarWrite,
    }
}

/// Return the config field name for one runtime app permission.
fn permission_name(permission: RuntimeAppPermission) -> &'static str {
    match permission {
        RuntimeAppPermission::Location => "location",
        RuntimeAppPermission::LocationBackground => "locationBackground",
        RuntimeAppPermission::Camera => "camera",
        RuntimeAppPermission::Microphone => "microphone",
        RuntimeAppPermission::Bluetooth => "bluetooth",
        RuntimeAppPermission::Notifications => "notifications",
        RuntimeAppPermission::ContactsRead => "contactsRead",
        RuntimeAppPermission::ContactsWrite => "contactsWrite",
        RuntimeAppPermission::MediaRead => "mediaRead",
        RuntimeAppPermission::MediaWrite => "mediaWrite",
        RuntimeAppPermission::Motion => "motion",
        RuntimeAppPermission::ClipboardRead => "clipboardRead",
        RuntimeAppPermission::CalendarRead => "calendarRead",
        RuntimeAppPermission::CalendarWrite => "calendarWrite",
    }
}

/// Return one lowercase scheme when the URL payload is absolute.
fn url_scheme(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once(':')?;

    Some(scheme.to_ascii_lowercase())
}

/// Build one explicit missing declaration error.
fn missing_declaration(
    operation: &'static str,
    declaration: impl Into<String>,
) -> Box<RuntimeError> {
    let declaration = declaration.into();
    let message = format!("{operation} requires one target app declaration: {declaration}");

    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoPermissionDenied),
        message,
    ))
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::{HostRequestRequirement, request_requirements};
    use crate::host::core::HostRequest;
    use destack_workspace::Platform;

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
            mime_type: Some("text/plain".to_string()),
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
            mime_type: Some("text/plain".to_string()),
        };

        // desktop file sharing stays declaration-free for now
        let requirements = request_requirements(Platform::Windows, &request);

        assert_eq!(requirements, Vec::new());
    }
}
