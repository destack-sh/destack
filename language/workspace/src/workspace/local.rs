use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::diagnostic::{DiagnosticsRequest, Error, FileDiagnostics};
use crate::file::{Commit, FileOperation, OpenFile, SourceUpdate};
use crate::watch::Watch;
use crate::{
    BenchInput, BenchOptions, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOptions,
    CacheOutput, CheckInput, CheckOutput, CleanInput, CleanOptions, CleanOutput, CommandContext,
    CommandError, CommandOptions, CommandOutcome, CommandProgress, CommandResult, CommandRevision,
    DocInput, DocOptions, DocOutput, DoctorInput, DoctorOptions, DoctorOutput, FormatInput,
    FormatOutput, InfoInput, InfoOptions, InfoOutput, Output, OutputBuffer, ProgressEvent,
    QueryInput, QueryOutput, RewriteInput, RewriteOutput, RunInput, RunOutput, SettingsInput,
    SettingsOptions, SettingsOutput, TargetsInput, TargetsOptions, TargetsOutput, TaskInput,
    TaskOptions, TaskOutput, TestInput, TestOptions, TestOutput, Workspace,
};
use dashmap::DashMap;
use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactReference, Bundle, BundleFile, Product,
};
use destack_repository::{Ref, Repository, Revision, Trace, TraceSnapshot, TraceView};
use destack_session::{SessionEvent, SessionEventHandler};
use destack_source::{
    Content, ContentId, DiagnosticCollection, Edit, File, FileId, OverlayFileSystem, TextRange,
};
use futures::future::BoxFuture;
use parking_lot::Mutex;

use super::root::WorkspaceRoot;
use super::{RunQueryInput, RunQueryResponse, SessionPin};
use crate::{ExportInput, ExportResult, ExportedFile, FileEdit, FileImage, QueryFile};

/// Local workspace used by tooling integrations.
pub struct LocalWorkspace {
    /// Repository for workspace resolution.
    pub(crate) repository: Arc<Repository>,
    /// Opened roots keyed by root path.
    pub(super) roots: DashMap<PathBuf, Arc<WorkspaceRoot>>,
    /// Open files keyed by source path.
    pub(crate) open_file_by_path: DashMap<PathBuf, OpenFile>,
    /// Overlay filesystem shared by live sessions.
    pub(crate) overlay_file_system: Option<Arc<OverlayFileSystem>>,

    /// Number of workers for each opened session.
    pub(crate) worker_limit: usize,

    /// Optional session event handler for local progress reporting.
    pub(super) event_handler: Option<SessionEventHandler>,
    /// Next id for private command session refs.
    pub(crate) next_command_session_id: AtomicU64,
}

impl std::fmt::Debug for LocalWorkspace {
    /// Format the visible workspace state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalWorkspace")
            .field("repository", &self.repository)
            .field("roots", &self.roots)
            .field("open_file_by_path", &self.open_file_by_path.len())
            .field("overlay_file_system", &self.overlay_file_system.is_some())
            .field("worker_limit", &self.worker_limit)
            .field("event_handler", &self.event_handler.is_some())
            .field("next_command_session_id", &self.next_command_session_id)
            .finish()
    }
}

impl LocalWorkspace {
    /// Return the default worker count for a local workspace.
    pub fn default_worker_count() -> usize {
        std::thread::available_parallelism().map_or(1, usize::from)
    }

    /// Create a local workspace for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        overlay_file_system: Option<Arc<OverlayFileSystem>>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, Error> {
        let workspace = Self {
            repository,
            roots: dashmap::DashMap::new(),
            open_file_by_path: dashmap::DashMap::new(),
            overlay_file_system,
            worker_limit,
            event_handler,
            next_command_session_id: AtomicU64::new(1),
        };

        for root in roots {
            workspace.open_root(root)?;
        }

