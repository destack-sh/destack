use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::Mutex;

use super::{Transport, TransportError};

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
