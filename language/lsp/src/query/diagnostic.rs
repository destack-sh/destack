use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use parking_lot::{Mutex, RwLock};
use tspp_core::StableHasher;
use tspp_lsp_server::{Client, LogRecord, UriExt, jsonrpc};
use tspp_lsp_types as lsp;
use tspp_repository::Revision;
use tspp_session::ArtifactPriority;
use tspp_source::{
    Diagnostic, DiagnosticLabel, DiagnosticReference, DiagnosticSeverity, DiagnosticTag,
    DiagnosticTarget,
};
use tspp_workspace::{FileDiagnostics, Workspace};

use super::{Document, DocumentSet};
use crate::server::{ProjectId, ProjectSet, internal_error, workspace_error};

/// Diagnostic delivery selected from client capabilities.
#[derive(Debug)]
pub(crate) struct DiagnosticDelivery {
    /// Active diagnostic delivery mode.
    mode: DiagnosticDeliveryMode,
    /// Completed workspace diagnostics at each root's latest requested revision.
    pub(crate) workspaces: Mutex<HashMap<PathBuf, Arc<WorkspaceDiagnostics>>>,
}

/// Completed workspace diagnostics at one source revision.
#[derive(Debug)]
pub(crate) struct WorkspaceDiagnostics {
    /// Source revision used to produce the reports.
    pub(crate) revision: Revision,
    /// Complete document reports.
    pub(crate) reports: Vec<lsp::WorkspaceFullDocumentDiagnosticReport>,
}

/// Active diagnostic delivery mode.
#[derive(Debug)]
enum DiagnosticDeliveryMode {
    /// Client initiated diagnostic requests.
    Pull(PullDiagnostics),
    /// Server initiated diagnostic publications.
    Push(PushDiagnostics),
}

impl DiagnosticDelivery {
    /// Create pull diagnostics.
    pub(crate) fn pull(is_refresh_supported: bool) -> Self {
        Self {
            mode: DiagnosticDeliveryMode::Pull(PullDiagnostics::new(is_refresh_supported)),
            workspaces: Mutex::new(HashMap::new()),
        }
    }

    /// Create push diagnostics.
    pub(crate) fn push() -> Self {
        Self {
            mode: DiagnosticDeliveryMode::Push(PushDiagnostics::default()),
            workspaces: Mutex::new(HashMap::new()),
        }
    }

    /// Schedule diagnostics after source state changes.
    pub(crate) fn schedule(
        &self,
        workspace: Arc<Workspace>,
        projects: Arc<RwLock<ProjectSet>>,
        client: Client,
    ) {
        match &self.mode {
            DiagnosticDeliveryMode::Pull(diagnostics) => diagnostics.schedule(client),
            DiagnosticDeliveryMode::Push(diagnostics) => {
                diagnostics.schedule(workspace, projects, client)
            }
        }
    }

    /// Remove diagnostics owned by one workspace root.
    pub(crate) async fn remove_root(&self, root: &Path, client: &Client) {
        // release cached diagnostics with their project
        self.workspaces.lock().remove(root);

        match &self.mode {
            DiagnosticDeliveryMode::Pull(diagnostics) => diagnostics.schedule(client.clone()),
            DiagnosticDeliveryMode::Push(diagnostics) => {
                diagnostics.remove_root(root, client).await
            }
        }
    }

    /// Remove diagnostics owned by one source file.
    pub(crate) async fn remove_file(&self, uri: lsp::Uri, client: &Client) {
        match &self.mode {
            DiagnosticDeliveryMode::Pull(diagnostics) => diagnostics.schedule(client.clone()),
            DiagnosticDeliveryMode::Push(diagnostics) => diagnostics.remove_file(uri, client).await,
        }
    }
}