        Ok(workspace)
    }

    /// Return opened roots as a stable path list.
    pub fn root_paths(&self) -> Vec<PathBuf> {
        self.roots.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Snapshot one workspace operation trace with repository display names.
    pub fn snapshot_trace(
        &self,
        revision: Revision,
        trace: &Trace,
        view: TraceView,
    ) -> Result<TraceSnapshot, Error> {
        trace.snapshot(
            view,
            |key| {
                self.repository
                    .artifact_display(revision, *key)
                    .map_err(Error::from)
            },
            |target| {
                self.repository
                    .target_display(revision, target)
                    .map_err(Error::from)
            },
        )
    }

    /// Allocate one private session ref for a command.
    pub(crate) fn next_command_session_ref(&self, root: &Path) -> Ref {
        let id = self.next_command_session_id.fetch_add(1, Ordering::Relaxed);

        Ref::new(format!("command:{}:{id}", root.display()))
    }

    /// Execute one command operation for the given root.
    pub(crate) async fn run_command<'a, T, O>(
        &self,
        root: &'a Path,
        common: &'a CommandOptions,
        revision: CommandRevision,
        progress: Option<CommandProgress>,
        execute: impl for<'context> FnOnce(
            &'context mut CommandContext<'_>,
        )
            -> BoxFuture<'context, CommandResult<CommandOutcome<T>>>,
    ) -> CommandResult<O>
    where
        O: From<Output<T>>,
    {
        let repository = Arc::clone(&self.repository);

        let event_handler = progress.map(session_progress_handler);

        // gather shared context
        let mut output = OutputBuffer::default();
        let mut context = CommandContext::new(
            self,
            root.to_path_buf(),
            repository,
            common,
            revision,
            &mut output,
            event_handler,
        )?;

        // execute the requested operation
        let result = execute(&mut context).await?;

        // finalize command output
        let CommandOutcome {
            diagnostics,
            exit_code,
            messages,
            files,
            data,
            module_count,
            profile_count,
            target_count,
        } = result;
        let revision = context.revision()?;
        let files = command_file_images(&context, revision, &diagnostics, &files)?;
        let success = exit_code == 0;
        let trace = common
            .trace
            .map(|view| context.command_trace(revision, view))
            .transpose()?;

        let output = Output {
            revision,
            success,
            exit_code,
            diagnostics: diagnostics.iter().cloned().collect(),
            files,
            messages,
            output: output.chunks,
            outputs: Vec::new(),
            trace,
            data,
            module_count,
            profile_count,
            target_count,
        };

        Ok(output.into())
    }
}

/// Build one session event handler forwarding throttled progress.
///
/// Counting stays exact; emission throttles to one event per interval
/// so slow transports never stall the workers.
fn session_progress_handler(progress: CommandProgress) -> SessionEventHandler {
    let interval = progress.interval();
    let throttle = Mutex::new((None::<Instant>, 0usize));

    Arc::new(move |event| match event {
        SessionEvent::TaskFinished { artifact_key, .. }
        | SessionEvent::TaskFailed { artifact_key, .. } => {
            let mut throttle = throttle.lock();
            throttle.1 += 1;
            let due = throttle.0.is_none_or(|last| last.elapsed() >= interval);
            if due {
                throttle.0 = Some(Instant::now());
                let _is_queued = progress.try_emit(ProgressEvent {
                    task: artifact_key.stage().name().to_string(),
                    message: Some(format!("{} artifacts", throttle.1)),
                    percent: None,
                });
            }
        }
        _ => {}
    })
}

