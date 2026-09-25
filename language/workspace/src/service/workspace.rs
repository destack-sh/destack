use futures::{FutureExt, pin_mut, select_biased};
use tspp_artifact::ArtifactPayload;
use tspp_core::Blob;
use tspp_repository as repository;
use tspp_repository::{Commit, Revision, TraceLevel};
use tspp_rpc::{Request, Response, ResponseSender, Status};

use super::*;
use crate::{
    Branch, BuildOutput, CheckOutput, CleanOutput, CommandProgress, DocOutput, DoctorOutput,
    ExportResult, FileImage, FormatOutput, InfoOutput, ProgressEvent, QueryOutput, RewriteOutput,
    RunQueryResponse, SettingsOutput, TargetsOutput, TaskOutput, TestOutput, Watch, WatchEvent,
    Workspace,
};

/// RPC operations over one TS++ workspace.
#[tspp_rpc::service(name = "tspp.workspace.Workspace")]
pub trait WorkspaceService {
    // =============================================================================
    // Workspace
    // =============================================================================

    /// Read one workspace's physical revision.
    #[rpc(name = "Revision", idempotency = "no_side_effects")]
    fn revision(request: RevisionRequest) -> Revision;

    /// Reload one workspace root from its host.
    #[rpc(name = "Reload", idempotency = "idempotent")]
    fn reload(request: ReloadRequest) -> Option<Commit>;

    // =============================================================================
    // Branch
    // =============================================================================

    /// List branches in one workspace.
    #[rpc(name = "ListBranches", idempotency = "no_side_effects")]
    fn list_branches(request: ListBranchesRequest) -> Vec<Branch>;

    /// Create one workspace branch.
    #[rpc(name = "CreateBranch")]
    fn create_branch(request: CreateBranchRequest) -> Branch;

    /// Read one workspace branch revision.
    #[rpc(name = "BranchRevision", idempotency = "no_side_effects")]
    fn branch_revision(request: BranchRevisionRequest) -> Revision;

    /// Remove one workspace branch.
    #[rpc(name = "RemoveBranch")]
    fn remove_branch(request: RemoveBranchRequest) -> ();

    // =============================================================================
    // Source
    // =============================================================================

    /// Commit source edits to physical workspace state.
    #[rpc(name = "Edit")]
    fn edit(request: EditRequest) -> Commit;

    /// Commit source edits to one exact branch revision.
    #[rpc(name = "EditBranch")]
    fn edit_branch(request: EditBranchRequest) -> Commit;

    /// Save selected branch files to physical state.
    #[rpc(name = "SaveBranch")]
    fn save_branch(request: SaveBranchRequest) -> Commit;

    /// Restore selected branch files from physical state.
    #[rpc(name = "RestoreBranch")]
    fn restore_branch(request: RestoreBranchRequest) -> Commit;

    /// Compare two exact workspace revisions.
    #[rpc(name = "Diff", idempotency = "no_side_effects")]
    fn diff(request: DiffRequest) -> Vec<repository::Change>;

    /// List files at one exact workspace revision.
    #[rpc(name = "ListFiles", idempotency = "no_side_effects")]
    fn list_files(request: ListFilesRequest) -> Vec<repository::File>;

    /// Format one source file or selected range.
    #[rpc(name = "FormatFile", idempotency = "no_side_effects")]
    fn format_file(request: FormatFileRequest) -> Option<FileEditResponse>;

    /// Read source files from one exact revision.
    #[rpc(name = "ReadFiles", idempotency = "no_side_effects")]
    fn read_files(request: ReadFilesRequest) -> Vec<FileImage>;

    // =============================================================================
    // Analysis
    // =============================================================================

    /// Check source state.
    #[rpc(
        name = "Check",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn check(request: CheckRequest) -> CheckOutput;

    /// Format source files or content.
    #[rpc(name = "Format", response_stream(ProgressEvent))]
    fn format(request: FormatRequest) -> FormatOutput;

