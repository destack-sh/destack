use std::io::{Read, Write};
use std::net::TcpStream;

use base64::Engine;
use base64::engine::general_purpose;
use sha1::{Digest, Sha1};

use super::WebSocketError;
use super::constants::{HEADER_LIMIT, WEBSOCKET_GUID};

/// Authenticated HTTP upgrade into one WebSocket connection.
#[derive(Debug, Clone)]
pub(super) struct WebSocketUpgrade {
    /// Required HTTP request path.
    path: String,
    /// Required request bearer token.
    token: String,
}

impl WebSocketUpgrade {
    /// Create one authenticated WebSocket upgrade.
    pub(super) fn new(path: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            token: token.into(),
        }
    }

    /// Accept one WebSocket upgrade request.
    pub(super) fn accept(&self, stream: &mut TcpStream) -> Result<(), WebSocketError> {
        let request = Self::read(stream)?;
        let key = self.parse(&request)?;
        let accept = Self::accept_key(&key);
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

    /// Read one complete HTTP request header.
    fn read(stream: &mut TcpStream) -> Result<Vec<u8>, WebSocketError> {
        let mut request = Vec::new();

        loop {
            let mut byte = [0; 1];
            if stream.read(&mut byte)? == 0 {
                return Err(WebSocketError::Handshake(
                    "connection closed before upgrade",
                ));
            }
            request.push(byte[0]);

            if request.ends_with(b"\r\n\r\n") {
                return Ok(request);
            }
            if request.len() > HEADER_LIMIT {
                return Err(WebSocketError::MessageTooLarge {
                    limit: HEADER_LIMIT,
                    actual: request.len(),
                });
            }
        }
    }

    /// Parse and validate one WebSocket upgrade request.
    fn parse(&self, request: &[u8]) -> Result<String, WebSocketError> {
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut parsed = httparse::Request::new(&mut headers);
        let status = parsed.parse(request)?;

        if status.is_partial() {
            return Err(WebSocketError::Handshake("incomplete request"));
        }
        if parsed.method != Some("GET") {
            return Err(WebSocketError::Handshake("expected GET request"));
        }
        if !Self::path_has_token(parsed.path, &self.path, &self.token) {
            return Err(WebSocketError::Handshake("invalid bearer token"));
        }
        if !Self::header_equals(parsed.headers, "upgrade", "websocket")? {
            return Err(WebSocketError::Handshake("missing WebSocket upgrade"));
        }
        if !Self::header_contains(parsed.headers, "connection", "upgrade")? {
            return Err(WebSocketError::Handshake("missing upgrade connection"));
        }
        if !Self::header_equals(parsed.headers, "sec-websocket-version", "13")? {
            return Err(WebSocketError::Handshake("unsupported WebSocket version"));
        }

        let key = Self::header(parsed.headers, "sec-websocket-key")?
            .ok_or(WebSocketError::Handshake("missing WebSocket key"))?;
        let decoded = general_purpose::STANDARD
            .decode(key)
            .map_err(|_| WebSocketError::Handshake("invalid WebSocket key"))?;
        if decoded.len() != 16 {
            return Err(WebSocketError::Handshake("invalid WebSocket key"));
        }

        Ok(key.to_string())
    }

    /// Return whether one request path carries the expected bearer token.
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

    /// Return one UTF-8 HTTP header value.
    fn header<'b>(
        headers: &'b [httparse::Header<'b>],
        name: &str,
    ) -> Result<Option<&'b str>, WebSocketError> {
        let header = headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name));
        let Some(header) = header else {
            return Ok(None);
        };
        let value = std::str::from_utf8(header.value)
            .map_err(|_| WebSocketError::Handshake("header value is not UTF-8"))?;

        Ok(Some(value.trim()))
    }

    /// Return whether one header equals the expected value.
    fn header_equals(
        headers: &[httparse::Header<'_>],
        name: &str,
        expected: &str,
    ) -> Result<bool, WebSocketError> {
        let value = Self::header(headers, name)?;

        Ok(value.is_some_and(|value| value.eq_ignore_ascii_case(expected)))
    }

    /// Return whether one comma-separated header contains the expected value.
    fn header_contains(
        headers: &[httparse::Header<'_>],
        name: &str,
        expected: &str,
    ) -> Result<bool, WebSocketError> {
        let value = Self::header(headers, name)?;
        let contains = value.is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(expected))
        });

        Ok(contains)
    }

    /// Return the WebSocket accept key for one client key.
    fn accept_key(key: &str) -> String {
        let mut hasher = Sha1::new();
        hasher.update(key.as_bytes());
        hasher.update(WEBSOCKET_GUID.as_bytes());
        let digest = hasher.finalize();

        general_purpose::STANDARD.encode(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::WebSocketUpgrade;

    /// Parse the complete RFC WebSocket upgrade vocabulary.
    #[test]
    fn test_parse_upgrade() {
        let request = request("13", "dGhlIHNhbXBsZSBub25jZQ==");
        let upgrade = WebSocketUpgrade::new("/rpc", "secret");

        assert_eq!(
            upgrade.parse(request.as_bytes()).expect("parse upgrade"),
            "dGhlIHNhbXBsZSBub25jZQ=="
        );
        assert_eq!(
            WebSocketUpgrade::accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    /// Reject protocol versions outside RFC 6455.
    #[test]
    fn test_reject_upgrade_version() {
        let request = request("12", "dGhlIHNhbXBsZSBub25jZQ==");
        let upgrade = WebSocketUpgrade::new("/rpc", "secret");
        let error = upgrade
            .parse(request.as_bytes())
            .expect_err("reject version");

        assert_eq!(
            error.to_string(),
            "WebSocket handshake failed: unsupported WebSocket version"
        );
    }

    /// Reject keys that are not exact 16-byte client nonces.
    #[test]
    fn test_reject_upgrade_key() {
        let request = request("13", "c2hvcnQ=");
        let upgrade = WebSocketUpgrade::new("/rpc", "secret");
        let error = upgrade.parse(request.as_bytes()).expect_err("reject key");

        assert_eq!(
            error.to_string(),
            "WebSocket handshake failed: invalid WebSocket key"
        );
    }

    /// Build one complete WebSocket upgrade request.
    fn request(version: &str, key: &str) -> String {
        format!(
            "GET /rpc?token=secret HTTP/1.1\r\n\
             Host: 127.0.0.1\r\n\
             Upgrade: websocket\r\n\
             Connection: keep-alive, Upgrade\r\n\
             Sec-WebSocket-Version: {version}\r\n\
             Sec-WebSocket-Key: {key}\r\n\
             \r\n"
        )
    }
}
