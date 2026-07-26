use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::{fs, iter};

#[cfg(test)]
use destack_artifact::MemoryBlobStore;
use destack_lsp_server::{Client, LanguageServer, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{
    DestackLayoutOverride, Environment, Revision, Settings, open_repository_from_fs,
};
use destack_source::{
    FileId, FileSystem, OverlayFileSystem, PatchSet, PhysicalFileSystem, TextRange, Uri,
};
use destack_workspace::{
    DiagnosticsRequest, FileDiagnostics, FileEdit, FileOperation, LocalWorkspace, QueryFile,
    ReloadReason, ReloadRequest, RevisionPolicy, RunQueryRequest, RunQueryResponse, Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};

use crate::server::error::{internal_error, workspace_error};
use crate::server::navigation::HierarchyContinuation;
use crate::server::source::SourceFiles;
use crate::server::{
    assist, completion, diagnostic, edit, format, navigation, position, source, token,
};
use crate::uri;

const AUTO_IMPORT_DETAIL_PREFIX: &str = "Auto import from ";

/// Canonicalize a root path for stable dedupe comparisons.
fn canonical_path(path: &Path) -> jsonrpc::Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| {
        jsonrpc::Error::invalid_params(format!(
            "failed to canonicalize workspace root {}: {error}",
            path.display()
        ))
    })
}

/// State carried into a code action resolve request.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeActionContinuation {
    /// The semantic revision that produced these edits.
    revision: Revision,
    /// The file path that anchored the action request.
    path: PathBuf,
    /// The workspace edits for the selected code action.
    patches: PatchSet,
}

/// Configuration for completion behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionSettings {
    /// Whether auto import completions are enabled.
    auto_imports: Option<bool>,
}

impl Default for CompletionSettings {
    fn default() -> Self {
        Self {
            auto_imports: Some(true),
        }
    }
}

/// Configuration for inlay hint behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlayHintSettings {
    /// Whether parameter name hints are enabled.
    parameter_hints: Option<bool>,
    /// Whether inferred type hints are enabled.
    type_hints: Option<bool>,
}

impl Default for InlayHintSettings {
    fn default() -> Self {
        Self {
            parameter_hints: Some(true),
            type_hints: Some(true),
        }
    }
}

/// LSP configuration settings.
#[derive(Debug)]
struct LspSettings {
    /// Whether auto import completions are enabled.
    completion_auto_imports: AtomicBool,
    /// Whether parameter name inlay hints are enabled.
    parameter_inlay_hints: AtomicBool,
    /// Whether inferred type inlay hints are enabled.
    type_inlay_hints: AtomicBool,
}

impl Default for LspSettings {
    /// Create default LSP settings.
    fn default() -> Self {
        Self {
            completion_auto_imports: AtomicBool::new(true),
            parameter_inlay_hints: AtomicBool::new(true),
            type_inlay_hints: AtomicBool::new(true),
        }
    }
}

/// LSP features supported by the connected client.
#[derive(Debug)]
struct ClientCapabilities {
    /// Whether completion label details are supported.
    completion_label_details: bool,
    /// Whether code action data payloads are supported.
    code_action_data: bool,
    /// Whether code action edits can be resolved lazily.
    code_action_edit_resolve: bool,
}

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client connection.
    pub(super) client: Client,
    /// Workspace used for semantic state.
    workspace: OnceLock<Arc<dyn Workspace>>,
    /// The watch registration ID.
    watch_registration_id: OnceLock<String>,
    /// Features supported by the connected client.
    client_capabilities: OnceLock<ClientCapabilities>,
    /// LSP configuration settings.
    settings: LspSettings,
}

