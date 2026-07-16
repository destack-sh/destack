use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use crate::diagnostic::{DiagnosticView, Error};
use crate::file::{Commit, FileOperation, OpenFile, SourceUpdate};
use crate::protocol::WatchPolicy;
use crate::watch::{Watch, WatchUpdate};
use crate::{
    BenchInput, BenchOptions, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOptions,
    CacheOutput, CheckInput, CheckOutput, CleanInput, CleanOptions, CleanOutput, CommandContext,
    CommandError, CommandOptions, CommandOutcome, CommandProgress, CommandResult, CommandRevision,
    DocInput, DocOptions, DocOutput, DoctorInput, DoctorOptions, DoctorOutput, FormatInput,
    FormatOutput, InfoInput, InfoOptions, InfoOutput, Output, OutputBuffer, ProgressEvent,
    RunInput, RunOutput, SettingsInput, SettingsOptions, SettingsOutput, TargetsInput,
    TargetsOptions, TargetsOutput, TaskInput, TaskOptions, TaskOutput, TestInput, TestOptions,
    TestOutput, Workspace, source_watch_options,
};
use dashmap::DashMap;
use destack_artifact::{
    ArtifactKey, ArtifactPayload, ArtifactReference, Bundle, BundleFile, Product,
};
use destack_repository::{Ref, Repository, Revision};
use destack_session::{Session, SessionEvent, SessionEventHandler};
use destack_source::{
    Content, ContentId, DiagnosticCollection, Edit, FileWatcher, OverlayFileSystem,
};
use parking_lot::Mutex;

use super::{
    DiagnosticsRequest, QueryRequest, QueryResult, ReloadRequest, UpdateBatch, ViewRequest,
    ViewResult,
};
use crate::{ExportRequest, ExportResult, ExportedFile};

/// Local workspace used by tooling integrations.
pub struct LocalWorkspace {
    /// Repository for workspace resolution.
    pub(crate) repository: Arc<Repository>,
    /// Sessions keyed by root path.
    pub(super) roots: DashMap<PathBuf, Arc<Session>>,
    /// Open files keyed by source path.
    pub(crate) open_file_by_path: DashMap<PathBuf, OpenFile>,
    /// Overlay filesystem shared by live sessions.
    pub(crate) overlay_file_system: Option<Arc<OverlayFileSystem>>,

