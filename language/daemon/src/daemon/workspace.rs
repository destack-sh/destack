use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use tspp_artifact::{ArtifactPayload, BuildId};
use tspp_core::Blob;
use tspp_repository as repository;
use tspp_repository::{Commit, Host, Repository, Revision, Settings, StorageLayoutOverride};
use tspp_rpc::{Code, Request, Response, ResponseSender, Status};
use tspp_session::Executor;
use tspp_workspace as workspace;
use workspace::{ProgressEvent, WatchEvent, Workspace, WorkspaceService};

use super::{DaemonError, WorkspaceWatch};
use crate::DaemonPeer;

/// Root-bound workspaces hosted by one daemon process.
#[derive(Clone)]
pub(crate) struct WorkspaceRegistry {
    /// Process artifact executor shared by every workspace session.
    executor: Arc<Executor>,
    /// Host capabilities shared by workspace repositories.
    host: Host,
    /// Machine settings shared by workspace repositories.
    settings: Settings,
    /// Machine layout shared while workspace-local paths remain root-relative.
    layout: StorageLayoutOverride,
    /// Live workspaces keyed by canonical root.
    registrations: Arc<RwLock<HashMap<PathBuf, WorkspaceRegistration>>>,
}

impl fmt::Debug for WorkspaceRegistry {
    /// Format the visible daemon workspace state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkspaceRegistry")
            .field("executor", &self.executor)
            .field("host", &self.host)
            .field("settings", &self.settings)
            .field("layout", &self.layout)
            .field("registrations", &self.registrations)
            .finish()
    }
}

impl WorkspaceRegistry {
    /// Create one registry containing an initial workspace.
    pub(crate) fn new(workspace: Workspace) -> Result<Self, DaemonError> {
        let executor = workspace.session().executor();
        let repository = workspace.session().repository();
        let host = repository.host().clone();
        let settings = repository.settings().clone();
        let layout = StorageLayoutOverride {
            home: Some(repository.layout().home.clone()),
            packages: Some(repository.layout().packages.clone()),
            cache: Some(repository.layout().cache.clone()),
        };
        let root = workspace.root().to_path_buf();
        let registration = WorkspaceRegistration::new(Arc::new(workspace))?;
        let registrations = HashMap::from([(root, registration)]);

        Ok(Self {
            executor,
            host,
            settings,
            layout,
            registrations: Arc::new(RwLock::new(registrations)),
        })
    }

    /// Return the toolchain build shared by every hosted workspace.
    pub(crate) fn build_id(&self) -> BuildId {
        self.host.build_id()
    }

    /// Open one physical workspace and return its shared instance.
    pub(crate) fn open(&self, root: &Path) -> Result<Arc<Workspace>, DaemonError> {
        let root = Self::canonicalize(root)?;
        if let Some(workspace) = self.get(&root) {
            return Ok(workspace);
        }

        // serialize construction so one root always has one workspace and watch
        let mut registrations = self.registrations.write();
        if let Some(registration) = registrations.get(&root) {
            return Ok(registration.workspace.clone());
        }
        let (repository, physical) = Repository::open(
            root.clone(),
            self.host.clone(),
            self.settings.clone(),
            self.layout.clone(),
        )
        .map_err(workspace::Error::from)?;
        let repository = Arc::new(repository);

        // restore cached artifacts valid at this revision
        repository
            .restore_artifacts(physical, self.executor.worker_count())
            .map_err(workspace::Error::from)?;

        let workspace = Workspace::new(repository, physical, self.executor.clone())?;
        let workspace = Arc::new(workspace);
        let registration = WorkspaceRegistration::new(workspace.clone())?;
        registrations.insert(root, registration);

        Ok(workspace)
    }