#[allow(clippy::too_many_arguments)]
impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        // initialize server state
        Self {
            client,
            workspace: OnceLock::new(),
            watch_registration_id: OnceLock::new(),
            client_capabilities: OnceLock::new(),
            settings: LspSettings::default(),
        }
    }

    /// Return the workspace after initialization.
    #[inline]
    fn workspace(&self) -> jsonrpc::Result<&Arc<dyn Workspace>> {
        self.workspace
            .get()
            .ok_or_else(|| internal_error("language server is not initialized"))
    }

    /// Return features supported by the initialized client.
    fn client_capabilities(&self) -> jsonrpc::Result<&ClientCapabilities> {
        self.client_capabilities
            .get()
            .ok_or_else(|| internal_error("client capabilities are not initialized"))
    }

    /// Build commit characters for completion items.
    fn completion_commit_characters(
        kind: query::CompletionItemKind,
        is_snippet: bool,
    ) -> Option<Vec<String>> {
        // snippets already contain their committed punctuation
        if is_snippet {
            return None;
        }

        let commit_characters: &[&str] = match kind {
            query::CompletionItemKind::AssociatedConst
            | query::CompletionItemKind::AssociatedType
            | query::CompletionItemKind::Method
            | query::CompletionItemKind::Function
            | query::CompletionItemKind::Constructor
            | query::CompletionItemKind::Field
            | query::CompletionItemKind::Variable
            | query::CompletionItemKind::Class
            | query::CompletionItemKind::Interface
            | query::CompletionItemKind::NewtypeInterface
            | query::CompletionItemKind::Newtype
            | query::CompletionItemKind::TypeAlias
            | query::CompletionItemKind::Extension
            | query::CompletionItemKind::Module
            | query::CompletionItemKind::Property
            | query::CompletionItemKind::Enum
            | query::CompletionItemKind::EnumMember
            | query::CompletionItemKind::Struct
            | query::CompletionItemKind::Constant
            | query::CompletionItemKind::TypeParameter
            | query::CompletionItemKind::ValueParameter
            | query::CompletionItemKind::BuiltinType => &[".", ",", ";", "("],
            query::CompletionItemKind::Keyword => &[" ", ";"],
            _ => &[],
        };

        if commit_characters.is_empty() {
            return None;
        }

        Some(
            commit_characters
                .iter()
                .map(|character| (*character).to_string())
                .collect(),
        )
    }

    /// Refresh configuration from the client.
    async fn refresh_configuration(&self) -> jsonrpc::Result<()> {
        let items = vec![
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.completion".to_string()),
            },
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.inlayHints".to_string()),
            },
        ];
        let values = self.client.configuration(items).await?;

        // update completion settings
        let completion = values
            .first()
            .ok_or_else(|| internal_error("client omitted completion configuration"))?;
        if !completion.is_null() {
            let parsed =
                from_value::<CompletionSettings>(completion.clone()).map_err(internal_error)?;
            if let Some(auto_imports) = parsed.auto_imports {
                self.settings
                    .completion_auto_imports
                    .store(auto_imports, Ordering::Relaxed);
            }
        }

        // update inlay hint settings
        let inlay_hints = values
            .get(1)
            .ok_or_else(|| internal_error("client omitted inlay hint configuration"))?;
        if !inlay_hints.is_null() {
            let parsed =
                from_value::<InlayHintSettings>(inlay_hints.clone()).map_err(internal_error)?;
            if let Some(parameter_hints) = parsed.parameter_hints {
                self.settings
                    .parameter_inlay_hints
                    .store(parameter_hints, Ordering::Relaxed);
            }
            if let Some(type_hints) = parsed.type_hints {
                self.settings
                    .type_inlay_hints
                    .store(type_hints, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Report an asynchronous LSP failure to the client.
    async fn report_error(&self, operation: &str, error: impl Display) {
        self.client
            .log_message(lsp::MessageType::ERROR, format!("{operation}: {error}"))
            .await;
    }

    /// Execute one program query through the workspace.
    fn query_program(
        &self,
        path: &Path,
        request: query::QueryRequest,
        revision: RevisionPolicy,
    ) -> jsonrpc::Result<RunQueryResponse> {
        let workspace = self.workspace()?;
        let root = workspace.root(path).map_err(workspace_error)?;
        let request = RunQueryRequest { revision, request };
        let result = workspace.run_query(&root, request);

        result.map_err(workspace_error)
    }

    /// Resolve one LSP document to exact workspace query state.
    fn resolve_query_file(&self, uri: &lsp::Uri) -> jsonrpc::Result<Option<QueryFile>> {
        let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(None);
        };
        let workspace = self.workspace()?;
        let root = workspace.root(&path).map_err(workspace_error)?;

        workspace
            .resolve_query_file(&root, path)
            .map_err(workspace_error)
    }

    /// Format one LSP document or selected text range.
    fn format_file(
        &self,
        uri: &lsp::Uri,
        range: Option<TextRange>,
    ) -> jsonrpc::Result<Option<FileEdit>> {
        let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(None);
        };
        let workspace = self.workspace()?;
        let root = workspace.root(&path).map_err(workspace_error)?;

        workspace
            .format_file(&root, path, range)
            .map_err(workspace_error)
    }

    /// Execute one module query against an exact file revision.
    fn query_module(
        &self,
        file: &QueryFile,
        request: query::QueryRequest,
    ) -> jsonrpc::Result<RunQueryResponse> {
        let workspace = self.workspace()?;
        let root = workspace.root(&file.path).map_err(workspace_error)?;
        let request = RunQueryRequest {
            revision: RevisionPolicy::Current(file.revision),
            request,
        };
        let result = workspace.run_query(&root, request);

        result.map_err(workspace_error)
    }

    /// Load source files from one exact query revision.
    fn source_files(
        &self,
        path: &Path,
        revision: Revision,
        file_ids: impl IntoIterator<Item = FileId>,
    ) -> jsonrpc::Result<SourceFiles> {
        let workspace = self.workspace()?;
        SourceFiles::load(workspace.as_ref(), path, revision, file_ids).map_err(workspace_error)
    }

    /// Load every source file referenced by one file's diagnostics.
    fn diagnostic_files(&self, diagnostics: &FileDiagnostics) -> jsonrpc::Result<SourceFiles> {
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
        if !file_ids.is_empty() {
            let path = diagnostics.file.path.as_deref().ok_or_else(|| {
                internal_error(format!(
                    "diagnostic source file {:?} has no workspace path",
                    diagnostics.file.id
                ))
            })?;
            let workspace = self.workspace()?;
            let root = workspace.root(path).map_err(workspace_error)?;
            let related = workspace
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
        }

        Ok(files)
    }

    /// Convert an LSP code action kind into service query kinds.
    fn query_code_action_kinds(kind: &lsp::CodeActionKind) -> Vec<query::CodeActionKind> {
        let kind_name = kind.as_str();

        // quick fixes and sub kinds
        if kind_name == lsp::CodeActionKind::QUICKFIX.as_str() || kind_name.starts_with("quickfix.")
        {
            return vec![query::CodeActionKind::QuickFix];
        }

        // extract refactors and sub kinds
        if kind_name == lsp::CodeActionKind::REFACTOR_EXTRACT.as_str()
            || kind_name.starts_with("refactor.extract.")
        {
            return vec![query::CodeActionKind::RefactorExtract];
        }

        // inline refactors and sub kinds
        if kind_name == lsp::CodeActionKind::REFACTOR_INLINE.as_str()
            || kind_name.starts_with("refactor.inline.")
        {
            return vec![query::CodeActionKind::RefactorInline];
        }

        // umbrella refactor kind
        if kind_name == lsp::CodeActionKind::REFACTOR.as_str() {
            return vec![
                query::CodeActionKind::RefactorExtract,
                query::CodeActionKind::RefactorInline,
            ];
        }

        Vec::new()
    }

    /// Build query code action context from LSP code action context.
    fn query_code_action_context(context: &lsp::CodeActionContext) -> query::CodeActionContext {
        let mut only = Vec::new();
        let mut seen = HashSet::new();

        // map each requested lsp kind to query code action kinds
        if let Some(kinds) = context.only.as_ref() {
            for kind in kinds {
                for query_kind in Self::query_code_action_kinds(kind) {
                    if seen.insert(query_kind) {
                        only.push(query_kind);
                    }
                }
            }
        }

        query::CodeActionContext { only }
    }

    /// Register file watchers with the client.
    async fn register_file_watchers(&self) -> jsonrpc::Result<()> {
        if self.watch_registration_id.get().is_some() {
            return Ok(());
        }

        let watchers = source::file_watchers();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = Some(to_value(options).map_err(internal_error)?);
        let registration = lsp::Registration {
            id: "destack.watch".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options,
        };

        self.client.register_capability(vec![registration]).await?;
        self.watch_registration_id
            .set("destack.watch".to_string())
            .map_err(|_| internal_error("file watcher registration changed concurrently"))?;

        Ok(())
    }

    /// Reload workspace source and read exact diagnostics.
    fn reload_diagnostics(&self) -> jsonrpc::Result<Vec<FileDiagnostics>> {
        let workspace = self.workspace()?;
        workspace
            .reload(ReloadRequest {
                roots: Vec::new(),
                reason: ReloadReason::Manual,
            })
            .map_err(workspace_error)?;

        workspace
            .diagnose(DiagnosticsRequest::All)
            .map_err(workspace_error)
    }

    /// Publish exact workspace diagnostics.
    async fn publish_diagnostics(&self, diagnostics: Vec<FileDiagnostics>) -> jsonrpc::Result<()> {
        for file_diagnostics in diagnostics {
            let uri = uri::source(&file_diagnostics.uri).ok_or_else(|| {
                internal_error(format!(
                    "diagnostic URI is not representable by LSP: {}",
                    file_diagnostics.uri
                ))
            })?;
            let files = self.diagnostic_files(&file_diagnostics)?;
            let diagnostics = file_diagnostics
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic::item(&diagnostic, &files))
                .collect::<jsonrpc::Result<Vec<_>>>()?;

            self.client
                .publish_diagnostics(uri, diagnostics, file_diagnostics.version)
                .await;
        }

        Ok(())
    }

    /// Reload configuration and publish diagnostics for every root.
    async fn change_configuration(&self) -> jsonrpc::Result<()> {
        self.refresh_configuration().await?;
        let diagnostics = self.reload_diagnostics()?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Open one editor document and publish diagnostics for its root.
    async fn open_document(&self, params: lsp::DidOpenTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let uri = Uri::from_string(params.text_document.uri.to_string());
        let content = source::normalize_line_endings(params.text_document.text);
        let workspace = self.workspace()?;
        workspace
            .file(FileOperation::OpenText {
                path: path.clone(),
                uri,
                version: params.text_document.version,
                content,
            })
            .map_err(workspace_error)?;

        let root = workspace.root(&path).map_err(workspace_error)?;
        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::Root(root))
            .map_err(workspace_error)?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Apply one editor document change and publish diagnostics for its root.
    async fn change_document(
        &self,
        params: lsp::DidChangeTextDocumentParams,
    ) -> jsonrpc::Result<()> {
        if params.content_changes.is_empty() {
            return Ok(());
        }

        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let uri = Uri::from_string(params.text_document.uri.to_string());
        let changes = source::changes(params.content_changes);
        let workspace = self.workspace()?;
        workspace
            .file(FileOperation::PatchText {
                path: path.clone(),
                uri,
                version: params.text_document.version,
                changes,
            })
            .map_err(workspace_error)?;

        let root = workspace.root(&path).map_err(workspace_error)?;
        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::Root(root))
            .map_err(workspace_error)?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Save one editor document and publish diagnostics for its root.
    async fn save_document(&self, params: lsp::DidSaveTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let content = params.text.map(source::normalize_line_endings);
        let workspace = self.workspace()?;
        workspace
            .file(FileOperation::SaveText {
                path: path.clone(),
                content,
            })
            .map_err(workspace_error)?;

        let root = workspace.root(&path).map_err(workspace_error)?;
        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::Root(root))
            .map_err(workspace_error)?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Close one editor document and publish diagnostics for its root.
    async fn close_document(&self, params: lsp::DidCloseTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let workspace = self.workspace()?;
        let root = workspace.root(&path).map_err(workspace_error)?;
        workspace
            .file(FileOperation::Close { path })
            .map_err(workspace_error)?;

        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::Root(root))
            .map_err(workspace_error)?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Apply one workspace-folder change.
    fn change_workspace_folders(
        &self,
        params: lsp::DidChangeWorkspaceFoldersParams,
    ) -> jsonrpc::Result<()> {
        let mut added = Vec::with_capacity(params.event.added.len());
        for folder in params.event.added {
            let path = folder
                .uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| {
                    jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                })?;
            added.push(path);
        }

        let mut removed = Vec::with_capacity(params.event.removed.len());
        for folder in params.event.removed {
            let path = folder
                .uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| {
                    jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                })?;
            removed.push(path);
        }

        // apply the validated root changes
        let workspace = self.workspace()?;
        for path in added {
            workspace.open(path).map_err(workspace_error)?;
        }
        for path in removed {
            workspace.close(&path).map_err(workspace_error)?;
        }

        Ok(())
    }

    /// Reload watched filesystem state and publish current diagnostics.
    async fn change_watched_files(
        &self,
        params: lsp::DidChangeWatchedFilesParams,
    ) -> jsonrpc::Result<()> {
        if params.changes.is_empty() {
            return Ok(());
        }

        let diagnostics = self.reload_diagnostics()?;

        self.publish_diagnostics(diagnostics).await
    }

    /// Clear diagnostics for one renamed file when neither path remains open.
    async fn clear_renamed_file_diagnostics(
        &self,
        old_uri: &str,
        new_uri: &str,
    ) -> jsonrpc::Result<()> {
        let old_uri = old_uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid old URI: {error}")))?;
        let new_uri = new_uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid new URI: {error}")))?;
        let old_path = old_uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("old URI is not a file URI"))?;
        let new_path = new_uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("new URI is not a file URI"))?;

        // keep diagnostics owned by open editor documents
        let workspace = self.workspace()?;
        let old_is_open = workspace.is_file_open(&old_path).map_err(workspace_error)?;
        let new_is_open = workspace.is_file_open(&new_path).map_err(workspace_error)?;
        if old_is_open || new_is_open {
            return Ok(());
        }

        self.client
            .publish_diagnostics(old_uri, Vec::new(), None)
            .await;

        Ok(())
    }

    /// Clear diagnostics for one deleted file when it is no longer open.
    async fn clear_deleted_file_diagnostics(&self, uri: &str) -> jsonrpc::Result<()> {
        let uri = uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid URI: {error}")))?;
        let path = uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("URI is not a file URI"))?;

        // keep diagnostics owned by an open editor document
        let is_open = self
            .workspace()?
            .is_file_open(&path)
            .map_err(workspace_error)?;
        if is_open {
            return Ok(());
        }

        self.client.publish_diagnostics(uri, Vec::new(), None).await;

        Ok(())
    }
}

