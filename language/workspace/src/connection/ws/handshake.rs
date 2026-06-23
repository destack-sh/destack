use std::io::{Read, Write};
use std::net::TcpStream;

use base64::Engine;
use base64::engine::general_purpose;
use sha1::{Digest, Sha1};

use super::WorkspaceWebSocketError;
use super::constants::{HEADER_LIMIT, WEBSOCKET_GUID};

/// Perform a WebSocket server handshake on one TCP stream.
pub(super) fn accept_handshake(
    stream: &mut TcpStream,
    path: &str,
    token: &str,
) -> Result<(), WorkspaceWebSocketError> {
    // read the HTTP upgrade request
    let request = read_request(stream)?;
    let key = parse_request(&request, path, token)?;

    // write the upgrade response
    let accept = websocket_accept(&key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         \r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}

/// Read one HTTP request header block.
fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>, WorkspaceWebSocketError> {
    let mut request = Vec::new();

    loop {
        let mut byte = [0u8; 1];
        let len = stream.read(&mut byte)?;
        if len == 0 {
            return Err(WorkspaceWebSocketError::Handshake(
                "connection closed before handshake",
            ));
        }

        request.push(byte[0]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
        if request.len() > HEADER_LIMIT {
            return Err(WorkspaceWebSocketError::PayloadTooLarge {
                limit: HEADER_LIMIT,
                actual: request.len(),
            });
        }
    }
}

/// Parse and validate one HTTP WebSocket upgrade request.
fn parse_request(
    request: &[u8],
    path: &str,
    token: &str,
) -> Result<String, WorkspaceWebSocketError> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut parsed = httparse::Request::new(&mut headers);
    let status = parsed.parse(request)?;

    // require complete header parsing
    if status.is_partial() {
        return Err(WorkspaceWebSocketError::Handshake("incomplete request"));
    }

    // validate method and required headers
    if parsed.method != Some("GET") {
        return Err(WorkspaceWebSocketError::Handshake("expected GET request"));
    }
    if !path_has_token(parsed.path, path, token) {
        return Err(WorkspaceWebSocketError::Handshake(
            "invalid websocket token",
        ));
    }
    if !header_equals(parsed.headers, "upgrade", "websocket") {
        return Err(WorkspaceWebSocketError::Handshake(
            "missing websocket upgrade",
        ));
    }
    if !header_contains(parsed.headers, "connection", "upgrade") {
        return Err(WorkspaceWebSocketError::Handshake(
            "missing upgrade connection",
        ));
    }

    header(parsed.headers, "sec-websocket-key")
        .map(str::to_string)
        .ok_or(WorkspaceWebSocketError::Handshake("missing websocket key"))
}

/// Return whether a request path carries the expected token.
fn path_has_token(request_path: Option<&str>, expected_path: &str, token: &str) -> bool {
    let Some(request_path) = request_path else {
        return false;
    };
    let Some((path, query)) = request_path.split_once('?') else {
        return false;
    };
    if path != expected_path {
        return false;
    }

    query.split('&').any(|parameter| {
        let Some((name, value)) = parameter.split_once('=') else {
            return false;
        };

        name == "token" && value == token
    })
}

/// Return one header value as UTF-8.
fn header<'a>(headers: &'a [httparse::Header<'a>], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .and_then(|header| std::str::from_utf8(header.value).ok())
}

/// Return whether one header equals the expected value.
fn header_equals(headers: &[httparse::Header<'_>], name: &str, expected: &str) -> bool {
    header(headers, name).is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

/// Return whether one comma-separated header contains the expected value.
fn header_contains(headers: &[httparse::Header<'_>], name: &str, expected: &str) -> bool {
    header(headers, name).is_some_and(|value| {
        value
            .split(',')
            .any(|part| part.trim().eq_ignore_ascii_case(expected))
    })
}

/// Return the WebSocket accept key for one client key.
fn websocket_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let digest = hasher.finalize();

    general_purpose::STANDARD.encode(digest)
}
