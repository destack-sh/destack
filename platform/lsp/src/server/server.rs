use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use dashmap::DashMap;
use destack_ast::NodeParentIndex;
use destack_daemon::{Daemon, DaemonMessage, DaemonMessageKind, DaemonUpdate, WatchBatch};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_lsp_server::{Client, LanguageServer, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_parser::Parser;
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{
    DiagnosticSeverity, File, FileId, FileSystem, FileWatchEvent, FileWatchEventKind, LanguageType,
    OverlayFileSystem, PhysicalFileSystem, Span, WATCHABLE_FILE_TYPES,
};
use destack_workspace::{FormatterOptions, Session, Workspace, query};
use serde_json::to_value;

use crate::query::assist::{code_lens_to_lsp, inlay_hint_to_lsp};
use crate::query::common::{byte_span_to_range, position_to_byte, span_to_location};
use crate::query::diagnostic::{code_action_to_lsp, diagnostic_to_lsp_diagnostic};
use crate::query::navigation::{
    call_hierarchy_item_symbol_id, call_hierarchy_item_to_lsp, definition_to_location,
    document_highlight_to_lsp, document_link_to_lsp, document_symbol_to_lsp,
    implementation_to_location, incoming_call_to_lsp, outgoing_call_to_lsp, selection_range_to_lsp,
    type_hierarchy_item_symbol_id, type_hierarchy_item_to_lsp, workspace_symbol_to_lsp,
};
use crate::query::refactor::batch_edit_to_workspace_edit;
use crate::query::semantic;

pub const CONFIG_GLOBS: [&str; 2] = ["**/dsconfig.json", "**/tsconfig*.json"];

/// State for an open document.
#[derive(Debug)]
struct OpenDocument {
    file_id: FileId,
}

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client connection.  
    pub(super) client: Client,
    /// The overlay file system.
    overlay_fs: Arc<OverlayFileSystem>,
    /// The session.
    session: OnceLock<Arc<Session>>,
    /// The daemon.
    daemon: OnceLock<Arc<Daemon>>,
    /// The open documents.
    open_documents: DashMap<String, OpenDocument>,
    /// The watch registration ID.
    watch_registration_id: OnceLock<String>,
}

impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        let physical_fs = Arc::new(PhysicalFileSystem::new());
        let overlay_fs = Arc::new(OverlayFileSystem::with_inner(physical_fs));
        Self {
            client,
            overlay_fs,
            session: OnceLock::new(),
            daemon: OnceLock::new(),
            open_documents: DashMap::new(),
            watch_registration_id: OnceLock::new(),
        }
    }

    /// Get the session (must be called after initialize).
    #[inline]
    fn session(&self) -> &Arc<Session> {
        self.session.get().expect("session not initialized")
    }

    /// Get the daemon (must be called after initialize).
    #[inline]
    fn daemon(&self) -> &Arc<Daemon> {
        self.daemon.get().expect("daemon not initialized")
    }

    /// Ensure the module backing a file is analyzed.
    fn ensure_analyzed_for_file(&self, file_id: FileId) {
        let session = self.session();
        let file = session.files.get(file_id);
        let Some(path) = file.path.as_ref() else {
            return;
        };

        if let Err(error) = self.daemon().analyze_path(path) {
            tracing::debug!(?error, path = ?path, "lsp.query.analyze_failed");
        }
    }

    /// Prepare a file for queries by ensuring analysis is current.
    fn get_analyzed_file(&self, file_id: FileId) -> Arc<File> {
        self.ensure_analyzed_for_file(file_id);
        self.session().files.get(file_id)
    }

    /// Register file watchers with the client.
    async fn register_file_watchers(&self) {
        if self.watch_registration_id.get().is_some() {
            return;
        }

        let watchers = build_file_watchers();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = match to_value(options) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.register.serialize_failed");
                return;
            }
        };
        let registration = lsp::Registration {
            id: "destack.watch".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options,
        };

        match self.client.register_capability(vec![registration]).await {
            Ok(()) => {
                let _ = self.watch_registration_id.set("destack.watch".to_string());
            }
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.register_failed");
            }
        }
    }

    /// Collect all active watch roots.
    fn watch_roots(&self) -> Vec<PathBuf> {
        // collect roots from active programs
        let session = self.session();
        let mut roots = Vec::new();
        for entry in session.programs.iter() {
            roots.push(entry.key().clone());
        }

        // fall back to the session root when no programs exist
        if roots.is_empty() {
            roots.push(session.cwd.clone());
        }

        roots
    }

    /// Apply watch events and publish diagnostics.
    async fn apply_watch_events(&self, events: Vec<FileWatchEvent>) {
        // skip empty batches
        if events.is_empty() {
            return;
        }

        // build the watch batch
        let started_at = Instant::now();
        let mut batch = WatchBatch::new(started_at);
        batch.events = events;
        batch.ended_at = Instant::now();

        // apply updates through the daemon
        let daemon = self.daemon().clone();
        let mut result = daemon.apply_watch_batch(&batch);

        // rescan when requested
        if result.rescan {
            let roots = self.watch_roots();
            let rescan = daemon.rescan_roots_with_analysis(&roots);
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        tracing::trace!(
            updates = result.updates.len(),
            rescan = result.rescan,
            "lsp.watch.apply"
        );

        // publish diagnostics for updates
        self.publish_watch_updates(result.updates).await;
        self.publish_watch_messages(result.messages).await;
    }

    /// Publish diagnostics for a batch of daemon updates.
    async fn publish_watch_updates(&self, updates: Vec<DaemonUpdate>) {
        // publish diagnostics per updated file
        let session = self.session().clone();
        for update in updates {
            let file = session.files.get(update.file_id);
            let Some(uri) = lsp_uri_for_file(&file) else {
                continue;
            };
            let diagnostics: Vec<lsp::Diagnostic> = update
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
                .collect();
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    /// Publish watch warnings for a batch.
    async fn publish_watch_messages(&self, messages: Vec<DaemonMessage>) {
        for message in messages {
            let message_type = match message.kind() {
                DaemonMessageKind::Info => lsp::MessageType::INFO,
                DaemonMessageKind::Warning => lsp::MessageType::WARNING,
                DaemonMessageKind::Error => lsp::MessageType::ERROR,
            };
            self.client
                .log_message(message_type, message.render())
                .await;
        }
    }

    /// Invalidate a module at the given path and publish diagnostics.
    async fn invalidate_and_publish(
        &self,
        uri: &lsp::Uri,
        path: &std::path::Path,
        content: String,
    ) {
        // apply the virtual file update through the daemon
        let updates = match self.daemon().update_virtual_file(path, content) {
            Ok(updates) => updates,
            Err(error) => {
                tracing::debug!(?error, "lsp.invalidate.file");
                return;
            }
        };

        // publish diagnostics for the updated files
        let session = self.session().clone();
        let primary_file_id = updates.first().map(|update| update.file_id);
        for update in updates {
            let file = session.files.get(update.file_id);
            let uri = if Some(update.file_id) == primary_file_id {
                Some(uri.clone())
            } else {
                lsp_uri_for_file(&file)
            };
            let Some(uri) = uri else {
                continue;
            };
            let diagnostics: Vec<lsp::Diagnostic> = update
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
                .collect();
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }
}

/// Build an LSP URI for a file using its on-disk path when available.
fn lsp_uri_for_file(file: &File) -> Option<lsp::Uri> {
    // prefer a file:// URI derived from the file path
    if let Some(path) = file.path.as_ref() {
        return lsp::Uri::from_file_path(path);
    }

    // fall back to parsing the stored uri string
    file.uri.as_ref().parse::<lsp::Uri>().ok()
}

/// Build file watcher patterns for the client.
fn build_file_watchers() -> Vec<lsp::FileSystemWatcher> {
    let mut watchers = Vec::new();
    for pattern in tracked_file_globs() {
        watchers.push(lsp::FileSystemWatcher {
            glob_pattern: pattern.to_string().into(),
            kind: None,
        });
    }

    watchers
}

/// Build the set of file globs tracked by the LSP.
fn tracked_file_globs() -> Vec<&'static str> {
    let mut patterns = Vec::new();
    for file_type in WATCHABLE_FILE_TYPES {
        for pattern in file_type.globs() {
            if !patterns.contains(pattern) {
                patterns.push(pattern);
            }
        }
    }

    // append config globs
    for pattern in CONFIG_GLOBS {
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }

    patterns
}

// ----------------------------------------------------------------------------
// LIFECYCLE
// ----------------------------------------------------------------------------

impl LanguageServer for DestackLanguageServer {
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.start")
            .await;

        // determine workspace root from params
        #[allow(deprecated)]
        let cwd = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().map(|p| p.into_owned()))
            .or_else(|| params.root_path.clone().map(PathBuf::from))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // create session with overlay filesystem
        let session = Session::new(cwd.clone()).with_fs(self.overlay_fs.clone());

        // discover workspace and attach configuration
        let resolver = Resolver::from_session(&session, ResolveOptions::default());
        let workspace = resolver
            .discover_workspace(&cwd)
            .unwrap_or_else(|_| Workspace::single_package(cwd.clone()));
        let root = workspace.root.clone();
        let session = Arc::new(session.with_workspace(workspace));
        session.add_root(root);
        let _ = self.session.set(session.clone());

        // create daemon for the session
        let daemon = Arc::new(Daemon::new(session));
        let _ = self.daemon.set(daemon);

        // build file operation filters for workspace notifications
        let file_operation_filters: Vec<lsp::FileOperationFilter> = tracked_file_globs()
            .into_iter()
            .map(|glob| lsp::FileOperationFilter {
                scheme: None,
                pattern: lsp::FileOperationPattern {
                    glob: glob.to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: Some(lsp::FileOperationPatternOptions {
                        ignore_case: Some(true),
                    }),
                },
            })
            .collect();

        // declare server capabilities
        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
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
                        legend: semantic::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
            folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(lsp::SelectionRangeProviderCapability::Simple(true)),
            document_link_provider: Some(lsp::DocumentLinkOptions {
                resolve_provider: Some(true),
                work_done_progress_options: Default::default(),
            }),
            rename_provider: Some(lsp::OneOf::Right(lsp::RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            code_action_provider: Some(lsp::CodeActionProviderCapability::Simple(true)),
            code_lens_provider: Some(lsp::CodeLensOptions {
                resolve_provider: Some(true),
            }),
            inlay_hint_provider: Some(lsp::OneOf::Left(true)),
            implementation_provider: Some(lsp::ImplementationProviderCapability::Simple(true)),
            call_hierarchy_provider: Some(lsp::CallHierarchyServerCapability::Simple(true)),
            // NOTE #Incomplete: type_hierarchy_provider not in lsp-types ServerCapabilities (?)
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_create: None,
                    will_rename: None,
                    will_delete: None,
                }),
                text_document_content: None,
            }),
            ..Default::default()
        };

        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.done")
            .await;

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
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized")
            .await;

        self.register_file_watchers().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.shutdown")
            .await;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let content = params.text_document.text;

        self.client
            .log_message(lsp::MessageType::INFO, format!("did_open: {uri_str}"))
            .await;

        let session = self.session();
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // set overlay so subsequent reads use editor content
        self.overlay_fs.set_overlay(&path, content.clone());

        // invalidate and publish diagnostics
        self.invalidate_and_publish(&params.text_document.uri, &path, content)
            .await;

        // track open document after the file registry is updated
        let Some(file_id) = session.files.get_id_by_path(&path) else {
            return;
        };
        self.open_documents
            .insert(uri_str, OpenDocument { file_id });
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        // full sync: we receive entire new content
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };
        let content = change.text;
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // update overlay with new content
        self.overlay_fs.set_overlay(&path, content.clone());

        // invalidate and publish diagnostics
        self.invalidate_and_publish(&params.text_document.uri, &path, content)
            .await;
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        self.open_documents.remove(&uri_str);

        // remove overlay to fall back to disk content
        if let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        {
            self.overlay_fs.remove_overlay(&path);
        }

        // clear diagnostics for closed file
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        let session = self.session();
        let daemon = self.daemon();

        for folder in params.event.added {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                session.get_or_create_program(path);
            }
        }

        for folder in params.event.removed {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                let _ = session.remove_root(&path);
                let _ = daemon.remove_program_handle(&path);
            }
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        // apply external file changes to the daemon
        let mut events = Vec::new();
        for change in params.changes {
            // resolve file path
            let Some(path) = change.uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // skip updates for open documents
            let uri_str = change.uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // remove any stale overlay for disk changes
            self.overlay_fs.remove_overlay(&path);

            // translate change into a watch event
            let kind = match change.typ {
                lsp::FileChangeType::CREATED => FileWatchEventKind::Created,
                lsp::FileChangeType::CHANGED => FileWatchEventKind::Modified,
                lsp::FileChangeType::DELETED => FileWatchEventKind::Deleted,
                _ => FileWatchEventKind::Modified,
            };
            events.push(FileWatchEvent {
                path,
                previous_path: None,
                kind,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn did_create_files(&self, params: lsp::CreateFilesParams) {
        // apply created file updates
        let session = self.session().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse the file uri
            let Ok(uri) = file.uri.parse::<lsp::Uri>() else {
                continue;
            };

            // skip open documents
            let uri_str = uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // resolve the file path
            let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // clear overlay so we read from disk
            self.overlay_fs.remove_overlay(&path);

            // skip files that are not yet visible on disk
            if session.fs.exists(&path).ok() != Some(true) {
                continue;
            }

            // record the create event
            events.push(FileWatchEvent {
                path,
                previous_path: None,
                kind: FileWatchEventKind::Created,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        // apply renamed file updates
        let session = self.session().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse rename uris
            let Ok(old_uri) = file.old_uri.parse::<lsp::Uri>() else {
                continue;
            };
            let Ok(new_uri) = file.new_uri.parse::<lsp::Uri>() else {
                continue;
            };

            // skip open documents
            let old_uri_str = old_uri.to_string();
            let new_uri_str = new_uri.to_string();
            if self.open_documents.contains_key(&old_uri_str)
                || self.open_documents.contains_key(&new_uri_str)
            {
                continue;
            }

            // resolve paths
            let Some(old_path) = old_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let Some(new_path) = new_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // clear overlays so we re-read from disk
            self.overlay_fs.remove_overlay(&old_path);
            self.overlay_fs.remove_overlay(&new_path);

            // skip files that are not yet visible on disk
            if session.fs.exists(&new_path).ok() != Some(true) {
                continue;
            }

            // record the rename event
            events.push(FileWatchEvent {
                path: new_path,
                previous_path: Some(old_path),
                kind: FileWatchEventKind::Renamed,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        // clear diagnostics for deleted files
        let mut events = Vec::new();
        for file in params.files {
            // parse the file uri
            let Ok(uri) = file.uri.parse::<lsp::Uri>() else {
                continue;
            };
            let uri_str = uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // resolve the file path
            if let Some(path) = uri.to_file_path().map(|path| path.into_owned()) {
                // clear overlay so we read from disk
                self.overlay_fs.remove_overlay(&path);

                // record the delete event
                events.push(FileWatchEvent {
                    path,
                    previous_path: None,
                    kind: FileWatchEventKind::Deleted,
                });
            }
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    // ------------------------------------------------------------------------
    // NAVIGATION
    // ------------------------------------------------------------------------

    async fn goto_definition(
        &self,
        params: lsp::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for definition
        let Some(result) = query::goto_definition(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP location
        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for declaration (same as definition for now)
        let Some(result) = query::goto_declaration(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for type definition
        let Some(result) = query::goto_type_definition(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        // resolve file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query for references (include declaration in results)
        let Some(refs) = query::find_references(session, doc.file_id, offset, true) else {
            return Ok(None);
        };
        if refs.is_empty() {
            return Ok(None);
        }

        // convert to LSP locations
        let locations: Vec<lsp::Location> = refs
            .references
            .iter()
            .filter_map(|span| span_to_location(session, *span))
            .collect();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn document_symbol(
        &self,
        params: lsp::DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::DocumentSymbolResponse>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for document symbols
        let symbols = query::document_symbols(session, doc.file_id);
        if symbols.is_empty() {
            return Ok(None);
        }

        // convert to LSP symbols
        let file = self.get_analyzed_file(doc.file_id);
        let lsp_symbols: Vec<lsp::DocumentSymbol> = symbols
            .iter()
            .filter_map(|s| document_symbol_to_lsp(&file, s))
            .collect();

        if lsp_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
        }
    }

    async fn symbol(
        &self,
        params: lsp::WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>>
    {
        let session = self.session();

        // query workspace symbols
        let symbols = query::workspace_symbols(session, &params.query, 100);

        // convert to LSP
        let lsp_symbols: Vec<lsp::SymbolInformation> = symbols
            .iter()
            .filter_map(|s| workspace_symbol_to_lsp(session, s))
            .collect();

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
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for highlights
        let highlights = query::document_highlight(session, doc.file_id, offset);
        if highlights.is_empty() {
            return Ok(None);
        }

        // convert to LSP highlights
        let lsp_highlights: Vec<lsp::DocumentHighlight> = highlights
            .iter()
            .filter_map(|h| document_highlight_to_lsp(&file, h))
            .collect();

        if lsp_highlights.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp_highlights))
        }
    }

    // ------------------------------------------------------------------------
    // ASSIST
    // ------------------------------------------------------------------------

    async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for hover info
        let Some(hover_info) = query::hover(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let range = hover_info.range.map(|span| byte_span_to_range(&file, span));

        Ok(Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: hover_info.to_markdown(),
            }),
            range,
        }))
    }

    async fn completion(
        &self,
        params: lsp::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp::CompletionResponse>> {
        // resolve file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query for completions
        let trigger = query::CompletionTrigger::Invoked;
        let completions = query::completions(session, doc.file_id, offset, trigger);
        if completions.is_empty() {
            return Ok(None);
        }

        // convert to LSP completion items
        let items: Vec<lsp::CompletionItem> = completions
            .into_iter()
            .map(|c| lsp::CompletionItem {
                label: c.label,
                kind: Some(completion_kind_to_lsp(c.kind)),
                detail: c.detail,
                documentation: c.documentation.map(|d| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: d,
                    })
                }),
                insert_text: c.insert_text,
                ..Default::default()
            })
            .collect();

        Ok(Some(lsp::CompletionResponse::Array(items)))
    }

    async fn signature_help(
        &self,
        params: lsp::SignatureHelpParams,
    ) -> jsonrpc::Result<Option<lsp::SignatureHelp>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for signature help
        let Some(help) = query::signature_help(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP signature help
        let signatures: Vec<lsp::SignatureInformation> = help
            .signatures
            .into_iter()
            .map(|s| lsp::SignatureInformation {
                label: s.label,
                documentation: s.documentation.map(|d| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: d,
                    })
                }),
                parameters: Some(
                    s.parameters
                        .into_iter()
                        .map(|p| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::Simple(p.label),
                            documentation: p.documentation.map(|d| {
                                lsp::Documentation::MarkupContent(lsp::MarkupContent {
                                    kind: lsp::MarkupKind::Markdown,
                                    value: d,
                                })
                            }),
                        })
                        .collect(),
                ),
                // note: active_parameter is on SignatureHelp, not per-signature
                active_parameter: None,
            })
            .collect();

        Ok(Some(lsp::SignatureHelp {
            signatures,
            active_signature: Some(help.active_signature as u32),
            active_parameter: Some(help.active_parameter as u32),
        }))
    }

    // ------------------------------------------------------------------------
    // SEMANTIC TOKENS
    // ------------------------------------------------------------------------

    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // query semantic tokens
        let tokens = query::semantic_tokens(session, doc.file_id);

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&file, &tokens);

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
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        let span = Span::new(doc.file_id, start, end);

        // query semantic tokens for range
        let tokens = query::semantic_tokens_range(session, doc.file_id, span);

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&file, &tokens);

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
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for folding ranges
        let ranges = query::folding_ranges(session, doc.file_id);
        if ranges.is_empty() {
            return Ok(None);
        }

        // convert to LSP folding ranges
        let lsp_ranges: Vec<lsp::FoldingRange> = ranges
            .into_iter()
            .map(|r| lsp::FoldingRange {
                start_line: r.start_line,
                start_character: r.start_character,
                end_line: r.end_line,
                end_character: r.end_character,
                kind: r.kind.map(|k| match k {
                    query::FoldingRangeKind::Comment => lsp::FoldingRangeKind::Comment,
                    query::FoldingRangeKind::Imports => lsp::FoldingRangeKind::Imports,
                    query::FoldingRangeKind::Region => lsp::FoldingRangeKind::Region,
                }),
                collapsed_text: r.collapsed_text,
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
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // get formatter options from program (respects dsconfig.json)
        let formatter = file
            .path
            .as_ref()
            .map(|p| session.find_program_for_path(p).formatter)
            .unwrap_or_default();

        // format the file
        let Some(formatted) = format_file(session, doc.file_id, &file, formatter) else {
            return Ok(None);
        };

        // return single edit replacing entire document
        let line_count = file.line_count();
        let last_line_len = file
            .get_line_str(line_count.saturating_sub(1))
            .map(|l| l.len())
            .unwrap_or(0);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp::Range {
                start: lsp::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp::Position {
                    line: line_count,
                    character: last_line_len as u32,
                },
            },
            new_text: formatted,
        }]))
    }

    async fn range_formatting(
        &self,
        params: lsp::DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // get formatter options from program
        let formatter = file
            .path
            .as_ref()
            .map(|p| session.find_program_for_path(p).formatter)
            .unwrap_or_default();

        // convert range to byte offsets
        let Some(start_offset) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end_offset) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };

        // format range
        let Some((formatted, edit_range)) =
            format_range(&file, formatter, start_offset, end_offset)
        else {
            return Ok(None);
        };

        // convert byte range back to LSP range
        let lsp_range = byte_span_to_range(&file, edit_range);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp_range,
            new_text: formatted,
        }]))
    }

    // ------------------------------------------------------------------------
    // SELECTION & NAVIGATION
    // ------------------------------------------------------------------------

    async fn selection_range(
        &self,
        params: lsp::SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::SelectionRange>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // convert positions to byte offsets
        let positions: Vec<u32> = params
            .positions
            .iter()
            .filter_map(|p| position_to_byte(&file, p))
            .collect();

        // query selection ranges
        let ranges = query::selection_ranges(session, doc.file_id, &positions);

        // convert to LSP
        let lsp_ranges: Vec<lsp::SelectionRange> = ranges
            .into_iter()
            .map(|r| selection_range_to_lsp(&file, r))
            .collect();

        Ok(Some(lsp_ranges))
    }

    async fn goto_implementation(
        &self,
        params: lsp::request::GotoImplementationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoImplementationResponse>> {
        // look up file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query implementation
        let Some(result) = query::goto_implementation(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP
        let Some(location) = implementation_to_location(session, &result) else {
            return Ok(None);
        };

        Ok(Some(lsp::GotoDefinitionResponse::Scalar(location)))
    }

    // ------------------------------------------------------------------------
    // DOCUMENT LINKS
    // ------------------------------------------------------------------------

    async fn document_link(
        &self,
        params: lsp::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentLink>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // query document links
        let links = query::document_links(session, doc.file_id);

        // convert to LSP
        let lsp_links: Vec<lsp::DocumentLink> = links
            .iter()
            .filter_map(|link| document_link_to_lsp(&file, link))
            .collect();

        Ok(Some(lsp_links))
    }

    async fn document_link_resolve(
        &self,
        params: lsp::DocumentLink,
    ) -> jsonrpc::Result<lsp::DocumentLink> {
        // links are already resolved in document_link
        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE ACTIONS
    // ------------------------------------------------------------------------

    async fn code_action(
        &self,
        params: lsp::CodeActionParams,
    ) -> jsonrpc::Result<Option<lsp::CodeActionResponse>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        let span = Span::new(doc.file_id, start, end);

        // query code actions
        let context = query::CodeActionContext::default();
        let actions = query::code_actions(session, doc.file_id, span, &context);

        // convert to LSP
        let lsp_actions: Vec<lsp::CodeActionOrCommand> = actions
            .iter()
            .filter_map(|a| code_action_to_lsp(session, a))
            .collect();

        Ok(Some(lsp_actions))
    }

    // ------------------------------------------------------------------------
    // CODE LENS
    // ------------------------------------------------------------------------

    async fn code_lens(
        &self,
        params: lsp::CodeLensParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CodeLens>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // query code lenses
        let lenses = query::code_lenses(session, doc.file_id);

        // convert to LSP
        let lsp_lenses: Vec<lsp::CodeLens> = lenses
            .iter()
            .map(|lens| code_lens_to_lsp(&file, lens))
            .collect();

        Ok(Some(lsp_lenses))
    }

    async fn code_lens_resolve(&self, params: lsp::CodeLens) -> jsonrpc::Result<lsp::CodeLens> {
        // lenses are already resolved
        Ok(params)
    }

    // ------------------------------------------------------------------------
    // INLAY HINTS
    // ------------------------------------------------------------------------

    async fn inlay_hint(
        &self,
        params: lsp::InlayHintParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::InlayHint>>> {
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        let span = Span::new(doc.file_id, start, end);

        // query inlay hints
        let hints = query::inlay_hints(session, doc.file_id, span);

        // convert to LSP
        let lsp_hints: Vec<lsp::InlayHint> = hints
            .iter()
            .filter_map(|h| inlay_hint_to_lsp(&file, h))
            .collect();

        Ok(Some(lsp_hints))
    }

    // ------------------------------------------------------------------------
    // RENAME
    // ------------------------------------------------------------------------

    async fn prepare_rename(
        &self,
        params: lsp::TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<lsp::PrepareRenameResponse>> {
        // look up file and position
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.position) else {
            return Ok(None);
        };

        // query prepare rename
        let Some(result) = query::prepare_rename(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP
        let range = byte_span_to_range(&file, result.range);
        Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: result.placeholder,
        }))
    }

    async fn rename(
        &self,
        params: lsp::RenameParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // look up file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query rename
        let Some(result) = query::rename(session, doc.file_id, offset, &params.new_name) else {
            return Ok(None);
        };

        // convert to LSP
        let workspace_edit = batch_edit_to_workspace_edit(session, &result.edits);
        Ok(Some(workspace_edit))
    }

    // ------------------------------------------------------------------------
    // CALL HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_call_hierarchy(
        &self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyItem>>> {
        // look up file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query prepare call hierarchy
        let Some(item) = query::prepare_call_hierarchy(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP
        let Some(lsp_item) = call_hierarchy_item_to_lsp(session, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // extract symbol_id from item data
        let Some(symbol_id) = call_hierarchy_item_symbol_id(&params.item) else {
            return Ok(Some(vec![]));
        };

        // reconstruct call hierarchy item
        let session = self.session();
        let Some(item) = query::call_hierarchy_item_from_symbol(session, symbol_id) else {
            return Ok(Some(vec![]));
        };

        // query incoming calls
        let calls = query::incoming_calls(session, &item);

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .filter_map(|c| incoming_call_to_lsp(session, c))
            .collect();

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // extract symbol_id from item data
        let Some(symbol_id) = call_hierarchy_item_symbol_id(&params.item) else {
            return Ok(Some(vec![]));
        };

        // reconstruct call hierarchy item
        let session = self.session();
        let Some(item) = query::call_hierarchy_item_from_symbol(session, symbol_id) else {
            return Ok(Some(vec![]));
        };

        // query outgoing calls
        let calls = query::outgoing_calls(session, &item);

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .filter_map(|c| outgoing_call_to_lsp(session, c))
            .collect();

        Ok(Some(lsp_calls))
    }

    // ------------------------------------------------------------------------
    // TYPE HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_type_hierarchy(
        &self,
        params: lsp::TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = self.get_analyzed_file(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        let Some(item) = query::prepare_type_hierarchy(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let Some(lsp_item) = type_hierarchy_item_to_lsp(session, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract symbol_id from item data
        let Some(symbol_id) = type_hierarchy_item_symbol_id(&params.item) else {
            return Ok(Some(vec![]));
        };

        // reconstruct type hierarchy item
        let session = self.session();
        let Some(item) = query::type_hierarchy_item_from_symbol(session, symbol_id) else {
            return Ok(Some(vec![]));
        };

        // query supertypes
        let supertypes = query::supertypes(session, &item);

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(session, t))
            .collect();

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract symbol_id from item data
        let Some(symbol_id) = type_hierarchy_item_symbol_id(&params.item) else {
            return Ok(Some(vec![]));
        };

        // reconstruct type hierarchy item
        let session = self.session();
        let Some(item) = query::type_hierarchy_item_from_symbol(session, symbol_id) else {
            return Ok(Some(vec![]));
        };

        // query subtypes
        let subtypes = query::subtypes(session, &item);

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(session, t))
            .collect();

        Ok(Some(lsp_items))
    }
}

// ----------------------------------------------------------------------------
// HELPERS
// ----------------------------------------------------------------------------

/// Convert completion kind to LSP completion item kind.
fn completion_kind_to_lsp(kind: query::CompletionKind) -> lsp::CompletionItemKind {
    match kind {
        query::CompletionKind::Text => lsp::CompletionItemKind::TEXT,
        query::CompletionKind::Method => lsp::CompletionItemKind::METHOD,
        query::CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
        query::CompletionKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        query::CompletionKind::Field => lsp::CompletionItemKind::FIELD,
        query::CompletionKind::Variable => lsp::CompletionItemKind::VARIABLE,
        query::CompletionKind::Class => lsp::CompletionItemKind::CLASS,
        query::CompletionKind::Interface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionKind::Module => lsp::CompletionItemKind::MODULE,
        query::CompletionKind::Property => lsp::CompletionItemKind::PROPERTY,
        query::CompletionKind::Unit => lsp::CompletionItemKind::UNIT,
        query::CompletionKind::Value => lsp::CompletionItemKind::VALUE,
        query::CompletionKind::Enum => lsp::CompletionItemKind::ENUM,
        query::CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
        query::CompletionKind::Snippet => lsp::CompletionItemKind::SNIPPET,
        query::CompletionKind::Color => lsp::CompletionItemKind::COLOR,
        query::CompletionKind::File => lsp::CompletionItemKind::FILE,
        query::CompletionKind::Reference => lsp::CompletionItemKind::REFERENCE,
        query::CompletionKind::Folder => lsp::CompletionItemKind::FOLDER,
        query::CompletionKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        query::CompletionKind::Constant => lsp::CompletionItemKind::CONSTANT,
        query::CompletionKind::Struct => lsp::CompletionItemKind::STRUCT,
        query::CompletionKind::Event => lsp::CompletionItemKind::EVENT,
        query::CompletionKind::Operator => lsp::CompletionItemKind::OPERATOR,
        query::CompletionKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
    }
}

/// Format a file and return the formatted content.
/// Uses the module's pre-parsed AST when available, falls back to re-parsing.
fn format_file(
    session: &Session,
    file_id: FileId,
    file: &Arc<File>,
    formatter: FormatterOptions,
) -> Option<String> {
    let language_type = LanguageType::from(file.ty);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };

    // try to use module's pre-parsed AST
    if let Some(module_lock) = query::get_module_by_file_id(session, file_id) {
        let module = module_lock.read();
        if let Some(ast) = module.ast_maybe() {
            let side_span = Parser::compute_side_span_from_tree(&ast.tree);
            let strings = ast.strings.clone().into_immutable();
            let context = DestackFormatContext {
                options: format_options,
                file: file.as_ref(),
                tree: &ast.tree,
                source_map: &ast.tree.source_map,
                parents: ast.parents.clone(),
                tokens: &ast.tokens,
                side_tokens: &ast.side_tokens,
                side_span: &side_span,
                strings: &strings,
            };

            return format_expressions(&context, &ast.roots);
        }
    }

    // fallback if module doesn't have AST yet, parse file
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();

    // bail if parse errors (don't format broken code)
    if parser
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return None;
    }

    // build format context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let context = DestackFormatContext {
        options: format_options,
        file: file.as_ref(),
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
    };

    format_expressions(&context, &expressions)
}

