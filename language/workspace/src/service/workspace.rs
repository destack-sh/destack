use std::sync::Arc;

use destack_artifact::ArtifactPayload;
use destack_repository::{Commit, Revision};
use destack_rpc::{Request, Response, ResponseSender, Status};
use destack_source::{Content, ContentId};
use futures::{FutureExt, pin_mut, select_biased};

use super::*;
use crate::{
    BenchOutput, BuildOutput, CacheOutput, CheckOutput, CleanOutput, CommandProgress, DocOutput,
    DoctorOutput, ExportResult, FileImage, FormatOutput, InfoOutput, ProgressEvent, QueryOutput,
    RewriteOutput, RunQueryResponse, SettingsOutput, TargetsOutput, TaskOutput, TestOutput,
    WatchEvent, Workspace,
};

impl WorkspaceService for Arc<Workspace> {
    /// Reload one workspace root from its host.
    async fn reload(
        &self,
        request: Request<ReloadRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        self.resolve_root(&request.value.root)?;
        let commit = Workspace::reload(self.as_ref())?;

        Ok(Response::new(commit))
    }

    /// Read one workspace root revision.
    async fn read_revision(
        &self,
        request: Request<ReadRevisionRequest>,
    ) -> Result<Response<Revision>, Status> {
        self.resolve_root(&request.value.root)?;
        let revision = Workspace::revision(self.as_ref())?;

        Ok(Response::new(revision))
    }

    /// Apply one editor file operation.
    async fn apply_file_operation(
        &self,
        request: Request<ApplyFileOperationRequest>,
    ) -> Result<Response<Option<Commit>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let commit = Workspace::apply_file_operation(self.as_ref(), request.operation)?;

        Ok(Response::new(commit))
    }

    /// Apply one atomic source update.
    async fn apply_source_update(
        &self,
        request: Request<ApplySourceUpdateRequest>,
    ) -> Result<Response<Commit>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let commit = Workspace::edit(self.as_ref(), request.update)?;

        Ok(Response::new(commit))
    }

    /// Return whether one source file is open.
    async fn is_file_open(
        &self,
        request: Request<IsFileOpenRequest>,
    ) -> Result<Response<bool>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let is_open = Workspace::is_file_open(self.as_ref(), &request.path)?;

        Ok(Response::new(is_open))
    }

    /// Format one source file or selected range.
    async fn format_file(
        &self,
        request: Request<FormatFileRequest>,
    ) -> Result<Response<Option<FileEditResponse>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let edit = Workspace::format_file(self.as_ref(), request.path, request.range)?;
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
        let files = Workspace::read_files(self.as_ref(), request.revision, request.file_ids)?;
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
        let command = Workspace::check(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::format(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::query(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::rewrite(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::build(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::test(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::doc(self.as_ref(), request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Run workspace benchmarks.
    async fn bench(
        &self,
        request: Request<BenchRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<BenchOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::bench(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::info(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::targets(self.as_ref(), request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Return cache locations.
    async fn cache(
        &self,
        request: Request<CacheRequest>,
        responses: ResponseSender<ProgressEvent>,
    ) -> Result<Response<CacheOutput>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let (progress, events) = CommandProgress::channel();
        let command = Workspace::cache(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::settings(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::doctor(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::task(self.as_ref(), request.input, Some(progress));

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
        let command = Workspace::clean(self.as_ref(), request.input, Some(progress));

        events.forward(responses, command).await
    }

    /// Read one exact artifact payload.
    async fn artifact(
        &self,
        request: Request<ArtifactRequest>,
    ) -> Result<Response<ArtifactPayload>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let artifact = Workspace::artifact(self.as_ref(), request.artifact)?;

        Ok(Response::new(artifact))
    }

    /// Store one content value.
    async fn store(&self, request: Request<StoreRequest>) -> Result<Response<ContentId>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let content = Workspace::store(self.as_ref(), request.content)?;

        Ok(Response::new(content))
    }

    /// Load one content value.
    async fn load(&self, request: Request<LoadRequest>) -> Result<Response<Content>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let content = Workspace::load(self.as_ref(), request.content)?;

        Ok(Response::new(content))
    }

    /// Materialize one artifact on the workspace host.
    async fn export(
        &self,
        request: Request<ExportRequest>,
    ) -> Result<Response<ExportResult>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let result = Workspace::export(self.as_ref(), request.input)?;

        Ok(Response::new(result))
    }

    /// Read exact diagnostics.
    async fn diagnose(
        &self,
        request: Request<DiagnoseRequest>,
    ) -> Result<Response<Vec<FileDiagnosticsResponse>>, Status> {
        let request = request.value;
        self.resolve_root(&request.root)?;
        let diagnostics = Workspace::diagnose(self.as_ref(), request.request).await?;
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
        let file = Workspace::resolve_query_file(self.as_ref(), request.path)?;
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
        let response = Workspace::run_query(self.as_ref(), request.input).await?;

        Ok(Response::new(response))
    }

    /// Watch one workspace root until cancellation.
    async fn watch(
        &self,
        request: Request<WatchRequest>,
        mut responses: ResponseSender<WatchEvent>,
    ) -> Result<Response<()>, Status> {
        self.resolve_root(&request.value.root)?;
        let mut watch = Workspace::watch(self.as_ref())?;

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
