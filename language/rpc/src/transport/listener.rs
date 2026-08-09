use std::fmt::Debug;
use std::sync::Arc;

use super::Transport;

/// Blocking source of RPC transports.
pub trait Listener: Debug + Send + Sync + 'static {
    /// Listener specific failure.
    type Error;

    /// Accept one transport, or return `None` after closure.
    fn accept(&self) -> Result<Option<Arc<dyn Transport>>, Self::Error>;

    /// Close this listener and interrupt a pending accept.
    fn close(&self) -> Result<(), Self::Error>;
}
