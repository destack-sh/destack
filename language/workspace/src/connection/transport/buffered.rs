use crossbeam_channel::{Receiver, Sender, bounded, select};
use parking_lot::Mutex;

use super::{Transport, TransportError};

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