    /// Close every registered workspace and physical watch.
    pub(crate) fn close_all(&self) -> Result<(), DaemonError> {
        let registrations = self
            .registrations
            .write()
            .drain()
            .map(|(_, registration)| registration)
            .collect::<Vec<_>>();
        let mut failures = registrations
            .into_iter()
            .filter_map(|registration| registration.close().err())
            .collect::<Vec<_>>();

        // wait for writes queued by the released workspace revisions
        let cache_result = self
            .host
            .flush_artifact_cache()
            .map_err(repository::RepositoryError::from)
            .map_err(workspace::Error::from)
            .map_err(DaemonError::from);
        failures.extend(cache_result.err());

        match DaemonError::combine(failures) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Return one open workspace by canonical root.
    pub(crate) fn workspace(&self, root: &Path) -> Result<Arc<Workspace>, Status> {
        self.get(root).ok_or_else(|| {
            Status::new(
                Code::NotFound,
                format!("workspace is not open: {}", root.display()),
            )
        })
    }

    /// Return one registered workspace without resolving its root.
    fn get(&self, root: &Path) -> Option<Arc<Workspace>> {
        self.registrations
            .read()
            .get(root)
            .map(|registration| registration.workspace.clone())
    }

    /// Return one canonical physical workspace root.
    fn canonicalize(root: &Path) -> Result<PathBuf, DaemonError> {
        std::fs::canonicalize(root)
            .map_err(|source| workspace::Error::Io {
                path: root.to_path_buf(),
                source,
            })
            .map_err(DaemonError::from)
    }
}

/// One registered workspace and its physical observation.
struct WorkspaceRegistration {
    /// Root-bound semantic workspace.
    workspace: Arc<Workspace>,
    /// Physical changes feeding the workspace.
    watch: WorkspaceWatch,
}

impl fmt::Debug for WorkspaceRegistration {
    /// Format the visible workspace registration.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Registration")
            .field("workspace", &self.workspace)
            .field("watch", &self.watch)
            .finish()
    }
}

impl WorkspaceRegistration {
    /// Register one workspace under physical observation.
    fn new(workspace: Arc<Workspace>) -> Result<Self, DaemonError> {
        let watch = WorkspaceWatch::start(workspace.clone())?;

        Ok(Self { workspace, watch })
    }

    /// Close the physical watch and semantic workspace.
    fn close(mut self) -> Result<(), DaemonError> {
        let result = self.watch.stop();
        self.workspace.close();

        result
    }
}

impl WorkspaceService for DaemonPeer {
    /// Read one workspace's physical revision.
    async fn revision(
        &self,
        request: Request<workspace::RevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::revision(&workspace, request).await
    }

    /// Reload one workspace from its host.
    async fn reload(
        &self,
        request: Request<workspace::ReloadRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::reload(&workspace, request).await
    }

