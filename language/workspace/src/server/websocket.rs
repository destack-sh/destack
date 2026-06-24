use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::{
    Error, LocalWorkspace, RootLease, Server, ServerLifecycle, ServerOptions, Service,
    ServiceError, WebSocketError, WebSocketListener, Workspace,
};

const SERVER_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Workspace protocol server bound to a WebSocket listener.
#[derive(Debug)]
pub struct WebSocketServer {
    /// WebSocket URL for remote workspace clients.
    url: String,
    /// Server lifecycle shared with accepted connections.
    lifecycle: ServerLifecycle,
    /// Accept loop thread.
    handle: Option<JoinHandle<()>>,
}

impl WebSocketServer {
    /// Open a WebSocket server for one local workspace root.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, WebSocketServerError> {
        let service = Service::new(std::env::temp_dir());
        let token = service.create_websocket_token()?;
        let options = ServerOptions::default();
        let max_frame_bytes = options.limits.max_frame_bytes as usize;
        let listener = WebSocketListener::bind(
            service.websocket_addr,
            max_frame_bytes,
            service.websocket_path.clone(),
            token.clone(),
        )?;
        let addr = listener.local_addr()?;
        let url = service.websocket_url(addr, &token);

        let worker_limit = LocalWorkspace::default_worker_count();
        let workspace = LocalWorkspace::open(root, worker_limit)?;
        let workspace: Arc<dyn Workspace> = Arc::new(workspace);
        let root_lease = Arc::new(RootLease::default());
        let lifecycle = ServerLifecycle::default();
        let handle = spawn_server(listener, workspace, root_lease, options, lifecycle.clone());

        Ok(Self {
            url,
            lifecycle,
            handle: Some(handle),
        })
    }

    /// Return the WebSocket URL for this server.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Close this server.
    pub fn close(&mut self) -> std::thread::Result<()> {
        self.lifecycle.request_shutdown();

        if let Some(handle) = self.handle.take() {
            handle.join()?;
        }

        Ok(())
    }
}

impl Drop for WebSocketServer {
    /// Close this server before dropping it.
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// WebSocket workspace server error.
#[derive(Debug)]
pub enum WebSocketServerError {
    /// Workspace service error.
    Service(ServiceError),
    /// WebSocket transport error.
    WebSocket(WebSocketError),
    /// Workspace open error.
    Workspace(Error),
}

impl From<ServiceError> for WebSocketServerError {
    /// Convert one workspace service error.
    fn from(error: ServiceError) -> Self {
        Self::Service(error)
    }
}

impl From<WebSocketError> for WebSocketServerError {
    /// Convert one WebSocket transport error.
    fn from(error: WebSocketError) -> Self {
        Self::WebSocket(error)
    }
}

impl From<Error> for WebSocketServerError {
    /// Convert one workspace error.
    fn from(error: Error) -> Self {
        Self::Workspace(error)
    }
}

impl std::fmt::Display for WebSocketServerError {
    /// Format this error for users.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(error) => write!(formatter, "{error}"),
            Self::WebSocket(error) => write!(formatter, "{error}"),
            Self::Workspace(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for WebSocketServerError {}

/// Spawn one workspace protocol accept loop.
fn spawn_server(
    listener: WebSocketListener,
    workspace: Arc<dyn Workspace>,
    root_lease: Arc<RootLease>,
    options: ServerOptions,
    lifecycle: ServerLifecycle,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut handles = Vec::new();

        // accept connections until shutdown
        while !lifecycle.is_shutting_down() {
            let transport = match listener.try_accept() {
                Ok(Some(transport)) => transport,
                Ok(None) => {
                    std::thread::sleep(SERVER_POLL_INTERVAL);
                    continue;
                }
                Err(error) => {
                    panic!("workspace websocket accept failed: {error}");
                }
            };

            // serve each connection independently
            let workspace = workspace.clone();
            let root_lease = root_lease.clone();
            let options = options.clone();
            let lifecycle = lifecycle.clone();
            handles.push(std::thread::spawn(move || {
                let server = Server::with_root_lease(workspace, root_lease, options, lifecycle);
                let _ = server.serve(transport.as_ref());
            }));
        }

        // wait for active connections to exit
        for handle in handles {
            if handle.join().is_err() {
                panic!("workspace protocol connection thread panicked");
            }
        }
    })
}