/// Return file images for diagnostics emitted by one command.
fn command_file_images(
    context: &CommandContext<'_>,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
    sources: &[Arc<File>],
) -> CommandResult<Vec<FileImage>> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    // retain files referenced by command data
    for file in sources {
        if seen.insert(file.id) {
            files.push(FileImage::from(file.as_ref()));
        }
    }

    // collect every file referenced by labels and suggestion patches
    for diagnostic in diagnostics.iter() {
        let mut file_ids = vec![diagnostic.primary_label().target.file()];
        file_ids.extend(diagnostic.labels().map(|label| label.target.file()));
        file_ids.extend(
            diagnostic
                .suggestions
                .iter()
                .flat_map(|suggestion| &suggestion.patches.files)
                .map(|patch| patch.file),
        );

        for file_id in file_ids {
            if !seen.insert(file_id) {
                continue;
            }

            let file = context.file(revision, file_id)?.ok_or_else(|| {
                CommandError::internal(format!(
                    "diagnostic references missing source file {file_id:?}"
                ))
            })?;
            files.push(FileImage::from(file.as_ref()));
        }
    }

    Ok(files)
}

impl Workspace for LocalWorkspace {
    fn home(&self) -> &Path {
        self.repository.path()
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, Error> {
        self.repository
            .file_system()
            .canonicalize(path)
            .map_err(|error| Error::Io {
                path: path.to_path_buf(),
                source: error,
            })
    }

    fn roots(&self) -> Vec<PathBuf> {
        LocalWorkspace::root_paths(self)
    }

    fn open(&self, root: PathBuf) -> Result<(), Error> {
        LocalWorkspace::open_root(self, root)
    }

    fn close(&self, root: &Path) -> Result<(), Error> {
        LocalWorkspace::close_root(self, root)
    }

    fn root(&self, path: &Path) -> Result<PathBuf, Error> {
        LocalWorkspace::root_at(self, path)
    }

    fn revision(&self, root: &Path) -> Result<Revision, Error> {
        LocalWorkspace::revision(self, root)
    }

    fn reload(&self, root: &Path) -> Result<Option<Commit>, Error> {
        LocalWorkspace::reload_root(self, root)
    }

    fn file(&self, root: &Path, operation: FileOperation) -> Result<Option<Commit>, Error> {
        LocalWorkspace::apply_file_operation(self, root, operation)
    }

    fn is_file_open(&self, root: &Path, path: &Path) -> Result<bool, Error> {
        let root = self.workspace_root(root)?;
        let path = self.resolve_path(root.as_ref(), path)?;

        Ok(LocalWorkspace::has_open_file(self, &path))
    }

    fn edit(&self, root: &Path, update: SourceUpdate) -> Result<Commit, Error> {
        if let Some(base) = update.base {
            LocalWorkspace::apply_source_edits_if_current(self, root, base, update.edits)
        } else {
            LocalWorkspace::apply_source_edits(self, root, update.edits)
        }
    }

    fn check<'a>(
        &'a self,
        root: &'a Path,
        request: CheckInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CheckOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_check_command(&request).await })
            })
            .await
        })
    }

    fn format<'a>(
        &'a self,
        root: &'a Path,
        request: FormatInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<FormatOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            let command_root = root.to_path_buf();
            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_format_command(&command_root, &request.source, request.mode)
                })
            })
            .await
        })
    }

    fn query<'a>(
        &'a self,
        root: &'a Path,
        request: QueryInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<QueryOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_query_command(&request).await })
            })
            .await
        })
    }

    fn rewrite<'a>(
        &'a self,
        root: &'a Path,
        request: RewriteInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<RewriteOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_rewrite_command(&request).await })
            })
            .await
        })
    }

    fn build<'a>(
        &'a self,
        root: &'a Path,
        request: BuildInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<BuildOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_build_command(&request).await })
            })
            .await
        })
    }

    fn run<'a>(
        &'a self,
        root: &'a Path,
        request: RunInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<RunOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.execute_run_command(&request).await })
            })
            .await
        })
    }

    fn test<'a>(
        &'a self,
        root: &'a Path,
        request: TestInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TestOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_test_command(&TestOptions::default()) })
            })
            .await
        })
    }

    fn doc<'a>(
        &'a self,
        root: &'a Path,
        request: DocInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DocOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_doc_command(&DocOptions::default()) })
            })
            .await
        })
    }

    fn bench<'a>(
        &'a self,
        root: &'a Path,
        request: BenchInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<BenchOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_bench_command(&BenchOptions::default()) })
            })
            .await
        })
    }

    fn info<'a>(
        &'a self,
        root: &'a Path,
        request: InfoInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<InfoOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_info_command(&InfoOptions { all: request.all }) })
            })
            .await
        })
    }

    fn targets<'a>(
        &'a self,
        root: &'a Path,
        request: TargetsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TargetsOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_targets_command(&TargetsOptions { all: request.all })
                })
            })
            .await
        })
    }

    fn cache<'a>(
        &'a self,
        root: &'a Path,
        request: CacheInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CacheOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_cache_command(&CacheOptions) })
            })
            .await
        })
    }

    fn settings<'a>(
        &'a self,
        root: &'a Path,
        request: SettingsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<SettingsOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_settings_command(&SettingsOptions) })
            })
            .await
        })
    }

    fn doctor<'a>(
        &'a self,
        root: &'a Path,
        request: DoctorInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DoctorOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_doctor_command(&DoctorOptions { full: request.full })
                })
            })
            .await
        })
    }

    fn task<'a>(
        &'a self,
        root: &'a Path,
        request: TaskInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TaskOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_task_command(&TaskOptions {
                        action: request.action.clone(),
                        projects: request.projects.clone(),
                        groups: request.groups.clone(),
                    })
                })
            })
            .await
        })
    }

    fn clean<'a>(
        &'a self,
        root: &'a Path,
        request: CleanInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CleanOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            let command_root = root.to_path_buf();
            self.run_command(root, &common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_clean_command(
                        &command_root,
                        &CleanOptions {
                            dir: request.dir.clone(),
                            dist: request.dist,
                            cache: request.cache,
                            all: request.all,
                            all_packages: request.all_packages,
                        },
                    )
                })
            })
            .await
        })
    }

    fn format_file(
        &self,
        root: &Path,
        path: PathBuf,
        range: Option<TextRange>,
    ) -> Result<Option<FileEdit>, Error> {
        LocalWorkspace::format_file(self, root, path, range)
    }

    fn read_files(
        &self,
        root: &Path,
        revision: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<Vec<Arc<File>>, Error> {
        // pin the requested immutable revision
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = repository.pin(revision)?;
        let session = SessionPin::new(session, revision);

        // read every requested file exactly
        let mut files = Vec::with_capacity(file_ids.len());
        for file_id in file_ids {
            files.push(session.file(file_id)?);
        }

        Ok(files)
    }

    fn run_query<'a>(
        &'a self,
        root: &'a Path,
        request: RunQueryInput,
    ) -> BoxFuture<'a, Result<RunQueryResponse, Error>> {
        Box::pin(LocalWorkspace::run_query(self, root, request))
    }

    fn resolve_query_file(&self, root: &Path, path: PathBuf) -> Result<Option<QueryFile>, Error> {
        LocalWorkspace::resolve_query_file(self, root, path)
    }

    fn diagnose(
        &self,
        request: DiagnosticsRequest,
    ) -> BoxFuture<'_, Result<Vec<FileDiagnostics>, Error>> {
        Box::pin(LocalWorkspace::diagnose(self, request))
    }

    fn artifact(&self, root: &Path, artifact: ArtifactReference) -> Result<ArtifactPayload, Error> {
        LocalWorkspace::revision(self, root)?;
        let payload = self.artifact_payload(artifact)?;

        Ok(payload)
    }

    fn store(&self, content: Content) -> Result<ContentId, Error> {
        self.repository.intern_content(content).map_err(Error::from)
    }

    fn load(&self, content: ContentId) -> Result<Content, Error> {
        let entry = self.repository.content(content)?;

        Ok(entry.payload().clone())
    }

    fn export(&self, root: &Path, request: ExportInput) -> Result<ExportResult, Error> {
        let revision = LocalWorkspace::revision(self, root)?;
        let payload = self.artifact(root, request.artifact)?;

        match payload {
            ArtifactPayload::Bundle(bundle) => self.export_bundle(root, bundle.as_ref(), &request),
            ArtifactPayload::Product(product) => {
                self.export_product(root, revision, product.as_ref(), &request)
            }
            payload => Err(Error::InvalidEdit {
                detail: format!(
                    "workspace export cannot materialize {} artifacts",
                    payload.name()
                ),
            }),
        }
    }

    fn watch(&self, root: &Path) -> Result<Watch, Error> {
        let root = self.workspace_root(root)?;

        root.watch()
    }
}

