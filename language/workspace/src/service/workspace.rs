use std::future::Future;
use std::sync::Arc;

use destack_artifact::ArtifactPayload;
use destack_repository::Revision;
use destack_rpc::{Code, Request, Response, ResponseSender, Status};
use destack_source::{Content, ContentId};
use futures::{FutureExt, pin_mut, select_biased};

use super::*;
use crate::{
    BenchOutput, BuildOutput, CacheOutput, CheckOutput, CleanOutput, CommandError, CommandProgress,
    Commit, DiagnosticsRequest, DocOutput, DoctorOutput, ExportResult, FileImage, FormatOutput,
    InfoOutput, ProgressEvent, ProgressEvents, QueryOutput, RewriteOutput, RunOutput,
    RunQueryResponse, SettingsOutput, TargetsOutput, TaskOutput, TestOutput, WatchEvent, Workspace,
};

/// Workspace shared by one or more RPC clients.
#[derive(Debug)]
pub struct SharedWorkspace {
    /// Workspace implementation.
    workspace: Arc<dyn Workspace>,
}

impl SharedWorkspace {
    /// Create one shared RPC workspace.
    pub fn new(workspace: Arc<dyn Workspace>) -> Self {
        Self { workspace }
    }

    /// Execute one command while forwarding bounded progress events.
    async fn forward_command<T>(
        mut responses: ResponseSender<ProgressEvent>,
        command: impl Future<Output = Result<T, CommandError>> + Send,
        mut events: ProgressEvents,
    ) -> Result<Response<T>, Status> {
        let command = command.fuse();
        pin_mut!(command);

        loop {
            // wait for cancellation, command completion, or the next progress event
            let event = {
                let canceled = responses.canceled().fuse();
                let event = events.receive().fuse();
                pin_mut!(canceled, event);

                select_biased! {
                    _ = canceled => {
                        return Err(Status::new(Code::Canceled, "RPC call was canceled"));
                    },
                    output = command => {
                        return output.map(Response::new).map_err(Status::from);
                    },
                    event = event => event,
                }
            };
            let Some(event) = event else {
                return command.await.map(Response::new).map_err(Status::from);
            };

            // preserve RPC stream backpressure between progress events
            responses
                .send(&event)
                .await
                .map_err(|error| error.into_status())?;
        }
    }
}

impl WorkspaceService for SharedWorkspace {
    /// Open one workspace root.
    async fn open_root(
        &self,
        request: Request<OpenRootRequest>,
    ) -> Result<Response<OpenRootResponse>, Status> {
        let root = self.workspace.canonicalize(&request.value.root)?;
        self.workspace.open(root.clone())?;
        let revision = self.workspace.revision(&root)?;

        Ok(Response::new(OpenRootResponse { root, revision }))
    }

    /// Reload one workspace root from its host.
    async fn reload(
        &self,
        request: Request<ReloadRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let commit = self.workspace.reload(&request.value.root)?;

        Ok(Response::new(commit))
    }

    /// Read one workspace root revision.
    async fn read_revision(
        &self,
        request: Request<ReadRevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        let revision = self.workspace.revision(&request.value.root)?;

        Ok(Response::new(revision))
    }

    /// Apply one editor file operation.
    async fn apply_file_operation(
        &self,
        request: Request<ApplyFileOperationRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let request = request.value;
        let commit = self.workspace.file(&request.root, request.operation)?;

        Ok(Response::new(commit))
    }

    /// Apply one atomic source update.
    async fn apply_source_update(
        &self,
        request: Request<ApplySourceUpdateRequest>,
    ) -> Result<Response<Commit>, Status> {
        let request = request.value;
        let commit = self.workspace.edit(&request.root, request.update)?;

        Ok(Response::new(commit))
    }

    /// Return whether one source file is open.
    async fn is_file_open(
        &self,
        request: Request<IsFileOpenRequest>,
    ) -> Result<Response<bool>, Status> {
        let request = request.value;
        let is_open = self.workspace.is_file_open(&request.root, &request.path)?;

        Ok(Response::new(is_open))
    }

    /// Format one source file or selected range.
    async fn format_file(
        &self,
        request: Request<FormatFileRequest>,
    ) -> Result<Response<Option<FileEditResponse>>, Status> {
        let request = request.value;
        let edit = self
            .workspace
            .format_file(&request.root, request.path, request.range)?;
        let edit = edit.as_ref().map(FileEditResponse::from);

        Ok(Response::new(edit))
    }

