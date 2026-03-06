use std::fs::File;
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::sync::Arc;

use wayland_client::protocol::{wl_data_device, wl_data_offer};
use wayland_client::{Connection, EventQueue, Proxy};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::WindowPosition;
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::core::{
    self as backend_core, WaylandConnectionDispatchState, WaylandDropSessionState,
};
use super::super::event;
use super::super::model::WaylandWindowBinding;

/// Return one preferred drop mime type for one offered mime-type set.
fn preferred_drop_mime_type(mime_types: &[String]) -> Option<String> {
    // probe this sequence in priority order
    for candidate in [
        "text/uri-list",
        "text/plain;charset=utf-8",
        "text/plain",
        "UTF8_STRING",
    ] {
        if mime_types.iter().any(|value| value == candidate) {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Convert one wayland fixed-coordinate lane into one window position.
fn position_from_fixed(x: f64, y: f64) -> WindowPosition {
    WindowPosition {
        x: x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        y: y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
    }
}

/// Decode one percent-encoded URI segment into one UTF-8 string.
fn decode_percent_encoded(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    // decode this byte sequence
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'%' {
            decoded.push(byte);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return None;
        }

        let upper = bytes[index + 1];
        let lower = bytes[index + 2];
        let upper = (upper as char).to_digit(16)?;
        let lower = (lower as char).to_digit(16)?;
        decoded.push(((upper << 4) | lower) as u8);
        index += 3;
    }

    String::from_utf8(decoded).ok()
}

/// Parse one file URI into one local path.
fn parse_file_uri(uri: &str) -> Option<String> {
    let suffix = uri.strip_prefix("file://")?;

    // parse file uri hosts: empty and localhost are accepted
    let path = if suffix.starts_with('/') {
        suffix
    } else if let Some(path) = suffix.strip_prefix("localhost/") {
        let mut value = String::with_capacity(path.len() + 1);
        value.push('/');
        value.push_str(path);
        return decode_percent_encoded(&value);
    } else {
        return None;
    };

    decode_percent_encoded(path)
}

/// Parse one text-uri-list payload into file paths.
fn parse_uri_list_payload(payload: &[u8]) -> Option<Vec<String>> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut paths = Vec::new();

    // parse newline-delimited file URIs and ignore comments
    for line in text.lines() {
        let value = line.trim();
        if value.is_empty() || value.starts_with('#') {
            continue;
        }

        let path = parse_file_uri(value)?;
        paths.push(path);
    }

    if paths.is_empty() {
        return None;
    }

    Some(paths)
}

/// Parsed payload variants for one finalized drop transfer.
enum ParsedDropPayload {
    /// Parsed file-path payload list.
    Files(Vec<String>),
    /// Parsed text payload.
    Text(String),
}

/// Parse one finalized drop payload for one accepted mime type.
fn parse_drop_payload(mime_type: &str, payload: Vec<u8>) -> Option<ParsedDropPayload> {
    // decode URI-list payloads into file paths
    if mime_type == "text/uri-list" {
        return parse_uri_list_payload(payload.as_slice()).map(ParsedDropPayload::Files);
    }

    // decode text payloads as UTF-8 strings
    String::from_utf8(payload).ok().map(ParsedDropPayload::Text)
}

/// Create one pipe pair for wayland data-offer payload transfer.
fn create_pipe_pair(operation: &'static str) -> RuntimeResult<(File, File)> {
    let mut file_descriptors = [0; 2];
    let result = unsafe { libc::pipe2(file_descriptors.as_mut_ptr(), libc::O_CLOEXEC) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return Err(backend_core::io_error(
            operation,
            format!("pipe2 failed for drop payload transfer: {error}"),
        ));
    }

    let read_file = unsafe { File::from_raw_fd(file_descriptors[0]) };
    let write_file = unsafe { File::from_raw_fd(file_descriptors[1]) };
    Ok((read_file, write_file))
}

/// Publish one drop-started event when one session has not started yet.
fn publish_drop_started(
    runtime_state: &Arc<backend_core::WaylandRuntimeState>,
    window_handle: WindowHandle,
    dispatch_state: &mut WaylandConnectionDispatchState,
) {
    // skip when this session already started
    if dispatch_state.drop_session_state.started {
        return;
    }

    dispatch_state.drop_session_state.started = true;
    event::publish_window_drop_started(runtime_state, window_handle);
}

/// Publish one file-hovered event for one active drop session.
fn publish_drop_hover(
    runtime_state: &Arc<backend_core::WaylandRuntimeState>,
    window_handle: WindowHandle,
    dispatch_state: &WaylandConnectionDispatchState,
) {
    event::publish_window_file_hovered(
        runtime_state,
        window_handle,
        dispatch_state.drop_session_state.last_hovered_path.clone(),
        dispatch_state.drop_session_state.position,
    );
}

/// Clear one active drop session and all associated offer state.
pub(crate) fn clear_drop_session(dispatch_state: &mut WaylandConnectionDispatchState) {
    if let Some(offer_id) = dispatch_state.drop_session_state.offer.clone() {
        dispatch_state.data_offer_state_by_id.remove(&offer_id);
    }

    dispatch_state.drop_session_state = WaylandDropSessionState::default();
}

/// Handle one `wl_data_offer` event for one active runtime connection.
pub(crate) fn handle_data_offer_event(
    dispatch_state: &mut WaylandConnectionDispatchState,
    offer: &wl_data_offer::WlDataOffer,
    event: wl_data_offer::Event,
) {
    // track offered mime types for this offer id
    if let wl_data_offer::Event::Offer { mime_type } = event {
        let offer_state = dispatch_state
            .data_offer_state_by_id
            .entry(offer.id())
            .or_default();
        if !offer_state.mime_types.contains(&mime_type) {
            offer_state.mime_types.push(mime_type);
        }
    }
}

/// Handle one `wl_data_device` event for one active runtime connection.
pub(crate) fn handle_data_device_event(
    dispatch_state: &mut WaylandConnectionDispatchState,
    event: wl_data_device::Event,
) {
    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        return;
    };

    // route this event variant
    match event {
        wl_data_device::Event::DataOffer { id } => {
            dispatch_state
                .data_offer_state_by_id
                .entry(id.id())
                .or_default();
        }
        wl_data_device::Event::Enter {
            serial,
            surface,
            x,
            y,
            id,
        } => {
            let Some(token) =
                backend_core::window_token_from_surface(&runtime_state, &surface.id())
            else {
                return;
            };
            let Some(window_handle) =
                backend_core::window_handle_from_id(&runtime_state, &token.window_id)
            else {
                return;
            };

            dispatch_state.drop_session_state.surface = Some(surface.id());
            dispatch_state.drop_session_state.position = Some(position_from_fixed(x, y));
            dispatch_state.drop_session_state.offer = id.as_ref().map(Proxy::id);
            dispatch_state.drop_session_state.drop_pending = false;
            dispatch_state.drop_session_state.last_hovered_path = None;
            publish_drop_started(&runtime_state, window_handle, dispatch_state);

            if let Some(offer) = id {
                let offered_mime_types = dispatch_state
                    .data_offer_state_by_id
                    .get(&offer.id())
                    .map(|value| value.mime_types.clone())
                    .unwrap_or_default();
                let accepted_mime_type = preferred_drop_mime_type(&offered_mime_types);
                dispatch_state.drop_session_state.accepted_mime_type = accepted_mime_type.clone();
                offer.accept(serial, accepted_mime_type);
            }

            publish_drop_hover(&runtime_state, window_handle, dispatch_state);
        }
        wl_data_device::Event::Motion { time: _, x, y } => {
            dispatch_state.drop_session_state.position = Some(position_from_fixed(x, y));

            let Some(surface_id) = dispatch_state.drop_session_state.surface.as_ref() else {
                return;
            };
            let Some(token) = backend_core::window_token_from_surface(&runtime_state, surface_id)
            else {
                return;
            };
            let Some(window_handle) =
                backend_core::window_handle_from_id(&runtime_state, &token.window_id)
            else {
                return;
            };

            publish_drop_hover(&runtime_state, window_handle, dispatch_state);
        }
        wl_data_device::Event::Drop => {
            dispatch_state.drop_session_state.drop_pending = true;
        }
        wl_data_device::Event::Leave => {
            if dispatch_state.drop_session_state.drop_pending {
                return;
            }

            let surface_id = dispatch_state.drop_session_state.surface.clone();
            if let Some(surface_id) = surface_id {
                if let Some(token) =
                    backend_core::window_token_from_surface(&runtime_state, &surface_id)
                    && let Some(window_handle) =
                        backend_core::window_handle_from_id(&runtime_state, &token.window_id)
                {
                    if let Some(previous_path) =
                        dispatch_state.drop_session_state.last_hovered_path.take()
                    {
                        event::publish_window_file_hover_left(
                            &runtime_state,
                            window_handle,
                            Some(previous_path),
                            dispatch_state.drop_session_state.position,
                        );
                    }

                    event::publish_window_drop_cancelled(&runtime_state, window_handle);
                }
            }

            clear_drop_session(dispatch_state);
        }
        wl_data_device::Event::Selection { id } => {
            if let Some(selection_offer) = id {
                selection_offer.destroy();
            }
        }
        _ => {}
    }
}

