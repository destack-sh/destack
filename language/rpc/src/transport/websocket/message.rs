use std::io::{Read, Write};
use std::net::TcpStream;

use super::WebSocketError;
use super::constants::{
    CONTROL_PAYLOAD_LIMIT, OPCODE_BINARY, OPCODE_CLOSE, OPCODE_CONTINUATION, OPCODE_PING,
    OPCODE_PONG, OPCODE_TEXT,
};
use super::handshake::WebSocketUpgrade;

/// One decoded WebSocket message.
#[derive(Debug)]
pub(super) struct WebSocketMessage {
    /// First frame opcode.
    pub opcode: u8,
    /// Decoded message payload.
    pub payload: Vec<u8>,
}

/// Stateful masked WebSocket message reader.
#[derive(Debug)]
pub(super) struct WebSocketReader {
    /// Client byte stream.
    stream: TcpStream,
    /// Fragmented data message opcode.
    opcode: Option<u8>,
    /// Fragmented data message bytes.
    payload: Vec<u8>,
    /// Largest accepted data message.
    max_message_bytes: usize,
    /// Pending authenticated HTTP upgrade.
    upgrade: Option<WebSocketUpgrade>,
}

impl WebSocketReader {
    /// Create one empty message reader.
    #[cfg(test)]
    pub(super) fn new(stream: TcpStream, max_message_bytes: usize) -> Self {
        Self {
            stream,
            opcode: None,
            payload: Vec::new(),
            max_message_bytes,
            upgrade: None,
        }
    }

    /// Create one reader that accepts its WebSocket upgrade on first input.
    pub(super) fn accept(
        stream: TcpStream,
        max_message_bytes: usize,
        upgrade: WebSocketUpgrade,
    ) -> Self {
        Self {
            stream,
            opcode: None,
            payload: Vec::new(),
            max_message_bytes,
            upgrade: Some(upgrade),
        }
    }

    /// Read one complete masked client data or control message.
    pub(super) fn read(&mut self) -> Result<WebSocketMessage, WebSocketError> {
        if let Some(upgrade) = &self.upgrade {
            upgrade.accept(&mut self.stream)?;
            self.upgrade = None;
        }

        loop {
            let remaining = self.max_message_bytes.saturating_sub(self.payload.len());
            let frame = WebSocketFrame::read(&mut self.stream, remaining)?;

            // return control frames without disturbing a fragmented data message
            if WebSocketFrame::is_control(frame.opcode) {
                return Ok(WebSocketMessage {
                    opcode: frame.opcode,
                    payload: frame.payload,
                });
            }

            // begin one data message or continue the current one
            if self.opcode.is_none() {
                if frame.opcode == OPCODE_CONTINUATION {
                    return Err(WebSocketError::UnsupportedMessage(
                        "unexpected continuation frame",
                    ));
                }
                self.opcode = Some(frame.opcode);
            } else if frame.opcode != OPCODE_CONTINUATION {
                return Err(WebSocketError::UnsupportedMessage(
                    "expected continuation frame",
                ));
            }
            self.payload.extend_from_slice(&frame.payload);

            if frame.is_final {
                let Some(opcode) = self.opcode.take() else {
                    return Err(WebSocketError::UnsupportedMessage("missing opcode"));
                };
                let payload = std::mem::take(&mut self.payload);

                return Ok(WebSocketMessage { opcode, payload });
            }
        }
    }
}

