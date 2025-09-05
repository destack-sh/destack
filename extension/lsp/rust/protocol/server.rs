//! Synchronous stdio JSON-RPC 2.0 server.

use std::io::{Read, Write};

use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::protocol::language_server::LanguageServer;
use crate::protocol::service::LspService;
use crate::protocol::{jsonrpc, types};

pub struct Server<R: Read, W: Write + Send + 'static> {
    reader: R,
    writer: W,
    socket: super::service::Socket,
}

impl<R: Read, W: Write + Send + 'static> Server<R, W> {
    pub fn new(reader: R, writer: W, socket: super::service::Socket) -> Self {
        Self {
            reader,
            writer,
            socket,
        }
    }

    /// Serve the JSON-RPC 2.0 LSP protocol over stdio (synchronously).
    pub fn serve<S: LanguageServer>(self, service: LspService<S>) -> std::io::Result<()> {
        let mut writer = self.writer;
        let rx = {
            // expose receiver by moving it out; service::Socket has private field
            // NOTE @Cleanup: service::Socket is in the same module tree, so access via pattern
            let super::service::Socket { rx } = self.socket;
            rx
        };
        let writer_handle = std::thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    jsonrpc::JsonRpcMessage::Response { id, result } => {
                        let resp = jsonrpc::JsonRpcResponse {
                            jsonrpc: "2.0",
                            id: Some(&id),
                            result: Some(&result),
                            error: None,
                        };
                        let _ = write_message(&mut writer, &resp);
                    }
                    jsonrpc::JsonRpcMessage::Error { id, error } => {
                        let resp = jsonrpc::JsonRpcResponse {
                            jsonrpc: "2.0",
                            id: Some(&id),
                            result: None,
                            error: Some(&error),
                        };
                        let _ = write_message(&mut writer, &resp);
                    }
                    jsonrpc::JsonRpcMessage::Notification { method, params } => {
                        let notif = jsonrpc::JsonRpcNotification {
                            jsonrpc: "2.0",
                            method: &method,
                            params: Some(&params),
                        };
                        let _ = write_message(&mut writer, &notif);
                    }
                }
            }
        });

        let server = service.server();
        let tx = service.sender();
        let mut reader = self.reader;
        while let Ok(req) = read_message(&mut reader) {
            Self::process_message(req, server.clone(), tx.clone());
        }

        let _ = writer_handle.join();
        Ok(())
    }

    /// Process a single JSON-RPC message by dispatching to the appropriate handler.
    fn process_message<S: LanguageServer>(
        req: jsonrpc::JsonRpcRequest,
        server: std::sync::Arc<S>,
        tx: std::sync::mpsc::Sender<jsonrpc::JsonRpcMessage>,
    ) {
        let id = req.id.clone();
        let method = req.method;
        let params = req.params.unwrap_or(JsonValue::Null);

        std::thread::spawn(move || {
            let send_response = |res: std::result::Result<JsonValue, jsonrpc::ResponseError>| {
                if let Some(idv) = id.clone() {
                    match res {
                        Ok(v) => {
                            let _ =
                                tx.send(jsonrpc::JsonRpcMessage::Response { id: idv, result: v });
                        }
                        Err(e) => {
                            let _ = tx.send(jsonrpc::JsonRpcMessage::Error { id: idv, error: e });
                        }
                    }
                }
            };

            match method.as_str() {
                // Server
                "initialize" => {
                    let request: types::InitializeParams =
                        serde_json::from_value(params).unwrap_or_default();
                    let response = server
                        .initialize(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "initialized" => {
                    let request: types::InitializedParams = serde_json::from_value(JsonValue::Null)
                        .unwrap_or(types::InitializedParams {});
                    server.initialized(request);
                }

                "shutdown" => {
                    let response = server.shutdown().map(|_| JsonValue::Null);
                    send_response(response);
                }

                // Document Synchronization
                "textDocument/didOpen" => {
                    let request: types::DidOpenTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.did_open(request);
                }

                "textDocument/didChange" => {
                    let request: types::DidChangeTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.did_change(request);
                }

                "textDocument/willSave" => {
                    let request: types::WillSaveTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.will_save(request);
                }

                "textDocument/willSaveWaitUntil" => {
                    let request: types::WillSaveTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .will_save_wait_until(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/didSave" => {
                    let request: types::DidSaveTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.did_save(request);
                }

                "textDocument/didClose" => {
                    let request: types::DidCloseTextDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.did_close(request);
                }

                // Notebook Document Synchronization
                "notebookDocument/didOpen" => {
                    let request: types::DidOpenNotebookDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.notebook_did_open(request);
                }

                "notebookDocument/didChange" => {
                    let request: types::DidChangeNotebookDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.notebook_did_change(request);
                }

                "notebookDocument/didSave" => {
                    let request: types::DidSaveNotebookDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.notebook_did_save(request);
                }

                "notebookDocument/didClose" => {
                    let request: types::DidCloseNotebookDocumentParams =
                        serde_json::from_value(params).unwrap();
                    server.notebook_did_close(request);
                }

                // Language Features
                "textDocument/definition" => {
                    let request: types::GotoDefinitionParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .goto_definition(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/references" => {
                    let request: types::ReferenceParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .references(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/hover" => {
                    let request: types::HoverParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .hover(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/completion" => {
                    let request: types::CompletionParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .completion(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "completionItem/resolve" => {
                    let request: types::CompletionItem = serde_json::from_value(params).unwrap();
                    let response = server
                        .completion_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/signatureHelp" => {
                    let request: types::SignatureHelpParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .signature_help(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/documentHighlight" => {
                    let request: types::DocumentHighlightParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .document_highlight(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/documentSymbol" => {
                    let request: types::DocumentSymbolParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .document_symbol(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/codeAction" => {
                    let request: types::CodeActionParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .code_action(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "codeAction/resolve" => {
                    let request: types::CodeAction = serde_json::from_value(params).unwrap();
                    let response = server
                        .code_action_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/codeLens" => {
                    let request: types::CodeLensParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .code_lens(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "codeLens/resolve" => {
                    let request: types::CodeLens = serde_json::from_value(params).unwrap();
                    let response = server
                        .code_lens_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/documentLink" => {
                    let request: types::DocumentLinkParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .document_link(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "documentLink/resolve" => {
                    let request: types::DocumentLink = serde_json::from_value(params).unwrap();
                    let response = server
                        .document_link_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/documentColor" => {
                    let request: types::DocumentColorParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .document_color(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/colorPresentation" => {
                    let request: types::ColorPresentationParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .color_presentation(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/formatting" => {
                    let request: types::DocumentFormattingParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .formatting(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/rangeFormatting" => {
                    let request: types::DocumentRangeFormattingParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .range_formatting(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/onTypeFormatting" => {
                    let request: types::DocumentOnTypeFormattingParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .on_type_formatting(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/rename" => {
                    let request: types::RenameParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .rename(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/prepareRename" => {
                    let request: types::TextDocumentPositionParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .prepare_rename(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/foldingRange" => {
                    let request: types::FoldingRangeParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .folding_range(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/selectionRange" => {
                    let request: types::SelectionRangeParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .selection_range(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/prepareCallHierarchy" => {
                    let request: types::CallHierarchyPrepareParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .prepare_call_hierarchy(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "callHierarchy/incomingCalls" => {
                    let request: types::CallHierarchyIncomingCallsParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .incoming_calls(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "callHierarchy/outgoingCalls" => {
                    let request: types::CallHierarchyOutgoingCallsParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .outgoing_calls(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/prepareTypeHierarchy" => {
                    let request: types::TypeHierarchyPrepareParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .prepare_type_hierarchy(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "typeHierarchy/supertypes" => {
                    let request: types::TypeHierarchySupertypesParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .supertypes(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "typeHierarchy/subtypes" => {
                    let request: types::TypeHierarchySubtypesParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .subtypes(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/semanticTokens/full" => {
                    let request: types::SemanticTokensParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .semantic_tokens_full(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/semanticTokens/full/delta" => {
                    let request: types::SemanticTokensDeltaParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .semantic_tokens_full_delta(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/semanticTokens/range" => {
                    let request: types::SemanticTokensRangeParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .semantic_tokens_range(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/inlayHint" => {
                    let request: types::InlayHintParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .inlay_hint(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "inlayHint/resolve" => {
                    let request: types::InlayHint = serde_json::from_value(params).unwrap();
                    let response = server
                        .inlay_hint_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "textDocument/moniker" => {
                    let request: types::MonikerParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .moniker(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "textDocument/linkedEditingRange" => {
                    let request: types::LinkedEditingRangeParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .linked_editing_range(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                // Workspace
                "workspace/symbol" => {
                    let request: types::WorkspaceSymbolParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .symbol(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "workspaceSymbol/resolve" => {
                    let request: types::WorkspaceSymbol = serde_json::from_value(params).unwrap();
                    let response = server
                        .symbol_resolve(request)
                        .map(|v| serde_json::to_value(v).unwrap());
                    send_response(response);
                }

                "workspace/didChangeConfiguration" => {
                    let request: types::DidChangeConfigurationParams =
                        serde_json::from_value(params).unwrap();
                    server.did_change_configuration(request);
                }

                "workspace/didChangeWorkspaceFolders" => {
                    let request: types::DidChangeWorkspaceFoldersParams =
                        serde_json::from_value(params).unwrap();
                    server.did_change_workspace_folders(request);
                }

                "workspace/willCreateFiles" => {
                    let request: types::CreateFilesParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .will_create_files(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "workspace/didCreateFiles" => {
                    let request: types::CreateFilesParams = serde_json::from_value(params).unwrap();
                    server.did_create_files(request);
                }

                "workspace/willRenameFiles" => {
                    let request: types::RenameFilesParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .will_rename_files(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "workspace/didRenameFiles" => {
                    let request: types::RenameFilesParams = serde_json::from_value(params).unwrap();
                    server.did_rename_files(request);
                }

                "workspace/willDeleteFiles" => {
                    let request: types::DeleteFilesParams = serde_json::from_value(params).unwrap();
                    let response = server
                        .will_delete_files(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                "workspace/didDeleteFiles" => {
                    let request: types::DeleteFilesParams = serde_json::from_value(params).unwrap();
                    server.did_delete_files(request);
                }

                "workspace/didChangeWatchedFiles" => {
                    let request: types::DidChangeWatchedFilesParams =
                        serde_json::from_value(params).unwrap();
                    server.did_change_watched_files(request);
                }

                "workspace/executeCommand" => {
                    let request: types::ExecuteCommandParams =
                        serde_json::from_value(params).unwrap();
                    let response = server
                        .execute_command(request)
                        .map(|opt| serde_json::to_value(opt).unwrap());
                    send_response(response);
                }

                _ => {}
            }
        });
    }
}

fn write_message<W: Write, T: Serialize>(writer: &mut W, value: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn read_message<R: Read>(reader: &mut R) -> std::io::Result<jsonrpc::JsonRpcRequest> {
    let mut header_buf = Vec::new();
    let mut last_four: [u8; 4] = [0; 4];
    loop {
        let mut byte = [0u8; 1];
        if reader.read_exact(&mut byte).is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof",
            ));
        }
        header_buf.push(byte[0]);
        let len = header_buf.len();
        if len >= 4 {
            last_four.copy_from_slice(&header_buf[len - 4..len]);
            if last_four == *b"\r\n\r\n" {
                break;
            }
        }
    }
    let header_str = String::from_utf8_lossy(&header_buf);
    let mut content_length: usize = 0;
    for line in header_str.split("\r\n") {
        let line = line.trim();
        if line.to_ascii_lowercase().starts_with("content-length:")
            && let Some(v) = line.split(':').nth(1)
        {
            content_length = v.trim().parse::<usize>().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let req: jsonrpc::JsonRpcRequest = serde_json::from_slice(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(req)
}

#[cfg(test)]
mod tests {
    // TODO @Incomplete: add integration tests use OS pipes
}
