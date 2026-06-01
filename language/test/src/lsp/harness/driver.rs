use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

use destack_lsp::tests::harness::{LspHarness, request_with_params, uri_for_path};
use destack_lsp_server::jsonrpc::Response;
use destack_lsp_types as lsp;
use destack_lsp_types::request::{
    GotoDeclarationParams, GotoImplementationParams, GotoTypeDefinitionParams,
};
use destack_source::TemporaryPhysicalFileSystem;
use serde::Serialize;
use tokio::runtime::{Builder, Runtime};

use crate::lsp::{LspFixture, Marker};

const DEFAULT_PROGRESS_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PROGRESS_MESSAGES: usize = 128;

/// One in-process LSP driver backed by the shared server test harness.
#[derive(Debug)]
pub struct LspDriver {
    /// The runtime used to drive async LSP operations.
    runtime: Runtime,
    /// The temporary workspace filesystem for this case.
    filesystem: TemporaryPhysicalFileSystem,
    /// The initialized in-process LSP harness.
    harness: LspHarness,
}

impl LspDriver {
    /// Create one driver and materialize the fixture files into a temp workspace.
    pub fn from_fixture(prefix: &str, fixture: &LspFixture) -> Result<Self, String> {
        // materialize the virtual workspace on disk before server initialization
        let filesystem = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        for file in fixture.files.iter() {
            filesystem
                .write_text(&file.path, &file.text)
                .map_err(|error| format!("failed to write fixture file {}: {error}", file.path))?;
        }

        // initialize the shared in-process lsp harness on its own runtime
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| format!("failed to build lsp runtime: {error}"))?;
        let harness = runtime.block_on(async {
            let mut harness = LspHarness::new(filesystem.root().to_path_buf());
            let params = initialize_params(&filesystem, fixture);

            harness.initialize_with_params(params).await;
            harness
        });

