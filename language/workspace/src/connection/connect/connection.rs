use std::sync::Arc;

use crate::connection::Client;

/// Connection to a workspace server.
#[derive(Debug)]
pub struct WorkspaceConnection {
    /// Protocol client for workspace requests.
    pub client: Arc<Client>,
}
