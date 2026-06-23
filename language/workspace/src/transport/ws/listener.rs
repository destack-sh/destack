use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

use crate::Transport;

use super::WebSocketError;
use super::handshake::accept_handshake;
use super::transport::websocket_transport;

/// Listener for workspace WebSocket connections.
#[derive(Debug)]
pub struct WebSocketListener {
    /// Underlying TCP listener.
    listener: TcpListener,
    /// Maximum protocol frame size for transports.
    max_frame_bytes: usize,
    /// Required request path for browser connections.
    path: String,
    /// Required bearer token for browser connections.
    token: String,
}

impl WebSocketListener {
    /// Bind a listener to a socket address.
    pub fn bind(
        addr: SocketAddr,
        max_frame_bytes: usize,
        path: String,
        token: String,
    ) -> Result<Self, WebSocketError> {
        // bind and configure the listener
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            listener,
            max_frame_bytes,
            path,
            token,
        })
    }

    /// Return the bound listener address.
    pub fn local_addr(&self) -> Result<SocketAddr, WebSocketError> {
        self.listener.local_addr().map_err(WebSocketError::Io)
    }

    /// Accept an incoming WebSocket connection when one is ready.
    pub fn try_accept(&self) -> Result<Option<Arc<dyn Transport>>, WebSocketError> {
        // accept the next TCP stream
        let (mut stream, _) = match self.listener.accept() {
            Ok(accepted) => accepted,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(WebSocketError::Io(error)),
        };

        // complete the WebSocket upgrade
        stream.set_nonblocking(false)?;
        accept_handshake(&mut stream, &self.path, &self.token)?;

        Ok(Some(websocket_transport(stream, self.max_frame_bytes)))
    }
}
