use destack_workspace::{Platform, RuntimeAppDeclaration, RuntimeAppPermission};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::request::HostRequest;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::Permission;

use super::{declaration, request_requirements};

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
        declaration::require_request_requirement(app, request.operation_name(), requirement)?;
    }

    Ok(())
}

/// Return the runtime app permission selector for one runtime permission.
pub(super) fn runtime_app_permission(permission: Permission) -> RuntimeAppPermission {
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
pub(super) fn permission_name(permission: RuntimeAppPermission) -> &'static str {
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