    /// File watcher used for local watch mode.
    pub(crate) file_watcher: Option<Arc<dyn FileWatcher>>,
    /// Active workspace watches keyed by primary root path.
    pub(crate) watches: DashMap<PathBuf, Arc<Watch>>,
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
            .field("sessions_by_root", &self.roots)
            .field("open_file_by_path", &self.open_file_by_path.len())
            .field("overlay_file_system", &self.overlay_file_system.is_some())
            .field("file_watcher", &self.file_watcher.is_some())
            .field("watches", &self.watches.len())
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
        file_watcher: Option<Arc<dyn FileWatcher>>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, Error> {
        let workspace = Self {
            repository,
            roots: dashmap::DashMap::new(),
            open_file_by_path: dashmap::DashMap::new(),
            overlay_file_system,
            file_watcher,
            watches: dashmap::DashMap::new(),
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

    /// Allocate one private session ref for a command.
    pub(crate) fn next_command_session_ref(&self, root: &Path) -> Ref {
        let id = self.next_command_session_id.fetch_add(1, Ordering::Relaxed);

        Ref::new(format!("command:{}:{id}", root.display()))
    }

    /// Execute one command operation for the given root.
    pub(crate) fn run_command<T, O>(
        &self,
        root: &Path,
        common: &CommandOptions,
        revision: CommandRevision,
        progress: Option<CommandProgress<'_>>,
        execute: impl FnOnce(&mut CommandContext<'_>) -> CommandResult<CommandOutcome<T>>,
    ) -> CommandResult<O>
    where
        O: From<Output<T>>,
    {
        std::thread::scope(|scope| {
            // forward throttled session progress to the connection
            let event_handler = progress.map(|progress| {
                let (sender, receiver) = mpsc::channel::<ProgressEvent>();
                scope.spawn(move || {
                    while let Ok(event) = receiver.recv() {
                        progress.emit(event);
                    }
                });

                session_progress_handler(sender, progress.interval())
            });

            self.run_command_inner(root, common, revision, event_handler, execute)
        })
    }

    /// Execute one command operation against one forked command session.
    fn run_command_inner<T, O>(
        &self,
        root: &Path,
        common: &CommandOptions,
        revision: CommandRevision,
        event_handler: Option<SessionEventHandler>,
        execute: impl FnOnce(&mut CommandContext<'_>) -> CommandResult<CommandOutcome<T>>,
    ) -> CommandResult<O>
    where
        O: From<Output<T>>,
    {
        let repository = Arc::clone(&self.repository);
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
        let result = execute(&mut context)?;

        // finalize command output
        let CommandOutcome {
            diagnostics,
            exit_code,
            messages,
            data,
            module_count,
            profile_count,
            target_count,
        } = result;
        let revision = context.revision()?;
        let files = command_file_images(&context, revision, &diagnostics)?;
        let success = exit_code == 0;

        let output = Output {
            revision,
            success,
            exit_code,
            diagnostics: diagnostics.iter().cloned().collect(),
            files,
            messages,
            output: output.chunks,
            outputs: Vec::new(),
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
fn session_progress_handler(
    sender: mpsc::Sender<ProgressEvent>,
    interval: Duration,
) -> SessionEventHandler {
    let throttle = Mutex::new((None::<Instant>, 0usize));

    Arc::new(move |event| match event {
        SessionEvent::TaskFinished { artifact_key, .. }
        | SessionEvent::TaskFailed { artifact_key, .. } => {
            let mut throttle = throttle.lock();
            throttle.1 += 1;
            let due = throttle.0.is_none_or(|last| last.elapsed() >= interval);
            if due {
                throttle.0 = Some(Instant::now());
                let _ = sender.send(ProgressEvent {
                    task: artifact_key.stage().name().to_string(),
                    message: Some(format!("{} artifacts", throttle.1)),
                    percent: None,
                    done: false,
                });
            }
        }
        SessionEvent::RunFinished { .. } => {
            let _ = sender.send(ProgressEvent {
                task: String::new(),
                message: None,
                percent: None,
                done: true,
            });
        }
        _ => {}
    })
}

/// Return file images for diagnostics emitted by one command.
fn command_file_images(
    context: &CommandContext<'_>,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> CommandResult<Vec<crate::FileImage>> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

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

            let Some(file) = context
                .repository
                .file(revision, file_id)
                .map_err(|error| CommandError::internal(error.to_string()))?
            else {
                continue;
            };
            files.push(crate::FileImage::from(file.as_ref()));
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

    fn reload(&self, request: ReloadRequest) -> Result<UpdateBatch, Error> {
        if request.roots.is_empty() {
            LocalWorkspace::reload_all(self)
        } else {
            LocalWorkspace::reload_roots(self, &request.roots)
        }
    }

    fn file(&self, operation: FileOperation) -> Result<UpdateBatch, Error> {
        LocalWorkspace::apply_file_operation(self, operation)
    }

    fn is_file_open(&self, path: &Path) -> Result<bool, Error> {
        Ok(LocalWorkspace::has_open_file(self, path))
    }

    fn edit(&self, root: &Path, update: SourceUpdate) -> Result<Commit, Error> {
        if let Some(base) = update.base {
            LocalWorkspace::apply_source_edits_if_current(self, root, base, update.edits)
        } else {
            LocalWorkspace::apply_source_edits(self, root, update.edits)
        }
    }

    fn check(
        &self,
        root: &Path,
        request: CheckInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CheckOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_check_command(&request)
        })
    }

    fn format(
        &self,
        root: &Path,
        request: FormatInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<FormatOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_format_command(root, &request.source, request.mode)
        })
    }

    fn build(
        &self,
        root: &Path,
        request: BuildInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BuildOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_build_command(&request)
        })
    }

    fn run(
        &self,
        root: &Path,
        request: RunInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<RunOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.execute_run_command(&request)
        })
    }

    fn test(
        &self,
        root: &Path,
        request: TestInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TestOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_test_command(&TestOptions::default())
        })
    }

    fn doc(
        &self,
        root: &Path,
        request: DocInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DocOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_doc_command(&DocOptions::default())
        })
    }

    fn bench(
        &self,
        root: &Path,
        request: BenchInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BenchOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_bench_command(&BenchOptions::default())
        })
    }

    fn info(
        &self,
        root: &Path,
        request: InfoInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<InfoOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_info_command(&InfoOptions { all: request.all })
        })
    }

    fn targets(
        &self,
        root: &Path,
        request: TargetsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TargetsOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_targets_command(&TargetsOptions { all: request.all })
        })
    }

    fn cache(
        &self,
        root: &Path,
        request: CacheInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CacheOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_cache_command(&CacheOptions)
        })
    }

    fn settings(
        &self,
        root: &Path,
        request: SettingsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<SettingsOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_settings_command(&SettingsOptions)
        })
    }

    fn doctor(
        &self,
        root: &Path,
        request: DoctorInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DoctorOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_doctor_command(&DoctorOptions { full: request.full })
        })
    }

    fn task(
        &self,
        root: &Path,
        request: TaskInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TaskOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_task_command(&TaskOptions {
                action: request.action.clone(),
                projects: request.projects.clone(),
                groups: request.groups.clone(),
            })
        })
    }

    fn clean(
        &self,
        root: &Path,
        request: CleanInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CleanOutput, CommandError> {
        let common = request.command_options();

        self.run_command(root, &common, request.revision, progress, |context| {
            context.run_clean_command(
                root,
                &CleanOptions {
                    dir: request.dir.clone(),
                    dist: request.dist,
                    cache: request.cache,
                    all: request.all,
                    all_packages: request.all_packages,
                },
            )
        })
    }

    fn query(&self, root: &Path, request: QueryRequest) -> Result<QueryResult, Error> {
        let revision = match request.expected_revision {
            Some(revision) => super::RevisionPolicy::Current(revision),
            None => super::RevisionPolicy::Latest,
        };

        LocalWorkspace::query_root(self, root, request.request, revision)
    }

    fn view(&self, root: &Path, request: ViewRequest) -> Result<ViewResult, Error> {
        match request {
            ViewRequest::Root { target } => {
                LocalWorkspace::root_snapshot(self, root, target.as_deref()).map(ViewResult::Root)
            }
            ViewRequest::File(request) => {
                LocalWorkspace::file_snapshot(self, root, request).map(ViewResult::File)
            }
            ViewRequest::FileImages(request) => {
                LocalWorkspace::file_images(self, root, request).map(ViewResult::FileImages)
            }
        }
    }

    fn diagnostics(&self, request: DiagnosticsRequest) -> Result<Vec<DiagnosticView>, Error> {
        match request {
            DiagnosticsRequest::All => LocalWorkspace::diagnostics(self),
            DiagnosticsRequest::Root(root) => LocalWorkspace::root_diagnostics(self, &root),
            DiagnosticsRequest::File(path) => {
                let diagnostics = LocalWorkspace::file_diagnostics(self, &path)?;

                Ok(diagnostics.into_iter().collect())
            }
        }
    }

    fn artifact(&self, root: &Path, artifact: ArtifactReference) -> Result<ArtifactPayload, Error> {
        let revision = LocalWorkspace::revision(self, root)?;
        let payload = self.artifact_payload(revision, artifact)?;

        Ok(payload)
    }

    fn store(&self, content: Content) -> Result<ContentId, Error> {
        self.repository.intern_content(content).map_err(Error::from)
    }

    fn load(&self, content: ContentId) -> Result<Content, Error> {
        let entry = self.repository.content(content)?;

        Ok(entry.payload().clone())
    }

    fn export(&self, root: &Path, request: ExportRequest) -> Result<ExportResult, Error> {
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

    fn watch(&self, roots: Vec<PathBuf>, policy: WatchPolicy) -> Result<(), Error> {
        let Some(root) = roots.first().cloned() else {
            return Err(Error::InvalidEdit {
                detail: "watch requires at least one root".to_string(),
            });
        };
        let Some(file_watcher) = self.file_watcher.as_ref() else {
            return Err(Error::InvalidEdit {
                detail: "workspace does not have a file watcher".to_string(),
            });
        };

        let watch = Arc::new(Watch::new(
            file_watcher.clone(),
            roots,
            source_watch_options(),
            policy.clone(),
        ));
        if let Some((_, previous)) = self.watches.remove(&root) {
            previous.stop();
        }
        self.watches.insert(root, watch);

        Ok(())
    }

    fn next_watch(&self, root: &Path) -> Result<Option<WatchUpdate>, Error> {
        let Some(watch) = self.watches.get(root).map(|watch| watch.clone()) else {
            return Err(Error::InvalidEdit {
                detail: format!("watch is not active for {}", root.display()),
            });
        };

        let Some(batch) = watch.next_batch() else {
            return Ok(None);
        };
        let updates = self.apply_watch_batch(&batch)?;

        Ok(Some(WatchUpdate { batch, updates }))
    }

    fn unwatch(&self, root: &Path) -> Result<(), Error> {
        if let Some((_, watch)) = self.watches.remove(root) {
            watch.stop();
        }

        Ok(())
    }
}

impl LocalWorkspace {
    /// Return one artifact payload by exact version.
    fn artifact_payload(
        &self,
        revision: Revision,
        artifact: ArtifactReference,
    ) -> Result<ArtifactPayload, Error> {
        if let Some(payload) = self.artifact_payload_in_memory(artifact)? {
            return Ok(payload);
        }

        if self.repository.load_artifact(revision, artifact.version)? {
            return self
                .artifact_payload_in_memory(artifact)?
                .ok_or_else(|| Error::Internal {
                    detail: format!("artifact payload did not load for {:?}", artifact.version),
                });
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
        let record = self
            .repository
            .artifact_table()
            .record(&artifact.version, self.repository.string_pool())
            .map_err(|error| Error::Internal {
                detail: error.to_string(),
            })?;
        let Some(record) = record else {
            return Ok(None);
        };
        let payload = record.decode_payload().map_err(|error| Error::Internal {
            detail: error.to_string(),
        })?;

        Ok(Some(payload))
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

        self.artifact_payload(revision, reference)
    }

    /// Export one bundle to the host filesystem.
    fn export_bundle(
        &self,
        root: &Path,
        bundle: &Bundle,
        request: &ExportRequest,
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
        request: &ExportRequest,
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
        request: &ExportRequest,
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
        request: &ExportRequest,
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
    fn apply_file_operation(&self, operation: FileOperation) -> Result<UpdateBatch, Error> {
        match operation {
            FileOperation::OpenText {
                path,
                uri,
                version,
                content,
            } => self.open_file(
                uri,
                version,
                Edit::SetText {
                    path,
                    text: content,
                },
            ),
            FileOperation::OpenBytes {
                path,
                uri,
                version,
                content,
            } => self.open_file(
                uri,
                version,
                Edit::SetBytes {
                    path,
                    bytes: content,
                },
            ),
            FileOperation::ChangeText {
                path,
                uri,
                version,
                content,
            } => self.change_file(
                uri,
                version,
                Edit::SetText {
                    path,
                    text: content,
                },
            ),
            FileOperation::ChangeBytes {
                path,
                uri,
                version,
                content,
            } => self.change_file(
                uri,
                version,
                Edit::SetBytes {
                    path,
                    bytes: content,
                },
            ),
            FileOperation::PatchText {
                path,
                uri,
                version,
                changes,
            } => self.patch_text_file(&path, uri, version, changes),
            FileOperation::SaveText { path, content } => self.save_text_file(&path, content),
            FileOperation::SaveBytes { path, content } => self.save_bytes_file(&path, content),
            FileOperation::Close { path } => self.close_file(&path),
            FileOperation::WriteText { path, content } => self.write_file(Edit::SetText {
                path,
                text: content,
            }),
            FileOperation::WriteBytes { path, content } => self.write_file(Edit::SetBytes {
                path,
                bytes: content,
            }),
            FileOperation::Remove { path } => self.write_file(Edit::Remove { path }),
            FileOperation::Move { from, to } => self.write_file(Edit::Move { from, to }),
        }
    }
}
