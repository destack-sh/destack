use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, Sender, bounded, select};

use crate::{Transport, TransportError};

/// One endpoint of an ordered in-memory transport pair.
#[derive(Debug)]
pub struct LoopbackTransport {
    /// Outbound message sender.
    sender: Sender<Vec<u8>>,
    /// Inbound message receiver.
    receiver: Receiver<Vec<u8>>,
    /// Local closure sender.
    close_sender: Sender<()>,
    /// Local closure receiver.
    close_receiver: Receiver<()>,
    /// Peer closure sender.
    peer_close_sender: Sender<()>,
    /// Whether closure was requested.
    is_closed: AtomicBool,
}

impl LoopbackTransport {
    /// Create one bounded ordered in-memory transport pair.
    pub fn pair(capacity: usize) -> (Self, Self) {
        let (left_sender, left_receiver) = bounded(capacity);
        let (right_sender, right_receiver) = bounded(capacity);
        let (left_close_sender, left_close_receiver) = bounded(1);
        let (right_close_sender, right_close_receiver) = bounded(1);
        let left = Self::new(
            left_sender,
            right_receiver,
            left_close_sender.clone(),
            left_close_receiver,
            right_close_sender.clone(),
        );
        let right = Self::new(
            right_sender,
            left_receiver,
            right_close_sender,
            right_close_receiver,
            left_close_sender,
        );

        (left, right)
    }

    /// Create one endpoint from explicit channels.
    fn new(
        sender: Sender<Vec<u8>>,
        receiver: Receiver<Vec<u8>>,
        close_sender: Sender<()>,
        close_receiver: Receiver<()>,
        peer_close_sender: Sender<()>,
    ) -> Self {
        Self {
            sender,
            receiver,
            close_sender,
            close_receiver,
            peer_close_sender,
            is_closed: AtomicBool::new(false),
        }
    }
}

impl Transport for LoopbackTransport {
    /// Send one complete in-memory message.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        if self.is_closed.load(Ordering::Acquire) {
            return Err(TransportError::Closed);
        }
        self.sender
            .send(message.to_vec())
            .map_err(|_| TransportError::Closed)
    }

    /// Receive one complete in-memory message.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        select! {
            recv(self.receiver) -> message => message.map_err(|_| TransportError::Closed),
            recv(self.close_receiver) -> _ => Err(TransportError::Closed),
        }
    }

    /// Close this in-memory endpoint and wake pending input.
    fn close(&self) -> Result<(), TransportError> {
        if !self.is_closed.swap(true, Ordering::AcqRel) {
            let _notified = self.close_sender.try_send(());
            let _peer_notified = self.peer_close_sender.try_send(());
        }

        Ok(())
    }
}
