use destack_artifact::Platform;
use destack_workspace::{AppOptions, AppPermission};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::request::HostRequest;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::Permission;

use super::{declaration, request_requirements};

/// Require that one host request is declared for the active target app.
pub(crate) fn require_declared_request(
    platform: Platform,
    app: &AppOptions,
    request: &HostRequest,
) -> RuntimeResult<()> {
    // request requirements
    let requirements = request_requirements(platform, request);

    // enforce one resolved declaration requirement at a time
    for requirement in &requirements {
        declaration::require_request_requirement(app, request.operation_name(), requirement)?;
    }

    Ok(())
}

/// Return the runtime app permission selector for one runtime permission.
pub(super) fn runtime_app_permission(permission: Permission) -> AppPermission {
    match permission {
        Permission::Location => AppPermission::Location,
        Permission::LocationBackground => AppPermission::LocationBackground,
        Permission::Camera => AppPermission::Camera,
        Permission::Microphone => AppPermission::Microphone,
        Permission::Bluetooth => AppPermission::Bluetooth,
        Permission::Notifications => AppPermission::Notifications,
        Permission::ContactsRead => AppPermission::ContactsRead,
        Permission::ContactsWrite => AppPermission::ContactsWrite,
        Permission::MediaRead => AppPermission::MediaRead,
        Permission::MediaWrite => AppPermission::MediaWrite,
        Permission::Motion => AppPermission::Motion,
        Permission::ClipboardRead => AppPermission::ClipboardRead,
        Permission::CalendarRead => AppPermission::CalendarRead,
        Permission::CalendarWrite => AppPermission::CalendarWrite,
    }
}

/// Return the config field name for one runtime app permission.
pub(super) fn permission_name(permission: AppPermission) -> &'static str {
    match permission {
        AppPermission::Location => "location",
        AppPermission::LocationBackground => "locationBackground",
        AppPermission::Camera => "camera",
        AppPermission::Microphone => "microphone",
        AppPermission::Bluetooth => "bluetooth",
        AppPermission::Notifications => "notifications",
        AppPermission::ContactsRead => "contactsRead",
        AppPermission::ContactsWrite => "contactsWrite",
        AppPermission::MediaRead => "mediaRead",
        AppPermission::MediaWrite => "mediaWrite",
        AppPermission::Motion => "motion",
        AppPermission::ClipboardRead => "clipboardRead",
        AppPermission::CalendarRead => "calendarRead",
        AppPermission::CalendarWrite => "calendarWrite",
    }
}

/// Return one lowercase scheme when the URL payload is absolute.
pub(super) fn url_scheme(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once(':')?;

    Some(scheme.to_ascii_lowercase())
}

/// Build one explicit missing declaration error.
pub(super) fn missing_declaration(
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
