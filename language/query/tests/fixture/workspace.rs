use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::executor::block_on;
use indexmap::IndexMap;
use tspp_artifact::{ArtifactKey, BuildId, IndexKind};
use tspp_query::{QueryRequest, QueryResponse};
use tspp_repository::{
    Edit, Environment, Execution, Host, Repository, Revision, Settings, StorageLayoutOverride,
    Trace, TraceLevel, TraceSnapshot, TraceView,
};
use tspp_session::{ArtifactPriority, Executor, Session};
use tspp_source::{FileSystem, MemoryFileSystem, TargetId};
use tspp_workspace::{RevisionPolicy, RunQueryInput, Workspace};

use super::{QueryChange, QueryFile};

/// Package declaration used when a fixture does not provide one.
const QUERY_MANIFEST: &str = r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "@test/query",
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;
/// Logical path of the shared query package declaration.
const QUERY_MANIFEST_PATH: &str = "package.json";
/// Temporary module used to resolve the shared query package profile.
const WARM_ANCHOR_PATH: &str = "__warm.tspp";
/// Environment variable enabling detailed timing reports.
const TIMINGS_ENV: &str = "TSPP_TIMINGS";
/// Environment variable selecting the query workspace worker count.
const WORKERS_ENV: &str = "TSPP_TEST_WORKERS";

/// One workspace shared by isolated query fixture revisions.
#[derive(Debug)]
pub(super) struct QueryWorkspace {
    /// The workspace root.
    root: PathBuf,
    /// The shared artifact repository.
    repository: Arc<Repository>,
    /// The query workspace.
    workspace: Workspace,
    /// The shared revision containing warmed library artifacts and query indexes.
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
            StorageLayoutOverride::default(),
        )
        .map_err(|error| format!("failed to open query repository: {error}"))?;
        let repository = Arc::new(repository);

        // read fixture execution controls
        let worker_count = match env::var(WORKERS_ENV) {
            Ok(value) => value
                .parse::<usize>()
                .map_err(|error| format!("invalid {WORKERS_ENV} value '{value}': {error}"))?,
            Err(env::VarError::NotPresent) => Executor::default_worker_count(),
            Err(env::VarError::NotUnicode(_)) => {
                return Err(format!("{WORKERS_ENV} is not valid UTF-8"));
            }
        };
        let has_timings =
            env::var_os(TIMINGS_ENV).is_some_and(|value| !value.is_empty() && value != "0");

        // create one threaded artifact executor
        let executor = Executor::new(Execution::Threaded, worker_count)
            .map_err(|error| format!("failed to create query artifact executor: {error}"))?;

        // install the shared package and its temporary anchor
        let manifest = repository
            .retain_blob(QUERY_MANIFEST.as_bytes())
            .map_err(|error| format!("failed to store query manifest: {error}"))?;
        let anchor = repository
            .retain_blob(b"")
            .map_err(|error| format!("failed to store query anchor: {error}"))?;
        let revision = repository
            .edit(
                revision,
                [
                    Edit::add_file(QUERY_MANIFEST_PATH, manifest),
                    Edit::add_file(WARM_ANCHOR_PATH, anchor),
                ],
            )
            .map_err(|error| format!("failed to create query base revision: {error}"))?
            .after;

        // resolve the builtin and fixture package profiles
        let library = repository.embedded_builtin();
        let library_target = TargetId::new(library.package_id(), "default");
        let library_profile = repository
            .profile_for_target(revision, library_target)
            .map_err(|error| format!("failed to resolve builtin library profile: {error}"))?
            .id();
        let anchor_module = repository
            .module_id_for_path(revision, Path::new(WARM_ANCHOR_PATH))
            .map_err(|error| format!("failed to resolve query anchor module: {error}"))?
            .ok_or_else(|| "query anchor module is not tracked".to_string())?;
        let package = repository
            .module(revision, anchor_module)
            .map_err(|error| format!("failed to load query anchor module: {error}"))?
            .ok_or_else(|| "query anchor module is not loaded".to_string())?
            .package_id;
        let query_target = TargetId::new(package, "default");
        let query_profile = repository
            .profile_for_target(revision, query_target)
            .map_err(|error| format!("failed to resolve query fixture profile: {error}"))?
            .id();

        // check the complete builtin library under both reusable profiles
        let session = Session::new(repository.clone(), executor.clone())
            .map_err(|error| format!("failed to start query warmup session: {error}"))?;
        let artifacts = library
            .module_ids()
            .flat_map(|module| {
                [
                    ArtifactKey::dir_checked(module, library_profile),
                    ArtifactKey::dir_checked(module, query_profile),
                ]
            })
            .collect::<Vec<_>>();
        let run = session.provide(revision, &artifacts, ArtifactPriority::Foreground);
        block_on(run.wait()).map_err(|error| format!("failed to warm builtin library: {error}"))?;

        // remove the temporary module while retaining its inherited artifacts
        let base_revision = repository
            .edit(revision, [Edit::remove_file(WARM_ANCHOR_PATH)])
            .map_err(|error| format!("failed to retire query anchor: {error}"))?
            .after;

        // index the builtin program once for incremental query forks
        let artifacts = IndexKind::ALL
            .map(|kind| ArtifactKey::program_index(query_profile, kind))
            .to_vec();
        let run = session.provide(base_revision, &artifacts, ArtifactPriority::Foreground);
        block_on(run.wait()).map_err(|error| format!("failed to warm query indexes: {error}"))?;

        // retain the exact warmed base through the shared workspace
        let workspace = Workspace::new(repository.clone(), base_revision, executor)
            .map_err(|error| format!("failed to open query workspace: {error}"))?;

        Ok(Self {
            root,
            repository,
            workspace,
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
        let mut edits = Vec::with_capacity(files.len());

        // commit every fixture file to the isolated revision
        for file in files.values() {
            let logical_path = query_logical_path(&file.path)?;
            let blob = self
                .repository
                .retain_blob(file.source.as_bytes())
                .map_err(|error| format!("failed to store query file: {error}"))?;
            let edit = if logical_path == QUERY_MANIFEST_PATH {
                Edit::set_file(logical_path, blob)
            } else {
                Edit::add_file(logical_path, blob)
            };
            edits.push(edit);
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
        self.repository
            .snapshot_trace(revision, &trace, TraceView::Detailed)
            .map_err(|error| error.to_string())
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
