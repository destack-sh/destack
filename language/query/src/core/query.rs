use std::sync::Arc;

use destack_repository::Repository;

/// Provider that builds query artifacts for one repository.
#[derive(Debug, Clone)]
pub struct Query {
    /// The repository being indexed.
    repository: Arc<Repository>,
}

impl Query {
    /// Create a query provider for one repository.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }

    /// Return the repository being indexed.
    pub(crate) fn repository(&self) -> &Repository {
        &self.repository
    }
}