impl WorkspaceDiagnostics {
    /// Select full or unchanged document reports with current editor versions.
    pub(crate) fn reports<'a>(
        &'a self,
        previous_ids: &'a mut BTreeMap<lsp::Uri, String>,
        documents: &'a HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>,
    ) -> impl Iterator<Item = lsp::WorkspaceDocumentDiagnosticReport> + 'a {
        self.reports.iter().map(|report| {
            // compare client results independently of current editor versions
            let version = report
                .uri
                .to_file_path()
                .and_then(|path| documents.get(path.as_ref()))
                .map(|document| i64::from(document.version));
            let previous_id = previous_ids.remove(&report.uri);
            let result_id = &report.full_document_diagnostic_report.result_id;

            if let Some(result_id) = previous_id.filter(|value| Some(value) == result_id.as_ref()) {
                lsp::WorkspaceDocumentDiagnosticReport::Unchanged(
                    lsp::WorkspaceUnchangedDocumentDiagnosticReport {
                        uri: report.uri.clone(),
                        version,
                        unchanged_document_diagnostic_report:
                            lsp::UnchangedDocumentDiagnosticReport { result_id },
                    },
                )
            } else {
                let mut report = report.clone();
                report.version = version;

                lsp::WorkspaceDocumentDiagnosticReport::Full(report)
            }
        })
    }
}

/// Encodes and publishes workspace diagnostics.
pub(crate) struct DiagnosticPublisher {
    /// Client receiving diagnostics.
    client: Client,
    /// Workspace providing related files.
    workspace: Arc<Workspace>,
    /// Open document identities at the diagnostic revision.
    documents: HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>,
}

impl DiagnosticPublisher {
    /// Create one diagnostic publisher.
    pub(crate) fn new(
        client: Client,
        workspace: Arc<Workspace>,
        documents: HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>,
    ) -> Self {
        Self {
            client,
            workspace,
            documents,
        }
    }

    /// Compute a deterministic result ID for one diagnostics payload.
    pub(crate) fn result_id(diagnostics: &[Diagnostic]) -> String {
        let mut hasher = StableHasher::new();
        diagnostics.hash(&mut hasher);

        format!("{:x}", hasher.finish_u64())
    }

    /// Encode a complete workspace document report.
    pub(crate) fn report(
        &self,
        diagnostics: &FileDiagnostics,
    ) -> jsonrpc::Result<lsp::WorkspaceFullDocumentDiagnosticReport> {
        // encode the complete diagnostic payload at its source revision
        let document = self.document(diagnostics)?;
        let documents = self.load_documents(diagnostics)?;
        let result_id = Self::result_id(&diagnostics.diagnostics);
        let items = diagnostics
            .diagnostics
            .iter()
            .map(|diagnostic| documents.diagnostic(diagnostic))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(lsp::WorkspaceFullDocumentDiagnosticReport {
            uri: document.uri,
            version: document.version.map(i64::from),
            full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                result_id: Some(result_id),
                items,
            },
        })
    }

    /// Return one diagnostic file's optional versioned document identifier.
    pub(crate) fn document(
        &self,
        diagnostics: &FileDiagnostics,
    ) -> jsonrpc::Result<lsp::OptionalVersionedTextDocumentIdentifier> {
        let document = diagnostics
            .file
            .path
            .as_ref()
            .and_then(|path| self.documents.get(path));
        if let Some(document) = document {
            return Ok(lsp::OptionalVersionedTextDocumentIdentifier::new(
                document.uri.clone(),
                document.version,
            ));
        }

        let project = ProjectId::from_root(self.workspace.root());
        let document = Document::new(diagnostics.file.clone());
        let uri = document.uri(project)?;

        Ok(lsp::OptionalVersionedTextDocumentIdentifier { uri, version: None })
    }

    /// Load every document referenced by one file's diagnostics.
    pub(crate) fn load_documents(
        &self,
        diagnostics: &FileDiagnostics,
    ) -> jsonrpc::Result<DocumentSet> {
        let mut file_ids = HashSet::new();
        for diagnostic in &diagnostics.diagnostics {
            for label in iter::once(&diagnostic.primary).chain(diagnostic.labels.iter()) {
                file_ids.insert(label.target.file());
            }
        }
        file_ids.remove(&diagnostics.file.id);

        // load cross-file labels from the same semantic revision
        let mut documents = DocumentSet::new(self.workspace.root());
        documents.insert(diagnostics.file.clone())?;
        if file_ids.is_empty() {
            return Ok(documents);
        }

        let related = self
            .workspace
            .read_files(diagnostics.revision, file_ids.iter().copied().collect())
            .map_err(workspace_error)?;

        // require exactly the requested related files
        for file in &related {
            if !file_ids.remove(&file.id) {
                return Err(internal_error(format!(
                    "workspace returned unrequested diagnostic file {:?}",
                    file.id
                )));
            }
        }
        if let Some(file_id) = file_ids.into_iter().next() {
            return Err(internal_error(format!(
                "workspace omitted diagnostic file {file_id:?}"
            )));
        }

        for file in related {
            documents.insert(file)?;
        }

        Ok(documents)
    }

    /// Publish workspace diagnostics and clear files absent from this result.
    async fn publish(
        &self,
        diagnostics: Vec<FileDiagnostics>,
        previous: &HashSet<lsp::Uri>,
    ) -> jsonrpc::Result<HashSet<lsp::Uri>> {
        let mut current = HashSet::new();

        // publish every file in the current result
        for file_diagnostics in diagnostics {
            let document = self.document(&file_diagnostics)?;
            let uri = document.uri;
            current.insert(uri.clone());
            let documents = self.load_documents(&file_diagnostics)?;
            let diagnostics = file_diagnostics
                .diagnostics
                .into_iter()
                .map(|diagnostic| documents.diagnostic(&diagnostic))
                .collect::<jsonrpc::Result<Vec<_>>>()?;

            self.client
                .publish_diagnostics(uri, diagnostics, document.version)
                .await;
        }

        // publish empty results for open files without diagnostics
        let mut open = self.documents.values().collect::<Vec<_>>();
        open.sort_unstable_by(|left, right| left.uri.as_str().cmp(right.uri.as_str()));
        for document in open {
            let uri = document.uri.clone();
            if current.insert(uri.clone()) {
                self.client
                    .publish_diagnostics(uri, Vec::new(), Some(document.version))
                    .await;
            }
        }

        // clear files omitted from the replacement result
        for uri in previous.difference(&current) {
            self.client
                .publish_diagnostics(uri.clone(), Vec::new(), None)
                .await;
        }

        Ok(current)
    }
}