impl WebSocketMessage {
    /// Write one unmasked server message.
    pub(super) fn write(
        stream: &mut TcpStream,
        opcode: u8,
        payload: &[u8],
    ) -> Result<(), WebSocketError> {
        let mut header = Vec::with_capacity(10);
        header.push(0x80 | opcode);
        if payload.len() <= 125 {
            header.push(payload.len() as u8);
        } else if u16::try_from(payload.len()).is_ok() {
            header.push(126);
            header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            header.push(127);
            header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        stream.write_all(&header)?;
        stream.write_all(payload)?;

        Ok(())
    }
}

/// One decoded WebSocket frame.
#[derive(Debug)]
struct WebSocketFrame {
    /// Whether this frame completes its message.
    is_final: bool,
    /// Frame opcode.
    opcode: u8,
    /// Decoded frame payload.
    payload: Vec<u8>,
}

impl WebSocketFrame {
    /// Read one masked client frame.
    fn read(stream: &mut TcpStream, payload_limit: usize) -> Result<Self, WebSocketError> {
        let mut header = [0; 2];
        stream.read_exact(&mut header)?;
        let is_final = header[0] & 0x80 != 0;
        let has_reserved_bits = header[0] & 0x70 != 0;
        let opcode = header[0] & 0x0f;
        let is_masked = header[1] & 0x80 != 0;
        let length_code = header[1] & 0x7f;
        let mut length = u64::from(length_code);

        if has_reserved_bits {
            return Err(WebSocketError::UnsupportedMessage(
                "reserved frame bits are not supported",
            ));
        }
        if !matches!(
            opcode,
            OPCODE_CONTINUATION
                | OPCODE_TEXT
                | OPCODE_BINARY
                | OPCODE_CLOSE
                | OPCODE_PING
                | OPCODE_PONG
        ) {
            return Err(WebSocketError::UnsupportedMessage("unknown opcode"));
        }

        if length_code == 126 {
            let mut bytes = [0; 2];
            stream.read_exact(&mut bytes)?;
            length = u64::from(u16::from_be_bytes(bytes));
            if length < 126 {
                return Err(WebSocketError::UnsupportedMessage(
                    "noncanonical frame length",
                ));
            }
        } else if length_code == 127 {
            let mut bytes = [0; 8];
            stream.read_exact(&mut bytes)?;
            length = u64::from_be_bytes(bytes);
            if length <= u64::from(u16::MAX) || length & (1_u64 << 63) != 0 {
                return Err(WebSocketError::UnsupportedMessage(
                    "noncanonical frame length",
                ));
            }
        }

        if !is_masked {
            return Err(WebSocketError::UnsupportedMessage(
                "client frames must be masked",
            ));
        }
        if Self::is_control(opcode) && !is_final {
            return Err(WebSocketError::UnsupportedMessage(
                "control frames cannot be fragmented",
            ));
        }
        let length = usize::try_from(length).map_err(|_| WebSocketError::MessageTooLarge {
            limit: payload_limit,
            actual: usize::MAX,
        })?;
        let limit = if Self::is_control(opcode) {
            CONTROL_PAYLOAD_LIMIT
        } else {
            payload_limit
        };
        if length > limit {
            return Err(WebSocketError::MessageTooLarge {
                limit,
                actual: length,
            });
        }

        let mut mask = [0; 4];
        stream.read_exact(&mut mask)?;
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload)?;
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % mask.len()];
        }

        Ok(Self {
            is_final,
            opcode,
            payload,
        })
    }

    /// Return whether one opcode identifies a control frame.
    fn is_control(opcode: u8) -> bool {
        matches!(opcode, OPCODE_CLOSE | OPCODE_PING | OPCODE_PONG)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};

    use super::*;
    use crate::transport::websocket::constants::{OPCODE_BINARY, OPCODE_PING};

    /// Preserve fragmented data while returning an interleaved control message.
    #[test]
    fn test_read_interleaved_control_message() {
        let (mut client, server) = stream_pair();
        write_masked(&mut client, false, OPCODE_BINARY, &[1, 2]);
        write_masked(&mut client, true, OPCODE_PING, &[9]);
        write_masked(&mut client, true, OPCODE_CONTINUATION, &[3, 4]);
        let mut reader = WebSocketReader::new(server, 4);

        let ping = reader.read().expect("read ping");
        assert_eq!(ping.opcode, OPCODE_PING);
        assert_eq!(ping.payload, [9]);

        let binary = reader.read().expect("read binary message");
        assert_eq!(binary.opcode, OPCODE_BINARY);
        assert_eq!(binary.payload, [1, 2, 3, 4]);
    }

    /// Create one connected TCP stream pair.
    fn stream_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind listener");
        let address = listener.local_addr().expect("read listener address");
        let client = TcpStream::connect(address).expect("connect client");
        let (server, _) = listener.accept().expect("accept client");

        (client, server)
    }

    /// Write one masked frame with a fixed masking key.
    fn write_masked(stream: &mut TcpStream, is_final: bool, opcode: u8, payload: &[u8]) {
        let mask = [0x11, 0x22, 0x33, 0x44];
        let mut frame = Vec::with_capacity(6 + payload.len());
        frame.push(if is_final { 0x80 | opcode } else { opcode });
        frame.push(0x80 | payload.len() as u8);
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % mask.len()]),
        );
        stream.write_all(&frame).expect("write frame");
    }
}