    /// Query source files with one structural pattern.
    #[rpc(
        name = "Query",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn query(request: QueryRequest) -> QueryOutput;

    /// Rewrite source files with one structural pattern.
    #[rpc(name = "Rewrite", response_stream(ProgressEvent))]
    fn rewrite(request: RewriteRequest) -> RewriteOutput;

    // =============================================================================
    // Build
    // =============================================================================

    /// Build target artifacts.
    #[rpc(name = "Build", response_stream(ProgressEvent))]
    fn build(request: BuildRequest) -> BuildOutput;

    /// Run workspace tests.
    #[rpc(name = "Test", response_stream(ProgressEvent))]
    fn test(request: TestRequest) -> TestOutput;

    /// Generate workspace documentation.
    #[rpc(name = "Doc", response_stream(ProgressEvent))]
    fn doc(request: DocRequest) -> DocOutput;

    // =============================================================================
    // Configuration
    // =============================================================================

    /// Return workspace information.
    #[rpc(
        name = "Info",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn info(request: InfoRequest) -> InfoOutput;

    /// Return configured targets.
    #[rpc(
        name = "Targets",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn targets(request: TargetsRequest) -> TargetsOutput;

    /// Return resolved settings.
    #[rpc(
        name = "Settings",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn settings(request: SettingsRequest) -> SettingsOutput;

    /// Diagnose workspace configuration and state.
    #[rpc(
        name = "Doctor",
        response_stream(ProgressEvent),
        idempotency = "no_side_effects"
    )]
    fn doctor(request: DoctorRequest) -> DoctorOutput;

    // =============================================================================
    // Task
    // =============================================================================

    /// Execute configured workspace tasks.
    #[rpc(name = "Task", response_stream(ProgressEvent))]
    fn task(request: TaskRequest) -> TaskOutput;

    /// Clean generated workspace state.
    #[rpc(name = "Clean", response_stream(ProgressEvent))]
    fn clean(request: CleanRequest) -> CleanOutput;

    // =============================================================================
    // Artifact
    // =============================================================================

    /// Read one exact artifact payload.
    #[rpc(name = "Artifact", idempotency = "no_side_effects")]
    fn artifact(request: ArtifactRequest) -> ArtifactPayload;

    /// Publish one artifact's storage bytes as a Blob.
    #[rpc(name = "Blob", idempotency = "idempotent")]
    fn blob(request: ArtifactRequest) -> Blob;

    /// Materialize one artifact on the workspace host.
    #[rpc(name = "Export")]
    fn export(request: ExportRequest) -> ExportResult;

    // =============================================================================
    // Language
    // =============================================================================

    /// Read exact diagnostics.
    #[rpc(name = "Diagnose", idempotency = "no_side_effects")]
    fn diagnose(request: DiagnoseRequest) -> Vec<FileDiagnosticsResponse>;

    /// Resolve one source file for semantic queries.
    #[rpc(name = "ResolveQueryFile", idempotency = "no_side_effects")]
    fn resolve_query_file(request: ResolveQueryFileRequest) -> Option<QueryFileResponse>;

    /// Execute one semantic query.
    #[rpc(name = "RunQuery", idempotency = "no_side_effects")]
    fn run_query(request: RunQueryRequest) -> RunQueryResponse;

    // =============================================================================
    // Watch
    // =============================================================================

    /// Watch one workspace root until cancellation.
    #[rpc(name = "Watch", response_stream(WatchEvent))]
    fn watch(request: WatchRequest) -> ();

    /// Watch one workspace branch until cancellation.
    #[rpc(name = "WatchBranch", response_stream(WatchEvent))]
    fn watch_branch(request: WatchBranchRequest) -> ();
}

impl WorkspaceService for Workspace {
    /// Read one workspace's physical revision.
    async fn revision(
        &self,
        request: Request<RevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        self.resolve_root(&request.value.root)?;

        Ok(Response::new(Workspace::revision(self)?))
    }