// ----------------------------------------------------------------------------
// LIFECYCLE
// ----------------------------------------------------------------------------

impl LanguageServer for DestackLanguageServer {
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        // determine root from params
        #[allow(deprecated)]
        let cwd = if let Some(path) = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().map(|p| p.into_owned()))
        {
            path
        } else if let Some(path) = params.root_path.clone().map(PathBuf::from) {
            path
        } else {
            std::env::current_dir().map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "failed to resolve current directory for initialize: {error}"
                ))
            })?
        };
        let cwd = canonical_path(&cwd)?;

        // collect initialization roots from workspace folders
        let mut initialize_roots = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders.as_ref() {
            for folder in workspace_folders {
                let path = folder
                    .uri
                    .to_file_path()
                    .map(|path| path.into_owned())
                    .ok_or_else(|| {
                        jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                    })?;

                let path = canonical_path(&path)?;
                if !initialize_roots.contains(&path) {
                    initialize_roots.push(path);
                }
            }
        }
        if initialize_roots.is_empty() {
            initialize_roots.push(cwd.clone());
        }

        // create repository with overlay filesystem
        let physical_fs = Arc::new(PhysicalFileSystem::new());
        let overlay_fs = Arc::new(OverlayFileSystem::with_inner(physical_fs));
        let repository = open_repository_from_fs(
            cwd.clone(),
            overlay_fs.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(internal_error)?;
        #[cfg(test)]
        let repository = repository.with_blob_store(Arc::new(MemoryBlobStore::new()));
        let root = canonical_path(repository.path())?;

        // merge discovery root with initialize roots
        let mut opened_roots = vec![root.clone()];
        for initialize_root in initialize_roots {
            if !opened_roots.contains(&initialize_root) {
                opened_roots.push(initialize_root);
            }
        }

        let repository = Arc::new(repository);
        // open the semantic workspace for the editor session
        let workspace: Arc<dyn Workspace> = Arc::new(
            LocalWorkspace::new(
                repository.clone(),
                Some(overlay_fs),
                None,
                opened_roots,
                LocalWorkspace::default_worker_count(),
                None,
            )
            .map_err(internal_error)?,
        );
        self.workspace
            .set(workspace)
            .map_err(|_| internal_error("language server workspace changed concurrently"))?;

        // record capabilities used by query projections
        let completion_label_details = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref())
            .and_then(|item| item.label_details_support)
            .unwrap_or(false);
        let code_action_capabilities = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.code_action.as_ref());
        let code_action_data = code_action_capabilities
            .and_then(|capabilities| capabilities.data_support)
            .unwrap_or(false);
        let code_action_edit_resolve = code_action_capabilities
            .and_then(|capabilities| capabilities.resolve_support.as_ref())
            .is_some_and(|resolve| resolve.properties.iter().any(|property| property == "edit"));
        let client_capabilities = ClientCapabilities {
            completion_label_details,
            code_action_data,
            code_action_edit_resolve,
        };
        self.client_capabilities
            .set(client_capabilities)
            .map_err(|_| internal_error("client capabilities changed concurrently"))?;

        // build file operation filters for root notifications
        let file_operation_filters: Vec<lsp::FileOperationFilter> = source::tracked_file_globs()
            .into_iter()
            .map(|glob| lsp::FileOperationFilter {
                scheme: Some("file".to_string()),
                pattern: lsp::FileOperationPattern {
                    glob: glob.to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: None,
                },
            })
            .collect();

        // declare server capabilities
        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
                lsp::TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(
                        lsp::SaveOptions {
                            include_text: Some(true),
                        }
                        .into(),
                    ),
                },
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
            definition_provider: Some(lsp::OneOf::Left(true)),
            declaration_provider: Some(lsp::DeclarationCapability::Simple(true)),
            type_definition_provider: Some(lsp::TypeDefinitionProviderCapability::Simple(true)),
            references_provider: Some(lsp::OneOf::Left(true)),
            document_symbol_provider: Some(lsp::OneOf::Left(true)),
            workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
            document_highlight_provider: Some(lsp::OneOf::Left(true)),
            completion_provider: Some(lsp::CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                resolve_provider: None,
                ..Default::default()
            }),
            signature_help_provider: Some(lsp::SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: Default::default(),
            }),
            semantic_tokens_provider: Some(
                lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp::SemanticTokensOptions {
                        legend: token::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            diagnostic_provider: Some(lsp::DiagnosticServerCapabilities::Options(
                lsp::DiagnosticOptions {
                    identifier: Some("destack".to_string()),
                    inter_file_dependencies: true,
                    workspace_diagnostics: true,
                    markup_message_support: None,
                    work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                },
            )),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_on_type_formatting_provider: Some(lsp::DocumentOnTypeFormattingOptions {
                first_trigger_character: ";".to_string(),
                more_trigger_character: Some(vec!["}".to_string()]),
            }),
            folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(lsp::SelectionRangeProviderCapability::Simple(true)),
            document_link_provider: Some(lsp::DocumentLinkOptions {
                resolve_provider: None,
                work_done_progress_options: Default::default(),
            }),
            rename_provider: Some(lsp::OneOf::Right(lsp::RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            code_action_provider: Some(lsp::CodeActionProviderCapability::Options(
                lsp::CodeActionOptions {
                    code_action_kinds: Some(vec![
                        lsp::CodeActionKind::QUICKFIX,
                        lsp::CodeActionKind::REFACTOR,
                        lsp::CodeActionKind::REFACTOR_EXTRACT,
                        lsp::CodeActionKind::REFACTOR_INLINE,
                    ]),
                    resolve_provider: Some(true),
                    work_done_progress_options: Default::default(),
                },
            )),
            code_lens_provider: Some(lsp::CodeLensOptions {
                resolve_provider: None,
            }),
            inlay_hint_provider: Some(lsp::OneOf::Left(true)),
            implementation_provider: Some(lsp::ImplementationProviderCapability::Simple(true)),
            call_hierarchy_provider: Some(lsp::CallHierarchyServerCapability::Simple(true)),
            type_hierarchy_provider: Some(lsp::OneOf::Left(true)),
            execute_command_provider: Some(lsp::ExecuteCommandOptions {
                commands: vec!["destack.reload".to_string(), "destack.reindex".to_string()],
                work_done_progress_options: Default::default(),
            }),
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: None,
                    did_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_create: None,
                    will_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_delete: None,
                }),
                text_document_content: None,
            }),
            ..Default::default()
        };

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        if let Err(error) = self.register_file_watchers().await {
            self.report_error("failed to register file watchers", error)
                .await;
        }
        if let Err(error) = self.refresh_configuration().await {
            self.report_error("failed to load editor configuration", error)
                .await;
        }
    }

    async fn did_change_configuration(&self, _: lsp::DidChangeConfigurationParams) {
        if let Err(error) = self.change_configuration().await {
            self.report_error("failed to apply editor configuration", error)
                .await;
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        if let Err(error) = self.open_document(params).await {
            self.report_error("failed to open editor document", error)
                .await;
        }
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        if let Err(error) = self.change_document(params).await {
            self.report_error("failed to change editor document", error)
                .await;
        }
    }

    async fn did_save(&self, params: lsp::DidSaveTextDocumentParams) {
        if let Err(error) = self.save_document(params).await {
            self.report_error("failed to save editor document", error)
                .await;
        }
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        if let Err(error) = self.close_document(params).await {
            self.report_error("failed to close editor document", error)
                .await;
        }
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        if let Err(error) = self.change_workspace_folders(params) {
            self.report_error("failed to change workspace folders", error)
                .await;
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        if let Err(error) = self.change_watched_files(params).await {
            self.report_error("failed to reload watched files", error)
                .await;
        }
    }

    async fn will_rename_files(
        &self,
        params: lsp::RenameFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // collect rename targets from uris
        let mut renames = Vec::new();
        let mut query_state: Option<(PathBuf, Revision)> = None;
        let workspace = self.workspace()?;
        for file in params.files {
            let old_uri = file
                .old_uri
                .parse::<lsp::Uri>()
                .map_err(|_| jsonrpc::Error::invalid_params("invalid old file URI"))?;
            let new_uri = file
                .new_uri
                .parse::<lsp::Uri>()
                .map_err(|_| jsonrpc::Error::invalid_params("invalid new file URI"))?;
            let old_path = old_uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("old URI is not a file URI"))?;
            let new_path = new_uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("new URI is not a file URI"))?;

            // enforce a single root for rename write consistency
            let root = workspace.root(&old_path).map_err(workspace_error)?;
            let new_root = workspace.root(&new_path).map_err(workspace_error)?;
            if root != new_root {
                return Err(jsonrpc::Error::invalid_params(
                    "file rename crosses workspace roots",
                ));
            }

            if let Some((existing_root, _)) = query_state.as_ref() {
                if existing_root != &root {
                    return Err(jsonrpc::Error::invalid_params(
                        "file renames span workspace roots",
                    ));
                }
            } else {
                let revision = workspace.revision(&root).map_err(workspace_error)?;
                query_state = Some((root, revision));
            }

            renames.push(query::FileRename { old_path, new_path });
        }
        if renames.is_empty() {
            return Ok(None);
        }
        let (query_root, query_revision) =
            query_state.ok_or_else(|| internal_error("file rename has no workspace state"))?;

        // build workspace edits for import specifiers
        let request = query::QueryRequest::RenameFiles(query::RenameFilesRequest { renames });
        let response = self.query_program(
            &query_root,
            request,
            RevisionPolicy::Current(query_revision),
        )?;
        let revision = response.revision;
        let query::QueryResponse::RenameFiles(response) = response.response else {
            return Err(internal_error("query did not return rename files"));
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };
        if edit.is_empty() {
            return Ok(None);
        }

        let file_ids = edit.files.iter().map(|file| file.file);
        let files = self.source_files(&query_root, revision, file_ids)?;
        let workspace_edit = edit::workspace(&edit, &files)?;

        Ok(Some(workspace_edit))
    }

    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        for file in params.files {
            let result = self
                .clear_renamed_file_diagnostics(&file.old_uri, &file.new_uri)
                .await;
            if let Err(error) = result {
                self.report_error("failed to clear renamed file diagnostics", error)
                    .await;
            }
        }
    }

    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        for file in params.files {
            let result = self.clear_deleted_file_diagnostics(&file.uri).await;
            if let Err(error) = result {
                self.report_error("failed to clear deleted file diagnostics", error)
                    .await;
            }
        }
    }

    // ------------------------------------------------------------------------
    // DIAGNOSTICS
    // ------------------------------------------------------------------------

    async fn diagnostic(
        &self,
        params: lsp::DocumentDiagnosticParams,
    ) -> jsonrpc::Result<lsp::DocumentDiagnosticReportResult> {
        let tracked_path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned());
        let Some(path) = tracked_path.as_ref() else {
            let report =
                lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                        result_id: None,
                        items: Vec::new(),
                    },
                });
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        };

        let mut diagnostics = self
            .workspace()?
            .diagnose(DiagnosticsRequest::File(path.clone()))
            .map_err(workspace_error)?;
        if diagnostics.len() > 1 {
            return Err(internal_error(format!(
                "workspace returned {} diagnostic files for one path",
                diagnostics.len()
            )));
        }
        let file_diagnostics = diagnostics.pop();
        let Some(file_diagnostics) = file_diagnostics else {
            let report =
                lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                        result_id: None,
                        items: Vec::new(),
                    },
                });
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        };
        let files = self.diagnostic_files(&file_diagnostics)?;
        let diagnostics = file_diagnostics.diagnostics;
        let result_id = diagnostic::result_id(&diagnostics);

        if params.previous_result_id.as_ref() == Some(&result_id) {
            let report = lsp::DocumentDiagnosticReport::Unchanged(
                lsp::RelatedUnchangedDocumentDiagnosticReport {
                    related_documents: None,
                    unchanged_document_diagnostic_report: lsp::UnchangedDocumentDiagnosticReport {
                        result_id,
                    },
                },
            );
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        }

        let items = diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic::item(&diagnostic, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        let report =
            lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                    result_id: Some(result_id),
                    items,
                },
            });

        Ok(lsp::DocumentDiagnosticReportResult::Report(report))
    }

    async fn workspace_diagnostic(
        &self,
        params: lsp::WorkspaceDiagnosticParams,
    ) -> jsonrpc::Result<lsp::WorkspaceDiagnosticReportResult> {
        let previous_ids: std::collections::HashMap<String, String> = params
            .previous_result_ids
            .into_iter()
            .map(|entry| (entry.uri.to_string(), entry.value))
            .collect();

        let diagnostics = self
            .workspace()?
            .diagnose(DiagnosticsRequest::All)
            .map_err(workspace_error)?;

        let mut items = Vec::new();
        for file_diagnostics in diagnostics {
            let uri = uri::source(&file_diagnostics.uri).ok_or_else(|| {
                internal_error(format!(
                    "diagnostic URI is not representable by LSP: {}",
                    file_diagnostics.uri
                ))
            })?;
            let files = self.diagnostic_files(&file_diagnostics)?;
            let diagnostics = file_diagnostics.diagnostics;
            let result_id = diagnostic::result_id(&diagnostics);
            let version = file_diagnostics.version.map(|version| version as i64);
            let uri_str = uri.to_string();
            let report = if previous_ids.get(&uri_str) == Some(&result_id) {
                lsp::WorkspaceDocumentDiagnosticReport::Unchanged(
                    lsp::WorkspaceUnchangedDocumentDiagnosticReport {
                        uri,
                        version,
                        unchanged_document_diagnostic_report:
                            lsp::UnchangedDocumentDiagnosticReport { result_id },
                    },
                )
            } else {
                let lsp_diagnostics = diagnostics
                    .into_iter()
                    .map(|diagnostic| diagnostic::item(&diagnostic, &files))
                    .collect::<jsonrpc::Result<Vec<_>>>()?;

                lsp::WorkspaceDocumentDiagnosticReport::Full(
                    lsp::WorkspaceFullDocumentDiagnosticReport {
                        uri,
                        version,
                        full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                            result_id: Some(result_id),
                            items: lsp_diagnostics,
                        },
                    },
                )
            };
            items.push(report);
        }

        Ok(lsp::WorkspaceDiagnosticReportResult::Report(
            lsp::WorkspaceDiagnosticReport { items },
        ))
    }

    // ------------------------------------------------------------------------
    // COMMANDS
    // ------------------------------------------------------------------------

    async fn execute_command(
        &self,
        params: lsp::ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<lsp::LSPAny>> {
        match params.command.as_str() {
            "destack.reload" | "destack.reindex" => {
                let diagnostics = self.reload_diagnostics()?;
                self.publish_diagnostics(diagnostics).await?;

                Ok(None)
            }
            command => Err(jsonrpc::Error::invalid_params(format!(
                "unsupported command: {command}"
            ))),
        }
    }

    // ------------------------------------------------------------------------
    // NAVIGATION
    // ------------------------------------------------------------------------

    async fn goto_definition(
        &self,
        params: lsp::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoDefinition(query::GotoDefinitionRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::GotoDefinition(response) = response.response else {
            return Err(internal_error("query did not return goto definition"));
        };

        // convert every exact target
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let links = navigation::links(&response.targets, &files)?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoDeclaration(query::GotoDeclarationRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::GotoDeclaration(response) = response.response else {
            return Err(internal_error("query did not return goto declaration"));
        };

        // convert every exact target
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let links = navigation::links(&response.targets, &files)?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoTypeDefinition(query::GotoTypeDefinitionRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::GotoTypeDefinition(response) = response.response else {
            return Err(internal_error("query did not return goto type definition"));
        };

        // convert every exact target
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let links = navigation::links(&response.targets, &files)?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset = position::offset(&source_file, &params.text_document_position.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::FindReferences(query::FindReferencesRequest {
            position,
            include_declaration: params.context.include_declaration,
        });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::FindReferences(response) = response.response else {
            return Err(internal_error("query did not return references"));
        };
        let references = response.references;
        if references.is_empty() {
            return Ok(None);
        }

        // convert to LSP locations
        let file_ids = references
            .iter()
            .map(|reference| reference.target.span.file);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let locations = references
            .iter()
            .map(|reference| position::location(reference.target.span, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(locations))
    }

    async fn document_symbol(
        &self,
        params: lsp::DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::DocumentSymbolResponse>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let request = query::QueryRequest::Outline(query::OutlineRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::Outline(response) = response.response else {
            return Err(internal_error("query did not return outline"));
        };
        let symbols = response.symbols;
        if symbols.is_empty() {
            return Ok(None);
        }

        // convert to LSP symbols
        let lsp_symbols = symbols
            .iter()
            .map(|symbol| navigation::outline(&source_file, symbol))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
    }

    async fn symbol(
        &self,
        params: lsp::WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>>
    {
        // query every workspace root independently
        let roots = self.workspace()?.roots();
        if roots.is_empty() {
            return Ok(None);
        }
        let mut symbols = Vec::new();
        for root in roots {
            let request = query::QueryRequest::SearchSymbols(query::SearchSymbolsRequest {
                query: params.query.clone(),
                max_results: 100,
            });
            let response = self.query_program(&root, request, RevisionPolicy::Latest)?;
            let revision = response.revision;
            let query::QueryResponse::SearchSymbols(response) = response.response else {
                return Err(internal_error("query did not return symbol search"));
            };

            // retain the root and revision required to project each result
            for symbol in response.symbols {
                let Some(order) = symbol.order(&params.query) else {
                    return Err(internal_error(format!(
                        "symbol search returned nonmatching symbol {}",
                        symbol.name
                    )));
                };
                symbols.push((order, root.clone(), revision, symbol));
            }
        }

        // apply one stable order and result bound across all roots
        symbols.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        symbols.truncate(100);

        // convert to LSP
        let mut lsp_symbols = Vec::new();
        let mut file_ids_by_root: HashMap<PathBuf, (Revision, Vec<FileId>)> = HashMap::new();
        for (_, root, revision, symbol) in symbols.iter() {
            let entry = file_ids_by_root
                .entry(root.clone())
                .or_insert_with(|| (*revision, Vec::new()));
            if entry.0 != *revision {
                return Err(internal_error(format!(
                    "symbol search mixed revisions for root {}",
                    root.display()
                )));
            }
            entry.1.push(symbol.target.span.file);
        }
        let mut files_by_root = HashMap::with_capacity(file_ids_by_root.len());
        for (root, (revision, file_ids)) in file_ids_by_root {
            let files = self.source_files(&root, revision, file_ids)?;
            files_by_root.insert(root, files);
        }

        for (_, root, _, symbol) in symbols.iter() {
            let Some(files) = files_by_root.get(root) else {
                return Err(internal_error(format!(
                    "symbol search file map is missing root {}",
                    root.display()
                )));
            };
            let lsp_symbol = navigation::search_symbol(symbol, files)?;
            lsp_symbols.push(lsp_symbol);
        }

        if lsp_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp::OneOf::Left(lsp_symbols)))
        }
    }

    async fn document_highlight(
        &self,
        params: lsp::DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentHighlight>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Highlight(query::HighlightRequest { position });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::Highlight(response) = response.response else {
            return Err(internal_error("query did not return highlights"));
        };
        let highlights = response.highlights;
        if highlights.is_empty() {
            return Ok(None);
        }

        // convert to LSP highlights
        let lsp_highlights = highlights
            .iter()
            .map(|highlight| navigation::highlight(&source_file, highlight))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_highlights))
    }

    // ------------------------------------------------------------------------
    // ASSIST
    // ------------------------------------------------------------------------

    async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Hover(query::HoverRequest { position });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::Hover(response) = response.response else {
            return Err(internal_error("query did not return hover"));
        };
        let Some(hover_info) = response.hover else {
            return Ok(None);
        };

        let range = position::range(&source_file, hover_info.range)?;

        Ok(Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: hover_info.to_markdown(),
            }),
            range: Some(range),
        }))
    }

    async fn completion(
        &self,
        params: lsp::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp::CompletionResponse>> {
        // map the LSP trigger kind to completion behavior
        let trigger = match params.context.as_ref().map(|ctx| ctx.trigger_kind) {
            Some(kind) if kind == lsp::CompletionTriggerKind::TRIGGER_CHARACTER => {
                let character = params
                    .context
                    .as_ref()
                    .and_then(|context| context.trigger_character.as_deref())
                    .ok_or_else(|| {
                        jsonrpc::Error::invalid_params("completion trigger character is missing")
                    })?;
                let mut characters = character.chars();
                let character = characters.next().ok_or_else(|| {
                    jsonrpc::Error::invalid_params("completion trigger character is empty")
                })?;
                if characters.next().is_some() {
                    return Err(jsonrpc::Error::invalid_params(
                        "completion trigger contains multiple characters",
                    ));
                }

                query::CompletionTrigger::Character(character)
            }
            Some(kind)
                if kind == lsp::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS =>
            {
                query::CompletionTrigger::Incomplete
            }
            None | Some(lsp::CompletionTriggerKind::INVOKED) => query::CompletionTrigger::Invoked,
            Some(_) => {
                return Err(jsonrpc::Error::invalid_params(
                    "completion trigger kind is unsupported",
                ));
            }
        };

        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset = position::offset(&source_file, &params.text_document_position.position)?;
        let position = query_file.position(offset);
        let include_auto_imports = self
            .settings
            .completion_auto_imports
            .load(Ordering::Relaxed);
        let request = query::QueryRequest::Completion(query::CompletionRequest {
            position,
            trigger,
            include_auto_imports,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::Completion(response) = response.response else {
            return Err(internal_error("query did not return completion"));
        };
        let is_incomplete = response.is_incomplete;
        let completions = response.items;
        if completions.is_empty() {
            return Ok(None);
        }

        // convert to LSP completion items
        let completion_label_details = self.client_capabilities()?.completion_label_details;
        let items: Vec<lsp::CompletionItem> = completions
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let insert_text_format = if item.edit.is_snippet {
                    Some(lsp::InsertTextFormat::SNIPPET)
                } else {
                    None
                };
                let insert_text_mode = if item.edit.is_snippet {
                    Some(lsp::InsertTextMode::ADJUST_INDENTATION)
                } else {
                    None
                };
                let commit_characters =
                    Self::completion_commit_characters(item.kind, item.edit.is_snippet);

                // convert additional text edits
                let additional_text_edits = if item.additional_edits.is_empty() {
                    None
                } else {
                    let edits = item
                        .additional_edits
                        .iter()
                        .map(|edit| {
                            if edit.span.file != query_file.file.id {
                                return Err(internal_error(format!(
                                    "completion edit targets another file: {:?}",
                                    edit.span
                                )));
                            }

                            Ok(lsp::TextEdit {
                                range: position::range(&source_file, edit.span)?,
                                new_text: edit.new_text.clone(),
                            })
                        })
                        .collect::<jsonrpc::Result<_>>()?;

                    Some(edits)
                };

                // preserve the query response order in clients that sort completion items
                let sort_text = Some(format!("{index:020}"));
                if item.edit.span.file != query_file.file.id {
                    return Err(internal_error(format!(
                        "completion edit targets another file: {:?}",
                        item.edit.span
                    )));
                }
                let text_edit = lsp::TextEdit {
                    range: position::range(&source_file, item.edit.span)?,
                    new_text: item.edit.new_text,
                };

                // move auto import sources into label details when supported
                let (detail, label_details) = if item.is_auto_import
                    && completion_label_details
                    && let Some(detail_text) = item.detail.as_ref()
                    && let Some(path) = detail_text.strip_prefix(AUTO_IMPORT_DETAIL_PREFIX)
                {
                    (
                        None,
                        Some(lsp::CompletionItemLabelDetails {
                            detail: None,
                            description: Some(path.to_string()),
                        }),
                    )
                } else {
                    (item.detail.clone(), None)
                };

                // transcribe documentation directly because the query already produced it
                let documentation = item.documentation.map(|documentation| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: documentation,
                    })
                });
                let (deprecated, tags) = if item.is_deprecated {
                    (Some(true), Some(vec![lsp::CompletionItemTag::DEPRECATED]))
                } else {
                    (None, None)
                };

                Ok(lsp::CompletionItem {
                    label: item.label,
                    label_details,
                    kind: Some(completion::kind(item.kind)),
                    detail,
                    documentation,
                    insert_text: None,
                    insert_text_format,
                    insert_text_mode,
                    sort_text,
                    preselect: if item.preselect { Some(true) } else { None },
                    deprecated,
                    tags,
                    commit_characters,
                    additional_text_edits,
                    data: None,
                    text_edit: Some(text_edit.into()),
                    ..Default::default()
                })
            })
            .collect::<jsonrpc::Result<_>>()?;

        Ok(Some(lsp::CompletionResponse::List(lsp::CompletionList {
            is_incomplete,
            items,
            ..Default::default()
        })))
    }

    async fn signature_help(
        &self,
        params: lsp::SignatureHelpParams,
    ) -> jsonrpc::Result<Option<lsp::SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::SignatureHelp(query::SignatureHelpRequest { position });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::SignatureHelp(response) = response.response else {
            return Err(internal_error("query did not return signature help"));
        };
        let Some(help) = response.help else {
            return Ok(None);
        };

        // convert to LSP signature help
        let signatures: Vec<lsp::SignatureInformation> = help
            .signatures
            .into_iter()
            .map(|signature| lsp::SignatureInformation {
                label: signature.label,
                documentation: signature.documentation.map(|documentation| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: documentation,
                    })
                }),
                parameters: Some(
                    signature
                        .parameters
                        .into_iter()
                        .map(|parameter| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::Simple(parameter.label),
                            documentation: parameter.documentation.map(|documentation| {
                                lsp::Documentation::MarkupContent(lsp::MarkupContent {
                                    kind: lsp::MarkupKind::Markdown,
                                    value: documentation,
                                })
                            }),
                        })
                        .collect(),
                ),
                active_parameter: None,
            })
            .collect();

        Ok(Some(lsp::SignatureHelp {
            signatures,
            active_signature: Some(help.active_signature as u32),
            active_parameter: help.active_parameter.map(|parameter| parameter as u32),
        }))
    }

    // ------------------------------------------------------------------------
    // SEMANTIC TOKENS
    // ------------------------------------------------------------------------

    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let request = query::QueryRequest::SemanticTokens(query::SemanticTokensRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::SemanticTokens(response) = response.response else {
            return Err(internal_error("query did not return semantic tokens"));
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = token::encode(&source_file, &tokens)?;

        Ok(Some(lsp::SemanticTokensResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: lsp_tokens,
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let start = position::offset(&source_file, &params.range.start)?;
        let end = position::offset(&source_file, &params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let request =
            query::QueryRequest::SemanticTokensRange(query::SemanticTokensRangeRequest { range });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::SemanticTokensRange(response) = response.response else {
            return Err(internal_error("query did not return range semantic tokens"));
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = token::encode(&source_file, &tokens)?;

        Ok(Some(lsp::SemanticTokensRangeResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: lsp_tokens,
            },
        )))
    }

    // ------------------------------------------------------------------------
    // FOLDING
    // ------------------------------------------------------------------------

    async fn folding_range(
        &self,
        params: lsp::FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::FoldingRange>>> {
        // resolve file
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        if !self
            .workspace()?
            .is_file_open(&path)
            .map_err(workspace_error)?
        {
            return Ok(None);
        }

        // query for folding ranges
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let request = query::QueryRequest::FoldingRanges(query::FoldingRangesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::FoldingRanges(response) = response.response else {
            return Err(internal_error("query did not return folding ranges"));
        };
        let ranges = response.ranges;
        if ranges.is_empty() {
            return Ok(None);
        }

        // convert to LSP folding ranges
        let lsp_ranges = ranges
            .into_iter()
            .map(|range| lsp::FoldingRange {
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
                kind: range.kind.map(|kind| match kind {
                    query::FoldingRangeKind::Comment => lsp::FoldingRangeKind::Comment,
                    query::FoldingRangeKind::Imports => lsp::FoldingRangeKind::Imports,
                    query::FoldingRangeKind::Region => lsp::FoldingRangeKind::Region,
                }),
                collapsed_text: range.collapsed_text,
            })
            .collect();

        Ok(Some(lsp_ranges))
    }

    // ------------------------------------------------------------------------
    // FORMATTING
    // ------------------------------------------------------------------------

    async fn formatting(
        &self,
        params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let Some(edit) = self.format_file(&params.text_document.uri, None)? else {
            return Ok(None);
        };
        let edit = format::edit(edit)?;

        Ok(Some(vec![edit]))
    }

    async fn range_formatting(
        &self,
        params: lsp::DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let range = source::text_range(params.range);
        let Some(edit) = self.format_file(&params.text_document.uri, Some(range))? else {
            return Ok(None);
        };
        let edit = format::edit(edit)?;

        Ok(Some(vec![edit]))
    }

    async fn on_type_formatting(
        &self,
        params: lsp::DocumentOnTypeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // build the trigger range in UTF-16 editor coordinates
        let end = params.text_document_position.position;
        let trigger_width = params.ch.encode_utf16().count() as u32;
        if trigger_width == 0 {
            return Err(jsonrpc::Error::invalid_params(
                "formatting trigger is empty",
            ));
        }
        let Some(start_character) = end.character.checked_sub(trigger_width) else {
            return Ok(None);
        };
        let start = lsp::Position {
            line: end.line,
            character: start_character,
        };
        let range = source::text_range(lsp::Range { start, end });

        // format against one exact workspace file
        let uri = &params.text_document_position.text_document.uri;
        let Some(edit) = self.format_file(uri, Some(range))? else {
            return Ok(None);
        };

        // require the exact workspace source to contain the reported trigger
        let start_offset = position::offset(&edit.file, &start)?;
        let end_offset = position::offset(&edit.file, &end)?;
        if edit
            .file
            .text()
            .get(start_offset as usize..end_offset as usize)
            != Some(params.ch.as_str())
        {
            return Ok(None);
        }

        // return the selected formatting edit
        let edit = format::edit(edit)?;

        Ok(Some(vec![edit]))
    }

    // ------------------------------------------------------------------------
    // SELECTION & NAVIGATION
    // ------------------------------------------------------------------------

    async fn selection_range(
        &self,
        params: lsp::SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::SelectionRange>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let positions: Vec<u32> = params
            .positions
            .iter()
            .map(|position| position::offset(&source_file, position))
            .collect::<jsonrpc::Result<_>>()?;
        let request = query::QueryRequest::SelectionRanges(query::SelectionRangesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
            offsets: positions,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::SelectionRanges(response) = response.response else {
            return Err(internal_error("query did not return selection ranges"));
        };
        let ranges = response.ranges;

        // convert to LSP
        let lsp_ranges = ranges
            .into_iter()
            .map(|range| navigation::selection(&source_file, range))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_ranges))
    }

    async fn goto_implementation(
        &self,
        params: lsp::request::GotoImplementationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoImplementationResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoImplementation(query::GotoImplementationRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::GotoImplementation(response) = response.response else {
            return Err(internal_error("query did not return goto implementation"));
        };

        // convert every exact target
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let links = navigation::links(&response.targets, &files)?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    // ------------------------------------------------------------------------
    // DOCUMENT LINKS
    // ------------------------------------------------------------------------

    async fn document_link(
        &self,
        params: lsp::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentLink>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let request = query::QueryRequest::Links(query::LinksRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::Links(response) = response.response else {
            return Err(internal_error("query did not return links"));
        };
        let links = response.links;
        if links.is_empty() {
            return Ok(None);
        }

        // convert to LSP
        let lsp_links = links
            .iter()
            .map(|link| navigation::document_link(&source_file, link))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_links))
    }

    // ------------------------------------------------------------------------
    // CODE ACTIONS
    // ------------------------------------------------------------------------

    async fn code_action(
        &self,
        params: lsp::CodeActionParams,
    ) -> jsonrpc::Result<Option<lsp::CodeActionResponse>> {
        let has_only_filter = params
            .context
            .only
            .as_ref()
            .is_some_and(|kinds| !kinds.is_empty());
        let context = Self::query_code_action_context(&params.context);

        // skip when lsp only filters were provided but none map to workspace kinds
        if has_only_filter && context.only.is_empty() {
            return Ok(None);
        }

        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let start = position::offset(&source_file, &params.range.start)?;
        let end = position::offset(&source_file, &params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let request =
            query::QueryRequest::CodeActions(query::CodeActionsRequest { range, context });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::CodeActions(response) = response.response else {
            return Err(internal_error("query did not return code actions"));
        };
        let mut actions = response.actions;

        // filter diagnostic linked quick fixes by requested diagnostic ids
        let diagnostic_ids: HashSet<String> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match diagnostic.code.as_ref() {
                Some(lsp::NumberOrString::String(value)) => Some(value.clone()),
                Some(lsp::NumberOrString::Number(value)) => Some(value.to_string()),
                _ => None,
            })
            .collect();
        if !diagnostic_ids.is_empty() {
            actions.retain(|action| {
                action
                    .diagnostic_id
                    .as_ref()
                    .is_none_or(|id| diagnostic_ids.contains(id))
            });
        }

        if actions.is_empty() {
            return Ok(None);
        }

        // convert to LSP
        let mut lsp_actions = Vec::new();
        let client_capabilities = self.client_capabilities()?;
        let prefer_lazy_code_action_edits =
            client_capabilities.code_action_data && client_capabilities.code_action_edit_resolve;
        let file_ids = if prefer_lazy_code_action_edits {
            Vec::new()
        } else {
            actions
                .iter()
                .flat_map(|action| action.patches.files.iter())
                .map(|file| file.file)
                .collect()
        };
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        for action in actions.iter() {
            let data = if prefer_lazy_code_action_edits {
                let data = to_value(CodeActionContinuation {
                    revision,
                    path: query_file.path.clone(),
                    patches: action.patches.clone(),
                })
                .map_err(internal_error)?;

                Some(data)
            } else {
                None
            };
            let include_edit = !prefer_lazy_code_action_edits;
            let lsp_action = diagnostic::code_action(action, include_edit, data, &files)?;
            lsp_actions.push(lsp_action);
        }

        Ok(Some(lsp_actions))
    }

    async fn code_action_resolve(
        &self,
        mut params: lsp::CodeAction,
    ) -> jsonrpc::Result<lsp::CodeAction> {
        if params.edit.is_some() {
            return Err(jsonrpc::Error::invalid_params(
                "code action is already resolved",
            ));
        }

        let Some(data) = params.data.take() else {
            return Err(jsonrpc::Error::invalid_params(
                "code action has no continuation",
            ));
        };

        let resolved = from_value::<CodeActionContinuation>(data)
            .map_err(|error| jsonrpc::Error::invalid_params(error.to_string()))?;

        let file_ids = resolved.patches.files.iter().map(|file| file.file);
        let files = self.source_files(&resolved.path, resolved.revision, file_ids)?;
        params.edit = Some(edit::workspace(&resolved.patches, &files)?);

        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE LENS
    // ------------------------------------------------------------------------

    async fn code_lens(
        &self,
        params: lsp::CodeLensParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CodeLens>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let request = query::QueryRequest::CodeLenses(query::CodeLensesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::CodeLenses(response) = response.response else {
            return Err(internal_error("query did not return code lenses"));
        };
        let lenses = response.lenses;

        // convert to LSP
        let lsp_lenses = lenses
            .iter()
            .map(|lens| assist::lens(&source_file, lens))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_lenses))
    }

    // ------------------------------------------------------------------------
    // INLAY HINTS
    // ------------------------------------------------------------------------

    async fn inlay_hint(
        &self,
        params: lsp::InlayHintParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::InlayHint>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let start = position::offset(&source_file, &params.range.start)?;
        let end = position::offset(&source_file, &params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let type_hints = self.settings.type_inlay_hints.load(Ordering::Relaxed);
        let parameter_hints = self.settings.parameter_inlay_hints.load(Ordering::Relaxed);
        let request = query::QueryRequest::InlayHints(query::InlayHintsRequest {
            range,
            type_hints,
            parameter_hints,
        });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::InlayHints(response) = response.response else {
            return Err(internal_error("query did not return inlay hints"));
        };
        let hints = response.hints;

        // convert to LSP
        let lsp_hints = hints
            .iter()
            .map(|hint| assist::hint(&source_file, hint))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_hints))
    }

    // ------------------------------------------------------------------------
    // RENAME
    // ------------------------------------------------------------------------

    async fn prepare_rename(
        &self,
        params: lsp::TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<lsp::PrepareRenameResponse>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset = position::offset(&source_file, &params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::RenameTarget(query::RenameTargetRequest { position });
        let response = self.query_module(&query_file, request)?;
        let query::QueryResponse::RenameTarget(response) = response.response else {
            return Err(internal_error("query did not return rename target"));
        };
        let Some(target) = response.target else {
            return Ok(None);
        };

        // convert to LSP
        let range = position::range(&source_file, target.target.span)?;
        Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: target.placeholder,
        }))
    }

    async fn rename(
        &self,
        params: lsp::RenameParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        let Some(query_file) =
            self.resolve_query_file(&params.text_document_position.text_document.uri)?
        else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset = position::offset(&source_file, &params.text_document_position.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Rename(query::RenameRequest {
            position,
            new_name: params.new_name.clone(),
        });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::Rename(response) = response.response else {
            return Err(internal_error("query did not return rename"));
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };

        // convert to LSP
        let file_ids = edit.files.iter().map(|file| file.file);
        let files = self.source_files(&query_file.path, revision, file_ids)?;
        let workspace_edit = edit::workspace(&edit, &files)?;

        Ok(Some(workspace_edit))
    }

    // ------------------------------------------------------------------------
    // CALL HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_call_hierarchy(
        &self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyItem>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::CallItem(query::CallItemRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::CallItem(response) = response.response else {
            return Err(internal_error("query did not return call item"));
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        // convert to LSP
        let files = self.source_files(&query_file.path, revision, [item.target.span.file])?;
        let lsp_item = navigation::call_item(&query_file.path, revision, &item, &files)?;

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // decode the exact hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::CallItem>::decode(params.item.data.as_ref())
                .map_err(jsonrpc::Error::invalid_params)?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query incoming calls
        let request = query::QueryRequest::IncomingCalls(query::IncomingCallsRequest { item });
        let response = self.query_program(&path, request, RevisionPolicy::Exact(revision))?;
        let revision = response.revision;
        let query::QueryResponse::IncomingCalls(response) = response.response else {
            return Err(internal_error("query did not return incoming calls"));
        };
        let calls = response.calls;

        // convert to LSP
        let file_ids = calls.iter().map(|call| call.from.target.span.file);
        let files = self.source_files(&path, revision, file_ids)?;
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .map(|call| navigation::incoming_call(&path, revision, call, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // decode the exact hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::CallItem>::decode(params.item.data.as_ref())
                .map_err(jsonrpc::Error::invalid_params)?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;
        let source_file_id = item.target.span.file;

        // query outgoing calls
        let request = query::QueryRequest::OutgoingCalls(query::OutgoingCallsRequest { item });
        let response = self.query_program(&path, request, RevisionPolicy::Exact(revision))?;
        let revision = response.revision;
        let query::QueryResponse::OutgoingCalls(response) = response.response else {
            return Err(internal_error("query did not return outgoing calls"));
        };
        let calls = response.calls;

        // convert to LSP
        let file_ids =
            iter::once(source_file_id).chain(calls.iter().map(|call| call.to.target.span.file));
        let files = self.source_files(&path, revision, file_ids)?;
        let source_file = files.file(source_file_id).map_err(workspace_error)?;
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .map(|call| navigation::outgoing_call(&path, revision, call, &source_file, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_calls))
    }

    // ------------------------------------------------------------------------
    // TYPE HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_type_hierarchy(
        &self,
        params: lsp::TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let source_file = query_file.file.clone();
        let offset =
            position::offset(&source_file, &params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::TypeItem(query::TypeItemRequest { position });
        let response = self.query_module(&query_file, request)?;
        let revision = response.revision;
        let query::QueryResponse::TypeItem(response) = response.response else {
            return Err(internal_error("query did not return type item"));
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        let files = self.source_files(&query_file.path, revision, [item.target.span.file])?;
        let lsp_item = navigation::type_item(&query_file.path, revision, &item, &files)?;

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // decode the exact hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::TypeItem>::decode(params.item.data.as_ref())
                .map_err(jsonrpc::Error::invalid_params)?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query supertypes
        let request = query::QueryRequest::Supertypes(query::SupertypesRequest { item });
        let response = self.query_program(&path, request, RevisionPolicy::Exact(revision))?;
        let revision = response.revision;
        let query::QueryResponse::Supertypes(response) = response.response else {
            return Err(internal_error("query did not return supertypes"));
        };
        let supertypes = response.items;

        // convert to LSP
        let file_ids = supertypes.iter().map(|item| item.target.span.file);
        let files = self.source_files(&path, revision, file_ids)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .map(|item| navigation::type_item(&path, revision, item, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // decode the exact hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::TypeItem>::decode(params.item.data.as_ref())
                .map_err(jsonrpc::Error::invalid_params)?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query subtypes
        let request = query::QueryRequest::Subtypes(query::SubtypesRequest { item });
        let response = self.query_program(&path, request, RevisionPolicy::Exact(revision))?;
        let revision = response.revision;
        let query::QueryResponse::Subtypes(response) = response.response else {
            return Err(internal_error("query did not return subtypes"));
        };
        let subtypes = response.items;

        // convert to LSP
        let file_ids = subtypes.iter().map(|item| item.target.span.file);
        let files = self.source_files(&path, revision, file_ids)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .map(|item| navigation::type_item(&path, revision, item, &files))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_items))
    }
}
