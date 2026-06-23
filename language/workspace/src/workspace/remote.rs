use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactPayload, ArtifactReference};
use destack_repository::Revision;
use destack_source::{Content, ContentId};
use parking_lot::Mutex;

use super::{
    DiagnosticsRequest, ExportRequest, ExportResult, QueryRequest, ReloadRequest, ViewRequest,
    ViewResult,
};
use crate::connection::Client;
use crate::diagnostic::{DiagnosticView, Error};
use crate::file::{Commit, FileOperation, SourceUpdate};
use crate::protocol::{self, RequestOptions, RootId, RootOpenOptions};
use crate::{
    BenchInput, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOutput, CheckInput,
    CheckOutput, CleanInput, CleanOutput, ClientError, CommandError, CommandProgress, DocInput,
    DocOutput, DoctorInput, DoctorOutput, FormatInput, FormatOutput, InfoInput, InfoOutput,
    InspectInput, InspectOutput, LintInput, LintOutput, ManifestInput, ManifestOutput,
    ProgressEvent, QueryResult, RevisionPolicy, RunInput, RunOutput, SettingsInput, SettingsOutput,
    TargetsInput, TargetsOutput, TaskInput, TaskOutput, TestInput, TestOutput, UpdateBatch,
    WatchPolicy, WatchUpdate, Workspace,
};

/// Workspace backed by a protocol client.
#[derive(Debug)]
pub struct RemoteWorkspace {
    /// Workspace root used to open protocol roots.
    root: PathBuf,
    /// Protocol client used for remote operations.
    client: Arc<Client>,
    /// Open protocol roots.
    roots: Mutex<Vec<RemoteRoot>>,
    /// Open file paths mirrored from successful protocol operations.
    open_files: Mutex<HashSet<PathBuf>>,
}

/// Open remote root handle.
#[derive(Debug, Clone)]
struct RemoteRoot {
    /// Root path requested by the client.
    requested: PathBuf,
    /// Canonical root path opened by the server.
    root: PathBuf,
    /// Protocol handle for the root.
    handle: RootId,
}

impl RemoteWorkspace {
    /// Create a workspace from one connected protocol client.
    pub fn new(client: Arc<Client>, root: PathBuf) -> Self {
        Self {
            root,
            client,
            roots: Mutex::new(Vec::new()),
            open_files: Mutex::new(HashSet::new()),
        }
    }

    /// Return the handle for one open root.
    fn handle_for_root(&self, root: &Path) -> Result<RootId, Error> {
        let roots = self.roots.lock();
        roots
            .iter()
            .find(|entry| entry.matches_root(root))
            .map(|entry| entry.handle)
            .ok_or_else(|| Error::PathNotInRoot {
                path: root.to_path_buf(),
            })
    }

    /// Return the handle for the open root owning one path.
    fn handle_for_path(&self, path: &Path) -> Result<RootId, Error> {
        let roots = self.roots.lock();
        let root = roots
            .iter()
            .filter(|entry| entry.contains_path(path))
            .max_by_key(|entry| entry.match_len(path));
        let Some(root) = root else {
            return Err(Error::PathNotInRoot {
                path: path.to_path_buf(),
            });
        };

        Ok(root.handle)
    }

    /// Return the first open root handle for workspace-level protocol calls.
    fn primary_handle(&self) -> Result<RootId, Error> {
        let roots = self.roots.lock();
        let Some(root) = roots.first() else {
            return Err(Error::Internal {
                detail: "remote workspace has no open roots".to_string(),
            });
        };

        Ok(root.handle)
    }

    /// Return open root paths.
    fn open_roots(&self) -> Vec<PathBuf> {
        self.roots
            .lock()
            .iter()
            .map(|entry| entry.root.clone())
            .collect()
    }

    /// Apply a protocol file operation and convert its updates.
    fn apply_file_operation(
        &self,
        handle: RootId,
        operation: FileOperation,
    ) -> Result<UpdateBatch, Error> {
        let response = self
            .client
            .apply_file_operation(handle, operation)
            .map_err(Self::workspace_error)?;

        Ok(response.updates)
    }

    /// Remove mirrored open file state under one root.
    fn remove_open_files_under(&self, root: &RemoteRoot) {
        let mut open_files = self.open_files.lock();

        open_files.retain(|path| !root.contains_path(path));
    }
}