        Ok(Self {
            runtime,
            filesystem,
            harness,
        })
    }

    /// Return the workspace root for this driver.
    pub fn workspace_root(&self) -> &Path {
        self.filesystem.root()
    }

    /// Resolve one fixture-relative file path inside the materialized workspace.
    pub fn path_for(&self, file_path: &str) -> PathBuf {
        self.filesystem.path_for(file_path)
    }

    /// Build one file URI for a fixture-relative path.
    pub fn uri_for(&self, file_path: &str) -> lsp::Uri {
        uri_for_path(&self.path_for(file_path))
    }

    /// Borrow the initialized LSP harness.
    pub fn harness(&mut self) -> &mut LspHarness {
        &mut self.harness
    }

    /// Read one materialized workspace file as UTF-8 text.
    pub fn read_file_text(&self, file_path: &str) -> Result<String, String> {
        let path = self.path_for(file_path);

        read_workspace_text(&path)
            .map_err(|error| format!("failed to read fixture file {file_path}: {error}"))
    }

    /// Overwrite one materialized workspace file with full text.
    pub fn write_file_text(&self, file_path: &str, text: &str) -> Result<(), String> {
        let path = self.path_for(file_path);

        fs::write(&path, text)
            .map_err(|error| format!("failed to write fixture file {file_path}: {error}"))
    }

    /// Create one materialized workspace file and notify the server.
    pub fn create_file_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        let path = self.path_for(file_path);
        let uri = uri_for_path(&path);

        // create parent directories before writing the new file
        if let Some(parent_path) = path.parent() {
            fs::create_dir_all(parent_path).map_err(|error| {
                format!("failed to create parent directories for {file_path}: {error}")
            })?;
        }

        // write the file to disk before notifying the server
        fs::write(&path, text)
            .map_err(|error| format!("failed to create fixture file {file_path}: {error}"))?;

        // notify the real didCreateFiles path so indexing updates stay honest
        self.runtime.block_on(async {
            self.harness.did_create(uri).await;
            self.harness.wait_for_mutation_idle().await;
        });

        Ok(())
    }

    /// Overwrite one closed workspace file and notify the watched-file path.
    pub fn change_closed_file_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        let path = self.path_for(file_path);
        let uri = uri_for_path(&path);

        // write the updated text to disk before notifying the server
        fs::write(&path, text)
            .map_err(|error| format!("failed to update fixture file {file_path}: {error}"))?;

        // notify the watched-file path for closed-document churn
        self.runtime.block_on(async {
            self.harness
                .did_change_watched(uri, lsp::FileChangeType::CHANGED)
                .await;
            self.harness.wait_for_mutation_idle().await;
        });

        Ok(())
    }

    /// Delete one materialized workspace file and notify the server.
    pub fn delete_file_text(&mut self, file_path: &str) -> Result<(), String> {
        let path = self.path_for(file_path);
        let uri = uri_for_path(&path);

        // remove the file before notifying the server
        fs::remove_file(&path)
            .map_err(|error| format!("failed to delete fixture file {file_path}: {error}"))?;

        // notify the real didDeleteFiles path so diagnostics clear honestly
        self.runtime.block_on(async {
            self.harness.did_delete(uri).await;
            self.harness.wait_for_mutation_idle().await;
        });

        Ok(())
    }

    /// Open one fixture file and wait until queued server mutations become idle.
    pub fn open_file(&mut self, file_path: &str, version: i32) -> Result<(), String> {
        // resolve the file text and uri before entering the runtime
        let text = self.read_file_text(file_path)?;

        self.open_file_with_text(file_path, &text, version)
    }

    /// Open one fixture file with explicit overlay text.
    pub fn open_file_with_text(
        &mut self,
        file_path: &str,
        text: &str,
        version: i32,
    ) -> Result<(), String> {
        // resolve the file uri before entering the runtime
        let path = self.path_for(file_path);
        let uri = uri_for_path(&path);

        // publish didOpen first so the server receives the editor overlay
        self.runtime.block_on(async {
            self.harness
                .did_open_with_version(uri.clone(), text, version)
                .await;
            self.harness.wait_for_mutation_idle().await;
        });

        Ok(())
    }

    /// Open every fixture file and wait until queued server mutations become idle.
    pub fn open_fixture_files(&mut self, fixture: &LspFixture) -> Result<(), String> {
        // open files in fixture order for deterministic diagnostics sequencing
        for file in fixture.files.iter() {
            self.open_file(&file.path, 1)?;
        }

        Ok(())
    }

    /// Send one full-text didChange notification for a file.
    pub fn change_file(&mut self, file_path: &str, text: &str, version: i32) {
        let uri = self.uri_for(file_path);

        self.runtime.block_on(async {
            self.harness.did_change(uri, text, version).await;
            self.harness.wait_for_mutation_idle().await;
        });
    }

    /// Send one didSave notification for a file.
    pub fn save_file(&mut self, file_path: &str, text: Option<String>) {
        let uri = self.uri_for(file_path);

        self.runtime.block_on(async {
            self.harness.did_save(uri, text).await;
            self.harness.wait_for_mutation_idle().await;
        });
    }

    /// Send one didClose notification for a file.
    pub fn close_file(&mut self, file_path: &str) {
        let uri = self.uri_for(file_path);

        self.runtime.block_on(async {
            self.harness.did_close(uri).await;
            self.harness.wait_for_mutation_idle().await;
        });
    }

    /// Wait for the next pushed diagnostics payload for one file.
    pub fn next_diagnostics_for(&mut self, file_path: &str) -> lsp::PublishDiagnosticsParams {
        let uri = self.uri_for(file_path);

        self.runtime
            .block_on(async { self.harness.next_diagnostics_for(&uri).await })
    }

    /// Resolve goto definition for one marker position.
    pub fn goto_definition_for_marker(
        &mut self,
        marker: &Marker,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let position = lsp::Position {
            line: marker.line as u32,
            character: marker.character as u32,
        };

        self.goto_definition(&marker.file_path, position)
    }

    /// Resolve goto definition for one file position.
    pub fn goto_definition(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        // build the protocol position from the parsed marker coordinates
        let uri = self.uri_for(file_path);

        // execute the real LSP request on the shared runtime
        Ok(self
            .runtime
            .block_on(async { self.harness.goto_definition(uri, position).await }))
    }

    /// Resolve goto declaration for one file position.
    pub fn goto_declaration(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/declaration",
                    GotoDeclarationParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve goto type definition for one file position.
    pub fn goto_type_definition(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/typeDefinition",
                    GotoTypeDefinitionParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve goto implementation for one file position.
    pub fn goto_implementation(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/implementation",
                    GotoImplementationParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve references for one marker position.
    pub fn find_references_for_marker(
        &mut self,
        marker: &Marker,
        include_declaration: bool,
    ) -> Result<Option<Vec<lsp::Location>>, String> {
        let position = lsp::Position {
            line: marker.line as u32,
            character: marker.character as u32,
        };

        self.find_references(&marker.file_path, position, include_declaration)
    }

    /// Resolve references for one file position.
    pub fn find_references(
        &mut self,
        file_path: &str,
        position: lsp::Position,
        include_declaration: bool,
    ) -> Result<Option<Vec<lsp::Location>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/references",
                    lsp::ReferenceParams {
                        text_document_position: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                        context: lsp::ReferenceContext {
                            include_declaration,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve hover for one file position.
    pub fn hover(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::Hover>, String> {
        let uri = self.uri_for(file_path);

        Ok(self
            .runtime
            .block_on(async { self.harness.hover(uri, position).await }))
    }

    /// Resolve document highlights for one file position.
    pub fn document_highlights(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<Vec<lsp::DocumentHighlight>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/documentHighlight",
                    lsp::DocumentHighlightParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve signature help for one file position.
    pub fn signature_help(
        &mut self,
        file_path: &str,
        position: lsp::Position,
        context: Option<lsp::SignatureHelpContext>,
    ) -> Result<Option<lsp::SignatureHelp>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/signatureHelp",
                    lsp::SignatureHelpParams {
                        context,
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve on-type formatting edits for one open file position.
    pub fn on_type_formatting(
        &mut self,
        file_path: &str,
        position: lsp::Position,
        ch: &str,
        options: lsp::FormattingOptions,
    ) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/onTypeFormatting",
                    lsp::DocumentOnTypeFormattingParams {
                        text_document_position: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        ch: ch.to_string(),
                        options,
                    },
                )
                .await
        }))
    }

    /// Resolve document symbols for one fixture-relative file path.
    pub fn document_symbols(
        &mut self,
        file_path: &str,
    ) -> Result<Option<lsp::DocumentSymbolResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/documentSymbol",
                    lsp::DocumentSymbolParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve selection ranges for one file and ordered caret positions.
    pub fn selection_ranges(
        &mut self,
        file_path: &str,
        positions: Vec<lsp::Position>,
    ) -> Result<Option<Vec<lsp::SelectionRange>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/selectionRange",
                    lsp::SelectionRangeParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        positions,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve call hierarchy prepare items for one file position.
    pub fn prepare_call_hierarchy(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<Vec<lsp::CallHierarchyItem>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/prepareCallHierarchy",
                    lsp::CallHierarchyPrepareParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve incoming call hierarchy edges for one prepared item.
    pub fn call_hierarchy_incoming(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Result<Option<Vec<lsp::CallHierarchyIncomingCall>>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "callHierarchy/incomingCalls",
                    lsp::CallHierarchyIncomingCallsParams {
                        item,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve outgoing call hierarchy edges for one prepared item.
    pub fn call_hierarchy_outgoing(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "callHierarchy/outgoingCalls",
                    lsp::CallHierarchyOutgoingCallsParams {
                        item,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve type hierarchy prepare items for one file position.
    pub fn prepare_type_hierarchy(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/prepareTypeHierarchy",
                    lsp::TypeHierarchyPrepareParams {
                        text_document_position_params: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve type hierarchy supertypes for one prepared item.
    pub fn type_hierarchy_supertypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "typeHierarchy/supertypes",
                    lsp::TypeHierarchySupertypesParams {
                        item,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve type hierarchy subtypes for one prepared item.
    pub fn type_hierarchy_subtypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "typeHierarchy/subtypes",
                    lsp::TypeHierarchySubtypesParams {
                        item,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve workspace symbols for one search query.
    pub fn workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>, String>
    {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "workspace/symbol",
                    lsp::WorkspaceSymbolParams {
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        query: query.to_string(),
                    },
                )
                .await
        }))
    }

    /// Resolve completions for one file position.
    pub fn completion(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::CompletionResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/completion",
                    lsp::CompletionParams {
                        text_document_position: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                        context: None,
                    },
                )
                .await
        }))
    }

    /// Resolve one previously returned completion item.
    pub fn completion_resolve(
        &mut self,
        item: lsp::CompletionItem,
    ) -> Result<Option<lsp::CompletionItem>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result("completionItem/resolve", item)
                .await
        }))
    }

    /// Resolve whole-document formatting edits for one open file.
    pub fn document_formatting(
        &mut self,
        file_path: &str,
        options: lsp::FormattingOptions,
    ) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/formatting",
                    lsp::DocumentFormattingParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        options,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve range formatting edits for one selected range.
    pub fn range_formatting(
        &mut self,
        file_path: &str,
        range: lsp::Range,
        options: lsp::FormattingOptions,
    ) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/rangeFormatting",
                    lsp::DocumentRangeFormattingParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        range,
                        options,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve folding ranges for one fixture-relative file path.
    pub fn folding_ranges(
        &mut self,
        file_path: &str,
    ) -> Result<Option<Vec<lsp::FoldingRange>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/foldingRange",
                    lsp::FoldingRangeParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve document links for one fixture-relative file path.
    pub fn document_links(
        &mut self,
        file_path: &str,
    ) -> Result<Option<Vec<lsp::DocumentLink>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/documentLink",
                    lsp::DocumentLinkParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve code actions for one file range.
    pub fn code_actions(
        &mut self,
        file_path: &str,
        range: lsp::Range,
    ) -> Result<Option<lsp::CodeActionResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/codeAction",
                    lsp::CodeActionParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        range,
                        context: lsp::CodeActionContext {
                            diagnostics: Vec::new(),
                            only: None,
                            trigger_kind: None,
                        },
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve one previously returned code action.
    pub fn code_action_resolve(
        &mut self,
        action: lsp::CodeAction,
    ) -> Result<Option<lsp::CodeAction>, String> {
        Ok(self.runtime.block_on(async {
            self.harness
                .request_result("codeAction/resolve", action)
                .await
        }))
    }

    /// Resolve code lenses for one fixture-relative file path.
    pub fn code_lenses(&mut self, file_path: &str) -> Result<Option<Vec<lsp::CodeLens>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/codeLens",
                    lsp::CodeLensParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve one previously returned code lens.
    pub fn code_lens_resolve(
        &mut self,
        lens: lsp::CodeLens,
    ) -> Result<Option<lsp::CodeLens>, String> {
        Ok(self
            .runtime
            .block_on(async { self.harness.request_result("codeLens/resolve", lens).await }))
    }

    /// Resolve full semantic tokens for one open file.
    pub fn semantic_tokens_full(
        &mut self,
        file_path: &str,
    ) -> Result<Option<lsp::SemanticTokensResult>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/semanticTokens/full",
                    lsp::SemanticTokensParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve semantic tokens delta for one open file and prior result id.
    pub fn semantic_tokens_full_delta(
        &mut self,
        file_path: &str,
        previous_result_id: String,
    ) -> Result<Option<lsp::SemanticTokensFullDeltaResult>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/semanticTokens/full/delta",
                    lsp::SemanticTokensDeltaParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        previous_result_id,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve semantic tokens for one selected range.
    pub fn semantic_tokens_range(
        &mut self,
        file_path: &str,
        range: lsp::Range,
    ) -> Result<Option<lsp::SemanticTokensRangeResult>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/semanticTokens/range",
                    lsp::SemanticTokensRangeParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        range,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve inlay hints for one selected range.
    pub fn inlay_hints(
        &mut self,
        file_path: &str,
        range: lsp::Range,
    ) -> Result<Option<Vec<lsp::InlayHint>>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/inlayHint",
                    lsp::InlayHintParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        range,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve rename edits for one marker position.
    pub fn rename_for_marker(
        &mut self,
        marker: &Marker,
        new_name: &str,
    ) -> Result<Option<lsp::WorkspaceEdit>, String> {
        let position = lsp::Position {
            line: marker.line as u32,
            character: marker.character as u32,
        };

        self.rename(&marker.file_path, position, new_name)
    }

    /// Resolve rename edits for one file position.
    pub fn rename(
        &mut self,
        file_path: &str,
        position: lsp::Position,
        new_name: &str,
    ) -> Result<Option<lsp::WorkspaceEdit>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/rename",
                    lsp::RenameParams {
                        text_document_position: lsp::TextDocumentPositionParams {
                            text_document: lsp::TextDocumentIdentifier::new(uri),
                            position,
                        },
                        new_name: new_name.to_string(),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await
        }))
    }

    /// Resolve prepare rename for one file position.
    pub fn prepare_rename(
        &mut self,
        file_path: &str,
        position: lsp::Position,
    ) -> Result<Option<lsp::PrepareRenameResponse>, String> {
        let uri = self.uri_for(file_path);

        Ok(self.runtime.block_on(async {
            self.harness
                .request_result(
                    "textDocument/prepareRename",
                    lsp::TextDocumentPositionParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        position,
                    },
                )
                .await
        }))
    }

    /// Request document diagnostics for one fixture-relative file path.
    pub fn document_diagnostic_for_file(
        &mut self,
        file_path: &str,
    ) -> Result<lsp::DocumentDiagnosticReportResult, String> {
        let uri = self.uri_for(file_path);
        let response = self.runtime.block_on(async {
            self.harness
                .call(request_with_params(
                    "textDocument/diagnostic",
                    12,
                    lsp::DocumentDiagnosticParams {
                        text_document: lsp::TextDocumentIdentifier::new(uri),
                        identifier: Some("destack".to_string()),
                        previous_result_id: None,
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                ))
                .await
        });
        let response =
            response.ok_or_else(|| "missing document diagnostic response".to_string())?;
        decode_response_result(&response)
    }

    /// Request full workspace diagnostics for the current workspace.
    pub fn workspace_diagnostic(&mut self) -> Result<lsp::WorkspaceDiagnosticReportResult, String> {
        let response = self.runtime.block_on(async {
            self.harness
                .call(request_with_params(
                    "workspace/diagnostic",
                    13,
                    lsp::WorkspaceDiagnosticParams {
                        identifier: None,
                        previous_result_ids: Vec::new(),
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                        partial_result_params: lsp::PartialResultParams {
                            partial_result_token: None,
                        },
                    },
                ))
                .await
        });
        let response =
            response.ok_or_else(|| "missing workspace diagnostic response".to_string())?;
        decode_response_result(&response)
    }

    /// Execute one workspace command and wait until queued mutations become idle.
    pub fn execute_command(&mut self, command: &str) -> Result<Option<lsp::LSPAny>, String> {
        let result = self.runtime.block_on(async {
            let result = self
                .harness
                .request_result(
                    "workspace/executeCommand",
                    lsp::ExecuteCommandParams {
                        command: command.to_string(),
                        arguments: vec![],
                        work_done_progress_params: lsp::WorkDoneProgressParams {
                            work_done_token: None,
                        },
                    },
                )
                .await;
            self.harness.wait_for_mutation_idle().await;

            result
        });

        Ok(result)
    }

    /// Wait until queued server mutations become idle.
    pub fn wait_for_mutation_idle(&mut self) {
        self.runtime.block_on(async {
            self.harness.wait_for_mutation_idle().await;
        });
    }

    /// Start one typed request and keep it pending.
    pub fn start_request<T>(&mut self, method: &str, params: T) -> Result<i64, String>
    where
        T: Serialize,
    {
        Ok(self
            .runtime
            .block_on(async { self.harness.start_request(method, params).await }))
    }

    /// Await one previously started request response.
    pub fn await_request(&mut self, request_id: i64) -> Result<Response, String> {
        self.runtime
            .block_on(async { self.harness.await_request(request_id).await })
            .ok_or_else(|| format!("missing response for request id {request_id}"))
    }

    /// Send protocol request cancellation for one request id.
    pub fn cancel_request(&mut self, request_id: i64) {
        self.runtime
            .block_on(async { self.harness.cancel_request(request_id).await });
    }

    /// Send work-done progress cancellation for one token.
    pub fn cancel_work_done_progress(&mut self, token: lsp::ProgressToken) {
        self.runtime
            .block_on(async { self.harness.cancel_work_done_progress(token).await });
    }

    /// Wait for the next partial progress payload on one token.
    pub fn next_partial_progress(
        &mut self,
        token: &lsp::ProgressToken,
    ) -> Result<serde_json::Value, String> {
        self.runtime.block_on(async {
            for _ in 0..MAX_PROGRESS_MESSAGES {
                let request = tokio::time::timeout(
                    DEFAULT_PROGRESS_TIMEOUT,
                    self.harness.next_client_request(),
                )
                .await
                .map_err(|_| "timeout waiting for partial progress".to_string())?;

                if request.method() != "$/progress" {
                    continue;
                }

                let params = request
                    .params()
                    .cloned()
                    .ok_or_else(|| "missing progress params".to_string())?;
                let progress: lsp::ProgressParams = serde_json::from_value(params)
                    .map_err(|error| format!("failed to decode progress params: {error}"))?;
                if &progress.token != token {
                    continue;
                }

                if let lsp::ProgressParamsValue::PartialResult(value) = progress.value {
                    return Ok(value);
                }
            }

            Err("missing partial progress result".to_string())
        })
    }

    /// Wait for one specific work-done progress kind on one token.
    pub fn wait_for_work_done_progress_kind(
        &mut self,
        token: &lsp::ProgressToken,
        expected_kind: &str,
    ) -> Result<(), String> {
        self.runtime.block_on(async {
            for _ in 0..MAX_PROGRESS_MESSAGES {
                let request = tokio::time::timeout(
                    DEFAULT_PROGRESS_TIMEOUT,
                    self.harness.next_client_request(),
                )
                .await
                .map_err(|_| format!("timeout waiting for work-done progress {expected_kind}"))?;

                if request.method() != "$/progress" {
                    continue;
                }

                let params = request
                    .params()
                    .cloned()
                    .ok_or_else(|| "missing progress params".to_string())?;
                let progress: lsp::ProgressParams = serde_json::from_value(params)
                    .map_err(|error| format!("failed to decode progress params: {error}"))?;
                if &progress.token != token {
                    continue;
                }

                let lsp::ProgressParamsValue::WorkDone(work_done_progress) = progress.value else {
                    continue;
                };

                let kind = match work_done_progress {
                    lsp::WorkDoneProgress::Begin(_) => "begin",
                    lsp::WorkDoneProgress::Report(_) => "report",
                    lsp::WorkDoneProgress::End(_) => "end",
                };
                if kind == expected_kind {
                    return Ok(());
                }
            }

            Err(format!("missing work-done progress kind {expected_kind}"))
        })
    }

    /// Run one async operation on the driver's runtime.
    pub fn block_on<T>(&mut self, future: impl Future<Output = T>) -> T {
        self.runtime.block_on(future)
    }
}

/// Build initialize params for one fixture-specific client capability set.
fn initialize_params(
    filesystem: &TemporaryPhysicalFileSystem,
    fixture: &LspFixture,
) -> lsp::InitializeParams {
    #[allow(deprecated)]
    let mut params = lsp::InitializeParams {
        root_uri: Some(uri_for_path(filesystem.root())),
        ..Default::default()
    };

    // enable lazy code action resolve only for fixtures that need it
    if fixture.expectations.code_actions.enable_resolve_support {
        params.capabilities.text_document = Some(lsp::TextDocumentClientCapabilities {
            code_action: Some(lsp::CodeActionClientCapabilities {
                data_support: Some(true),
                resolve_support: Some(lsp::CodeActionCapabilityResolveSupport {
                    properties: vec!["edit".to_string()],
                }),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    params
}

/// Read one materialized workspace file as UTF-8 text.
fn read_workspace_text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Decode one typed JSON-RPC result payload from a successful response.
fn decode_response_result<T>(response: &Response) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let result = response
        .result()
        .cloned()
        .ok_or_else(|| "response did not contain a result payload".to_string())?;
    serde_json::from_value(result).map_err(|error| format!("failed to decode response: {error}"))
}
