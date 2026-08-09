use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactPayload;
use destack_repository::{
    Commit, DestackLayoutOverride, Host, Revision, Settings, open_repository,
};
use destack_rpc::{Code, Request, Response, ResponseSender, Status};
use destack_session::Executor;
use destack_source::{Content, ContentId, OverlayFileSystem};
use destack_workspace as workspace;
use parking_lot::RwLock;
use workspace::{ProgressEvent, WatchEvent, Workspace, WorkspaceService};

use super::{DaemonError, WorkspaceWatch};

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
    layout: DestackLayoutOverride,
    /// Editor overlay shared by workspace repositories.
    overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Live workspaces keyed by canonical root.
    registration_by_root: Arc<RwLock<HashMap<PathBuf, Registration>>>,
}

impl std::fmt::Debug for WorkspaceRegistry {
    /// Format the visible daemon workspace state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceRegistry")
            .field("executor", &self.executor)
            .field("host", &self.host)
            .field("settings", &self.settings)
            .field("layout", &self.layout)
            .field("overlay_file_system", &self.overlay_file_system.is_some())
            .field("registration_by_root", &self.registration_by_root)
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
        let layout = DestackLayoutOverride {
            home: Some(repository.layout().home.clone()),
            packages: Some(repository.layout().packages.clone()),
            workspace_cache: None,
        };
        let overlay_file_system = workspace.overlay_file_system();
        let root = workspace.root().to_path_buf();
        let registration = Registration::new(Arc::new(workspace))?;
        let registration_by_root = HashMap::from([(root, registration)]);

        Ok(Self {
            executor,
            host,
            settings,
            layout,
            overlay_file_system,
            registration_by_root: Arc::new(RwLock::new(registration_by_root)),
        })
    }

    /// Open one physical workspace and return its shared instance.
    pub(crate) fn open(&self, root: &Path) -> Result<Arc<Workspace>, DaemonError> {
        let root = Self::canonicalize(root)?;
        if let Some(workspace) = self.get(&root) {
            return Ok(workspace);
        }

        // serialize construction so one root always has one workspace and watch
        let mut registrations = self.registration_by_root.write();
        if let Some(registration) = registrations.get(&root) {
            return Ok(registration.workspace.clone());
        }
        let repository = open_repository(
            root.clone(),
            self.host.clone(),
            self.settings.clone(),
            self.layout.clone(),
        )
        .map_err(workspace::Error::from)?;
        let repository = Arc::new(repository);
        let workspace = Workspace::new(
            repository,
            self.overlay_file_system.clone(),
            self.executor.clone(),
        )?;
        let workspace = Arc::new(workspace);
        let registration = Registration::new(workspace.clone())?;
        registrations.insert(root, registration);

        Ok(workspace)
    }

    /// Close one physical workspace.
    pub(crate) fn close(&self, root: &Path) -> Result<(), DaemonError> {
        let registration = self.registration_by_root.write().remove(root);
        let Some(registration) = registration else {
            return Ok(());
        };

        registration.close()?;

        Ok(())
    }

    /// Close every registered workspace and physical watch.
    pub(crate) fn close_all(&self) -> Result<(), DaemonError> {
        let registrations = self
            .registration_by_root
            .write()
            .drain()
            .map(|(_, registration)| registration)
            .collect::<Vec<_>>();
        let failures = registrations
            .into_iter()
            .filter_map(|registration| registration.close().err())
            .collect();

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
        self.registration_by_root
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
struct Registration {
    /// Root-bound semantic workspace.
    workspace: Arc<Workspace>,
    /// Physical changes feeding the workspace.
    watch: WorkspaceWatch,
}

impl std::fmt::Debug for Registration {
    /// Format the visible workspace registration.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Registration")
            .field("workspace", &self.workspace)
            .field("watch", &self.watch)
            .finish()
    }
}

impl Registration {
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

impl WorkspaceService for WorkspaceRegistry {
    /// Reload one workspace from its host.
    async fn reload(
        &self,
        request: Request<workspace::ReloadRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::reload(&workspace, request).await
    }

    /// Read one workspace revision.
    async fn read_revision(
        &self,
        request: Request<workspace::ReadRevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::read_revision(&workspace, request).await
    }

    /// Apply one editor file operation.
    async fn apply_file_operation(
        &self,
        request: Request<workspace::ApplyFileOperationRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::apply_file_operation(&workspace, request).await
    }

    /// Apply one atomic source update.
    async fn apply_source_update(
        &self,
        request: Request<workspace::ApplySourceUpdateRequest>,
    ) -> Result<Response<Commit>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::apply_source_update(&workspace, request).await
    }

    /// Return whether one source file is open.
    async fn is_file_open(
        &self,
        request: Request<workspace::IsFileOpenRequest>,
    ) -> Result<Response<bool>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::is_file_open(&workspace, request).await
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

    /// Run one workspace target.
    async fn run(
        &self,
        request: Request<workspace::RunRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::RunOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::run(&workspace, request, responses).await
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

    /// Run workspace benchmarks.
    async fn bench(
        &self,
        request: Request<workspace::BenchRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::BenchOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::bench(&workspace, request, responses).await
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

    /// Return cache locations.
    async fn cache(
        &self,
        request: Request<workspace::CacheRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<workspace::CacheOutput>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::cache(&workspace, request, responses).await
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

    /// Store one content value.
    async fn store(
        &self,
        request: Request<workspace::StoreRequest>,
    ) -> Result<Response<ContentId>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::store(&workspace, request).await
    }

    /// Load one content value.
    async fn load(
        &self,
        request: Request<workspace::LoadRequest>,
    ) -> Result<Response<Content>, Status> {
        let workspace = self.workspace(&request.value.root)?;

        WorkspaceService::load(&workspace, request).await
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
}
