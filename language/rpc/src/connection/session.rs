use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use super::server::{Server, ServerConnection};
use super::{ConnectionError, HostWaker, MessageReceiver, MessageSender, ServerError};
use crate::protocol::{Handshake, HandshakeCodec, HandshakeResponse, MessageCodec};
use crate::{ConnectionOptions, Registry, Transport, TransportError};

/// In-process RPC server session driven by explicit binary messages.
#[derive(Debug)]
pub struct Session {
    /// Server providing negotiation and services.
    server: Server,
    /// Outbound message transport.
    transport: Arc<SessionTransport>,
    /// Callback used to wake the embedding host.
    host_waker: HostWaker,
    /// Current negotiation or call state.
    state: Mutex<SessionState>,
}

impl Session {
    /// Create one unnegotiated in-process session.
    pub fn new(services: Registry, options: ConnectionOptions) -> Result<Self, ServerError> {
        options
            .limits
            .validate()
            .map_err(ConnectionError::Rejected)?;
        let transport = Arc::new(SessionTransport::default());
        let host_waker = HostWaker::default();
        let server = Server::new(services, options);

        Ok(Self {
            server,
            transport,
            host_waker,
            state: Mutex::new(SessionState::Handshake),
        })
    }

    /// Install the callback invoked when a cooperative call becomes ready.
    pub fn set_wake_handler(&self, handler: Arc<dyn Fn() + Send + Sync + 'static>) {
        self.host_waker.set(handler);
    }

