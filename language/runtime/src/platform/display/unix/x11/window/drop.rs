use std::sync::Arc;

use super::constants::XDND_ACCEPTED;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, CLIENT_MESSAGE_EVENT, ClientMessageData, ClientMessageEvent,
    ConnectionExt as XprotoConnectionExt, EventMask, SelectionNotifyEvent,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::WindowPosition;
use crate::platform::resource;

use crate::platform::display::unix::x11::model::{X11WindowHostState, XdndPayload};
use crate::platform::display::unix::x11::{core, event};

/// Clear one window-local xdnd session state.
pub(crate) fn clear_xdnd_state(host_state: &mut X11WindowHostState) {
    host_state.xdnd_source_window = None;
    host_state.xdnd_version = None;
    host_state.xdnd_types.clear();
    host_state.xdnd_target_type = None;
    host_state.xdnd_position = None;
    host_state.xdnd_payload = None;
    host_state.xdnd_dragging = false;
    host_state.xdnd_last_hovered_path = None;
}

/// Decode one percent-encoded URI segment into one UTF-8 string.
fn decode_percent_encoded(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    // scan the payload and decode percent-escaped bytes inline
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

/// Parse one file URI into one local-path string.
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

/// Parse one `text/uri-list` payload into file paths.
fn parse_uri_list_payload(payload: &[u8]) -> Option<Vec<String>> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut paths = Vec::new();

    // parse newline-delimited file URIs, ignoring comment lines
    for line in text.lines() {
        let uri = line.trim();
        if uri.is_empty() || uri.starts_with('#') {
            continue;
        }

        let path = parse_file_uri(uri)?;
        paths.push(path);
    }

    if paths.is_empty() {
        return None;
    }

    Some(paths)
}

/// Return one preferred xdnd target type for one offered type set.
fn preferred_xdnd_type(offers: &[Atom], atoms: &core::X11Atoms) -> Option<Atom> {
    if offers.contains(&atoms.text_uri_list) {
        return Some(atoms.text_uri_list);
    }

    if offers.contains(&atoms.utf8_string) {
        return Some(atoms.utf8_string);
    }

    if offers.contains(&atoms.text) {
        return Some(atoms.text);
    }

    None
}

/// Read one full xdnd type list from `XdndTypeList`.
fn xdnd_type_list(
    connection_state: &core::X11ConnectionState,
    source_window: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<Atom>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            source_window,
            connection_state.atoms.xdnd_type_list,
            AtomEnum::ATOM,
            0,
            u32::MAX,
        )
        .map_err(|error| core::io_error(operation, format!("get_property failed: {error}")))?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("get_property reply failed: {error}"))
        })?;

    Ok(reply.value32().map_or_else(Vec::new, Iterator::collect))
}

/// Parse one offered xdnd type set from one `XdndEnter` message.
fn xdnd_enter_types(
    connection_state: &core::X11ConnectionState,
    value: &ClientMessageEvent,
    operation: &'static str,
) -> RuntimeResult<Vec<Atom>> {
    let data = value.data.as_data32();
    let source_window = data[0];
    let flags = data[1];
    let has_more_types = (flags & 1) != 0;

    // parse inline type lanes when no external type list is present
    if !has_more_types {
        let mut types = Vec::new();
        for atom in [data[2], data[3], data[4]] {
            if atom != 0 {
                types.push(atom);
            }
        }

        return Ok(types);
    }

    xdnd_type_list(connection_state, source_window, operation)
}