    /// Read source files from one exact revision.
    async fn read_files(
        &self,
        request: Request<ReadFilesRequest>,
    ) -> Result<Response<Vec<FileImage>>, Status> {
        let request = request.value;
        let files = self
            .workspace
            .read_files(&request.root, request.revision, request.file_ids)?;
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
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .check(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Format source files or content.
    async fn format(
        &self,
        request: Request<FormatRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<FormatOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .format(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Query source files with one structural pattern.
    async fn query(
        &self,
        request: Request<QueryRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<QueryOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .query(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Rewrite source files with one structural pattern.
    async fn rewrite(
        &self,
        request: Request<RewriteRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<RewriteOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .rewrite(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Build target artifacts.
    async fn build(
        &self,
        request: Request<BuildRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<BuildOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .build(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Run one workspace target.
    async fn run(
        &self,
        request: Request<RunRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<RunOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .run(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Run workspace tests.
    async fn test(
        &self,
        request: Request<TestRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TestOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .test(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Generate workspace documentation.
    async fn doc(
        &self,
        request: Request<DocRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<DocOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .doc(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Run workspace benchmarks.
    async fn bench(
        &self,
        request: Request<BenchRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<BenchOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .bench(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Return workspace information.
    async fn info(
        &self,
        request: Request<InfoRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<InfoOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .info(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Return configured targets.
    async fn targets(
        &self,
        request: Request<TargetsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TargetsOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .targets(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Return cache locations.
    async fn cache(
        &self,
        request: Request<CacheRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<CacheOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .cache(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Return resolved settings.
    async fn settings(
        &self,
        request: Request<SettingsRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<SettingsOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .settings(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Diagnose workspace configuration and state.
    async fn doctor(
        &self,
        request: Request<DoctorRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<DoctorOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .doctor(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Execute configured workspace tasks.
    async fn task(
        &self,
        request: Request<TaskRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<TaskOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .task(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Clean generated workspace state.
    async fn clean(
        &self,
        request: Request<CleanRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<CleanOutput>, Status> {
        let request = request.value;
        let (progress, events) = CommandProgress::channel();
        let command = self
            .workspace
            .clean(&request.root, request.input, Some(progress));

        Self::forward_command(responses, command, events).await
    }

    /// Read one exact artifact payload.
    async fn artifact(
        &self,
        request: Request<ArtifactRequest>,
    ) -> Result<Response<ArtifactPayload>, Status> {
        let request = request.value;
        let artifact = self.workspace.artifact(&request.root, request.artifact)?;

        Ok(Response::new(artifact))
    }

    /// Store one content value.
    async fn store(&self, request: Request<StoreRequest>) -> Result<Response<ContentId>, Status> {
        let content = self.workspace.store(request.value.content)?;

        Ok(Response::new(content))
    }

    /// Load one content value.
    async fn load(&self, request: Request<LoadRequest>) -> Result<Response<Content>, Status> {
        let content = self.workspace.load(request.value.content)?;

        Ok(Response::new(content))
    }

    /// Materialize one artifact on the workspace host.
    async fn export(
        &self,
        request: Request<ExportRequest>,
    ) -> Result<Response<ExportResult>, Status> {
        let request = request.value;
        let result = self.workspace.export(&request.root, request.input)?;

        Ok(Response::new(result))
    }

    /// Read exact diagnostics.
    async fn diagnose(
        &self,
        request: Request<DiagnosticsRequest>,
    ) -> Result<Response<Vec<FileDiagnosticsResponse>>, Status> {
        let diagnostics = self.workspace.diagnose(request.value).await?;
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
        let file = self
            .workspace
            .resolve_query_file(&request.root, request.path)?;
        let file = file.as_ref().map(QueryFileResponse::from);

        Ok(Response::new(file))
    }

    /// Execute one semantic query.
    async fn run_query(
        &self,
        request: Request<RunQueryRequest>,
    ) -> Result<Response<RunQueryResponse>, Status> {
        let request = request.value;
        let response = self
            .workspace
            .run_query(&request.root, request.input)
            .await?;

        Ok(Response::new(response))
    }

    /// Watch one workspace root until cancellation.
    async fn watch(
        &self,
        request: Request<WatchRequest>,
        mut responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        let mut watch = self.workspace.watch(&request.value.root)?;

        loop {
            // wait for cancellation or the next committed change
            let event = {
                let canceled = responses.canceled().fuse();
                let event = watch.next().fuse();
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