    /// Reload one workspace root from its host.
    async fn reload(
        &self,
        request: Request<ReloadRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        self.resolve_root(&request.value.root)?;
        let commit = Workspace::reload(self)?;

        Ok(Response::new(commit))
    }

    /// List branches in one workspace.
    async fn list_branches(
        &self,
        request: Request<ListBranchesRequest>,
    ) -> Result<Response<Vec<Branch>>, Status> {
        self.resolve_root(&request.value.root)?;

        Ok(Response::new(Workspace::branches(self)?))
    }

    /// Create one workspace branch.
    async fn create_branch(
        &self,
        request: Request<CreateBranchRequest>,
    ) -> Result<Response<Branch>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let branch = Workspace::create_branch(self, request.name, request.revision)?;

        Ok(Response::new(branch))
    }

    /// Read one workspace branch revision.
    async fn branch_revision(
        &self,
        request: Request<BranchRevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let revision = Workspace::branch_revision(self, &request.name)?;

        Ok(Response::new(revision))
    }

    /// Remove one workspace branch.
    async fn remove_branch(
        &self,
        request: Request<RemoveBranchRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        Workspace::remove_branch(self, &request.name)?;

        Ok(Response::new(()))
    }

    /// Commit source edits to physical workspace state.
    async fn edit(&self, request: Request<EditRequest>) -> Result<Response<Commit>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let commit = Workspace::edit(self, request.revision, request.edits)?;

        Ok(Response::new(commit))
    }

    /// Commit source edits to one exact branch revision.
    async fn edit_branch(
        &self,
        request: Request<EditBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let trace = self.start_trace(TraceLevel::Disabled);
        let commit = Workspace::edit_branch(
            self,
            &request.name,
            request.revision,
            request.edits,
            trace.as_ref(),
        )?;

        Ok(Response::new(commit))
    }

    /// Save selected branch files to physical state.
    async fn save_branch(
        &self,
        request: Request<SaveBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let commit = Workspace::save_branch(
            self,
            &request.name,
            request.revision,
            request.physical,
            request.files,
        )?;

        Ok(Response::new(commit))
    }

    /// Restore selected branch files from physical state.
    async fn restore_branch(
        &self,
        request: Request<RestoreBranchRequest>,
    ) -> Result<Response<Commit>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let commit = Workspace::restore_branch(
            self,
            &request.name,
            request.revision,
            request.physical,
            request.files,
        )?;

        Ok(Response::new(commit))
    }

    /// Compare two exact workspace revisions.
    async fn diff(
        &self,
        request: Request<DiffRequest>,
    ) -> Result<Response<Vec<repository::Change>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let changes = Workspace::diff(self, request.before, request.after)?;

        Ok(Response::new(changes))
    }

    /// List files at one exact workspace revision.
    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<Vec<repository::File>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let files = Workspace::files(self, request.revision)?;

        Ok(Response::new(files))
    }

    /// Format one source file or selected range.
    async fn format_file(
        &self,
        request: Request<FormatFileRequest>,
    ) -> Result<Response<Option<FileEditResponse>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let edit = Workspace::format_file(self, request.revision, request.path, request.range)?;
        let edit = edit.as_ref().map(FileEditResponse::from);

        Ok(Response::new(edit))
    }

    /// Read source files from one exact revision.
    async fn read_files(
        &self,
        request: Request<ReadFilesRequest>,
    ) -> Result<Response<Vec<FileImage>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let files = Workspace::read_files(self, request.revision, request.file_ids)?;
        let files = files
            .iter()
            .map(|file| FileImage::from(file.as_ref()))
            .collect();

        Ok(Response::new(files))
    }

    /// Check source state.
    async fn check(
        &self,
        request: Request<CheckRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<CheckOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::check(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Format source files or content.
    async fn format(
        &self,
        request: Request<FormatRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<FormatOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::format(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Query source files with one structural pattern.
    async fn query(
        &self,
        request: Request<QueryRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<QueryOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::query(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Rewrite source files with one structural pattern.
    async fn rewrite(
        &self,
        request: Request<RewriteRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<RewriteOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::rewrite(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Build target artifacts.
    async fn build(
        &self,
        request: Request<BuildRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<BuildOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::build(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Run workspace tests.
    async fn test(
        &self,
        request: Request<TestRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TestOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::test(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Generate workspace documentation.
    async fn doc(
        &self,
        request: Request<DocRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<DocOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::doc(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Return workspace information.
    async fn info(
        &self,
        request: Request<InfoRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<InfoOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::info(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Return configured targets.
    async fn targets(
        &self,
        request: Request<TargetsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TargetsOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::targets(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Return resolved settings.
    async fn settings(
        &self,
        request: Request<SettingsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<SettingsOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::settings(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Diagnose workspace configuration and state.
    async fn doctor(
        &self,
        request: Request<DoctorRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<DoctorOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::doctor(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Execute configured workspace tasks.
    async fn task(
        &self,
        request: Request<TaskRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TaskOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::task(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Clean generated workspace state.
    async fn clean(
        &self,
        request: Request<CleanRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<CleanOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::clean(self, request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Read one exact artifact payload.
    async fn artifact(
        &self,
        request: Request<ArtifactRequest>,
    ) -> Result<Response<ArtifactPayload>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let artifact = Workspace::artifact(self, request.artifact)?;

        Ok(Response::new(artifact))
    }

    /// Publish one artifact's storage bytes as a Blob.
    async fn blob(&self, request: Request<ArtifactRequest>) -> Result<Response<Blob>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let blob = Workspace::blob(self, request.artifact)?;

        Ok(Response::new(blob))
    }

    /// Materialize one artifact on the workspace host.
    async fn export(
        &self,
        request: Request<ExportRequest>,
    ) -> Result<Response<ExportResult>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let result = Workspace::export(self, request.input)?;

        Ok(Response::new(result))
    }

    /// Read exact diagnostics.
    async fn diagnose(
        &self,
        request: Request<DiagnoseRequest>,
    ) -> Result<Response<Vec<FileDiagnosticsResponse>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let diagnostics = Workspace::diagnose(self, request.revision, request.request).await?;
        let diagnostics = diagnostics
            .iter()
            .map(FileDiagnosticsResponse::from)
            .collect();

        Ok(Response::new(diagnostics))
    }

    /// Resolve one source file for semantic queries.
    async fn resolve_query_file(
        &self,
        request: Request<ResolveQueryFileRequest>,
    ) -> Result<Response<Option<QueryFileResponse>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let file = Workspace::resolve_query_file(self, request.revision, request.uri)?;
        let file = file.as_ref().map(QueryFileResponse::from);

        Ok(Response::new(file))
    }

    /// Execute one semantic query.
    async fn run_query(
        &self,
        request: Request<RunQueryRequest>,
    ) -> Result<Response<RunQueryResponse>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let response = Workspace::run_query(self, request.input).await?;

        Ok(Response::new(response))
    }

    /// Watch physical workspace state until cancellation.
    async fn watch(
        &self,
        request: Request<WatchRequest>,
        responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        self.resolve_root(&request.value.root)?;
        let watch = Workspace::watch(self)?;

        watch.send(responses).await
    }

    /// Watch one workspace branch until cancellation.
    async fn watch_branch(
        &self,
        request: Request<WatchBranchRequest>,
        responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        self.resolve_root(&request.value.root)?;
        let watch = Workspace::watch_branch(self, &request.value.name)?;

        watch.send(responses).await
    }
}

impl Watch {
    /// Send this watch over one RPC response stream.
    async fn send(
        mut self,
        mut responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        loop {
            // wait for cancellation or the next committed change
            let event = {
                let canceled = responses.canceled().fuse();
                let event = self.next().fuse();
                pin_mut!(canceled, event);

                select_biased! {
                    _ = canceled => return Ok(Response::new(())),
                    event = event => event?,
                }
            };

            // preserve RPC stream backpressure between semantic events
            responses
                .send(&event)
                .await
                .map_err(|error| error.into_status())?;
        }
    }
}