/// Send one `XdndStatus` message for one drag source.
fn send_xdnd_status(
    connection_state: &core::X11ConnectionState,
    target_window: u32,
    source_window: u32,
    accepted: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let accepted = if accepted { XDND_ACCEPTED } else { 0 };
    let action = if accepted != 0 {
        connection_state.atoms.xdnd_action_copy
    } else {
        0
    };
    let message = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window: target_window,
        type_: connection_state.atoms.xdnd_status,
        data: ClientMessageData::from([target_window, accepted, 0, 0, action]),
    };

    connection_state
        .connection
        .send_event(false, source_window, EventMask::NO_EVENT, message)
        .map_err(|error| core::io_error(operation, format!("send_event failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Send one `XdndFinished` message for one drag source.
fn send_xdnd_finished(
    connection_state: &core::X11ConnectionState,
    target_window: u32,
    source_window: u32,
    accepted: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let accepted = if accepted { XDND_ACCEPTED } else { 0 };
    let action = if accepted != 0 {
        connection_state.atoms.xdnd_action_copy
    } else {
        0
    };
    let message = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window: target_window,
        type_: connection_state.atoms.xdnd_finished,
        data: ClientMessageData::from([target_window, accepted, action, 0, 0]),
    };

    connection_state
        .connection
        .send_event(false, source_window, EventMask::NO_EVENT, message)
        .map_err(|error| core::io_error(operation, format!("send_event failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Request one xdnd selection payload transfer.
fn request_xdnd_selection(
    connection_state: &core::X11ConnectionState,
    target_window: u32,
    target_type: Atom,
    timestamp: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    connection_state
        .connection
        .convert_selection(
            target_window,
            connection_state.atoms.xdnd_selection,
            target_type,
            connection_state.atoms.xdnd_selection,
            timestamp,
        )
        .map_err(|error| core::io_error(operation, format!("convert_selection failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Read one transferred xdnd selection payload from one target window.
fn read_xdnd_selection_payload(
    connection_state: &core::X11ConnectionState,
    target_window: u32,
    target_type: Atom,
    operation: &'static str,
) -> RuntimeResult<Option<XdndPayload>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            target_window,
            connection_state.atoms.xdnd_selection,
            target_type,
            0,
            u32::MAX,
        )
        .map_err(|error| core::io_error(operation, format!("get_property failed: {error}")))?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("get_property reply failed: {error}"))
        })?;

    // parse file-list payloads when `text/uri-list` was requested
    if target_type == connection_state.atoms.text_uri_list {
        let Some(paths) = parse_uri_list_payload(&reply.value) else {
            return Ok(None);
        };
        return Ok(Some(XdndPayload::Files(paths)));
    }

    // parse UTF-8 text payloads when `UTF8_STRING` or `TEXT` was requested
    if target_type == connection_state.atoms.utf8_string
        || target_type == connection_state.atoms.text
    {
        let text = String::from_utf8(reply.value).ok();
        let text = text.map(|value| value.trim_end_matches('\0').to_string());
        let Some(text) = text else {
            return Ok(None);
        };
        if text.is_empty() {
            return Ok(None);
        }
        return Ok(Some(XdndPayload::Text(text)));
    }

    Ok(None)
}

/// Handle one `XdndEnter` client message.
pub(crate) fn handle_xdnd_enter(
    connection_state: &core::X11ConnectionState,
    host_state: &mut X11WindowHostState,
    value: &ClientMessageEvent,
) -> RuntimeResult<()> {
    let data = value.data.as_data32();
    let source_window = data[0];
    let version = data[1] >> 24;
    let offered_types =
        xdnd_enter_types(connection_state, value, "destack.display.window.eventRead")?;
    let target_type = preferred_xdnd_type(&offered_types, &connection_state.atoms);

    clear_xdnd_state(host_state);
    host_state.xdnd_source_window = Some(source_window);
    host_state.xdnd_version = Some(version);
    host_state.xdnd_types = offered_types;
    host_state.xdnd_target_type = target_type;

    Ok(())
}

/// Handle one `XdndPosition` client message.
pub(crate) fn handle_xdnd_position(
    connection_state: &core::X11ConnectionState,
    host_state: &mut X11WindowHostState,
    value: &ClientMessageEvent,
) -> RuntimeResult<()> {
    let data = value.data.as_data32();
    let source_window = data[0];

    // decode desktop-space coordinates from packed 16-bit values
    let packed = data[2];
    let x = ((packed >> 16) & 0xffff) as u16 as i16;
    let y = (packed & 0xffff) as u16 as i16;
    host_state.xdnd_position = Some(WindowPosition {
        x: i32::from(x),
        y: i32::from(y),
    });

    // reject when source window or supported payload type is missing
    let source_matches = host_state.xdnd_source_window == Some(source_window);
    let Some(target_type) = host_state.xdnd_target_type else {
        send_xdnd_status(
            connection_state,
            host_state.window,
            source_window,
            false,
            "destack.display.window.eventRead",
        )?;
        return Ok(());
    };
    if !source_matches {
        send_xdnd_status(
            connection_state,
            host_state.window,
            source_window,
            false,
            "destack.display.window.eventRead",
        )?;
        return Ok(());
    }

    // request selection transfer for the latest drag position
    let version = host_state.xdnd_version.unwrap_or(5);
    let timestamp = if version == 0 {
        x11rb::CURRENT_TIME
    } else {
        data[3]
    };
    request_xdnd_selection(
        connection_state,
        host_state.window,
        target_type,
        timestamp,
        "destack.display.window.eventRead",
    )?;
    send_xdnd_status(
        connection_state,
        host_state.window,
        source_window,
        true,
        "destack.display.window.eventRead",
    )?;

    Ok(())
}

/// Handle one `XdndDrop` client message.
pub(crate) fn handle_xdnd_drop(
    runtime_state: &Arc<core::X11RuntimeState>,
    window_handle: resource::WindowHandle,
    connection_state: &core::X11ConnectionState,
    host_state: &mut X11WindowHostState,
    value: &ClientMessageEvent,
) -> RuntimeResult<()> {
    let source_window = value.data.as_data32()[0];
    let source_matches = host_state.xdnd_source_window == Some(source_window);
    let payload = host_state.xdnd_payload.clone();
    let position = host_state.xdnd_position;
    let previous_hover = host_state.xdnd_last_hovered_path.clone();
    let was_dragging = host_state.xdnd_dragging;

    let mut accepted = false;
    if source_matches && was_dragging && payload.is_some() {
        accepted = true;
    }

    // publish final drop payload events
    if accepted {
        if let Some(payload) = payload {
            match payload {
                XdndPayload::Files(paths) => {
                    for path in paths {
                        event::publish_window_file_dropped(
                            runtime_state,
                            window_handle,
                            Some(path),
                            position,
                        );
                    }
                }
                XdndPayload::Text(text) => {
                    event::publish_window_text_dropped(
                        runtime_state,
                        window_handle,
                        text,
                        position,
                    );
                }
            }
        }
        event::publish_window_drop_completed(runtime_state, window_handle);
    }
    // otherwise publish one cancelled drop sequence when a drag session was active
    else if was_dragging {
        if previous_hover.is_some() {
            event::publish_window_file_hover_left(
                runtime_state,
                window_handle,
                previous_hover,
                position,
            );
        }
        event::publish_window_drop_cancelled(runtime_state, window_handle);
    }

    send_xdnd_finished(
        connection_state,
        host_state.window,
        source_window,
        accepted,
        "destack.display.window.eventRead",
    )?;
    clear_xdnd_state(host_state);

    Ok(())
}

/// Handle one `XdndLeave` client message.
pub(crate) fn handle_xdnd_leave(
    runtime_state: &Arc<core::X11RuntimeState>,
    window_handle: resource::WindowHandle,
    host_state: &mut X11WindowHostState,
) {
    // publish a cancelled drag sequence when one drag session is active
    if host_state.xdnd_dragging {
        if host_state.xdnd_last_hovered_path.is_some() {
            event::publish_window_file_hover_left(
                runtime_state,
                window_handle,
                host_state.xdnd_last_hovered_path.clone(),
                host_state.xdnd_position,
            );
        }
        event::publish_window_drop_cancelled(runtime_state, window_handle);
    }

    clear_xdnd_state(host_state);
}

/// Handle one `SelectionNotify` event for xdnd data transfers.
pub(crate) fn handle_xdnd_selection_notify(
    runtime_state: &Arc<core::X11RuntimeState>,
    window_handle: resource::WindowHandle,
    connection_state: &core::X11ConnectionState,
    host_state: &mut X11WindowHostState,
    value: &SelectionNotifyEvent,
) -> RuntimeResult<()> {
    // ignore selection events outside the xdnd property lane
    if value.property != connection_state.atoms.xdnd_selection {
        return Ok(());
    }

    // ignore selection events when no drag source or target type is active
    if host_state.xdnd_source_window.is_none() {
        return Ok(());
    }
    let Some(target_type) = host_state.xdnd_target_type else {
        return Ok(());
    };

    // parse one transferred payload snapshot from the selection property
    let Some(payload) = read_xdnd_selection_payload(
        connection_state,
        host_state.window,
        target_type,
        "destack.display.window.eventRead",
    )?
    // otherwise fall back
    else {
        return Ok(());
    };

    // publish drop-started once for the first accepted payload
    if !host_state.xdnd_dragging {
        host_state.xdnd_dragging = true;
        event::publish_window_drop_started(runtime_state, window_handle);
    }

    // publish hover payload updates for file drops
    if let XdndPayload::Files(paths) = &payload {
        let path = paths.first().cloned();
        if host_state.xdnd_last_hovered_path != path && host_state.xdnd_last_hovered_path.is_some()
        {
            event::publish_window_file_hover_left(
                runtime_state,
                window_handle,
                host_state.xdnd_last_hovered_path.clone(),
                host_state.xdnd_position,
            );
        }

        host_state.xdnd_last_hovered_path = path.clone();
        event::publish_window_file_hovered(
            runtime_state,
            window_handle,
            path,
            host_state.xdnd_position,
        );
    }

    host_state.xdnd_payload = Some(payload);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode_percent_encoded, parse_file_uri, parse_uri_list_payload};

    /// Decode percent-encoded UTF-8 path segments.
    #[test]
    fn test_decode_percent_encoded_decodes_utf8_bytes() {
        let decoded = decode_percent_encoded("/tmp/My%20File%20%E2%9C%85.txt");
        assert_eq!(decoded.as_deref(), Some("/tmp/My File ✅.txt"));
    }

    /// Reject malformed percent-encoded path segments.
    #[test]
    fn test_decode_percent_encoded_rejects_malformed_sequence() {
        assert_eq!(decode_percent_encoded("/tmp/bad%2"), None);
        assert_eq!(decode_percent_encoded("/tmp/bad%ZZ"), None);
    }

    /// Parse local file URIs and localhost file URIs.
    #[test]
    fn test_parse_file_uri_accepts_local_hosts() {
        let direct = parse_file_uri("file:///tmp/example%20file.txt");
        assert_eq!(direct.as_deref(), Some("/tmp/example file.txt"));

        let localhost = parse_file_uri("file://localhost/tmp/example%20file.txt");
        assert_eq!(localhost.as_deref(), Some("/tmp/example file.txt"));
    }

    /// Reject non-local file URI hosts.
    #[test]
    fn test_parse_file_uri_rejects_non_local_hosts() {
        assert_eq!(parse_file_uri("file://remote-host/tmp/example.txt"), None);
    }

    /// Parse one text/uri-list payload into decoded local paths.
    #[test]
    fn test_parse_uri_list_payload_decodes_paths_and_skips_comments() {
        let payload = b"# comment\nfile:///tmp/a%20b.txt\nfile://localhost/tmp/c.txt\n";
        let paths = parse_uri_list_payload(payload).expect("uri list should parse");
        assert_eq!(
            paths,
            vec!["/tmp/a b.txt".to_string(), "/tmp/c.txt".to_string()]
        );
    }
}
