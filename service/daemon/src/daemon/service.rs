use std::io;
use std::sync::Arc;

use destack_compiler::CompilerOptions;
use destack_session::{SessionEventHandler, SessionObservationHandler};
use destack_workspace::Repository;

use crate::Daemon;
use crate::protocol::{
    FrameCodec, FramedTransport, ProtocolServer, ProtocolServerError, ProtocolServerOptions,
    Transport,
};

/// Options for starting a daemon service.
#[derive(Clone, Default)]
pub struct DaemonServiceOptions {
    /// Compiler options for daemon work.
    pub compiler_options: CompilerOptions,
    /// Optional session event handler for in process progress.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Optional session observation handler for in process instrumentation.
    pub session_observation_handler: Option<SessionObservationHandler>,
    /// Protocol server options.
    pub protocol: ProtocolServerOptions,
}

impl std::fmt::Debug for DaemonServiceOptions {
    /// Format the visible daemon service options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DaemonServiceOptions")
            .field("compiler_options", &self.compiler_options)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field(
                "session_observation_handler",
                &self.session_observation_handler.is_some(),
            )
            .field("protocol", &self.protocol)
            .finish()
    }
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
    /// Create a daemon service for a repository with defaults.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self::with_options(repository, DaemonServiceOptions::default())
    }

    /// Create a daemon service with explicit options.
    pub fn with_options(repository: Arc<Repository>, options: DaemonServiceOptions) -> Self {
        let daemon = Arc::new(Daemon::with_options(
            repository,
            options.compiler_options.clone(),
            options.session_event_handler.clone(),
            options.session_observation_handler.clone(),
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
