use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query::{QueryRequest, QueryResponse};
use destack_repository::{Edit, Ref, Repository, Revision};
use destack_source::Content;
use destack_workspace::{LocalWorkspace, RevisionPolicy};

use crate::core::SharedMemoryWorkspace;

use super::QueryFixture;

/// Package declaration used when a fixture does not provide one.
const QUERY_MANIFEST: &str = r#"{
  "include": ["**/*.ds"]
}
"#;

/// One workspace shared by isolated query fixture revisions.
#[derive(Debug)]
pub(super) struct QueryWorkspace {
    /// The workspace root.
    root: PathBuf,
    /// The shared artifact repository.
    repository: Arc<Repository>,
    /// The query workspace.
    local_workspace: LocalWorkspace,
    /// The immutable empty base revision.
    base_revision: Revision,
}

impl QueryWorkspace {
    /// Create one shared query workspace.
    pub(super) fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        let memory_workspace = SharedMemoryWorkspace::new(root.clone());
        let repository = memory_workspace.repository();
        let worker_count = LocalWorkspace::default_worker_count();
        let local_workspace = LocalWorkspace::new(
            repository.clone(),
            None,
            None,
            vec![root.clone()],
            worker_count,
            None,
        )
        .map_err(|error| format!("failed to open query workspace: {error}"))?;
        let reference = Ref::for_root(&root);
        let base_revision = repository
            .current(&reference)
            .map_err(|error| format!("failed to read query base revision: {error}"))?;

        Ok(Self {
            root,
            repository,
            local_workspace,
            base_revision,
        })
    }

    /// Return the workspace root.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// Return the shared artifact repository.
    pub(super) fn repository(&self) -> &Repository {
        self.repository.as_ref()
    }

    /// Fork one isolated revision containing exactly one query fixture.
    pub(super) fn fork(&self, fixture: &QueryFixture) -> Result<Revision, String> {
        let mut edits = Vec::with_capacity(fixture.files.len() + 1);

        // provide a minimal manifest only when the fixture omits one
        if !fixture.files.contains_key(Path::new("destack.json")) {
            edits.push(Edit::set_text("destack.json", QUERY_MANIFEST));
        }

        // publish every fixture file into the isolated revision
        for file in fixture.files.values() {
            let logical_path = file
                .path
                .to_str()
                .ok_or_else(|| format!("query file path '{}' is not UTF-8", file.path.display()))?;
            edits.push(Edit::SetFile {
                logical_path: logical_path.to_string(),
                content: Content::Text {
                    content: file.source.clone(),
                },
            });
        }

        self.repository
            .fork_with_edits(self.base_revision, edits)
            .map_err(|error| format!("failed to fork query revision: {error}"))
    }

    /// Execute one query against an exact revision.
    pub(super) fn query(
        &self,
        revision: Revision,
        request: QueryRequest,
    ) -> Result<QueryResponse, String> {
        self.local_workspace
            .query_root(&self.root, request, RevisionPolicy::Exact(revision))
            .map(|result| result.response)
            .map_err(|error| format!("query execution failed: {error}"))
    }
}
