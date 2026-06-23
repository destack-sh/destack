use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::protocol::FrameCodec;
use crate::{Transport, TransportError};

use super::WebSocketError;
use super::constants::{
    CONTROL_PAYLOAD_LIMIT, OPCODE_BINARY, OPCODE_CLOSE, OPCODE_CONTINUATION, OPCODE_PING,
    OPCODE_PONG, OPCODE_TEXT,
};

/// Build a protocol transport over a WebSocket stream.
pub(super) fn websocket_transport(stream: TcpStream, max_frame_bytes: usize) -> Arc<dyn Transport> {
    Arc::new(WebSocketTransport::new(stream, max_frame_bytes))
}

/// Transport backed by a WebSocket stream.
#[derive(Debug)]
struct WebSocketTransport {
    /// Stream used for message exchange.
    stream: Mutex<Option<TcpStream>>,
    /// Codec for protocol frame payloads.
    codec: FrameCodec,
}

impl WebSocketTransport {
    /// Build a WebSocket transport from a TCP stream.
    fn new(stream: TcpStream, max_frame_bytes: usize) -> Self {
        Self {
            stream: Mutex::new(Some(stream)),
            codec: FrameCodec::new(max_frame_bytes),
        }
    }
}

impl Transport for WebSocketTransport {
    /// Send a payload over the WebSocket transport.
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        // lock the stream for this send
        let mut stream = self.stream.lock();
        let stream = stream.as_mut().ok_or(TransportError::Closed)?;

        // encode protocol payload into one WebSocket binary message
        let frame = self.codec.encode(payload)?;
        WebSocketMessage::write(stream, OPCODE_BINARY, &frame)?;
        stream.flush()?;

        Ok(())
    }

    /// Receive a payload from the WebSocket transport.
    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        // lock the stream for this receive
        let mut stream = self.stream.lock();
        let stream = stream.as_mut().ok_or(TransportError::Closed)?;

        loop {
            // read one WebSocket message
            let message = WebSocketMessage::read(stream)?;

            // decode protocol frame payloads from binary messages
            match message.opcode {
                OPCODE_BINARY => return self.codec.decode(&message.payload).map_err(Into::into),
                OPCODE_CLOSE => {
                    let _ = WebSocketMessage::write(stream, OPCODE_CLOSE, &[]);
                    return Err(TransportError::Closed);
                }
                OPCODE_PING => {
                    WebSocketMessage::write(stream, OPCODE_PONG, &message.payload)?;
                    stream.flush()?;
                }
                OPCODE_PONG => {}
                OPCODE_TEXT => {
                    return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                        "text frames are not supported",
                    )));
                }
                _ => {
                    return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                        "unknown opcode",
                    )));
                }
            }
        }
    }

    /// Close the transport.
    fn close(&self) {
        let mut stream = self.stream.lock();
        if let Some(mut stream) = stream.take() {
            let _ = WebSocketMessage::write(&mut stream, OPCODE_CLOSE, &[]);
            let _ = stream.flush();
        }
    }
}

/// One decoded WebSocket message.
#[derive(Debug)]
struct WebSocketMessage {
    /// WebSocket opcode.
    opcode: u8,
    /// Decoded payload bytes.
    payload: Vec<u8>,
}

impl WebSocketMessage {
    /// Read one WebSocket message.
    fn read(stream: &mut TcpStream) -> Result<Self, TransportError> {
        let mut payload = Vec::new();
        let mut message_opcode = None;

        loop {
            // read the next frame
            let frame = WebSocketFrame::read(stream)?;

            // initialize message opcode from the first frame
            if message_opcode.is_none() {
                if frame.opcode == OPCODE_CONTINUATION {
                    return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                        "unexpected continuation frame",
                    )));
                }
                message_opcode = Some(frame.opcode);
            } else if frame.opcode != OPCODE_CONTINUATION {
                return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                    "expected continuation frame",
                )));
            }

            // append frame payload
            payload.extend_from_slice(&frame.payload);

            if frame.fin {
                // resolve the opcode captured from the first frame
                let Some(opcode) = message_opcode else {
                    return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                        "missing message opcode",
                    )));
                };

                return Ok(Self { opcode, payload });
            }
        }
    }

    /// Write one unmasked server WebSocket message.
    fn write(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<(), TransportError> {
        // write fixed header and payload length
        let mut header = Vec::with_capacity(10);
        header.push(0x80 | opcode);
        if payload.len() <= 125 {
            header.push(payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            header.push(126);
            header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            header.push(127);
            header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }

        // write the frame bytes
        stream.write_all(&header)?;
        stream.write_all(payload)?;

        Ok(())
    }
}

/// One decoded WebSocket frame.
#[derive(Debug)]
struct WebSocketFrame {
    /// Whether this is the final frame for a message.
    fin: bool,
    /// WebSocket opcode.
    opcode: u8,
    /// Decoded payload bytes.
    payload: Vec<u8>,
}

impl WebSocketFrame {
    /// Read one WebSocket frame.
    fn read(stream: &mut TcpStream) -> Result<Self, TransportError> {
        // read the fixed header
        let mut header = [0u8; 2];
        stream.read_exact(&mut header)?;
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0f;
        let is_masked = header[1] & 0x80 != 0;
        let mut len = u64::from(header[1] & 0x7f);

        // read extended length
        if len == 126 {
            let mut bytes = [0u8; 2];
            stream.read_exact(&mut bytes)?;
            len = u64::from(u16::from_be_bytes(bytes));
        } else if len == 127 {
            let mut bytes = [0u8; 8];
            stream.read_exact(&mut bytes)?;
            len = u64::from_be_bytes(bytes);
        }

        // validate client mask and payload size
        if !is_masked {
            return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                "client frames must be masked",
            )));
        }
        if Self::is_control_opcode(opcode) && !fin {
            return Err(TransportError::from(WebSocketError::UnsupportedFrame(
                "control frames must not be fragmented",
            )));
        }
        let len = usize::try_from(len).map_err(|_| {
            TransportError::from(WebSocketError::PayloadTooLarge {
                limit: usize::MAX,
                actual: usize::MAX,
            })
        })?;
        if Self::is_control_opcode(opcode) && len > CONTROL_PAYLOAD_LIMIT {
            return Err(TransportError::from(WebSocketError::PayloadTooLarge {
                limit: CONTROL_PAYLOAD_LIMIT,
                actual: len,
            }));
        }

        // read and unmask payload
        let mut mask = [0u8; 4];
        stream.read_exact(&mut mask)?;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload)?;
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }

        Ok(Self {
            fin,
            opcode,
            payload,
        })
    }

    /// Return whether an opcode is a control frame opcode.
    fn is_control_opcode(opcode: u8) -> bool {
        matches!(opcode, OPCODE_CLOSE | OPCODE_PING | OPCODE_PONG)
    }
}

impl From<WebSocketError> for TransportError {
    /// Convert a WebSocket error into a transport error.
    fn from(error: WebSocketError) -> Self {
        match error {
            WebSocketError::Frame(error) => TransportError::Frame(error),
            WebSocketError::Io(error) => TransportError::Io(error),
            WebSocketError::Closed => TransportError::Closed,
            error => TransportError::Io(std::io::Error::other(error)),
        }
    }
}
