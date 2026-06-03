use std::io::{Read, Write};

use crossbeam_channel::{Receiver, Sender, bounded, select};
use parking_lot::Mutex;

use super::{FrameCodec, FrameError};

/// Transport for framed protocol payloads.
pub trait Transport: Send + Sync {
    /// Send a framed payload.
    fn send(&self, payload: &[u8]) -> Result<(), TransportError>;
    /// Receive a framed payload.
    fn recv(&self) -> Result<Vec<u8>, TransportError>;
    /// Close the transport.
    fn close(&self);
}

/// Transport errors for framed payloads.
#[derive(Debug)]
pub enum TransportError {
    /// Frame codec error.
    Frame(FrameError),
    /// Io error.
    Io(std::io::Error),
    /// Transport is closed.
    Closed,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format transport errors
        match self {
            TransportError::Frame(error) => write!(f, "{error}"),
            TransportError::Io(error) => write!(f, "transport io error: {error}"),
            TransportError::Closed => write!(f, "transport closed"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<FrameError> for TransportError {
    fn from(error: FrameError) -> Self {
        TransportError::Frame(error)
    }
}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        TransportError::Io(error)
    }
}

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

/// In memory transport for tests and loopback usage.
#[derive(Debug)]
pub struct LoopbackTransport {
    /// Sender for outbound payloads.
    sender: Mutex<Option<Sender<Vec<u8>>>>,
    /// Receiver for inbound payloads.
    receiver: Mutex<Option<Receiver<Vec<u8>>>>,
}

impl LoopbackTransport {
    /// Create a new loopback transport endpoint.
    pub fn new(sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            sender: Mutex::new(Some(sender)),
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

impl Transport for LoopbackTransport {
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        // enqueue the payload
        let sender = self.sender.lock();
        let sender = sender.as_ref().ok_or(TransportError::Closed)?;
        sender
            .send(payload.to_vec())
            .map_err(|_| TransportError::Closed)
    }

    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        // receive the payload
        let receiver = self.receiver.lock();
        let receiver = receiver.as_ref().ok_or(TransportError::Closed)?;
        receiver.recv().map_err(|_| TransportError::Closed)
    }

    fn close(&self) {
        // drop sender and receiver handles
        let mut sender = self.sender.lock();
        let mut receiver = self.receiver.lock();
        let _ = sender.take();
        let _ = receiver.take();
    }
}

/// Create a pair of loopback transports.
pub fn loopback_transport_pair(capacity: usize) -> (LoopbackTransport, LoopbackTransport) {
    // create bounded channels for each direction
    let (tx_a, rx_a) = bounded(capacity);
    let (tx_b, rx_b) = bounded(capacity);

    let a = LoopbackTransport::new(tx_a, rx_b);
    let b = LoopbackTransport::new(tx_b, rx_a);

    (a, b)
}

/// Buffered transport wrapper with bounded queues.
pub struct BufferedTransport {
    /// Inner transport implementation.
    inner: std::sync::Arc<dyn Transport>,
    /// Sender for outbound payloads.
    outbound: Mutex<Option<Sender<Vec<u8>>>>,
    /// Sender for shutdown signals.
    shutdown: Sender<()>,
}

impl std::fmt::Debug for BufferedTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferedTransport").finish()
    }
}

impl BufferedTransport {
    /// Create a buffered transport with bounded queues.
    pub fn new(inner: std::sync::Arc<dyn Transport>, outbound_capacity: usize) -> Self {
        // allocate bounded queues
        let (outbound_tx, outbound_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
            bounded(outbound_capacity);
        let (shutdown_tx, shutdown_rx) = bounded(1);

        // spawn writer loop
        let writer_inner = inner.clone();
        let writer_shutdown = shutdown_rx.clone();
        std::thread::spawn(move || {
            // writer loop
            loop {
                let payload = select! {
                    recv(writer_shutdown) -> _ => break,
                    recv(outbound_rx) -> message => match message {
                        Ok(payload) => payload,
                        Err(_) => break,
                    },
                };

                if writer_inner.send(&payload).is_err() {
                    break;
                }
            }
        });

        Self {
            inner,
            outbound: Mutex::new(Some(outbound_tx)),
            shutdown: shutdown_tx,
        }
    }
}

impl Transport for BufferedTransport {
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        // enqueue outbound payload
        let outbound = self.outbound.lock();
        let outbound = outbound.as_ref().ok_or(TransportError::Closed)?;
        outbound
            .send(payload.to_vec())
            .map_err(|_| TransportError::Closed)
    }

    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        // receive inbound payload directly
        self.inner.recv()
    }

    fn close(&self) {
        // signal shutdown and close inner transport
        let mut outbound = self.outbound.lock();
        let _ = outbound.take();
        let _ = self.shutdown.send(());
        self.inner.close();
    }
}
