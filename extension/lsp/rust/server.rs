//! Async language server implementation built on tower-lsp-server.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use tokio::sync::RwLock;
use tower_lsp_server::Client;

use dyst_package::Workspace;

#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client that the language server is connected to.
    pub(super) client: Client,
    /// The workspaces that the language server is connected to.
    pub(super) workspaces: RwLock<HashMap<String, Arc<RwLock<Workspace>>>>,

    /// The next watch id to use for the language server.
    pub(super) next_watch_id: AtomicU64,
    /// The file watchers for each workspace.
    pub(super) workspace_watch_ids: RwLock<HashMap<String, String>>,
}

impl DestackLanguageServer {
    /// Create a new language server instance with the given client.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspaces: RwLock::new(HashMap::new()),
            next_watch_id: AtomicU64::new(0),
            workspace_watch_ids: RwLock::new(HashMap::new()),
        }
    }
}