/// Format expressions and return the result string.
fn format_expressions<T>(context: &DestackFormatContext<'_>, expressions: &[T]) -> Option<String>
where
    T: Copy,
    for<'a> T: destack_fir::format::Format<DestackFormatContext<'a>>,
{
    let mut result = String::new();
    for (i, expr) in expressions.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).ok()?;
        let printed = formatted.print().ok()?;
        result.push_str(printed.as_str());
        if i < expressions.len() - 1 {
            result.push('\n');
        }
    }

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Some(result)
}

/// Format a range within a file and return the formatted content with the actual range.
fn format_range(
    file: &Arc<File>,
    formatter: FormatterOptions,
    start_offset: u32,
    end_offset: u32,
) -> Option<(String, Span)> {
    // parse file
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();

    // bail if parse errors
    if parser
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return None;
    }

    // find expressions that overlap with the range
    let overlapping: Vec<_> = expressions
        .iter()
        .filter(|expr_id| {
            let span = parser.tree.get_span(**expr_id);
            span.start < end_offset && span.end > start_offset
        })
        .copied()
        .collect();

    if overlapping.is_empty() {
        return None;
    }

    // compute the actual range we're formatting (union of overlapping expressions)
    let first_span = parser.tree.get_span(overlapping[0]);
    let last_span = parser.tree.get_span(*overlapping.last().unwrap());
    let actual_range = Span::new(file.id, first_span.start, last_span.end);

    // build format context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type,
        ..formatter.into()
    };
    let context = DestackFormatContext {
        options: format_options,
        file: file.as_ref(),
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
    };

    // format overlapping expressions
    let mut result = String::new();
    for (i, expr) in overlapping.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).ok()?;
        let printed = formatted.print().ok()?;
        result.push_str(printed.as_str());
        if i < overlapping.len() - 1 {
            result.push('\n');
        }
    }

    // ensure trailing newline if we're at end of file
    let is_at_end = last_span.end >= file.len.saturating_sub(1);
    if is_at_end && !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Some((result, actual_range))
}
