use super::TransportError;

/// Ordered reliable transport preserving binary message boundaries.
pub trait Transport: std::fmt::Debug + Send + Sync + 'static {
    /// Send one complete binary message.
    fn send(&self, message: &[u8]) -> Result<(), TransportError>;

    /// Receive one complete binary message.
    fn receive(&self) -> Result<Vec<u8>, TransportError>;

    /// Close this transport.
    fn close(&self) -> Result<(), TransportError>;
}