/// Finalize one pending drop session and publish payload events.
pub(crate) fn finalize_pending_drop_session(
    connection: &Connection,
    event_queue: &mut EventQueue<WaylandConnectionDispatchState>,
    dispatch_state: &mut WaylandConnectionDispatchState,
    operation: &'static str,
) -> RuntimeResult<()> {
    // skip when no drop transfer is pending
    if !dispatch_state.drop_session_state.drop_pending {
        return Ok(());
    }

    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        clear_drop_session(dispatch_state);
        return Ok(());
    };
    let Some(surface_id) = dispatch_state.drop_session_state.surface.clone() else {
        clear_drop_session(dispatch_state);
        return Ok(());
    };
    let Some(token) = backend_core::window_token_from_surface(&runtime_state, &surface_id) else {
        clear_drop_session(dispatch_state);
        return Ok(());
    };
    let Some(window_handle) = backend_core::window_handle_from_id(&runtime_state, &token.window_id)
    else {
        clear_drop_session(dispatch_state);
        return Ok(());
    };
    let Some(offer_id) = dispatch_state.drop_session_state.offer.clone() else {
        event::publish_window_drop_cancelled(&runtime_state, window_handle);
        clear_drop_session(dispatch_state);
        return Ok(());
    };

    let offered_mime_types = dispatch_state
        .data_offer_state_by_id
        .get(&offer_id)
        .map(|value| value.mime_types.clone())
        .unwrap_or_default();
    let mime_type = dispatch_state
        .drop_session_state
        .accepted_mime_type
        .clone()
        .or_else(|| preferred_drop_mime_type(&offered_mime_types));
    let Some(mime_type) = mime_type else {
        event::publish_window_drop_cancelled(&runtime_state, window_handle);
        clear_drop_session(dispatch_state);
        return Ok(());
    };

    let data_offer =
        wl_data_offer::WlDataOffer::from_id(connection, offer_id.clone()).map_err(|error| {
            backend_core::io_error(operation, format!("invalid wl_data_offer id: {error}"))
        })?;
    let (mut read_file, write_file) = create_pipe_pair(operation)?;

    data_offer.receive(mime_type.clone(), write_file.as_fd());
    drop(write_file);
    backend_core::flush_queue(event_queue, operation)?;

    let mut poll_file_descriptor = libc::pollfd {
        fd: read_file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let poll_result = unsafe { libc::poll(&mut poll_file_descriptor, 1, 250) };
    if poll_result < 0 {
        let error = std::io::Error::last_os_error();
        return Err(backend_core::io_error(
            operation,
            format!("drop payload poll failed: {error}"),
        ));
    }

    let mut payload = Vec::new();
    if poll_result > 0 {
        read_file.read_to_end(&mut payload).map_err(|error| {
            backend_core::io_error(operation, format!("drop payload read failed: {error}"))
        })?;
    }

    if let Some(previous_path) = dispatch_state.drop_session_state.last_hovered_path.take() {
        event::publish_window_file_hover_left(
            &runtime_state,
            window_handle,
            Some(previous_path),
            dispatch_state.drop_session_state.position,
        );
    }

    // publish drop payload events for accepted payload variants
    let parsed_payload = parse_drop_payload(&mime_type, payload);
    let accepted = if let Some(parsed_payload) = parsed_payload {
        match parsed_payload {
            ParsedDropPayload::Files(paths) => {
                for path in paths {
                    event::publish_window_file_dropped(
                        &runtime_state,
                        window_handle,
                        Some(path),
                        dispatch_state.drop_session_state.position,
                    );
                }

                true
            }
            ParsedDropPayload::Text(text) => {
                event::publish_window_text_dropped(
                    &runtime_state,
                    window_handle,
                    text,
                    dispatch_state.drop_session_state.position,
                );

                true
            }
        }
    } else {
        false
    };

    // finish one accepted offer when protocol supports explicit completion
    if accepted && data_offer.version() >= 3 {
        data_offer.finish();
    }
    data_offer.destroy();

    if accepted {
        event::publish_window_drop_completed(&runtime_state, window_handle);
    } else {
        event::publish_window_drop_cancelled(&runtime_state, window_handle);
    }

    clear_drop_session(dispatch_state);
    Ok(())
}

