use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use futures::future::BoxFuture;
use parking_lot::Mutex;
use tspp_artifact::{ArtifactKey, ArtifactPayload, ArtifactReference, Bundle, BundleFile, Product};
use tspp_core::Blob;
use tspp_repository::{Repository, Revision, RootKind, Trace, TraceLevel};
use tspp_session::{Executor, Session};
use tspp_source::{File, FileId};

use super::{BackgroundRun, Lifecycle, State, WorkspacePin};
use crate::{
    BuildInput, BuildOutput, CheckInput, CheckOutput, CleanInput, CleanOptions, CleanOutput,
    CommandContext, CommandError, CommandOptions, CommandOutcome, CommandProgress, CommandResult,
    CommandRevision, DocInput, DocOptions, DocOutput, DoctorInput, DoctorOptions, DoctorOutput,
    Error, ExportInput, ExportResult, ExportedFile, FormatInput, FormatOutput, InfoInput,
    InfoOptions, InfoOutput, Output, OutputBuffer, QueryInput, QueryOutput, RewriteInput,
    RewriteOutput, SettingsInput, SettingsOptions, SettingsOutput, TargetsInput, TargetsOptions,
    TargetsOutput, TaskInput, TaskOptions, TaskOutput, TestInput, TestOptions, TestOutput,
    WatchState,
};

/// One live TS++ workspace rooted at one repository path.
pub struct Workspace {
    /// Canonical workspace root.
    pub(crate) root: PathBuf,
    /// Repository for workspace resolution.
    pub(crate) repository: Arc<Repository>,
    /// Repository-specific artifact computation session.
    pub(crate) session: Arc<Session>,
    /// Serialized workspace lifecycle and mutation state.
    pub(crate) state: Arc<Mutex<State>>,
    /// Workspace watch state.
    pub(crate) watch: Arc<Mutex<WatchState>>,
    /// Latest proactive editor artifact run.
    pub(crate) background_run: Mutex<Option<BackgroundRun>>,
}

impl std::fmt::Debug for Workspace {
    /// Format the visible workspace state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Workspace")
            .field("root", &self.root)
            .field("repository", &self.repository)
            .field("session", &self.session)
            .field("state", &self.state)
            .field("watch", &self.watch)
            .field(
                "background_revision",
                &self
                    .background_run
                    .lock()
                    .as_ref()
                    .map(BackgroundRun::revision),
            )
            .finish()
    }
}

impl Workspace {
    /// Create one workspace over an exact physical repository revision.
    pub fn new(
        repository: Arc<Repository>,
        physical: Revision,
        executor: Arc<Executor>,
    ) -> Result<Self, Error> {
        let root = repository.path().to_path_buf();
        let physical = repository.pin(physical)?;
        let session = Arc::new(Session::new(repository.clone(), executor)?);
        Ok(Self {
            root,
            repository,
            session,
            state: Arc::new(Mutex::new(State {
                lifecycle: Lifecycle::Open,
                physical,
                branches: HashMap::new(),
            })),
            watch: Arc::new(Mutex::new(WatchState::default())),
            background_run: Mutex::new(None),
        })
    }

    /// Return this workspace's canonical root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the kind of this workspace root.
    pub fn kind(&self, revision: Revision) -> Result<RootKind, Error> {
        Ok(self.repository.root(revision)?.kind)
    }

    /// Persist successful artifacts from the current physical revision.
    pub fn persist_artifacts(&self, revision: Revision) -> Result<bool, Error> {
        let session = self.pin_physical()?;
        let current = session.revision();
        if revision != current {
            return Ok(false);
        }

        Ok(session.persist_artifacts())
    }

    /// Wait for every queued artifact cache write on this host.
    pub fn flush_artifact_cache(&self) -> Result<(), Error> {
        self.repository.flush_artifact_cache().map_err(Into::into)
    }

    /// Start one trace configured for this workspace executor.
    pub fn start_trace(&self, level: TraceLevel) -> Arc<Trace> {
        self.session.start_trace(level)
    }

    /// Execute one command operation.
    pub(crate) async fn run_command<'a, T, O>(
        &self,
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
        let repository = self.repository.clone();

        let event_handler = progress.map(CommandProgress::event_handler);

        // gather shared context
        let mut output = OutputBuffer::default();
        let mut context = CommandContext::new(
            self,
            repository,
            common,
            revision,
            &mut output,
            event_handler,
        )?;

        // execute the requested operation
        let result = execute(&mut context).await?;

        // queue completed physical artifacts outside compiler execution
        let revision = context.revision();
        let trace = context.trace();
        let is_queued = trace
            .span("cache.enqueue", || self.persist_artifacts(revision))
            .map_err(|error| CommandError::internal(error.to_string()))?;
        trace.add_counter("cache.write_queued", u64::from(is_queued));

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
        let files = context.file_images(revision, &diagnostics, &files)?;
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

impl Workspace {
    /// Return one canonical path according to the workspace host.
    pub fn canonicalize(&self, path: &Path) -> Result<PathBuf, Error> {
        self.repository
            .file_system()
            .canonicalize(path)
            .map_err(|error| Error::Io {
                path: path.to_path_buf(),
                source: error,
            })
    }