impl DocumentSet {
    /// Encode one TS++ diagnostic as an LSP diagnostic.
    pub(crate) fn diagnostic(&self, diagnostic: &Diagnostic) -> jsonrpc::Result<lsp::Diagnostic> {
        let primary = diagnostic.primary_label();
        let primary_document = self.document(primary.target.file())?;
        if !primary_document.contains_label(primary) {
            return Err(internal_error(format!(
                "diagnostic {} primary label does not match source file {:?}",
                diagnostic.id,
                primary_document.id()
            )));
        }
        let range = primary_document.diagnostic_range(primary)?;

        // map severity
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => Some(lsp::DiagnosticSeverity::ERROR),
            DiagnosticSeverity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
            DiagnosticSeverity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
        };

        // build related locations
        let mut related_locations = Vec::with_capacity(diagnostic.labels.len());
        for label in diagnostic.labels() {
            let document = self.document(label.target.file())?;
            if !document.contains_label(label) {
                return Err(internal_error(format!(
                    "diagnostic {} label does not match source file {:?}",
                    diagnostic.id,
                    document.id()
                )));
            }

            let uri = document.uri(self.project)?;
            let range = document.diagnostic_range(label)?;
            let message = label
                .message
                .clone()
                .unwrap_or_else(|| diagnostic.message.clone());

            related_locations.push(lsp::DiagnosticRelatedInformation {
                location: lsp::Location { uri, range },
                message,
            });
        }
        let related_information = if related_locations.is_empty() {
            None
        } else {
            Some(related_locations)
        };

        // map tags
        let tags = if diagnostic.tags.is_empty() {
            None
        } else {
            Some(
                diagnostic
                    .tags
                    .iter()
                    .map(|tag| match tag {
                        DiagnosticTag::Unnecessary => lsp::DiagnosticTag::UNNECESSARY,
                        DiagnosticTag::Deprecated => lsp::DiagnosticTag::DEPRECATED,
                    })
                    .collect(),
            )
        };

        Ok(lsp::Diagnostic {
            range,
            severity,
            code: Some(lsp::NumberOrString::String(diagnostic.id.clone())),
            code_description: None,
            source: Some("tspp".to_string()),
            message: diagnostic.message.clone(),
            related_information,
            tags,
            data: Some(
                serde_json::to_value(DiagnosticReference::from(diagnostic))
                    .map_err(internal_error)?,
            ),
        })
    }
}

