use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;

use super::IpcError;
use super::frame::{FRAME_HEADER_BYTES, Frame};
use super::platform::{
    IpcStream, clone_stream, configure_stream, connect_stream, interrupt_stream,
};
use crate::{Transport, TransportError};

/// Maximum time a local receive blocks before observing closure.
const RECEIVE_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Framed RPC transport over one local byte stream.
#[derive(Debug)]
pub struct IpcTransport {
    /// Framed message reader.
    reader: Mutex<IpcStream>,
    /// Framed message writer.
    writer: Mutex<IpcStream>,
    /// Stream used to interrupt blocking reads.
    control: IpcStream,
    /// Largest accepted message.
    max_message_bytes: usize,
    /// Whether closure was requested.
    is_closed: AtomicBool,
}

impl IpcTransport {
    /// Connect one framed local RPC transport.
    pub fn connect(path: &Path, max_message_bytes: usize) -> Result<Arc<dyn Transport>, IpcError> {
        let stream = connect_stream(path)?;
        let transport = Self::new(stream, max_message_bytes)?;

        Ok(transport)
    }

    /// Create one framed local RPC transport.
    pub(super) fn new(stream: IpcStream, max_message_bytes: usize) -> Result<Arc<Self>, IpcError> {
        configure_stream(&stream, RECEIVE_POLL_INTERVAL)?;
        let writer = clone_stream(&stream)?;
        let control = clone_stream(&stream)?;
        let transport = Self {
            reader: Mutex::new(stream),
            writer: Mutex::new(writer),
            control,
            max_message_bytes,
            is_closed: AtomicBool::new(false),
        };

        Ok(Arc::new(transport))
    }

    /// Read one exact byte sequence while observing closure.
    fn read_exact(&self, reader: &mut IpcStream, bytes: &mut [u8]) -> Result<(), TransportError> {
        let mut offset = 0;

        while offset < bytes.len() {
            if self.is_closed.load(Ordering::Acquire) {
                return Err(TransportError::Closed);
            }
            match reader.read(&mut bytes[offset..]) {
                Ok(0) => return Err(TransportError::Closed),
                Ok(read) => offset += read,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::Interrupted
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => return Err(error.into()),
            }
        }

        Ok(())
    }
}

impl Transport for IpcTransport {
    /// Send one framed message.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        if self.is_closed.load(Ordering::Acquire) {
            return Err(TransportError::Closed);
        }
        if message.len() > self.max_message_bytes {
            return Err(TransportError::MessageTooLarge {
                limit: self.max_message_bytes,
                actual: message.len(),
            });
        }

        let frame = Frame::new(message.len())?;
        let header = frame.encode();
        let mut writer = self.writer.lock();
        writer.write_all(&header)?;
        writer.write_all(message)?;
        writer.flush()?;

        Ok(())
    }

    /// Receive one framed message.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        let mut reader = self.reader.lock();
        let mut header = [0; FRAME_HEADER_BYTES];
        self.read_exact(&mut reader, &mut header)?;
        let frame = Frame::decode(header)?;
        let message_bytes = frame.message_bytes() as usize;
        if message_bytes > self.max_message_bytes {
            return Err(TransportError::MessageTooLarge {
                limit: self.max_message_bytes,
                actual: message_bytes,
            });
        }
        let mut message = vec![0; message_bytes];
        self.read_exact(&mut reader, &mut message)?;

        Ok(message)
    }

    /// Close this local connection and interrupt pending input.
    fn close(&self) -> Result<(), TransportError> {
        if self.is_closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        interrupt_stream(&self.control).map_err(|error| match error {
            IpcError::Io(error) => TransportError::Io(error),
            IpcError::InvalidPath(path) => TransportError::Io(std::io::Error::other(format!(
                "invalid IPC path during close: {}",
                path.display()
            ))),
        })
    }
}