    /// List branches in one workspace.
    async fn list_branches(
        &self,
        request: Request<workspace::ListBranchesRequest>,
    ) -> Result<Response<Vec<workspace::Branch>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::list_branches(&workspace, request).await
    }

    /// Create one workspace branch.
    async fn create_branch(
        &self,
        request: Request<workspace::CreateBranchRequest>,
    ) -> Result<Response<workspace::Branch>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::create_branch(&workspace, request).await
    }

    /// Read one workspace branch revision.
    async fn branch_revision(
        &self,
        request: Request<workspace::BranchRevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::branch_revision(&workspace, request).await
    }

    /// Remove one workspace branch.
    async fn remove_branch(
        &self,
        request: Request<workspace::RemoveBranchRequest>,
    ) -> Result<Response<()>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::remove_branch(&workspace, request).await
    }

    /// Commit source edits to physical workspace state.
    async fn edit(
        &self,
        request: Request<workspace::EditRequest>,
    ) -> Result<Response<Commit>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::edit(&workspace, request).await
    }

    /// Commit source edits to one exact branch revision.
    async fn edit_branch(
        &self,
        request: Request<workspace::EditBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::edit_branch(&workspace, request).await
    }

    /// Save selected branch files to physical state.
    async fn save_branch(
        &self,
        request: Request<workspace::SaveBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::save_branch(&workspace, request).await
    }

    /// Restore selected branch files from physical state.
    async fn restore_branch(
        &self,
        request: Request<workspace::RestoreBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::restore_branch(&workspace, request).await
    }

    /// Compare two exact workspace revisions.
    async fn diff(
        &self,
        request: Request<workspace::DiffRequest>,
    ) -> Result<Response<Vec<repository::Change>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::diff(&workspace, request).await
    }

    /// List files at one exact workspace revision.
    async fn list_files(
        &self,
        request: Request<workspace::ListFilesRequest>,
    ) -> Result<Response<Vec<repository::File>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::list_files(&workspace, request).await
    }

    /// Format one source file or selected range.
    async fn format_file(
        &self,
        request: Request<workspace::FormatFileRequest>,
    ) -> Result<Response<Option<workspace::FileEditResponse>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::format_file(&workspace, request).await
    }

    /// Read source files from one exact revision.
    async fn read_files(
        &self,
        request: Request<workspace::ReadFilesRequest>,
    ) -> Result<Response<Vec<workspace::FileImage>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::read_files(&workspace, request).await
    }

    /// Check source state.
    async fn check(
        &self,
        request: Request<workspace::CheckRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::CheckOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::check(&workspace, request, responses).await
    }

    /// Format source files or content.
    async fn format(
        &self,
        request: Request<workspace::FormatRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::FormatOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::format(&workspace, request, responses).await
    }

    /// Query source files with one structural pattern.
    async fn query(
        &self,
        request: Request<workspace::QueryRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::QueryOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::query(&workspace, request, responses).await
    }

    /// Rewrite source files with one structural pattern.
    async fn rewrite(
        &self,
        request: Request<workspace::RewriteRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::RewriteOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::rewrite(&workspace, request, responses).await
    }

    /// Build target artifacts.
    async fn build(
        &self,
        request: Request<workspace::BuildRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::BuildOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::build(&workspace, request, responses).await
    }

    /// Run workspace tests.
    async fn test(
        &self,
        request: Request<workspace::TestRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::TestOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::test(&workspace, request, responses).await
    }

    /// Generate workspace documentation.
    async fn doc(
        &self,
        request: Request<workspace::DocRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::DocOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::doc(&workspace, request, responses).await
    }

    /// Return workspace information.
    async fn info(
        &self,
        request: Request<workspace::InfoRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::InfoOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::info(&workspace, request, responses).await
    }

    /// Return configured targets.
    async fn targets(
        &self,
        request: Request<workspace::TargetsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::TargetsOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::targets(&workspace, request, responses).await
    }

    /// Return resolved settings.
    async fn settings(
        &self,
        request: Request<workspace::SettingsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::SettingsOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::settings(&workspace, request, responses).await
    }

    /// Diagnose workspace configuration and state.
    async fn doctor(
        &self,
        request: Request<workspace::DoctorRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::DoctorOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::doctor(&workspace, request, responses).await
    }

    /// Execute configured workspace tasks.
    async fn task(
        &self,
        request: Request<workspace::TaskRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::TaskOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::task(&workspace, request, responses).await
    }

    /// Clean generated workspace state.
    async fn clean(
        &self,
        request: Request<workspace::CleanRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::CleanOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::clean(&workspace, request, responses).await
    }

    /// Read one exact artifact payload.
    async fn artifact(
        &self,
        request: Request<workspace::ArtifactRequest>,
    ) -> Result<Response<ArtifactPayload>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::artifact(&workspace, request).await
    }

    /// Publish one artifact's storage bytes as a Blob.
    async fn blob(
        &self,
        request: Request<workspace::ArtifactRequest>,
    ) -> Result<Response<Blob>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::blob(&workspace, request).await
    }

    /// Materialize one artifact on the workspace host.
    async fn export(
        &self,
        request: Request<workspace::ExportRequest>,
    ) -> Result<Response<workspace::ExportResult>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::export(&workspace, request).await
    }

    /// Read exact diagnostics.
    async fn diagnose(
        &self,
        request: Request<workspace::DiagnoseRequest>,
    ) -> Result<Response<Vec<workspace::FileDiagnosticsResponse>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::diagnose(&workspace, request).await
    }

    /// Resolve one source file for semantic queries.
    async fn resolve_query_file(
        &self,
        request: Request<workspace::ResolveQueryFileRequest>,
    ) -> Result<Response<Option<workspace::QueryFileResponse>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::resolve_query_file(&workspace, request).await
    }

    /// Execute one semantic query.
    async fn run_query(
        &self,
        request: Request<workspace::RunQueryRequest>,
    ) -> Result<Response<workspace::RunQueryResponse>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::run_query(&workspace, request).await
    }

    /// Watch one workspace until cancellation.
    async fn watch(
        &self,
        request: Request<workspace::WatchRequest>,
        responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::watch(&workspace, request, responses).await
    }

    /// Watch one workspace branch until cancellation.
    async fn watch_branch(
        &self,
        request: Request<workspace::WatchBranchRequest>,
        responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::watch_branch(&workspace, request, responses).await
    }
}
