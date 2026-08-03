use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query::{QueryRequest, QueryResponse};
use destack_repository::{Edit, Ref, Repository, Revision, Trace, TraceSnapshot, TraceView};
use destack_source::Content;
use destack_workspace::{LocalWorkspace, RevisionPolicy, RunQueryRequest};
use indexmap::IndexMap;

use crate::core::SharedMemoryWorkspace;

use super::{QueryChange, QueryFile};

/// Package declaration used when a fixture does not provide one.
const QUERY_MANIFEST: &str = r#"{
  "name": "@test/query",
  "targets": {
    "default": {
      "include": ["**/*.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;
/// Environment variable enabling detailed timing reports.
const TIMINGS_ENV: &str = "DESTACK_TIMINGS";
/// Environment variable selecting the query workspace worker count.
const WORKERS_ENV: &str = "DESTACK_TEST_WORKERS";

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
    /// Whether detailed query traces should be retained.
    has_timings: bool,
}

/// One query response and its optional detailed trace.
pub(super) struct QueryExecution {
    /// The query response.
    pub(super) response: QueryResponse,
    /// The complete query operation trace when requested.
    pub(super) trace: Option<TraceSnapshot>,
}

impl QueryWorkspace {
    /// Create one shared query workspace.
    pub(super) fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        let memory_workspace = SharedMemoryWorkspace::new(root.clone());
        let repository = memory_workspace.repository();
        let worker_count = match env::var(WORKERS_ENV) {
            Ok(value) => value
                .parse::<usize>()
                .map_err(|error| format!("invalid {WORKERS_ENV} value '{value}': {error}"))?,
            Err(env::VarError::NotPresent) => LocalWorkspace::default_worker_count(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(format!("{WORKERS_ENV} is not valid UTF-8"));
            }
        };
        let local_workspace = LocalWorkspace::new(
            repository.clone(),
            None,
            None,
            vec![root.clone()],
            worker_count,
            None,
        )
        .map_err(|error| format!("failed to open query workspace: {error}"))?;
        let has_timings =
            env::var_os(TIMINGS_ENV).is_some_and(|value| !value.is_empty() && value != "0");
        let session = local_workspace
            .session(&root)
            .map_err(|error| format!("failed to read query session: {error}"))?;
        session.set_tracing(has_timings);
        let reference = Ref::for_root(&root);
        let base_revision = repository
            .current(&reference)
            .map_err(|error| format!("failed to read query base revision: {error}"))?;

        Ok(Self {
            root,
            repository,
            local_workspace,
            base_revision,
            has_timings,
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

    /// Fork one isolated revision containing exact initial files.
    pub(super) fn fork(&self, files: &IndexMap<PathBuf, QueryFile>) -> Result<Revision, String> {
        let mut edits = Vec::with_capacity(files.len() + 1);

        // provide a minimal manifest only when the fixture omits one
        if !files.contains_key(Path::new("destack.json")) {
            edits.push(Edit::set_text("destack.json", QUERY_MANIFEST));
        }

        // publish every fixture file into the isolated revision
        for file in files.values() {
            edits.push(Edit::try_from(file)?);
        }

        self.repository
            .fork_with_edits(self.base_revision, edits)
            .map_err(|error| format!("failed to fork query revision: {error}"))
    }

    /// Fork one revision with exact workspace changes.
    pub(super) fn fork_changes(
        &self,
        revision: Revision,
        changes: &[QueryChange],
    ) -> Result<Revision, String> {
        let edits = changes
            .iter()
            .map(Edit::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        self.repository
            .fork_with_edits(revision, edits)
            .map_err(|error| format!("failed to fork query revision: {error}"))
    }

    /// Execute one query against an exact revision.
    pub(super) fn query(
        &self,
        revision: Revision,
        request: QueryRequest,
    ) -> Result<QueryExecution, String> {
        let run = self
            .local_workspace
            .start_query(
                &self.root,
                RunQueryRequest {
                    revision: RevisionPolicy::Exact(revision),
                    request,
                },
            )
            .map_err(|error| format!("query scheduling failed: {error}"))?;
        let trace = run.trace();
        let response = run
            .wait()
            .map_err(|error| format!("query execution failed: {error}"))?
            .response;
        let trace = self
            .has_timings
            .then(|| self.snapshot_trace(revision, trace))
            .transpose()?;

        Ok(QueryExecution { response, trace })
    }

    /// Return whether detailed fixture timings are enabled.
    pub(super) fn has_timings(&self) -> bool {
        self.has_timings
    }

    /// Begin one fixture operation trace when timings are enabled.
    pub(super) fn begin_trace(&self) -> Option<Arc<Trace>> {
        self.has_timings
            .then(|| Trace::new(self.repository.host().clock(), 1, true))
    }

    /// Finish and snapshot one fixture operation trace.
    pub(super) fn finish_trace(
        &self,
        revision: Revision,
        trace: Arc<Trace>,
    ) -> Result<TraceSnapshot, String> {
        trace.finish();

        self.snapshot_trace(revision, trace)
    }

    /// Snapshot one completed query trace.
    fn snapshot_trace(
        &self,
        revision: Revision,
        trace: Arc<Trace>,
    ) -> Result<TraceSnapshot, String> {
        trace.snapshot(
            TraceView::Detailed,
            |key| {
                self.repository
                    .artifact_display(revision, *key)
                    .map_err(|error| error.to_string())
            },
            |target| {
                self.repository
                    .target_display(revision, target)
                    .map_err(|error| error.to_string())
            },
        )
    }
}

impl TryFrom<&QueryFile> for Edit {
    type Error = String;

    /// Convert one complete query file into a repository edit.
    fn try_from(file: &QueryFile) -> Result<Self, Self::Error> {
        let logical_path = query_logical_path(&file.path)?;

        Ok(Edit::SetFile {
            logical_path,
            content: Content::Text {
                content: file.source.clone(),
            },
        })
    }
}

impl TryFrom<&QueryChange> for Edit {
    type Error = String;

    /// Convert one query fixture change into a repository edit.
    fn try_from(change: &QueryChange) -> Result<Self, Self::Error> {
        let edit = match change {
            QueryChange::Add(file) => {
                let logical_path = query_logical_path(&file.path)?;

                Edit::AddFile {
                    logical_path,
                    content: Content::Text {
                        content: file.source.clone(),
                    },
                }
            }
            QueryChange::Set(file) => Edit::try_from(file)?,
            QueryChange::Remove(path) => Edit::remove_file(query_logical_path(path)?),
            QueryChange::Move { from, to } => {
                Edit::move_file(query_logical_path(from)?, query_logical_path(to)?)
            }
        };

        Ok(edit)
    }
}

/// Return one UTF-8 repository logical path.
fn query_logical_path(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("query file path '{}' is not UTF-8", path.display()))
}
