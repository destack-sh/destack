use super::TransportError;

/// Transport for framed protocol payloads.
pub trait Transport: Send + Sync {
    /// Send a framed payload.
    fn send(&self, payload: &[u8]) -> Result<(), TransportError>;
    /// Receive a framed payload.
    fn recv(&self) -> Result<Vec<u8>, TransportError>;
    /// Close the transport.
    fn close(&self);
}