    /// Dispatch one complete inbound message and return all resulting outbound messages.
    pub fn dispatch(&self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, ServerError> {
        let result = self.dispatch_message(bytes);
        match result {
            Ok(messages) => Ok(messages),
            Err(error) => {
                self.terminate();

                Err(error)
            }
        }
    }

    /// Poll cooperatively ready calls and return their outbound messages.
    pub fn poll(&self) -> Result<Vec<Vec<u8>>, ServerError> {
        let result = self.poll_ready();
        match result {
            Ok(messages) => Ok(messages),
            Err(error) => {
                self.terminate();

                Err(error)
            }
        }
    }

    /// Return whether cooperative calls requested another poll.
    pub fn is_ready(&self) -> Result<bool, ServerError> {
        let state = self.state.lock();
        match &*state {
            SessionState::Handshake => Err(ConnectionError::Protocol(
                "cannot inspect an unnegotiated session".to_string(),
            )
            .into()),
            SessionState::Connected { connection, .. } => Ok(connection.is_ready()),
            SessionState::Closed => Err(ConnectionError::Closed.into()),
        }
    }

    /// Close this session and cancel every active request.
    pub fn close(&self) -> Result<(), ServerError> {
        self.terminate();

        Ok(())
    }

    /// Dispatch one message while the session remains healthy.
    fn dispatch_message(&self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, ServerError> {
        self.host_waker.acknowledge();
        let routed = {
            let mut state = self.state.lock();
            match &mut *state {
                SessionState::Handshake => {
                    self.handshake(bytes, &mut state)?;

                    None
                }
                SessionState::Connected {
                    receiver,
                    connection,
                } => receiver
                    .push(bytes)?
                    .map(|message| (connection.clone(), message)),
                SessionState::Closed => return Err(ConnectionError::Closed.into()),
            }
        };

        // poll without holding session state so wake callbacks may re-enter safely
        if let Some((connection, message)) = routed {
            connection.route(message)?;
            connection.poll()?;
        }

        Ok(self.transport.take())
    }

    /// Poll ready calls while the session remains healthy.
    fn poll_ready(&self) -> Result<Vec<Vec<u8>>, ServerError> {
        self.host_waker.acknowledge();
        let connection = match &*self.state.lock() {
            SessionState::Handshake => {
                return Err(ConnectionError::Protocol(
                    "cannot poll an unnegotiated session".to_string(),
                )
                .into());
            }
            SessionState::Connected { connection, .. } => connection.clone(),
            SessionState::Closed => return Err(ConnectionError::Closed.into()),
        };
        connection.poll()?;

        Ok(self.transport.take())
    }

    /// Terminate this session and cancel every active request.
    fn terminate(&self) {
        self.host_waker.clear();
        let connection = {
            let mut state = self.state.lock();
            let connection = match &*state {
                SessionState::Connected { connection, .. } => Some(connection.clone()),
                SessionState::Handshake | SessionState::Closed => None,
            };
            *state = SessionState::Closed;

            connection
        };
        if let Some(connection) = connection {
            connection.disconnect();
        }
        self.transport.shutdown();
    }

    /// Negotiate one frozen handshake and construct the active message connection.
    fn handshake(&self, bytes: &[u8], state: &mut SessionState) -> Result<(), ServerError> {
        let initial_limit =
            usize::try_from(self.server.limits().max_message_bytes).map_err(|_| {
                ConnectionError::Protocol(
                    "local message limit does not fit this platform".to_string(),
                )
            })?;
        let handshake_codec = HandshakeCodec::new(initial_limit);
        let handshake = handshake_codec
            .decode(bytes)
            .map_err(ConnectionError::from)?;
        let Handshake::Request(request) = handshake else {
            return Err(ConnectionError::Protocol("expected handshake request".to_string()).into());
        };

        // negotiate the grammar, limits, and requested services together
        let accepted = self.server.negotiate(&request);
        let (version, limits, offers) = match accepted {
            Ok(accepted) => accepted,
            Err(status) => {
                let response = HandshakeResponse::Rejected {
                    peer: self.server.peer().clone(),
                    status,
                };
                let bytes = handshake_codec
                    .encode(&Handshake::Response(response))
                    .map_err(ConnectionError::from)?;
                self.transport.send(&bytes).map_err(ConnectionError::from)?;
                *state = SessionState::Closed;
                self.transport.shutdown();

                return Ok(());
            }
        };

        // build the exact negotiated message codecs
        let message_limit = usize::try_from(limits.max_message_bytes).map_err(|_| {
            ConnectionError::Protocol(
                "negotiated message limit does not fit this platform".to_string(),
            )
        })?;
        let payload_limit = usize::try_from(limits.max_payload_bytes).map_err(|_| {
            ConnectionError::Protocol(
                "negotiated payload limit does not fit this platform".to_string(),
            )
        })?;
        let response = HandshakeResponse::Accepted {
            version,
            limits,
            peer: self.server.peer().clone(),
            services: offers.clone(),
        };
        let bytes = HandshakeCodec::new(message_limit)
            .encode(&Handshake::Response(response))
            .map_err(ConnectionError::from)?;
        self.transport.send(&bytes).map_err(ConnectionError::from)?;

        // route messages through the negotiated service methods
        let transport: Arc<dyn Transport> = self.transport.clone();
        let codec = MessageCodec::new(version, message_limit);
        let receiver = MessageReceiver::new(codec.clone(), payload_limit, transport.clone());
        let sender = MessageSender::new(codec, payload_limit, transport);
        let connection =
            Arc::new(
                self.server
                    .open(sender, offers, limits, self.host_waker.clone()),
            );
        *state = SessionState::Connected {
            receiver,
            connection,
        };

        Ok(())
    }
}

impl Drop for Session {
    /// Cancel active requests when this session is abandoned.
    fn drop(&mut self) {
        self.host_waker.clear();
        if let SessionState::Connected { connection, .. } = self.state.get_mut() {
            connection.disconnect();
        }
        self.transport.shutdown();
    }
}

/// Current state of one in-process session.
#[derive(Debug)]
enum SessionState {
    /// Waiting for one handshake request.
    Handshake,
    /// Negotiated and routing call messages.
    Connected {
        /// Deferred payload assembler.
        receiver: MessageReceiver,
        /// Service call router.
        connection: Arc<ServerConnection>,
    },
    /// Permanently closed.
    Closed,
}

/// Queue-backed outbound transport for one in-process session.
#[derive(Debug, Default)]
struct SessionTransport {
    /// Complete outbound binary messages.
    messages: Mutex<Vec<Vec<u8>>>,
    /// Whether the owning session has closed.
    is_closed: AtomicBool,
}

impl SessionTransport {
    /// Drain every complete outbound message in send order.
    fn take(&self) -> Vec<Vec<u8>> {
        std::mem::take(&mut *self.messages.lock())
    }

    /// Close this outbound queue.
    fn shutdown(&self) {
        self.is_closed.store(true, Ordering::Release);
    }
}

impl Transport for SessionTransport {
    /// Queue one complete outbound message.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        if self.is_closed.load(Ordering::Acquire) {
            return Err(TransportError::Closed);
        }

        self.messages.lock().push(message.to_vec());

        Ok(())
    }

    /// Reject blocking input because dispatch supplies messages explicitly.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Closed)
    }

    /// Close this outbound queue.
    fn close(&self) -> Result<(), TransportError> {
        self.shutdown();

        Ok(())
    }
}
