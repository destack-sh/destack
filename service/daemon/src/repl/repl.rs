use std::sync::Arc;

use destack_workspace::Session;

/// REPL state owned by the daemon.
#[derive(Debug, Clone)]
pub struct Repl {
    /// The session backing the REPL.
    pub session: Arc<Session>,
}

impl Repl {
    /// Create a new REPL state.
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }
}
