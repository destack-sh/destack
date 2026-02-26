use std::io;
use std::sync::Arc;

use destack_compiler::CompilerOptions;
use destack_workspace::Session;

use crate::Daemon;
use crate::protocol::{
    FrameCodec, FramedTransport, ProtocolServer, ProtocolServerError, ProtocolServerOptions,
    Transport,
};

/// Options for starting a daemon service.
#[derive(Debug, Clone, Default)]
pub struct DaemonServiceOptions {
    /// Compiler options for daemon work.
    pub compiler_options: CompilerOptions,
    /// Protocol server options.
    pub protocol: ProtocolServerOptions,
}

/// Daemon service entrypoint for protocol connections.
#[derive(Debug)]
pub struct DaemonService {
    /// The daemon backing this service.
    daemon: Arc<Daemon>,
    /// The protocol server for this service.
    server: ProtocolServer,
    /// The server options in use.
    options: DaemonServiceOptions,
}

impl DaemonService {
    /// Create a daemon service for a session with defaults.
    pub fn new(session: Arc<Session>) -> Self {
        Self::with_options(session, DaemonServiceOptions::default())
    }

    /// Create a daemon service with explicit options.
    pub fn with_options(session: Arc<Session>, options: DaemonServiceOptions) -> Self {
        let daemon = Arc::new(Daemon::with_options(
            session,
            options.compiler_options.clone(),
        ));
        let server = ProtocolServer::with_options(daemon.clone(), options.protocol.clone());
        Self {
            daemon,
            server,
            options,
        }
    }

    /// Return the daemon backing this service.
    pub fn daemon(&self) -> &Arc<Daemon> {
        &self.daemon
    }

    /// Serve protocol requests over the provided transport.
    pub fn serve_transport<T: Transport>(&self, transport: &T) -> Result<(), DaemonServiceError> {
        self.server
            .serve(transport)
            .map_err(DaemonServiceError::Protocol)
    }

    /// Serve protocol requests over stdio.
    pub fn serve_stdio(&self) -> Result<(), DaemonServiceError> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let codec = FrameCodec::new(self.options.protocol.limits.max_frame_bytes as usize);
        let transport = FramedTransport::new(stdin, stdout, codec);
        self.serve_transport(&transport)
    }
}

/// Errors returned by daemon service helpers.
#[derive(Debug)]
pub enum DaemonServiceError {
    /// Protocol server error.
    Protocol(ProtocolServerError),
}

impl std::fmt::Display for DaemonServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonServiceError::Protocol(error) => write!(f, "daemon service error: {error}"),
        }
    }
}

impl std::error::Error for DaemonServiceError {}
