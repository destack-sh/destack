use std::sync::Arc;

use destack_workspace::Repository;

/// REPL state owned by the daemon.
#[derive(Debug, Clone)]
pub struct Repl {
    /// The repository backing the REPL.
    pub repository: Arc<Repository>,
}

impl Repl {
    /// Create a new REPL state.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }
}
