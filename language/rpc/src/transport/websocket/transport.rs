use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use super::WebSocketError;
use super::constants::{OPCODE_BINARY, OPCODE_CLOSE, OPCODE_PING, OPCODE_PONG, OPCODE_TEXT};
use super::handshake::WebSocketUpgrade;
use super::message::{WebSocketMessage, WebSocketReader};
use crate::{Transport, TransportError};

/// RPC transport over WebSocket binary messages.
#[derive(Debug)]
pub struct WebSocketTransport {
    /// WebSocket message reader.
    reader: Mutex<WebSocketReader>,
    /// WebSocket message writer.
    writer: Mutex<TcpStream>,
    /// Stream used to interrupt blocking input during close.
    control: TcpStream,
    /// Largest accepted message.
    max_message_bytes: usize,
    /// Whether closure was requested.
    is_closed: AtomicBool,
}

impl WebSocketTransport {
    /// Create one WebSocket RPC transport.
    pub(super) fn new(
        stream: TcpStream,
        max_message_bytes: usize,
        path: String,
        token: String,
    ) -> Result<Arc<Self>, WebSocketError> {
        let writer = stream.try_clone()?;
        let control = stream.try_clone()?;
        let transport = Self {
            reader: Mutex::new(WebSocketReader::accept(
                stream,
                max_message_bytes,
                WebSocketUpgrade::new(path, token),
            )),
            writer: Mutex::new(writer),
            control,
            max_message_bytes,
            is_closed: AtomicBool::new(false),
        };

        Ok(Arc::new(transport))
    }

    /// Validate one message byte length.
    fn validate(&self, actual: usize) -> Result<(), TransportError> {
        if actual > self.max_message_bytes {
            Err(TransportError::MessageTooLarge {
                limit: self.max_message_bytes,
                actual,
            })
        } else {
            Ok(())
        }
    }
}

impl Transport for WebSocketTransport {
    /// Send one binary WebSocket message.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        if self.is_closed.load(Ordering::Acquire) {
            return Err(TransportError::Closed);
        }
        self.validate(message.len())?;
        let mut writer = self.writer.lock();
        WebSocketMessage::write(&mut writer, OPCODE_BINARY, message)
            .map_err(TransportError::from)?;
        writer.flush()?;

        Ok(())
    }

    /// Receive one binary WebSocket message.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        let mut reader = self.reader.lock();

        loop {
            let message = reader.read().map_err(|error| match error {
                WebSocketError::Io(error) => TransportError::from_receive(error),
                error => TransportError::from(error),
            })?;
            match message.opcode {
                OPCODE_BINARY => return Ok(message.payload),
                OPCODE_CLOSE => {
                    return Err(TransportError::Closed);
                }
                OPCODE_PING => {
                    let mut writer = self.writer.lock();
                    WebSocketMessage::write(&mut writer, OPCODE_PONG, &message.payload)
                        .map_err(TransportError::from)?;
                    writer.flush()?;
                }
                OPCODE_PONG => {}
                OPCODE_TEXT => {
                    return Err(WebSocketError::UnsupportedMessage(
                        "text messages are not supported",
                    )
                    .into());
                }
                _ => {
                    return Err(WebSocketError::UnsupportedMessage("unknown opcode").into());
                }
            }
        }
    }

    /// Close this WebSocket connection.
    fn close(&self) -> Result<(), TransportError> {
        if self.is_closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let mut writer = self.writer.lock();
        let write = WebSocketMessage::write(&mut writer, OPCODE_CLOSE, &[])
            .and_then(|()| writer.flush().map_err(WebSocketError::from));
        let shutdown = self.control.shutdown(Shutdown::Both);

        match (write, shutdown) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error.into()),
            (Ok(()), Err(error)) => Err(error.into()),
            (Err(write), Err(shutdown)) => Err(std::io::Error::other(format!(
                "WebSocket close failed: {write}; socket shutdown failed: {shutdown}"
            ))
            .into()),
        }
    }
}

impl From<WebSocketError> for TransportError {
    /// Convert one WebSocket transport failure.
    fn from(error: WebSocketError) -> Self {
        match error {
            WebSocketError::Io(error) => Self::Io(error),
            error => Self::Io(std::io::Error::other(error)),
        }
    }
}