impl Document {
    /// Return whether one diagnostic label addresses this exact document.
    fn contains_label(&self, label: &DiagnosticLabel) -> bool {
        label.target.file() == self.id() && label.blob == self.file().blob()
    }

    /// Build the LSP range for one diagnostic label.
    fn diagnostic_range(&self, label: &DiagnosticLabel) -> jsonrpc::Result<lsp::Range> {
        match label.target {
            DiagnosticTarget::Span(span) => self.range(span),
            DiagnosticTarget::File(_) => Ok(lsp::Range::default()),
        }
    }
}

/// Requests fresh pull diagnostics after source changes.
#[derive(Debug)]
struct PullDiagnostics {
    /// Whether the client accepts refresh requests.
    is_refresh_supported: bool,
    /// Pending refresh request.
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl PullDiagnostics {
    /// Create client initiated diagnostics.
    fn new(is_refresh_supported: bool) -> Self {
        Self {
            is_refresh_supported,
            task: Mutex::new(None),
        }
    }

    /// Request one diagnostic refresh.
    fn schedule(&self, client: Client) {
        if !self.is_refresh_supported {
            return;
        }

        let cancelled_client = client.clone();
        let task = tokio::spawn(async move {
            // request fresh diagnostics from the client
            client.log(LogRecord::new("diagnostics.refresh.started").field("mode", "pull"));
            let started = Instant::now();
            let status = match client.workspace_diagnostic_refresh().await {
                Ok(()) => "ok",
                Err(error) => {
                    client.report_error("diagnostics.refresh", internal_error(error));

                    "error"
                }
            };

            // report the completed refresh request
            client.log(
                LogRecord::new("diagnostics.refresh.finished")
                    .field("mode", "pull")
                    .field("status", status)
                    .field("duration_us", started.elapsed().as_micros()),
            );
        });

        // replace the superseded refresh
        if let Some(previous) = self.task.lock().replace(task)
            && !previous.is_finished()
        {
            previous.abort();
            cancelled_client.log(
                LogRecord::new("diagnostics.cancelled")
                    .field("mode", "pull")
                    .field("reason", "superseded"),
            );
        }
    }
}

impl Drop for PullDiagnostics {
    /// Abort a pending diagnostic refresh.
    fn drop(&mut self) {
        if let Some(task) = self.task.lock().take() {
            task.abort();
        }
    }
}

/// Replaces diagnostic publications by workspace root.
#[derive(Debug, Default)]
struct PushDiagnostics {
    /// Next diagnostic task identity.
    next_id: AtomicU64,
    /// Latest diagnostic task for each root.
    tasks: Arc<Mutex<HashMap<PathBuf, DiagnosticTask>>>,
    /// File URIs published by each root's latest completed task.
    published: Arc<Mutex<HashMap<PathBuf, HashSet<lsp::Uri>>>>,
}

impl PushDiagnostics {
    /// Schedule diagnostics for one changed workspace root.
    fn schedule(
        &self,
        workspace: Arc<Workspace>,
        projects: Arc<RwLock<ProjectSet>>,
        client: Client,
    ) {
        let root = workspace.root().to_path_buf();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tasks = self.tasks.clone();
        let published = self.published.clone();
        let task_root = root.clone();
        let cancelled_client = client.clone();
        let handle = tokio::spawn(async move {
            // report the scheduled publication
            client.log(
                LogRecord::new("diagnostics.scheduled")
                    .field("mode", "push")
                    .field("task_id", id)
                    .field("root", task_root.display()),
            );

            // schedule diagnostic artifacts for the current revision
            let started = Instant::now();
            let revision = { projects.read().revision(workspace.root()) };
            let revision = match revision {
                Some(Ok(revision)) => revision,
                Some(Err(error)) => {
                    client.report_error("diagnostics.revision.read", error);

                    return;
                }
                None => return,
            };
            let run = match workspace.start_diagnostics(revision, ArtifactPriority::Background) {
                Ok(run) => run,
                Err(error) => {
                    client.report_error("diagnostics.schedule", workspace_error(error));

                    return;
                }
            };
            let revision = run.revision();
            let artifact_run_id = run.artifact_run_id();
            let record = LogRecord::new("diagnostics.started")
                .field("mode", "push")
                .field("task_id", id)
                .field("revision", revision)
                .field("run_id", artifact_run_id);
            client.log(record);

            // wait cooperatively for the scheduled diagnostics
            let outcome = run.wait().await;
            let files = outcome.diagnostics.len();
            let failures = outcome.failures.len();

            // reject a publication replaced by a newer task
            let is_current = tasks
                .lock()
                .get(&task_root)
                .is_some_and(|task| task.id == id);
            if !is_current {
                return;
            }

            // snapshot documents only while their branch remains at this revision
            let documents = { projects.read().documents(workspace.root(), revision) };
            let documents = match documents {
                Ok(Some(documents)) => documents,
                Ok(None) => return,
                Err(error) => {
                    client.report_error("diagnostics.revision.read", error);

                    return;
                }
            };

            // publish only the current diagnostic result
            let publisher = DiagnosticPublisher::new(client.clone(), workspace, documents);
            let previous = published
                .lock()
                .get(&task_root)
                .cloned()
                .unwrap_or_default();
            let current = publisher.publish(outcome.diagnostics, &previous).await;
            let is_publication_failed = current.is_err();
            let current = match current {
                Ok(current) => Some(current),
                Err(error) => {
                    client.report_error("diagnostics.publish", error);

                    None
                }
            };

            // discard a publication replaced while it was sent
            let is_current = tasks
                .lock()
                .get(&task_root)
                .is_some_and(|task| task.id == id);
            if !is_current {
                return;
            }

            // retain files published by the current task
            if let Some(current) = current {
                published.lock().insert(task_root, current);
            }

            // report artifact and publication failures together
            let status = if failures == 0 && !is_publication_failed {
                "ok"
            } else {
                "error"
            };
            let record = LogRecord::new("diagnostics.finished")
                .field("mode", "push")
                .field("task_id", id)
                .field("revision", revision)
                .field("run_id", artifact_run_id)
                .field("status", status)
                .field("files", files)
                .field("failures", failures)
                .field("duration_us", started.elapsed().as_micros());
            client.log(record);

            // report run failures after publishing every completed diagnostic
            for failure in outcome.failures {
                client.report_error("diagnostics.read", workspace_error(failure));
            }
        });
        let previous = self
            .tasks
            .lock()
            .insert(root, DiagnosticTask { id, handle });

        // abort the superseded artifact wait
        if let Some(previous) = previous
            && !previous.handle.is_finished()
        {
            previous.handle.abort();
            cancelled_client.log(
                LogRecord::new("diagnostics.cancelled")
                    .field("mode", "push")
                    .field("task_id", previous.id)
                    .field("reason", "superseded"),
            );
        }
    }