/// Reset one window-local drop state snapshot.
pub(crate) fn reset_drop_state(
    context: &BindingCallContext,
    binding: &mut WaylandWindowBinding,
) -> RuntimeResult<()> {
    backend_core::clear_drop_session_for_surface(
        context,
        &binding.host.surface,
        "destack.display.window.close",
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_file_uri, parse_uri_list_payload, preferred_drop_mime_type};

    /// Parse local file URIs and decode percent-encoded bytes.
    #[test]
    fn test_parse_file_uri_accepts_local_paths() {
        let direct = parse_file_uri("file:///tmp/alpha%20beta.txt");
        assert_eq!(direct, Some("/tmp/alpha beta.txt".to_string()));

        let localhost = parse_file_uri("file://localhost/tmp/value.txt");
        assert_eq!(localhost, Some("/tmp/value.txt".to_string()));
    }

    /// Reject malformed and non-local file URIs.
    #[test]
    fn test_parse_file_uri_rejects_non_local_and_malformed_values() {
        assert_eq!(parse_file_uri("https://example.invalid/value"), None);
        assert_eq!(parse_file_uri("file://remote-host/tmp/value.txt"), None);
        assert_eq!(parse_file_uri("file:///tmp/%ZZ"), None);
    }

    /// Parse URI-list payloads and ignore comments and empty lines.
    #[test]
    fn test_parse_uri_list_payload_parses_multiple_paths() {
        let payload = b"# comment\n\nfile:///tmp/a.txt\nfile:///tmp/b%20c.txt\n";
        let parsed = parse_uri_list_payload(payload);

        assert_eq!(
            parsed,
            Some(vec!["/tmp/a.txt".to_string(), "/tmp/b c.txt".to_string(),])
        );
    }

    /// Select one preferred mime type from one offered set.
    #[test]
    fn test_preferred_drop_mime_type_prefers_uri_list_then_utf8_text() {
        let uri_first =
            preferred_drop_mime_type(&["text/plain".to_string(), "text/uri-list".to_string()]);
        assert_eq!(uri_first, Some("text/uri-list".to_string()));

        let text_only = preferred_drop_mime_type(&["text/plain;charset=utf-8".to_string()]);
        assert_eq!(text_only, Some("text/plain;charset=utf-8".to_string()));
    }

    /// Convert floating-point coordinates into clamped integer positions.
    #[test]
    fn test_position_from_fixed_rounds_and_clamps_values() {
        let rounded = position_from_fixed(10.4, 20.6);
        assert_eq!(rounded.x, 10);
        assert_eq!(rounded.y, 21);

        let clamped = position_from_fixed(f64::INFINITY, f64::NEG_INFINITY);
        assert_eq!(clamped.x, i32::MAX);
        assert_eq!(clamped.y, i32::MIN);
    }

    /// Parse URI-list drops into file payload variants.
    #[test]
    fn test_parse_drop_payload_parses_uri_list_as_files() {
        let payload = b"file:///tmp/a.txt\nfile:///tmp/b.txt\n".to_vec();
        let parsed = parse_drop_payload("text/uri-list", payload);

        let Some(ParsedDropPayload::Files(paths)) = parsed else {
            panic!("uri-list payload should decode into file list");
        };
        assert_eq!(
            paths,
            vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()]
        );
    }

    /// Parse UTF-8 text drops into text payload variants.
    #[test]
    fn test_parse_drop_payload_parses_text_payload() {
        let payload = b"hello world".to_vec();
        let parsed = parse_drop_payload("text/plain", payload);

        let Some(ParsedDropPayload::Text(text)) = parsed else {
            panic!("text payload should decode into text variant");
        };
        assert_eq!(text, "hello world".to_string());
    }

    /// Reject malformed payloads for their selected mime type.
    #[test]
    fn test_parse_drop_payload_rejects_malformed_payloads() {
        let malformed_uri_payload = b"file://remote-host/tmp/value.txt\n".to_vec();
        let malformed_uri = parse_drop_payload("text/uri-list", malformed_uri_payload);
        assert!(malformed_uri.is_none());

        let invalid_utf8_payload = vec![0xff, 0xfe];
        let invalid_utf8 = parse_drop_payload("text/plain", invalid_utf8_payload);
        assert!(invalid_utf8.is_none());
    }
}
