use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::core::{io_operation_error, pathbuf_from_file_uri};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{DocumentDescriptorValue, DocumentPickOptionsValue};
use crate::platform::os::document::{
    DOCUMENT_PICK_OPERATION, document_descriptor_value_from_path, validate_document_pick_options,
    validated_document_extensions,
};
use crate::runtime::action::ActionSet;

/// The desktop portal bus name.
const DESKTOP_PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
/// The desktop portal object path.
const DESKTOP_PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
/// The desktop portal file chooser interface.
const FILE_CHOOSER_INTERFACE: &str = "org.freedesktop.portal.FileChooser";
/// The desktop portal request interface.
const REQUEST_INTERFACE: &str = "org.freedesktop.portal.Request";
/// The desktop portal request path prefix.
const REQUEST_PATH_PREFIX: &str = "/org/freedesktop/portal/desktop/request";
/// The desktop portal open-file title.
const DOCUMENT_PICK_TITLE: &str = "Select documents";
/// The desktop portal empty parent window identifier.
const EMPTY_PARENT_WINDOW: &str = "";
/// The portal response code for successful completion.
const PORTAL_RESPONSE_SUCCESS: u32 = 0;
/// The portal response code for cancellation.
const PORTAL_RESPONSE_CANCELLED: u32 = 1;
/// The portal file-filter kind for filename patterns.
const PORTAL_FILTER_PATTERN: u32 = 0;
/// The portal file-filter kind for MIME types.
const PORTAL_FILTER_MIME_TYPE: u32 = 1;

/// Return dynamic Unix document request actions.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Unix desktop document request through the active backend.
pub(crate) fn submit_document_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsDocumentPick { options } => {
            let descriptors = pick_documents(context, options)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::DocumentDescriptors(descriptors),
            )))
        }
        _ => Ok(None),
    }
}

/// Pick documents through the desktop portal file chooser.
fn pick_documents(
    _context: &RequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    validate_document_pick_options(options)?;

    let connection = Connection::session().map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to connect to the desktop session bus: {error}"),
        )
    })?;
    let file_chooser = Proxy::new(
        &connection,
        DESKTOP_PORTAL_BUS_NAME,
        DESKTOP_PORTAL_PATH,
        FILE_CHOOSER_INTERFACE,
    )
    .map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to bind the desktop portal file chooser: {error}"),
        )
    })?;
    let handle_token = portal_handle_token(context);
    let request_path = portal_request_path(&connection, &handle_token)?;
    let request_proxy = Proxy::new(
        &connection,
        DESKTOP_PORTAL_BUS_NAME,
        request_path.as_str(),
        REQUEST_INTERFACE,
    )
    .map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to bind the desktop portal request handle: {error}"),
        )
    })?;
    let mut responses = request_proxy.receive_signal("Response").map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to subscribe to the desktop portal response signal: {error}"),
        )
    })?;
    let portal_options = portal_open_file_options(options, &handle_token)?;
    let returned_request_path: OwnedObjectPath = file_chooser
        .call(
            "OpenFile",
            &(EMPTY_PARENT_WINDOW, DOCUMENT_PICK_TITLE, portal_options),
        )
        .map_err(|error| {
            io_operation_error(
                DOCUMENT_PICK_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("desktop portal OpenFile failed: {error}"),
            )
        })?;

    // the returned handle should match the predicted path when handle_token is honored
    if returned_request_path != request_path {
        return Err(io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "desktop portal returned one unexpected request handle: expected {}, got {}",
                request_path.as_str(),
                returned_request_path.as_str()
            ),
        ));
    }

    let response = responses.next().ok_or_else(|| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "desktop portal closed the request without one response",
        )
    })?;
    let (response_code, results): (u32, HashMap<String, OwnedValue>) =
        response.body().deserialize().map_err(|error| {
            io_operation_error(
                DOCUMENT_PICK_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to decode the desktop portal response payload: {error}"),
            )
        })?;

    // cancellation should resolve to an empty selection just like the native picker lanes
    if response_code == PORTAL_RESPONSE_CANCELLED {
        return Ok(Vec::new());
    }

    // any non-success response is a real backend failure
    if response_code != PORTAL_RESPONSE_SUCCESS {
        return Err(io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("desktop portal document picker failed with response code {response_code}"),
        ));
    }

    descriptors_from_portal_results(results)
}

/// Return whether the desktop portal file chooser is reachable on the session bus.
fn desktop_portal_file_chooser_is_available() -> bool {
    let Ok(connection) = Connection::session() else {
        return false;
    };

    Proxy::new(
        &connection,
        DESKTOP_PORTAL_BUS_NAME,
        DESKTOP_PORTAL_PATH,
        FILE_CHOOSER_INTERFACE,
    )
    .is_ok()
}

/// Build one desktop portal request-handle token for one picker invocation.
fn portal_handle_token(context: &RequestContext) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_nanos())
        .unwrap_or(0);

    format!(
        "destack_document_pick_{}_{}",
        context.host_session_id.0, timestamp
    )
}

/// Build one desktop portal request path from the session bus unique name and handle token.
fn portal_request_path(
    connection: &Connection,
    handle_token: &str,
) -> RuntimeResult<OwnedObjectPath> {
    let unique_name = connection.unique_name().ok_or_else(|| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "desktop session bus did not expose one unique connection name",
        )
    })?;

    portal_request_path_for_unique_name(unique_name.as_str(), handle_token)
}

