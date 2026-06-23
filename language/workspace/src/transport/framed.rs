use std::io::{Read, Write};

use parking_lot::Mutex;

use super::{Transport, TransportError};
use crate::protocol::FrameCodec;

/// Framed transport backed by reader and writer streams.
#[derive(Debug)]
pub struct FramedTransport<R, W> {
    /// The reader for framed payloads.
    reader: Mutex<Option<R>>,
    /// The writer for framed payloads.
    writer: Mutex<Option<W>>,
    /// The frame codec to use.
    codec: FrameCodec,
}

impl<R, W> FramedTransport<R, W>
where
    R: Read + Send,
    W: Write + Send,
{
    /// Create a new framed transport.
    pub fn new(reader: R, writer: W, codec: FrameCodec) -> Self {
        Self {
            reader: Mutex::new(Some(reader)),
            writer: Mutex::new(Some(writer)),
            codec,
        }
    }
}

impl<R, W> Transport for FramedTransport<R, W>
where
    R: Read + Send,
    W: Write + Send,
{
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        // encode and write the frame
        let mut writer = self.writer.lock();
        let writer = writer.as_mut().ok_or(TransportError::Closed)?;
        self.codec.write_to(writer, payload)?;
        writer.flush()?;

        Ok(())
    }

    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        // read and decode the frame
        let mut reader = self.reader.lock();
        let reader = reader.as_mut().ok_or(TransportError::Closed)?;
        let payload = self.codec.read_from(reader)?;

        Ok(payload)
    }

    fn close(&self) {
        // drop reader and writer handles
        let mut reader = self.reader.lock();
        let mut writer = self.writer.lock();
        let _ = reader.take();
        let _ = writer.take();
    }
}
