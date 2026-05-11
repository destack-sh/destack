use zbus::blocking::{Connection, Proxy};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

/// The freedesktop notification bus name.
const NOTIFICATION_BUS_NAME: &str = "org.freedesktop.Notifications";
/// The freedesktop notification object path.
const NOTIFICATION_OBJECT_PATH: &str = "/org/freedesktop/Notifications";
/// The freedesktop notification interface name.
const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";
/// The freedesktop actions action name.
const NOTIFICATION_CAPABILITY_ACTIONS: &str = "actions";

/// Return whether one freedesktop notification host is reachable on the session bus.
pub(super) fn unix_notification_server_available() -> bool {
    unix_notification_capabilities().is_ok()
}

/// Return whether the active notification server supports freedesktop actions.
pub(super) fn unix_notification_supports_actions() -> RuntimeResult<bool> {
    let actions = unix_notification_capabilities()?;

    Ok(actions
        .iter()
        .any(|action| action == NOTIFICATION_CAPABILITY_ACTIONS))
}

/// Close one freedesktop notification by server identifier.
pub(super) fn close_notification(server_id: u32) -> RuntimeResult<()> {
    let connection = Connection::session().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!(
                "destack.os.notification.cancel could not connect to the Unix session bus: {error}"
            ),
        ))
        .boxed()
    })?;
    let proxy = Proxy::new(
        &connection,
        NOTIFICATION_BUS_NAME,
        NOTIFICATION_OBJECT_PATH,
        NOTIFICATION_INTERFACE,
    )
    .map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!(
                "destack.os.notification.cancel could not open the Unix notification proxy: {error}"
            ),
        ))
        .boxed()
    })?;

    let _: () =
        proxy
            .call("CloseNotification", &(server_id,))
            .map_err(|error| {
                RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification.cancel failed in the Unix notification host: {error}"),
        ))
        .boxed()
            })?;

    Ok(())
}

/// Return notification server actions from the freedesktop host.
fn unix_notification_capabilities() -> RuntimeResult<Vec<String>> {
    let connection = Connection::session().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification could not connect to the Unix session bus: {error}"),
        ))
        .boxed()
    })?;
    let proxy = Proxy::new(
        &connection,
        NOTIFICATION_BUS_NAME,
        NOTIFICATION_OBJECT_PATH,
        NOTIFICATION_INTERFACE,
    )
    .map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification could not open the Unix notification proxy: {error}"),
        ))
        .boxed()
    })?;
    let actions = proxy.call("GetCapabilities", &()).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("destack.os.notification could not query Unix notification actions: {error}"),
        ))
        .boxed()
    })?;

    Ok(actions)
}