/// Build one desktop portal request path from one unique bus name and handle token.
fn portal_request_path_for_unique_name(
    unique_name: &str,
    handle_token: &str,
) -> RuntimeResult<OwnedObjectPath> {
    let sender = unique_name
        .trim_start_matches(':')
        .replace('.', "_")
        .replace('-', "_");
    let request_path = format!("{REQUEST_PATH_PREFIX}/{sender}/{handle_token}");

    OwnedObjectPath::try_from(request_path.clone()).map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("desktop portal request path is invalid: {request_path}: {error}"),
        )
    })
}

/// Build one desktop portal `OpenFile` option map.
fn portal_open_file_options(
    options: &DocumentPickOptionsValue,
    handle_token: &str,
) -> RuntimeResult<HashMap<String, OwnedValue>> {
    let mut portal_options = HashMap::new();
    let filters = portal_filters(options)?;

    portal_options.insert("handle_token".to_string(), OwnedValue::from(handle_token));
    portal_options.insert("multiple".to_string(), OwnedValue::from(options.multiple));
    portal_options.insert(
        "directory".to_string(),
        OwnedValue::from(options.allow_directories),
    );

    if !filters.is_empty() {
        let filters = Value::from(filters);
        let filters = OwnedValue::try_from(filters).map_err(|error| {
            io_operation_error(
                DOCUMENT_PICK_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to encode desktop portal document filters: {error}"),
            )
        })?;

        portal_options.insert("filters".to_string(), filters);
    }

    Ok(portal_options)
}

/// Build desktop portal file filters from one picker option set.
fn portal_filters(
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<(String, Vec<(u32, String)>)>> {
    let mut filters = Vec::new();
    let extensions = validated_document_extensions(options)?;

    // pattern filters
    if !extensions.is_empty() {
        let patterns = extensions
            .into_iter()
            .map(|extension| (PORTAL_FILTER_PATTERN, format!("*.{extension}")))
            .collect::<Vec<_>>();

        filters.push(("Supported files".to_string(), patterns));
    }

    // content-type filters
    if !options.content_types.is_empty() {
        let content_types = options
            .content_types
            .iter()
            .cloned()
            .map(|content_type| (PORTAL_FILTER_MIME_TYPE, content_type))
            .collect::<Vec<_>>();

        filters.push(("Supported types".to_string(), content_types));
    }

    Ok(filters)
}

/// Decode document descriptors from one successful desktop portal response.
fn descriptors_from_portal_results(
    mut results: HashMap<String, OwnedValue>,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let uris_value = results.remove("uris").ok_or_else(|| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "desktop portal response did not include any selected uris",
        )
    })?;
    let uris = Vec::<String>::try_from(uris_value).map_err(|error| {
        io_operation_error(
            DOCUMENT_PICK_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("desktop portal uris payload was malformed: {error}"),
        )
    })?;
    let mut descriptors = Vec::with_capacity(uris.len());

    // decode one selected local URI at a time
    for uri in uris {
        let path = pathbuf_from_file_uri(&uri, "uris").map_err(|error| {
            io_operation_error(
                DOCUMENT_PICK_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("desktop portal returned one unsupported document uri: {error}"),
            )
        })?;
        let descriptor = document_descriptor_value_from_path(&path)?;

        descriptors.push(descriptor);
    }

    Ok(descriptors)
}

#[cfg(test)]
mod tests {
    use super::{
        PORTAL_FILTER_MIME_TYPE, PORTAL_FILTER_PATTERN, portal_filters,
        portal_request_path_for_unique_name,
    };
    use crate::platform::os::abi_generated::DocumentPickOptionsValue;

    /// Build one minimal picker option payload for Unix portal tests.
    fn document_pick_options() -> DocumentPickOptionsValue {
        DocumentPickOptionsValue {
            extensions: Vec::new(),
            content_types: Vec::new(),
            multiple: false,
            allow_directories: false,
        }
    }

    /// Build the stable desktop portal request path for one bus unique name.
    #[test]
    fn test_portal_request_path_normalizes_unique_name() {
        let request_path =
            portal_request_path_for_unique_name(":1.420", "destack_document_pick_token")
                .expect("request path should normalize one unique name");

        assert_eq!(
            request_path.as_str(),
            "/org/freedesktop/portal/desktop/request/1_420/destack_document_pick_token"
        );
    }

    /// Build both extension and content-type filters for the desktop portal chooser.
    #[test]
    fn test_portal_filters_keep_patterns_and_content_types_distinct() {
        let mut options = document_pick_options();
        options.extensions = vec!["txt".to_string(), "md".to_string()];
        options.content_types = vec!["text/plain".to_string(), "text/markdown".to_string()];

        let filters = portal_filters(&options).expect("portal filters should encode both kinds");

        assert_eq!(filters.len(), 2);
        assert_eq!(
            filters[0],
            (
                "Supported files".to_string(),
                vec![
                    (PORTAL_FILTER_PATTERN, "*.txt".to_string()),
                    (PORTAL_FILTER_PATTERN, "*.md".to_string())
                ]
            )
        );
        assert_eq!(
            filters[1],
            (
                "Supported types".to_string(),
                vec![
                    (PORTAL_FILTER_MIME_TYPE, "text/plain".to_string()),
                    (PORTAL_FILTER_MIME_TYPE, "text/markdown".to_string())
                ]
            )
        );
    }
}
