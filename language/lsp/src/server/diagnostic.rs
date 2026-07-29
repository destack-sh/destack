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
use destack_query as query;
use destack_source::{
    Diagnostic, DiagnosticLabel, DiagnosticReference, DiagnosticSeverity, DiagnosticTag, File, Span,
};
use destack_workspace::{DiagnosticsRequest, FileDiagnostics, LocalWorkspace, Workspace};
use parking_lot::Mutex;
use serde_json::Value;

use super::artifact::ArtifactRunGuard;
use super::error::{internal_error, workspace_error};
use super::source::SourceFiles;
use super::{edit, position};
use crate::uri;

const DIAGNOSTIC_DELAY: Duration = Duration::from_millis(150);

/// Debounces and replaces diagnostic publications by workspace root.
#[derive(Debug, Default)]
pub(super) struct DiagnosticScheduler {
    /// Next diagnostic task identity.
    next_id: AtomicU64,
    /// Latest diagnostic task for each root.
    tasks: Arc<Mutex<HashMap<PathBuf, DiagnosticTask>>>,
    /// File URIs published by each root's latest completed task.
    published: Arc<Mutex<HashMap<PathBuf, HashSet<lsp::Uri>>>>,
}

impl DiagnosticScheduler {
    /// Schedule diagnostics for one changed workspace root.
    pub(super) fn schedule(&self, root: PathBuf, workspace: Arc<LocalWorkspace>, client: Client) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let tasks = self.tasks.clone();
        let published = self.published.clone();
        let task_root = root.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(DIAGNOSTIC_DELAY).await;

            // schedule exact diagnostic artifacts after the debounce interval
            let run = match workspace.start_diagnostics(DiagnosticsRequest::Root(task_root.clone()))
            {
                Ok(run) => run,
                Err(error) => {
                    client
                        .log_message(
                            lsp::MessageType::ERROR,
                            format!("failed to schedule diagnostics: {error}"),
                        )
                        .await;

                    return;
                }
            };
            let revisions = run.revisions();
            let mut guard = ArtifactRunGuard::new(run.cancellations());

            // wait without blocking the async language server
            let diagnostics = match tokio::task::spawn_blocking(move || run.wait()).await {
                Ok(Ok(diagnostics)) => diagnostics,
                Ok(Err(error)) => {
                    client
                        .log_message(
                            lsp::MessageType::ERROR,
                            format!("failed to read diagnostics: {error}"),
                        )
                        .await;

                    return;
                }
                Err(error) => {
                    client
                        .log_message(
                            lsp::MessageType::ERROR,
                            format!("diagnostic worker failed: {error}"),
                        )
                        .await;

                    return;
                }
            };
            guard.disarm();

            // reject superseded tasks and revisions
            let is_current = tasks
                .lock()
                .get(&task_root)
                .is_some_and(|task| task.id == id);
            let revisions_are_current = revisions.iter().all(|(root, revision)| {
                workspace
                    .revision(root)
                    .is_ok_and(|current| current == *revision)
            });
            if !is_current || !revisions_are_current {
                return;
            }

            // publish only the current exact diagnostic result
            let publisher = DiagnosticPublisher::new(client, workspace);
            let previous = published
                .lock()
                .get(&task_root)
                .cloned()
                .unwrap_or_default();
            match publisher.publish(diagnostics, &previous).await {
                Ok(current) => {
                    let is_current = tasks
                        .lock()
                        .get(&task_root)
                        .is_some_and(|task| task.id == id);
                    if is_current {
                        published.lock().insert(task_root, current);
                    }
                }
                Err(error) => {
                    publisher
                        .client
                        .log_message(
                            lsp::MessageType::ERROR,
                            format!("failed to publish diagnostics: {error}"),
                        )
                        .await;
                }
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
    pub(super) async fn clear(&self, root: &Path, client: &Client) {
        if let Some(task) = self.tasks.lock().remove(root) {
            task.handle.abort();
        }
        let published = self.published.lock().remove(root).unwrap_or_default();

        // clear every file last published for the removed root
        for uri in published {
            client.publish_diagnostics(uri, Vec::new(), None).await;
        }
    }
}

impl Drop for DiagnosticScheduler {
    /// Abort pending diagnostic publications.
    fn drop(&mut self) {
        for (_, task) in self.tasks.lock().drain() {
            task.handle.abort();
        }
        self.published.lock().clear();
    }
}

/// Converts and publishes exact workspace diagnostics.
pub(super) struct DiagnosticPublisher {
    /// Client receiving diagnostics.
    client: Client,
    /// Workspace providing exact related files.
    workspace: Arc<LocalWorkspace>,
}

impl DiagnosticPublisher {
    /// Create one diagnostic publisher.
    pub(super) fn new(client: Client, workspace: Arc<LocalWorkspace>) -> Self {
        Self { client, workspace }
    }