    /// Cancel one root and clear every diagnostic it published.
    async fn remove_root(&self, root: &Path, client: &Client) {
        let task = { self.tasks.lock().remove(root) };
        if let Some(task) = task {
            task.handle.abort();
            client.log(
                LogRecord::new("diagnostics.cancelled")
                    .field("mode", "push")
                    .field("task_id", task.id)
                    .field("reason", "root-removed"),
            );
        }
        let published = self.published.lock().remove(root).unwrap_or_default();

        // clear every file last published for the removed root
        for uri in published {
            client.publish_diagnostics(uri, Vec::new(), None).await;
        }
    }

    /// Remove one file from published diagnostic state.
    async fn remove_file(&self, uri: lsp::Uri, client: &Client) {
        self.published.lock().retain(|_, files| {
            files.remove(&uri);

            !files.is_empty()
        });

        client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}

impl Drop for PushDiagnostics {
    /// Abort pending diagnostic publications.
    fn drop(&mut self) {
        for (_, task) in self.tasks.lock().drain() {
            task.handle.abort();
        }
        self.published.lock().clear();
    }
}

/// One pending diagnostic publication.
#[derive(Debug)]
struct DiagnosticTask {
    /// Monotonic identity used to reject superseded publications.
    id: u64,
    /// Async diagnostic task.
    handle: tokio::task::JoinHandle<()>,
}