impl LocalWorkspace {
    /// Return one artifact payload by exact version.
    fn artifact_payload(&self, artifact: ArtifactReference) -> Result<ArtifactPayload, Error> {
        if artifact.key != artifact.version.key {
            return Err(Error::Internal {
                detail: format!(
                    "artifact reference key {:?} does not match version {:?}",
                    artifact.key, artifact.version
                ),
            });
        }

        if let Some(payload) = self.artifact_payload_in_memory(artifact)? {
            return Ok(payload);
        }

        if let Some(payload) = self.repository.load_artifact(artifact.version)? {
            return Ok(payload);
        }

        Err(Error::Internal {
            detail: format!("artifact payload is missing for {:?}", artifact.version),
        })
    }

    /// Return one in-memory artifact payload by exact version.
    fn artifact_payload_in_memory(
        &self,
        artifact: ArtifactReference,
    ) -> Result<Option<ArtifactPayload>, Error> {
        let payload = self.repository.artifact_table().payload(&artifact.version);

        Ok(payload)
    }

    /// Return one artifact payload by key in one revision.
    fn artifact_payload_for_key(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> Result<ArtifactPayload, Error> {
        let version = self
            .repository
            .artifact_version(revision, &key)?
            .ok_or_else(|| Error::Internal {
                detail: format!("artifact version is missing for {key:?}"),
            })?;
        let reference = ArtifactReference { key, version };

        self.artifact_payload(reference)
    }

    /// Export one bundle to the host filesystem.
    fn export_bundle(
        &self,
        root: &Path,
        bundle: &Bundle,
        request: &ExportInput,
    ) -> Result<ExportResult, Error> {
        let mut files = Vec::new();

        // materialize each bundle file under the requested directory
        for file in bundle.files() {
            let file = self.export_bundle_file(root, file, request)?;
            files.push(file);
        }

        Ok(ExportResult { files })
    }

    /// Export one product to the host filesystem.
    fn export_product(
        &self,
        root: &Path,
        revision: Revision,
        product: &Product,
        request: &ExportInput,
    ) -> Result<ExportResult, Error> {
        let mut files = Vec::new();

        // materialize each bundle included by the product
        for target in product.targets.values() {
            if !target.includes_bundle {
                continue;
            }

            let key = ArtifactKey::bundle(target.target.package_id(), target.target);
            let payload = self.artifact_payload_for_key(revision, key)?;
            let ArtifactPayload::Bundle(bundle) = payload else {
                return Err(Error::Internal {
                    detail: format!("product target did not resolve to bundle artifact: {key:?}"),
                });
            };
            let result = self.export_bundle(root, bundle.as_ref(), request)?;
            files.extend(result.files);
        }

        Ok(ExportResult { files })
    }

    /// Export one bundle file to the host filesystem.
    fn export_bundle_file(
        &self,
        root: &Path,
        file: &BundleFile,
        request: &ExportInput,
    ) -> Result<ExportedFile, Error> {
        let path = self.export_file_path(root, file, request)?;
        let content = self.repository.content(file.content)?.payload().clone();

        // refuse to overwrite existing output unless explicitly allowed
        let exists = self
            .repository
            .file_system()
            .exists(&path)
            .map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
        if exists && !request.overwrite {
            return Err(Error::InvalidEdit {
                detail: format!("export output already exists: {}", path.display()),
            });
        }

        // ensure the output directory exists
        if let Some(parent) = path.parent() {
            self.repository
                .file_system()
                .create_dir_all(parent)
                .map_err(|source| Error::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
        }

        // write the payload in its native content representation
        let size_bytes = match content {
            Content::Text { content } => {
                let size_bytes = content.len() as u64;
                self.repository
                    .file_system()
                    .write_string(&path, &content)
                    .map_err(|source| Error::Io {
                        path: path.clone(),
                        source,
                    })?;

                size_bytes
            }
            Content::Binary { content } => {
                let size_bytes = content.len() as u64;
                self.repository
                    .file_system()
                    .write(&path, &content)
                    .map_err(|source| Error::Io {
                        path: path.clone(),
                        source,
                    })?;

                size_bytes
            }
        };

        Ok(ExportedFile {
            path,
            content: file.content,
            size_bytes,
        })
    }

    /// Resolve one bundle file export path.
    fn export_file_path(
        &self,
        root: &Path,
        file: &BundleFile,
        request: &ExportInput,
    ) -> Result<PathBuf, Error> {
        let Some(source_path) = file.uri.to_path_buf() else {
            return Err(Error::InvalidEdit {
                detail: format!("bundle file URI is not path-like: {}", file.uri),
            });
        };
        let relative = if source_path.is_absolute() {
            source_path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .map_err(|_| Error::InvalidEdit {
                    detail: format!(
                        "bundle file path is outside export root: {}",
                        source_path.display()
                    ),
                })?
        } else {
            source_path
        };

        // reject paths that escape the output directory
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(Error::InvalidEdit {
                detail: format!(
                    "bundle file path is not export-safe: {}",
                    relative.display()
                ),
            });
        }

        Ok(request.directory.join(relative))
    }

    /// Apply one local file operation.
    fn apply_file_operation(
        &self,
        root: &Path,
        operation: FileOperation,
    ) -> Result<Option<Commit>, Error> {
        let workspace_root = self.workspace_root(root)?;
        let operation = self.resolve_operation(workspace_root.as_ref(), operation)?;

        match operation {
            FileOperation::OpenText {
                path,
                uri,
                version,
                content,
            } => self
                .open_file(
                    uri,
                    version,
                    Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map(Some),
            FileOperation::OpenBytes {
                path,
                uri,
                version,
                content,
            } => self
                .open_file(
                    uri,
                    version,
                    Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map(Some),
            FileOperation::ChangeText {
                path,
                uri,
                version,
                content,
            } => self
                .change_file(
                    uri,
                    version,
                    Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map(Some),
            FileOperation::ChangeBytes {
                path,
                uri,
                version,
                content,
            } => self
                .change_file(
                    uri,
                    version,
                    Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map(Some),
            FileOperation::PatchText {
                path,
                uri,
                version,
                changes,
            } => self.patch_text_file(&path, uri, version, changes).map(Some),
            FileOperation::SaveText { path, content } => {
                self.save_text_file(&path, content).map(Some)
            }
            FileOperation::SaveBytes { path, content } => {
                self.save_bytes_file(&path, content).map(Some)
            }
            FileOperation::Close { path } => self.close_file(&path),
            FileOperation::WriteText { path, content } => self
                .write_file(Edit::SetText {
                    path,
                    text: content,
                })
                .map(Some),
            FileOperation::WriteBytes { path, content } => self
                .write_file(Edit::SetBytes {
                    path,
                    bytes: content,
                })
                .map(Some),
            FileOperation::Remove { path } => self.write_file(Edit::Remove { path }).map(Some),
            FileOperation::Move { from, to } => self.write_file(Edit::Move { from, to }).map(Some),
        }
    }
}
