use vobject::Component;
use zbus::blocking::Proxy;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::request::eds::open_calendar_backend;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;

use super::event::calendar_component_from_text;
use super::{CALENDAR_INTERFACE, EDS_MOD_THIS, EDS_OPERATION_FLAGS};

/// Read one event list payload from one source calendar.
pub(super) fn get_calendar_components(
    source_uid: &str,
    query: &str,
    operation: &'static str,
) -> RuntimeResult<Vec<Component>> {
    with_calendar_proxy(source_uid, operation, |proxy| {
        let calendars: Vec<String> = proxy
            .call("GetObjectList", &(query, EDS_OPERATION_FLAGS))
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution calendar list failed: {error}"),
                )
            })?;
        let mut components = Vec::with_capacity(calendars.len());

        // calendar parsing
        for calendar_text in calendars {
            let component = calendar_component_from_text(&calendar_text, operation)?;
            components.push(component);
        }

        Ok(components)
    })
}

/// Read one event calendar payload from one source calendar.
pub(super) fn get_calendar_component(
    source_uid: &str,
    event_uid: &str,
    operation: &'static str,
) -> RuntimeResult<Component> {
    with_calendar_proxy(source_uid, operation, |proxy| {
        let calendar_text: String = proxy
            .call(
                "GetObject",
                &(event_uid, String::new(), EDS_OPERATION_FLAGS),
            )
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution calendar read failed: {error}"),
                )
            })?;

        calendar_component_from_text(&calendar_text, operation)
    })
}

/// Create one or more calendar objects in one source calendar.
pub(super) fn create_calendar_objects(
    source_uid: &str,
    objects: &[String],
    operation: &'static str,
) -> RuntimeResult<Vec<String>> {
    with_calendar_proxy(source_uid, operation, |proxy| {
        proxy
            .call("CreateObjects", &(objects.to_vec(), EDS_OPERATION_FLAGS))
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution calendar create failed: {error}"),
                )
            })
    })
}

/// Modify one or more calendar objects in one source calendar.
pub(super) fn modify_calendar_objects(
    source_uid: &str,
    objects: &[String],
    operation: &'static str,
) -> RuntimeResult<()> {
    with_calendar_proxy(source_uid, operation, |proxy| {
        proxy
            .call::<_, _, ()>(
                "ModifyObjects",
                &(
                    objects.to_vec(),
                    EDS_MOD_THIS.to_string(),
                    EDS_OPERATION_FLAGS,
                ),
            )
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution calendar update failed: {error}"),
                )
            })
    })
}

/// Remove one or more calendar objects from one source calendar.
pub(super) fn remove_calendar_objects(
    source_uid: &str,
    objects: &[(String, String)],
    operation: &'static str,
) -> RuntimeResult<()> {
    with_calendar_proxy(source_uid, operation, |proxy| {
        proxy
            .call::<_, _, ()>(
                "RemoveObjects",
                &(
                    objects.to_vec(),
                    EDS_MOD_THIS.to_string(),
                    EDS_OPERATION_FLAGS,
                ),
            )
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("evolution calendar delete failed: {error}"),
                )
            })
    })
}

/// Call one typed calendar proxy and close it afterward.
fn with_calendar_proxy<T>(
    source_uid: &str,
    operation: &'static str,
    callback: impl FnOnce(&Proxy<'_>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let backend = open_calendar_backend(source_uid, operation)?;
    let proxy = Proxy::new(
        &backend.connection,
        backend.bus_name.as_str(),
        backend.object_path.as_str(),
        CALENDAR_INTERFACE,
    )
    .map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to bind one evolution calendar proxy: {error}"),
        )
    })?;

    // provider session open
    proxy.call::<_, _, ()>("Open", &()).map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to open one evolution calendar backend: {error}"),
        )
    })?;

    let result = callback(&proxy);

    // backend cleanup
    if let Err(error) = proxy.call::<_, _, ()>("Close", &()) {
        tracing::warn!(
            target: "destack.runtime.host.unix.calendar",
            ?error,
            "failed to close evolution calendar backend",
        );
    }

    result
}
