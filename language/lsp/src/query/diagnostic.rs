use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use destack_core::StableHasher;
use destack_lsp_server::{Client, jsonrpc};
use destack_lsp_types as lsp;
use destack_source::{
    Diagnostic, DiagnosticLabel, DiagnosticReference, DiagnosticSeverity, DiagnosticTag,
    DiagnosticTarget,
};
use destack_workspace::{DiagnosticsRequest, FileDiagnostics, Workspace};
use parking_lot::Mutex;

use super::{Document, DocumentSet, ToLspUri};
use crate::server::{internal_error, workspace_error};

/// Delay used to replace superseded diagnostic work.
const DIAGNOSTIC_DELAY: Duration = Duration::from_millis(150);

/// Diagnostic delivery selected from client capabilities.
#[derive(Debug)]
pub(crate) struct DiagnosticDelivery {
    /// Active diagnostic delivery mode.
    mode: DiagnosticDeliveryMode,
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
        }
    }

    /// Create push diagnostics.
    pub(crate) fn push() -> Self {
        Self {
            mode: DiagnosticDeliveryMode::Push(PushDiagnostics::default()),
        }
    }

    /// Schedule diagnostics after source state changes.
    pub(crate) fn schedule(&self, workspace: Arc<Workspace>, client: Client) {
        match &self.mode {
            DiagnosticDeliveryMode::Pull(diagnostics) => diagnostics.schedule(client),
            DiagnosticDeliveryMode::Push(diagnostics) => diagnostics.schedule(workspace, client),
        }
    }

    /// Remove diagnostics owned by one workspace root.
    pub(crate) async fn remove_root(&self, root: &Path, client: &Client) {
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

/// Encodes and publishes workspace diagnostics.
pub(crate) struct DiagnosticPublisher {
    /// Client receiving diagnostics.
    client: Client,
    /// Workspace providing related files.
    workspace: Arc<Workspace>,
}

impl DiagnosticPublisher {
    /// Create one diagnostic publisher.
    pub(crate) fn new(client: Client, workspace: Arc<Workspace>) -> Self {
        Self { client, workspace }
    }

    /// Compute a deterministic result ID for one diagnostics payload.
    pub(crate) fn result_id(diagnostics: &[Diagnostic]) -> String {
        let mut hasher = StableHasher::new();
        diagnostics.hash(&mut hasher);

        format!("{:x}", hasher.finish_u64())
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
        let mut documents = DocumentSet::new();
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
            let uri = file_diagnostics.uri.to_lsp_uri().ok_or_else(|| {
                internal_error(format!(
                    "diagnostic URI is not representable by LSP: {}",
                    file_diagnostics.uri
                ))
            })?;
            current.insert(uri.clone());
            let documents = self.load_documents(&file_diagnostics)?;
            let diagnostics = file_diagnostics
                .diagnostics
                .into_iter()
                .map(|diagnostic| documents.diagnostic(&diagnostic))
                .collect::<jsonrpc::Result<Vec<_>>>()?;

            self.client
                .publish_diagnostics(uri, diagnostics, file_diagnostics.version)
                .await;
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
    /// Encode one Destack diagnostic as an LSP diagnostic.
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

            let uri = document.uri()?;
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
            source: Some("destack".to_string()),
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

    /// Request one debounced diagnostic refresh.
    fn schedule(&self, client: Client) {
        if !self.is_refresh_supported {
            return;
        }

        let task = tokio::spawn(async move {
            tokio::time::sleep(DIAGNOSTIC_DELAY).await;
            if let Err(error) = client.workspace_diagnostic_refresh().await {
                client
                    .report_error("diagnostics.refresh", internal_error(error))
                    .await;
            }
        });

        // replace the superseded refresh
        if let Some(previous) = self.task.lock().replace(task) {
            previous.abort();
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

/// Debounces and replaces diagnostic publications by workspace root.
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
    fn schedule(&self, workspace: Arc<Workspace>, client: Client) {
        let root = workspace.root().to_path_buf();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tasks = self.tasks.clone();
        let published = self.published.clone();
        let task_root = root.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(DIAGNOSTIC_DELAY).await;

            // schedule diagnostic artifacts after the debounce interval
            let run = match workspace.start_diagnostics(DiagnosticsRequest::All) {
                Ok(run) => run,
                Err(error) => {
                    client
                        .report_error("diagnostics.schedule", workspace_error(error))
                        .await;

                    return;
                }
            };
            let revision = run.revision();

            // wait cooperatively for the scheduled diagnostics
            let outcome = run.wait().await;

            // reject a publication replaced by a newer task
            let is_current = tasks
                .lock()
                .get(&task_root)
                .is_some_and(|task| task.id == id);
            if !is_current {
                return;
            }

            // reject a workspace invalidated while diagnostics were running
            let current = match workspace.revision() {
                Ok(current) => current,
                Err(error) => {
                    client
                        .report_error("diagnostics.revision.read", workspace_error(error))
                        .await;

                    return;
                }
            };
            if current != revision {
                return;
            }

            // publish only the current diagnostic result
            let publisher = DiagnosticPublisher::new(client.clone(), workspace);
            let previous = published
                .lock()
                .get(&task_root)
                .cloned()
                .unwrap_or_default();
            let current = match publisher.publish(outcome.diagnostics, &previous).await {
                Ok(current) => Some(current),
                Err(error) => {
                    client.report_error("diagnostics.publish", error).await;

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

            // report run failures after publishing every completed diagnostic
            for failure in outcome.failures {
                client
                    .report_error("diagnostics.read", workspace_error(failure))
                    .await;
            }
        });
        let previous = self
            .tasks
            .lock()
            .insert(root, DiagnosticTask { id, handle });

        // abort the superseded debounce or artifact wait
        if let Some(previous) = previous {
            previous.handle.abort();
        }
    }

    /// Cancel one root and clear every diagnostic it published.
    async fn remove_root(&self, root: &Path, client: &Client) {
        if let Some(task) = self.tasks.lock().remove(root) {
            task.handle.abort();
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