    /// Load every source file referenced by one file's diagnostics.
    pub(super) fn files(&self, diagnostics: &FileDiagnostics) -> jsonrpc::Result<SourceFiles> {
        let mut file_ids = HashSet::new();
        for diagnostic in &diagnostics.diagnostics {
            for label in iter::once(&diagnostic.primary).chain(diagnostic.labels.iter()) {
                file_ids.insert(label.target.file());
            }
        }
        file_ids.remove(&diagnostics.file.id);

        // load cross-file labels from the same semantic revision
        let mut files = SourceFiles::new();
        files
            .insert(diagnostics.file.clone())
            .map_err(workspace_error)?;
        if file_ids.is_empty() {
            return Ok(files);
        }

        let path = diagnostics.file.path.as_deref().ok_or_else(|| {
            internal_error(format!(
                "diagnostic source file {:?} has no workspace path",
                diagnostics.file.id
            ))
        })?;
        let root = self.workspace.root(path).map_err(workspace_error)?;
        let related = self
            .workspace
            .read_files(
                &root,
                diagnostics.revision,
                file_ids.iter().copied().collect(),
            )
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
            files.insert(file).map_err(workspace_error)?;
        }

        Ok(files)
    }

    /// Publish exact workspace diagnostics and clear files absent from this result.
    pub(super) async fn publish(
        &self,
        diagnostics: Vec<FileDiagnostics>,
        previous: &HashSet<lsp::Uri>,
    ) -> jsonrpc::Result<HashSet<lsp::Uri>> {
        let mut current = HashSet::new();

        // publish every file in the current result
        for file_diagnostics in diagnostics {
            let uri = uri::source(&file_diagnostics.uri).ok_or_else(|| {
                internal_error(format!(
                    "diagnostic URI is not representable by LSP: {}",
                    file_diagnostics.uri
                ))
            })?;
            current.insert(uri.clone());
            let files = self.files(&file_diagnostics)?;
            let diagnostics = file_diagnostics
                .diagnostics
                .into_iter()
                .map(|diagnostic| item(&diagnostic, &files))
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

/// One pending diagnostic publication.
#[derive(Debug)]
struct DiagnosticTask {
    /// Monotonic identity used to reject superseded publications.
    id: u64,
    /// Async diagnostic task.
    handle: tokio::task::JoinHandle<()>,
}

/// Convert a Destack diagnostic to an LSP diagnostic.
pub(super) fn item(
    diagnostic: &Diagnostic,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::Diagnostic> {
    let primary = diagnostic.primary_label();
    let primary_file = files.file(primary.target.file()).map_err(workspace_error)?;
    if !label_matches_file(primary, &primary_file) {
        return Err(internal_error(format!(
            "diagnostic {} primary label does not match source file {:?}",
            diagnostic.id, primary_file.id
        )));
    }

    let primary_span = primary
        .target
        .span()
        .unwrap_or_else(|| Span::empty(primary.target.file()));
    let range = position::range(&primary_file, primary_span)?;

    // severity
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => Some(lsp::DiagnosticSeverity::ERROR),
        DiagnosticSeverity::Warning => Some(lsp::DiagnosticSeverity::WARNING),
        DiagnosticSeverity::Note => Some(lsp::DiagnosticSeverity::INFORMATION),
    };

    // related locations
    let mut related_locations = Vec::with_capacity(diagnostic.labels.len());
    for label in diagnostic.labels() {
        let file = files.file(label.target.file()).map_err(workspace_error)?;
        if !label_matches_file(label, &file) {
            return Err(internal_error(format!(
                "diagnostic {} label does not match source file {:?}",
                diagnostic.id, file.id
            )));
        }

        let span = label
            .target
            .span()
            .unwrap_or_else(|| Span::empty(label.target.file()));
        let uri = uri::file(&file)?;
        let range = position::range(&file, span)?;
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

    // tags
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

    // diagnostic
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
            serde_json::to_value(DiagnosticReference::from(diagnostic)).map_err(internal_error)?,
        ),
    })
}

/// Compute a deterministic result id for a diagnostics payload.
pub(super) fn result_id(diagnostics: &[Diagnostic]) -> String {
    let mut hasher = StableHasher::new();
    diagnostics.hash(&mut hasher);

    format!("{:x}", hasher.finish_u64())
}

/// Return true when one diagnostic label still points at the same file content.
fn label_matches_file(label: &DiagnosticLabel, file: &File) -> bool {
    label.target.file() == file.id && label.content == file.content_id()
}

/// Convert a workspace code action kind to an LSP code action kind.
fn code_action_kind(kind: query::CodeActionKind) -> lsp::CodeActionKind {
    match kind {
        query::CodeActionKind::QuickFix => lsp::CodeActionKind::QUICKFIX,
        query::CodeActionKind::RefactorExtract => lsp::CodeActionKind::REFACTOR_EXTRACT,
        query::CodeActionKind::RefactorInline => lsp::CodeActionKind::REFACTOR_INLINE,
    }
}

/// Convert a code action to an LSP code action.
pub(super) fn code_action(
    action: &query::CodeAction,
    diagnostics: Option<Vec<lsp::Diagnostic>>,
    include_edit: bool,
    data: Option<Value>,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::CodeActionOrCommand> {
    let edit = if include_edit && !action.patches.is_empty() {
        Some(edit::workspace(&action.patches, files)?)
    } else {
        None
    };

    Ok(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: action.title.clone(),
        kind: Some(code_action_kind(action.kind)),
        diagnostics,
        edit,
        command: None,
        is_preferred: Some(action.is_preferred),
        disabled: None,
        data,
    }))
}