impl RemoteRoot {
    /// Return whether this root matches one root path.
    fn matches_root(&self, root: &Path) -> bool {
        root == self.root || root == self.requested
    }

    /// Return whether this root contains one source path.
    fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root) || path.starts_with(&self.requested)
    }

    /// Return the matched root length for longest-root selection.
    fn match_len(&self, path: &Path) -> usize {
        let mut len = 0;
        if path.starts_with(&self.root) {
            len = len.max(self.root.as_os_str().len());
        }
        if path.starts_with(&self.requested) {
            len = len.max(self.requested.as_os_str().len());
        }

        len
    }

    /// Return this path under the canonical server root.
    fn canonical_path(&self, path: &Path) -> Option<PathBuf> {
        if path.starts_with(&self.root) {
            return Some(path.to_path_buf());
        }

        let suffix = path.strip_prefix(&self.requested).ok()?;
        Some(self.root.join(suffix))
    }
}

impl Workspace for RemoteWorkspace {
    fn home(&self) -> &Path {
        &self.root
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, Error> {
        let roots = self.roots.lock();
        let path = roots
            .iter()
            .find_map(|entry| entry.canonical_path(path))
            .unwrap_or_else(|| path.to_path_buf());

        Ok(path)
    }

    fn roots(&self) -> Vec<PathBuf> {
        RemoteWorkspace::open_roots(self)
    }

    fn open(&self, root: PathBuf) -> Result<(), Error> {
        let requested = root.clone();
        let response = self
            .client
            .open_root(self.root.clone(), root, RootOpenOptions::default())
            .map_err(Self::workspace_error)?;
        let mut roots = self.roots.lock();
        if let Some(entry) = roots
            .iter_mut()
            .find(|entry| entry.matches_root(&requested) || entry.matches_root(&response.root))
        {
            entry.requested = requested;
            entry.root = response.root;
            entry.handle = response.handle;
        } else {
            roots.push(RemoteRoot {
                requested,
                root: response.root,
                handle: response.handle,
            });
        }

        Ok(())
    }

    fn close(&self, root: &Path) -> Result<(), Error> {
        let mut roots = self.roots.lock();
        let Some(index) = roots.iter().position(|entry| entry.matches_root(root)) else {
            return Ok(());
        };
        let entry = roots.remove(index);
        drop(roots);

        self.client
            .close_root(entry.handle)
            .map_err(Self::workspace_error)?;
        self.remove_open_files_under(&entry);

        Ok(())
    }

    fn root(&self, path: &Path) -> Result<PathBuf, Error> {
        RemoteWorkspace::root_for_path(self, path)
    }

    fn revision(&self, root: &Path) -> Result<Revision, Error> {
        let handle = self.handle_for_root(root)?;

        self.client
            .current_revision(handle)
            .map_err(Self::workspace_error)
    }

    fn reload(&self, request: ReloadRequest) -> Result<UpdateBatch, Error> {
        let roots = if request.roots.is_empty() {
            self.roots()
        } else {
            request.roots
        };
        let mut batch = UpdateBatch::default();

        for root in roots {
            let handle = self.handle_for_root(&root)?;
            let response = self
                .client
                .reload_root(handle, request.reason)
                .map_err(Self::workspace_error)?;

            batch.updates.extend(response.updates.updates);
            batch.messages.extend(response.updates.messages);
        }

        Ok(batch)
    }

    fn file(&self, operation: FileOperation) -> Result<UpdateBatch, Error> {
        let path = operation.path().to_path_buf();
        let handle = self.handle_for_path(&path)?;
        let is_open = matches!(
            operation,
            FileOperation::OpenText { .. } | FileOperation::OpenBytes { .. }
        );
        let is_close = matches!(operation, FileOperation::Close { .. });
        let result = self.apply_file_operation(handle, operation)?;

        if is_open {
            self.open_files.lock().insert(path);
        } else if is_close {
            self.open_files.lock().remove(&path);
        }

        Ok(result)
    }

    fn is_file_open(&self, path: &Path) -> bool {
        self.open_files.lock().contains(path)
    }

    fn edit(&self, root: &Path, update: SourceUpdate) -> Result<Commit, Error> {
        let handle = self.handle_for_root(root)?;
        let response = self
            .client
            .apply_source_update(handle, update)
            .map_err(Self::workspace_error)?;

        Ok(response.commit)
    }

    fn check(
        &self,
        root: &Path,
        request: CheckInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CheckOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .check(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn lint(
        &self,
        root: &Path,
        request: LintInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<LintOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .lint(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn format(
        &self,
        root: &Path,
        request: FormatInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<FormatOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .format(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn build(
        &self,
        root: &Path,
        request: BuildInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BuildOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .build(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn run(
        &self,
        root: &Path,
        request: RunInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<RunOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .run(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn test(
        &self,
        root: &Path,
        request: TestInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TestOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .test(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn doc(
        &self,
        root: &Path,
        request: DocInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DocOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .doc(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn bench(
        &self,
        root: &Path,
        request: BenchInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BenchOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .bench(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn info(
        &self,
        root: &Path,
        request: InfoInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<InfoOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .info(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn inspect(
        &self,
        root: &Path,
        request: InspectInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<InspectOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .inspect(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn manifest(
        &self,
        root: &Path,
        request: ManifestInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<ManifestOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .manifest(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn targets(
        &self,
        root: &Path,
        request: TargetsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TargetsOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .targets(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn cache(
        &self,
        root: &Path,
        request: CacheInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CacheOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .cache(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn settings(
        &self,
        root: &Path,
        request: SettingsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<SettingsOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .settings(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn doctor(
        &self,
        root: &Path,
        request: DoctorInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DoctorOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .doctor(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn task(
        &self,
        root: &Path,
        request: TaskInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TaskOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .task(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn clean(
        &self,
        root: &Path,
        request: CleanInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CleanOutput, CommandError> {
        let handle = self
            .handle_for_root(root)
            .map_err(Self::remote_command_error)?;
        let mut notify = Self::client_progress(progress);

        self.client
            .clean(handle, request, RequestOptions::default(), &mut notify)
            .map_err(Self::command_error)
    }

    fn query(&self, root: &Path, request: QueryRequest) -> Result<QueryResult, Error> {
        let revision = match request.expected_revision {
            Some(revision) => RevisionPolicy::Current(revision),
            None => RevisionPolicy::Latest,
        };
        let handle = self.handle_for_root(root)?;

        self.query_handle(handle, request.request, revision)
    }

    fn view(&self, root: &Path, request: ViewRequest) -> Result<ViewResult, Error> {
        let handle = self.handle_for_root(root)?;

        match request {
            ViewRequest::Root { target } => self
                .client
                .root_snapshot(handle, target)
                .map(ViewResult::Root)
                .map_err(Self::workspace_error),
            ViewRequest::File(request) => self
                .client
                .file_snapshot(handle, request)
                .map(ViewResult::File)
                .map_err(Self::workspace_error),
            ViewRequest::FileImages(request) => self
                .client
                .file_images(handle, request)
                .map(ViewResult::FileImages)
                .map_err(Self::workspace_error),
        }
    }

    fn diagnostics(&self, request: DiagnosticsRequest) -> Result<Vec<DiagnosticView>, Error> {
        match request {
            DiagnosticsRequest::All => self.all_diagnostics(),
            DiagnosticsRequest::Root(root) => self.root_diagnostics(&root),
            DiagnosticsRequest::File(path) => self
                .file_diagnostics(&path)
                .map(|diagnostics| diagnostics.into_iter().collect()),
        }
    }

    fn artifact(&self, root: &Path, artifact: ArtifactReference) -> Result<ArtifactPayload, Error> {
        let handle = self.handle_for_root(root)?;

        self.client
            .artifact(handle, artifact)
            .map_err(Self::workspace_error)
    }

    fn store(&self, content: Content) -> Result<ContentId, Error> {
        let handle = self.primary_handle()?;

        self.client
            .store(handle, content)
            .map_err(Self::workspace_error)
    }

    fn load(&self, content: ContentId) -> Result<Content, Error> {
        let handle = self.primary_handle()?;

        self.client
            .load(handle, content)
            .map_err(Self::workspace_error)
    }

    fn export(&self, root: &Path, request: ExportRequest) -> Result<ExportResult, Error> {
        let handle = self.handle_for_root(root)?;

        self.client
            .export(handle, request)
            .map_err(Self::workspace_error)
    }

    fn watch(&self, roots: Vec<PathBuf>, policy: WatchPolicy) -> Result<(), Error> {
        let Some(root) = roots.first() else {
            return Err(Error::InvalidEdit {
                detail: "watch requires at least one root".to_string(),
            });
        };

        let handle = self.handle_for_root(root)?;
        self.client
            .start_watch(handle, roots, policy.start_options())
            .map_err(Self::workspace_error)?;

        Ok(())
    }

    fn next_watch(&self, root: &Path) -> Result<Option<WatchUpdate>, Error> {
        let handle = self.handle_for_root(root)?;
        let response = self
            .client
            .next_watch_batch(handle)
            .map_err(Self::workspace_error)?;
        let Some(batch) = response.batch else {
            return Ok(None);
        };

        Ok(Some(WatchUpdate {
            batch,
            updates: response.updates,
        }))
    }

    fn unwatch(&self, root: &Path) -> Result<(), Error> {
        let handle = self.handle_for_root(root)?;
        self.client
            .stop_watch(handle)
            .map_err(Self::workspace_error)?;

        Ok(())
    }
}

impl RemoteWorkspace {
    /// Run one query through a resolved root handle.
    fn query_handle(
        &self,
        handle: RootId,
        request: destack_query::QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let expected_revision = match revision {
            RevisionPolicy::Latest => Some(
                self.client
                    .current_revision(handle)
                    .map_err(Self::workspace_error)?,
            ),
            RevisionPolicy::Exact(revision) | RevisionPolicy::Current(revision) => Some(revision),
        };
        let response = self
            .client
            .execute_query(
                handle,
                protocol::QueryRequestBody {
                    expected_revision,
                    request,
                },
            )
            .map_err(Self::workspace_error)?;

        Ok(QueryResult {
            revision: response.revision,
            response: response.response,
        })
    }

    /// Return one open root for a source path.
    fn root_for_path(&self, path: &Path) -> Result<PathBuf, Error> {
        let roots = self.roots.lock();
        let root = roots
            .iter()
            .filter(|entry| entry.contains_path(path))
            .max_by_key(|entry| entry.match_len(path));
        let Some(root) = root else {
            return Err(Error::PathNotInRoot {
                path: path.to_path_buf(),
            });
        };

        Ok(root.root.clone())
    }

    /// Return diagnostics for all open roots.
    fn all_diagnostics(&self) -> Result<Vec<DiagnosticView>, Error> {
        let roots = self.open_roots();
        let diagnostics = roots
            .iter()
            .map(|root| self.root_diagnostics(root))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(diagnostics)
    }

    /// Return diagnostics for one open root.
    fn root_diagnostics(&self, root: &Path) -> Result<Vec<DiagnosticView>, Error> {
        let handle = self.handle_for_root(root)?;
        let diagnostics = self
            .client
            .diagnostic_snapshots(handle)
            .map_err(Self::workspace_error)?
            .into_iter()
            .map(DiagnosticView::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(diagnostics)
    }

    /// Return diagnostics for one source file.
    fn file_diagnostics(&self, path: &Path) -> Result<Option<DiagnosticView>, Error> {
        let handle = self.handle_for_path(path)?;
        let diagnostics = self
            .client
            .file_diagnostics(handle, path.to_path_buf())
            .map_err(Self::workspace_error)?
            .map(DiagnosticView::try_from)
            .transpose()?;

        Ok(diagnostics)
    }

    /// Convert a client error into a workspace error.
    fn workspace_error(error: ClientError) -> Error {
        Error::Internal {
            detail: error.to_string(),
        }
    }

    /// Convert a workspace error into a command error.
    fn remote_command_error(error: Error) -> CommandError {
        CommandError::invalid_input(error.to_string())
    }

    /// Convert a client error into a command error.
    fn command_error(error: ClientError) -> CommandError {
        CommandError::internal(error.to_string())
    }

    /// Forward remote progress events to the caller.
    fn client_progress(progress: Option<CommandProgress<'_>>) -> impl FnMut(ProgressEvent) + '_ {
        move |event| {
            if let Some(progress) = progress {
                progress.emit(event);
            }
        }
    }
}
