use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::BuildId;
use destack_query::{QueryRequest, QueryResponse};
use destack_repository::{
    DestackLayoutOverride, Edit, Environment, Execution, Host, Repository, Revision, Settings,
    Trace, TraceLevel, TraceSnapshot, TraceView,
};
use destack_session::Executor;
use destack_source::{FileSystem, MemoryFileSystem};
use destack_workspace::{RevisionPolicy, RunQueryInput, Workspace};
use futures::executor::block_on;
use indexmap::IndexMap;

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
    workspace: Workspace,
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
        // open one empty in memory repository
        let root = root.into();
        let file_system = Arc::new(MemoryFileSystem::new());
        file_system
            .create_dir_all(&root)
            .map_err(|error| format!("failed to create query workspace: {error}"))?;
        let host = Host::new(BuildId::test(), Environment::capture_process(), file_system);
        let (repository, revision) = Repository::open(
            root.clone(),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(|error| format!("failed to open query repository: {error}"))?;
        let repository = Arc::new(repository);

        // create one threaded semantic workspace
        let worker_count = match env::var(WORKERS_ENV) {
            Ok(value) => value
                .parse::<usize>()
                .map_err(|error| format!("invalid {WORKERS_ENV} value '{value}': {error}"))?,
            Err(env::VarError::NotPresent) => Executor::default_worker_count(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(format!("{WORKERS_ENV} is not valid UTF-8"));
            }
        };
        let executor = Executor::new(Execution::Threaded, worker_count)
            .map_err(|error| format!("failed to create query artifact executor: {error}"))?;
        let workspace = Workspace::new(repository.clone(), revision, executor)
            .map_err(|error| format!("failed to open query workspace: {error}"))?;
        let has_timings =
            env::var_os(TIMINGS_ENV).is_some_and(|value| !value.is_empty() && value != "0");

        Ok(Self {
            root,
            repository,
            workspace,
            base_revision: revision,
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
            let blob = self
                .repository
                .retain_blob(QUERY_MANIFEST.as_bytes())
                .map_err(|error| format!("failed to store query manifest: {error}"))?;
            edits.push(Edit::add_file("destack.json", blob));
        }

        // commit every fixture file to the isolated revision
        for file in files.values() {
            let logical_path = query_logical_path(&file.path)?;
            let blob = self
                .repository
                .retain_blob(file.source.as_bytes())
                .map_err(|error| format!("failed to store query file: {error}"))?;
            edits.push(Edit::add_file(logical_path, blob));
        }

        self.repository
            .edit(self.base_revision, edits)
            .map(|commit| commit.after)
            .map_err(|error| format!("failed to edit query revision: {error}"))
    }

    /// Fork one revision with exact workspace changes.
    pub(super) fn fork_changes(
        &self,
        revision: Revision,
        changes: &[QueryChange],
        trace: &Trace,
    ) -> Result<Revision, String> {
        trace.add_counter("edit.changes", changes.len() as u64);
        let edits = trace.span("edit.lower", || {
            changes
                .iter()
                .map(|change| self.change_edit(change))
                .collect::<Result<Vec<_>, _>>()
        })?;

        trace
            .span("revision.edit", || self.repository.edit(revision, edits))
            .map(|commit| commit.after)
            .map_err(|error| format!("failed to edit query revision: {error}"))
    }

    /// Execute one query against an exact revision.
    pub(super) fn query(
        &self,
        revision: Revision,
        request: QueryRequest,
    ) -> Result<QueryExecution, String> {
        let run = self
            .workspace
            .start_query(
                RunQueryInput {
                    revision: RevisionPolicy::Exact(revision),
                    request,
                },
                if self.has_timings {
                    TraceLevel::Timings
                } else {
                    TraceLevel::Disabled
                },
            )
            .map_err(|error| format!("query scheduling failed: {error}"))?;
        let trace = run.trace();
        let response = block_on(run.wait())
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

    /// Start one fixture operation trace.
    pub(super) fn begin_trace(&self) -> Arc<Trace> {
        let level = if self.has_timings {
            TraceLevel::Timings
        } else {
            TraceLevel::Disabled
        };

        self.workspace.start_trace(level)
    }

    /// Finish one fixture operation trace and retain requested timings.
    pub(super) fn finish_trace(
        &self,
        revision: Revision,
        trace: Arc<Trace>,
    ) -> Result<Option<TraceSnapshot>, String> {
        trace.finish();

        self.has_timings
            .then(|| self.snapshot_trace(revision, trace))
            .transpose()
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

    /// Build one repository edit for a query fixture change.
    fn change_edit(&self, change: &QueryChange) -> Result<Edit, String> {
        match change {
            QueryChange::Add(file) => {
                let logical_path = query_logical_path(&file.path)?;
                let blob = self
                    .repository
                    .retain_blob(file.source.as_bytes())
                    .map_err(|error| format!("failed to store query file: {error}"))?;

                Ok(Edit::add_file(logical_path, blob))
            }
            QueryChange::Edit(target) => {
                let logical_path = query_logical_path(&target.path)?;
                let blob = self
                    .repository
                    .retain_blob(target.source.as_bytes())
                    .map_err(|error| format!("failed to store query file: {error}"))?;

                Ok(Edit::set_file(logical_path, blob))
            }
            QueryChange::Remove(path) => Ok(Edit::remove_file(query_logical_path(path)?)),
            QueryChange::Move { from, to } => Ok(Edit::move_file(
                query_logical_path(from)?,
                query_logical_path(to)?,
            )),
        }
    }
}

/// Return one UTF-8 repository logical path.
fn query_logical_path(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("query file path '{}' is not UTF-8", path.display()))
}