    /// Check source state.
    pub fn check<'a>(
        &'a self,
        request: CheckInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CheckOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_check_command(&request).await })
            })
            .await
        })
    }

    /// Format source files or content.
    pub fn format<'a>(
        &'a self,
        request: FormatInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<FormatOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            let command_root = self.root.clone();
            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_format_command(&command_root, &request.source, request.mode)
                })
            })
            .await
        })
    }

    /// Query source files with one structural pattern.
    pub fn query<'a>(
        &'a self,
        request: QueryInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<QueryOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_query_command(&request).await })
            })
            .await
        })
    }

    /// Rewrite source files with one structural pattern.
    pub fn rewrite<'a>(
        &'a self,
        request: RewriteInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<RewriteOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_rewrite_command(&request).await })
            })
            .await
        })
    }

    /// Build target artifacts.
    pub fn build<'a>(
        &'a self,
        request: BuildInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<BuildOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_build_command(&request).await })
            })
            .await
        })
    }

    /// Run workspace tests.
    pub fn test<'a>(
        &'a self,
        request: TestInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TestOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_test_command(&TestOptions::default()) })
            })
            .await
        })
    }

    /// Generate workspace documentation.
    pub fn doc<'a>(
        &'a self,
        request: DocInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DocOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_doc_command(&DocOptions::default()).await })
            })
            .await
        })
    }

    /// Return workspace information.
    pub fn info<'a>(
        &'a self,
        request: InfoInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<InfoOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_info_command(&InfoOptions { all: request.all }) })
            })
            .await
        })
    }

    /// Return configured targets.
    pub fn targets<'a>(
        &'a self,
        request: TargetsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TargetsOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_targets_command(&TargetsOptions { all: request.all })
                })
            })
            .await
        })
    }

    /// Return resolved settings.
    pub fn settings<'a>(
        &'a self,
        request: SettingsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<SettingsOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move { context.run_settings_command(&SettingsOptions) })
            })
            .await
        })
    }

    /// Diagnose workspace configuration and state.
    pub fn doctor<'a>(
        &'a self,
        request: DoctorInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DoctorOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_doctor_command(&DoctorOptions { full: request.full })
                })
            })
            .await
        })
    }

    /// Execute configured workspace tasks.
    pub fn task<'a>(
        &'a self,
        request: TaskInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TaskOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
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

    /// Clean generated workspace state.
    pub fn clean<'a>(
        &'a self,
        request: CleanInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CleanOutput, CommandError>> {
        Box::pin(async move {
            let common = request.command_options();

            self.run_command(&common, request.revision, progress, move |context| {
                Box::pin(async move {
                    context.run_clean_command(&CleanOptions {
                        dir: request.dir.clone(),
                        all_packages: request.all_packages,
                    })
                })
            })
            .await
        })
    }

    /// Read source files from one exact revision.
    pub fn read_files(
        &self,
        revision: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<Vec<Arc<File>>, Error> {
        // pin the requested immutable revision
        let revision = self.repository.pin(revision)?;
        let session = WorkspacePin::new(
            self.root.clone(),
            self.session(),
            revision,
            Arc::downgrade(&self.state),
        );

        // read every requested file exactly
        let mut files = Vec::with_capacity(file_ids.len());
        for file_id in file_ids {
            files.push(session.file(file_id)?);
        }

        Ok(files)
    }

    /// Return one artifact payload.
    pub fn artifact(&self, artifact: ArtifactReference) -> Result<ArtifactPayload, Error> {
        self.revision()?;
        let payload = self.artifact_payload(artifact)?;

        Ok(payload)
    }

    /// Publish one artifact's storage bytes as a Blob.
    pub fn blob(&self, artifact: ArtifactReference) -> Result<Blob, Error> {
        let payload = self.artifact(artifact)?;
        let bytes = payload.as_ref().encode().map_err(|error| Error::Internal {
            detail: error.to_string(),
        })?;
        let blob = self.repository.retain_blob(bytes.as_ref())?;

        Ok(blob)
    }

    /// Materialize one artifact on the workspace host.
    pub fn export(&self, request: ExportInput) -> Result<ExportResult, Error> {
        let revision = self.revision()?;
        let payload = self.artifact(request.artifact)?;

        match payload {
            ArtifactPayload::Bundle(bundle) => {
                self.export_bundle(&self.root, bundle.as_ref(), &request)
            }
            ArtifactPayload::Product(product) => {
                self.export_product(&self.root, revision, product.as_ref(), &request)
            }
            payload => Err(Error::InvalidEdit {
                detail: format!(
                    "workspace export cannot materialize {} artifacts",
                    payload.name()
                ),
            }),
        }
    }
}

impl Workspace {
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

        let payload = self
            .repository
            .artifact_table()
            .payload(&artifact.version)
            .map_err(|error| Error::Internal {
                detail: error.to_string(),
            })?;
        if let Some(payload) = payload {
            return Ok(payload);
        }

        Err(Error::Internal {
            detail: format!("artifact payload is missing for {:?}", artifact.version),
        })
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
        let memory = self
            .repository
            .blob_store()
            .open(file.blob)
            .map_err(|error| Error::Internal {
                detail: error.to_string(),
            })?;

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

        // materialize the exact Blob bytes
        self.repository
            .file_system()
            .write(&path, memory.bytes())
            .map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;

        Ok(ExportedFile {
            path,
            blob: file.blob,
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
}
