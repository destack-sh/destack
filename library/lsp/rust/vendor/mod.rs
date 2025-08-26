use std::io::{Read, Write};
use std::sync::{Arc, mpsc};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub mod jsonrpc {
	use super::*;

	#[derive(Debug, Serialize, Deserialize)]
	pub struct ResponseError {
		pub code: i32,
		pub message: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub data: Option<JsonValue>,
	}

	pub type Result<T> = std::result::Result<T, ResponseError>;
}

pub mod lsp_types {
	use super::*;

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	#[serde(transparent)]
	pub struct Uri(pub String);

	impl std::fmt::Display for Uri {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			self.0.fmt(f)
		}
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
	pub struct Position {
		pub line: u32,
		pub character: u32,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	pub struct Range {
		pub start: Position,
		pub end: Position,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct TextDocumentIdentifier {
		pub uri: Uri,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct VersionedTextDocumentIdentifier {
		pub uri: Uri,
		pub version: i32,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct TextDocumentItem {
		pub uri: Uri,
		pub language_id: Option<String>,
		pub version: Option<i32>,
		pub text: String,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct TextDocumentContentChangeEvent {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub range: Option<Range>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub range_length: Option<u32>,
		pub text: String,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct DidOpenTextDocumentParams {
		pub text_document: TextDocumentItem,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct DidChangeTextDocumentParams {
		pub text_document: VersionedTextDocumentIdentifier,
		pub content_changes: Vec<TextDocumentContentChangeEvent>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	pub struct InitializeParams {}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	pub struct InitializedParams {}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct ServerInfo {
		pub name: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub version: Option<String>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	pub struct WorkDoneProgressOptions {}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct SemanticTokensLegend {
		pub token_types: Vec<SemanticTokenType>,
		pub token_modifiers: Vec<SemanticTokenModifier>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct SemanticTokensOptions {
		pub legend: SemanticTokensLegend,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub range: Option<bool>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub full: Option<SemanticTokensFullOptions>,
		#[serde(default)]
		pub work_done_progress_options: WorkDoneProgressOptions,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum SemanticTokensFullOptions {
		Bool(bool),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct InitializeResult {
		pub capabilities: ServerCapabilities,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub server_info: Option<ServerInfo>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct ServerCapabilities {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub text_document_sync: Option<TextDocumentSyncCapability>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub hover_provider: Option<HoverProviderCapability>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub semantic_tokens_provider: Option<SemanticTokensServerCapabilities>,
	}

	impl Default for ServerCapabilities {
		fn default() -> Self {
			Self {
				text_document_sync: None,
				hover_provider: None,
				semantic_tokens_provider: None,
			}
		}
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub enum TextDocumentSyncCapability {
		Kind(TextDocumentSyncKind),
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
	pub enum TextDocumentSyncKind {
		None = 0,
		Full = 1,
		Incremental = 2,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub enum HoverProviderCapability {
		Simple(bool),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct TextDocumentPositionParams {
		pub text_document: TextDocumentIdentifier,
		pub position: Position,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct HoverParams {
		#[serde(flatten)]
		pub text_document_position_params: TextDocumentPositionParams,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct Hover {
		pub contents: HoverContents,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub range: Option<Range>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum HoverContents {
		Scalar(MarkedString),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum MarkedString {
		String(String),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct SemanticTokensParams {
		pub text_document: TextDocumentIdentifier,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct SemanticTokensRangeParams {
		pub text_document: TextDocumentIdentifier,
		pub range: Range,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	#[serde(rename_all = "camelCase")]
	pub struct SemanticTokens {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub result_id: Option<String>,
		pub data: Vec<SemanticToken>,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
	#[serde(rename_all = "camelCase")]
	pub struct SemanticToken {
		pub delta_line: u32,
		pub delta_start: u32,
		pub length: u32,
		pub token_type: u32,
		pub token_modifiers_bitset: u32,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum SemanticTokensResult {
		Tokens(SemanticTokens),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum SemanticTokensRangeResult {
		Tokens(SemanticTokens),
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(rename_all = "lowercase")]
	pub enum SemanticTokenType {
		Comment,
		Keyword,
		String,
		Number,
		Operator,
		Function,
		Type,
		Variable,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(rename_all = "lowercase")]
	pub enum SemanticTokenModifier {}

	#[repr(u8)]
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
	pub enum MessageType {
		Error = 1,
		Warning = 2,
		Info = 3,
		Log = 4,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct LogMessageParams {
		#[serde(rename = "type")]
		pub r#type: MessageType,
		pub message: String,
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum SemanticTokensServerCapabilities {
		SemanticTokensOptions(SemanticTokensOptions),
	}
}

#[derive(Debug)]
pub struct Client {
	tx: mpsc::Sender<OutgoingMessage>,
}

impl Client {
	pub fn log_message(&self, ty: lsp_types::MessageType, message: impl Into<String>) {
		let params = lsp_types::LogMessageParams { r#type: ty, message: message.into() };
		let notification = OutgoingMessage::Notification {
			method: "window/logMessage".to_string(),
			params: serde_json::to_value(params).unwrap_or(JsonValue::Null),
		};
		let _ = self.tx.send(notification);
	}
}

pub trait LanguageServer: Send + Sync + 'static {
	fn initialize(&self, _params: lsp_types::InitializeParams) -> jsonrpc::Result<lsp_types::InitializeResult> { Ok(lsp_types::InitializeResult { capabilities: lsp_types::ServerCapabilities::default(), server_info: None }) }
	fn initialized(&self, _params: lsp_types::InitializedParams) {}
	fn shutdown(&self) -> jsonrpc::Result<()> { Ok(()) }
	fn did_open(&self, _params: lsp_types::DidOpenTextDocumentParams) {}
	fn did_change(&self, _params: lsp_types::DidChangeTextDocumentParams) {}
	fn semantic_tokens_full(&self, _params: lsp_types::SemanticTokensParams) -> jsonrpc::Result<Option<lsp_types::SemanticTokensResult>> { Ok(None) }
	fn semantic_tokens_range(&self, _params: lsp_types::SemanticTokensRangeParams) -> jsonrpc::Result<Option<lsp_types::SemanticTokensRangeResult>> { Ok(None) }
	fn hover(&self, _params: lsp_types::HoverParams) -> jsonrpc::Result<Option<lsp_types::Hover>> { Ok(None) }
}

pub use crate::vendor::lsp_types as lsp;

#[derive(Debug)]
pub struct LspService<S: LanguageServer> {
	server: Arc<S>,
	outgoing_tx: mpsc::Sender<OutgoingMessage>,
}

impl<S: LanguageServer> LspService<S> {
	pub fn new<F>(factory: F) -> (Self, Socket)
	where
		F: FnOnce(Client) -> S,
	{
		let (tx, rx) = mpsc::channel();
		let client = Client { tx: tx.clone() };
		let server = Arc::new(factory(client));
		(
			Self { server, outgoing_tx: tx },
			Socket { rx },
		)
	}

	fn sender(&self) -> mpsc::Sender<OutgoingMessage> { self.outgoing_tx.clone() }

	fn server(&self) -> Arc<S> { self.server.clone() }
}

#[derive(Debug)]
pub struct Socket {
	rx: mpsc::Receiver<OutgoingMessage>,
}

#[derive(Debug)]
enum OutgoingMessage {
	Response { id: JsonValue, result: JsonValue },
	Error { id: JsonValue, error: jsonrpc::ResponseError },
	Notification { method: String, params: JsonValue },
}

#[derive(Serialize)]
struct JsonRpcResponse<'a> {
	jsonrpc: &'a str,
	#[serde(skip_serializing_if = "Option::is_none")] id: Option<&'a JsonValue>,
	#[serde(skip_serializing_if = "Option::is_none")] result: Option<&'a JsonValue>,
	#[serde(skip_serializing_if = "Option::is_none")] error: Option<&'a jsonrpc::ResponseError>,
}

#[derive(Serialize)]
struct JsonRpcNotification<'a> {
	jsonrpc: &'a str,
	method: &'a str,
	#[serde(skip_serializing_if = "Option::is_none")] params: Option<&'a JsonValue>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
	jsonrpc: String,
	#[serde(default)] id: Option<JsonValue>,
	method: String,
	#[serde(default)] params: Option<JsonValue>,
}

pub struct Server<R: Read, W: Write> {
	reader: R,
	writer: W,
	socket: Socket,
}

impl<R: Read, W: Write> Server<R, W> {
	pub fn new(reader: R, writer: W, socket: Socket) -> Self { Self { reader, writer, socket } }

	/// Serve the JSON-RPC 2.0 LSP protocol over stdio synchronously.
	pub fn serve<S: LanguageServer>(mut self, service: LspService<S>) -> std::io::Result<()> {
		// writer thread handles outgoing messages
		let mut writer = self.writer;
		let rx = self.socket.rx;
		let writer_handle = std::thread::spawn(move || {
			while let Ok(msg) = rx.recv() {
				match msg {
					OutgoingMessage::Response { id, result } => {
						let resp = JsonRpcResponse { jsonrpc: "2.0", id: Some(&id), result: Some(&result), error: None };
						let _ = write_message(&mut writer, &resp);
					}
					OutgoingMessage::Error { id, error } => {
						let resp = JsonRpcResponse { jsonrpc: "2.0", id: Some(&id), result: None, error: Some(&error) };
						let _ = write_message(&mut writer, &resp);
					}
					OutgoingMessage::Notification { method, params } => {
						let notif = JsonRpcNotification { jsonrpc: "2.0", method: &method, params: Some(&params) };
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
						let send_response = |res: std::result::Result<JsonValue, jsonrpc::ResponseError>| {
							if let Some(idv) = id.clone() {
								match res {
									Ok(v) => { let _ = tx_clone.send(OutgoingMessage::Response { id: idv, result: v }); }
									Err(e) => { let _ = tx_clone.send(OutgoingMessage::Error { id: idv, error: e }); }
								}
							}
						};

						match method.as_str() {
							"initialize" => {
								let p: lsp_types::InitializeParams = serde_json::from_value(params).unwrap_or_default();
								let r = server_clone.initialize(p).map(|v| serde_json::to_value(v).unwrap());
								send_response(r);
							}
							"initialized" => {
								let p: lsp_types::InitializedParams = serde_json::from_value(JsonValue::Null).unwrap_or_default();
								server_clone.initialized(p);
							}
							"shutdown" => {
								let r = server_clone.shutdown().map(|_| JsonValue::Null);
								send_response(r);
							}
							"textDocument/didOpen" => {
								let p: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(params).unwrap();
								server_clone.did_open(p);
							}
							"textDocument/didChange" => {
								let p: lsp_types::DidChangeTextDocumentParams = serde_json::from_value(params).unwrap();
								server_clone.did_change(p);
							}
							"textDocument/semanticTokens/full" => {
								let p: lsp_types::SemanticTokensParams = serde_json::from_value(params).unwrap();
								let r = server_clone.semantic_tokens_full(p).map(|opt| serde_json::to_value(opt).unwrap());
								send_response(r);
							}
							"textDocument/semanticTokens/range" => {
								let p: lsp_types::SemanticTokensRangeParams = serde_json::from_value(params).unwrap();
								let r = server_clone.semantic_tokens_range(p).map(|opt| serde_json::to_value(opt).unwrap());
								send_response(r);
							}
							"textDocument/hover" => {
								let p: lsp_types::HoverParams = serde_json::from_value(params).unwrap();
								let r = server_clone.hover(p).map(|opt| serde_json::to_value(opt).unwrap());
								send_response(r);
							}
							_ => {
								// ignore unknown methods for now
							}
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
			return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof"));
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
	let req: JsonRpcRequest = serde_json::from_slice(&body).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
	Ok(req)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::duplex;

	#[test]
	fn test_jsonrpc_roundtrip() {
		let (mut a, mut b) = duplex(1024);
		let req = serde_json::json!({
			"jsonrpc": "2.0",
			"id": 1,
			"method": "initialize",
			"params": {}
		});
		let body = serde_json::to_vec(&req).unwrap();
		let header = format!("Content-Length: {}\r\n\r\n", body.len());
		std::thread::spawn(move || {
			a.write_all(header.as_bytes()).unwrap();
			a.write_all(&body).unwrap();
		});

		let parsed = read_message(&mut b).unwrap();
		assert_eq!(parsed.method, "initialize");
	}
}

