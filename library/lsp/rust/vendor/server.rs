//! Synchronous stdio JSON-RPC 2.0 server.

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::vendor::language_server::LanguageServer;
use crate::vendor::service::LspService;
use crate::vendor::{jsonrpc, lsp_types};

#[derive(Debug)]
pub enum OutgoingMessage {
    Response {
        id: JsonValue,
        result: JsonValue,
    },
    Error {
        id: JsonValue,
        error: jsonrpc::ResponseError,
    },
    Notification {
        method: String,
        params: JsonValue,
    },
}

#[derive(Serialize)]
struct JsonRpcResponse<'a> {
    jsonrpc: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a jsonrpc::ResponseError>,
}

#[derive(Serialize)]
struct JsonRpcNotification<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<&'a JsonValue>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<JsonValue>,
    method: String,
    #[serde(default)]
    params: Option<JsonValue>,
}

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

    /// Serve the JSON-RPC 2.0 LSP protocol over stdio synchronously.
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
                    OutgoingMessage::Response { id, result } => {
                        let resp = JsonRpcResponse {
                            jsonrpc: "2.0",
                            id: Some(&id),
                            result: Some(&result),
                            error: None,
                        };
                        let _ = write_message(&mut writer, &resp);
                    }
                    OutgoingMessage::Error { id, error } => {
                        let resp = JsonRpcResponse {
                            jsonrpc: "2.0",
                            id: Some(&id),
                            result: None,
                            error: Some(&error),
                        };
                        let _ = write_message(&mut writer, &resp);
                    }
                    OutgoingMessage::Notification { method, params } => {
                        let notif = JsonRpcNotification {
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
        loop {
            match read_message(&mut reader) {
                Ok(req) => {
                    let id = req.id.clone();
                    let method = req.method;
                    let params = req.params.unwrap_or(JsonValue::Null);
                    let tx_clone = tx.clone();
                    let server_clone = server.clone();
                    std::thread::spawn(move || {
                        let send_response = |res: std::result::Result<
                            JsonValue,
                            jsonrpc::ResponseError,
                        >| {
                            if let Some(idv) = id.clone() {
                                match res {
                                    Ok(v) => {
                                        let _ = tx_clone
                                            .send(OutgoingMessage::Response { id: idv, result: v });
                                    }
                                    Err(e) => {
                                        let _ = tx_clone
                                            .send(OutgoingMessage::Error { id: idv, error: e });
                                    }
                                }
                            }
                        };

                        match method.as_str() {
                            "initialize" => {
                                let p: lsp_types::InitializeParams =
                                    serde_json::from_value(params).unwrap_or_default();
                                let r = server_clone
                                    .initialize(p)
                                    .map(|v| serde_json::to_value(v).unwrap());
                                send_response(r);
                            }
                            "initialized" => {
                                let p: lsp_types::InitializedParams =
                                    serde_json::from_value(JsonValue::Null)
                                        .unwrap_or(lsp_types::InitializedParams {});
                                server_clone.initialized(p);
                            }
                            "shutdown" => {
                                let r = server_clone.shutdown().map(|_| JsonValue::Null);
                                send_response(r);
                            }
                            "textDocument/didOpen" => {
                                let p: lsp_types::DidOpenTextDocumentParams =
                                    serde_json::from_value(params).unwrap();
                                server_clone.did_open(p);
                            }
                            "textDocument/didChange" => {
                                let p: lsp_types::DidChangeTextDocumentParams =
                                    serde_json::from_value(params).unwrap();
                                server_clone.did_change(p);
                            }
                            "textDocument/semanticTokens/full" => {
                                let p: lsp_types::SemanticTokensParams =
                                    serde_json::from_value(params).unwrap();
                                let r = server_clone
                                    .semantic_tokens_full(p)
                                    .map(|opt| serde_json::to_value(opt).unwrap());
                                send_response(r);
                            }
                            "textDocument/semanticTokens/range" => {
                                let p: lsp_types::SemanticTokensRangeParams =
                                    serde_json::from_value(params).unwrap();
                                let r = server_clone
                                    .semantic_tokens_range(p)
                                    .map(|opt| serde_json::to_value(opt).unwrap());
                                send_response(r);
                            }
                            "textDocument/hover" => {
                                let p: lsp_types::HoverParams =
                                    serde_json::from_value(params).unwrap();
                                let r = server_clone
                                    .hover(p)
                                    .map(|opt| serde_json::to_value(opt).unwrap());
                                send_response(r);
                            }
                            _ => {}
                        }
                    });
                }
                Err(_) => break,
            }
        }

        let _ = writer_handle.join();
        Ok(())
    }
}

fn write_message<W: Write, T: Serialize>(writer: &mut W, value: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn read_message<R: Read>(reader: &mut R) -> std::io::Result<JsonRpcRequest> {
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
        if line.to_ascii_lowercase().starts_with("content-length:") {
            if let Some(v) = line.split(':').nth(1) {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let req: JsonRpcRequest = serde_json::from_slice(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(req)
}

#[cfg(test)]
mod tests {
    // TODO @Incomplete: add integration tests using OS pipes
}
