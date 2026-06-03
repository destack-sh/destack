use std::io::Write;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::protocol::{FrameCodec, Transport, TransportError};

use super::platform::IpcStream;

/// Build a protocol transport over an ipc stream.
pub(super) fn ipc_transport(stream: IpcStream, max_frame_bytes: usize) -> Arc<dyn Transport> {
    Arc::new(IpcTransport::new(stream, max_frame_bytes))
}

/// Transport backed by an ipc stream.
#[derive(Debug)]
struct IpcTransport {
    /// Stream used for message exchange.
    stream: Mutex<Option<IpcStream>>,
    /// Codec for framing messages.
    codec: FrameCodec,
}

impl IpcTransport {
    /// Build a transport from a stream and codec limits.
    fn new(stream: IpcStream, max_frame_bytes: usize) -> Self {
        Self {
            stream: Mutex::new(Some(stream)),
            codec: FrameCodec::new(max_frame_bytes),
        }
    }
}

impl Transport for IpcTransport {
    /// Send a payload over the transport.
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        // lock the stream for this send
        let mut stream = self.stream.lock();
        let stream = stream.as_mut().ok_or(TransportError::Closed)?;

        // write the frame and flush
        self.codec.write_to(stream, payload)?;
        stream.flush()?;

        Ok(())
    }

    /// Receive a payload from the transport.
    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        // lock the stream for this receive
        let mut stream = self.stream.lock();
        let stream = stream.as_mut().ok_or(TransportError::Closed)?;

        // read the next frame
        let payload = self.codec.read_from(stream)?;

        Ok(payload)
    }

    /// Close the transport.
    fn close(&self) {
        let mut stream = self.stream.lock();
        let _ = stream.take();
    }
}
