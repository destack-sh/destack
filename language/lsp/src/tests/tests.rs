use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Duration;

use destack_lsp_server::jsonrpc::{self, Id};
use destack_lsp_server::{ClientSocket, ExitedError, LspService};
use destack_lsp_types as lsp;
use destack_source::TemporaryPhysicalFileSystem;
use futures::{FutureExt, SinkExt, StreamExt};
use serde_json::{Value, from_value, to_value};
use tower::{Service, ServiceExt};

use crate::DestackLanguageServer;

/// Maximum time to wait for one server initiated message.
const CLIENT_MESSAGE_TIMEOUT: Duration = Duration::from_secs(15);

/// Package configuration used by LSP integration tests.
const DESTACK_JSON: &str = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "entry": ["main.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// One isolated language server and physical workspace.
pub(super) struct TestServer {
    /// Temporary workspace storage.
    file_system: TemporaryPhysicalFileSystem,
    /// In-process LSP service.
    service: LspService<DestackLanguageServer>,
    /// Server initiated messages and client responses.
    socket: ClientSocket,
    /// Next client request identity.
    next_request_id: i64,
}

impl TestServer {
    /// Create one configured language server.
    pub(super) fn new(name: &str) -> Self {
        let file_system = TemporaryPhysicalFileSystem::new_with_prefix(name);
        file_system.write_text_or_error("destack.json", DESTACK_JSON);
        let (service, socket) = LspService::new(DestackLanguageServer::new);

        Self {
            file_system,
            service,
            socket,
            next_request_id: 1,
        }
    }

    /// Create one language server rooted at an editor folder without a manifest.
    pub(super) fn new_editor_folder(name: &str) -> Self {
        let file_system = TemporaryPhysicalFileSystem::new_with_prefix(name);
        let (service, socket) = LspService::new(DestackLanguageServer::new);

        Self {
            file_system,
            service,
            socket,
            next_request_id: 1,
        }
    }

    /// Create one configured package below the editor folder.
    pub(super) fn create_package(&self, path: impl AsRef<Path>) {
        self.file_system
            .write_text_or_error(path.as_ref().join("destack.json"), DESTACK_JSON);
    }

    /// Write one workspace document.
    pub(super) fn write(&self, path: impl AsRef<Path>, source: &str) -> TestDocument {
        let path = self.file_system.write_text_or_error(path, source);

        self.document(path)
    }

    /// Initialize the server against the test workspace.
    #[allow(deprecated)]
    pub(super) async fn initialize(
        &mut self,
        capabilities: lsp::ClientCapabilities,
        initialization_options: Option<Value>,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        let params = lsp::InitializeParams {
            capabilities,
            initialization_options,
            root_uri: Some(self.root_uri()),
            ..lsp::InitializeParams::default()
        };

        self.request::<lsp::request::Initialize>(params).await
    }

    /// Notify the server that client initialization is complete.
    pub(super) async fn initialized(&mut self) {
        self.notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await;
    }

    /// Open one editor document.
    pub(super) async fn open(&mut self, document: &TestDocument, version: i32, source: &str) {
        self.notify::<lsp::notification::DidOpenTextDocument>(document.open(version, source))
            .await;
    }

    /// Apply changes to one editor document.
    pub(super) async fn change(
        &mut self,
        document: &TestDocument,
        version: i32,
        changes: impl IntoIterator<Item = lsp::TextDocumentContentChangeEvent>,
    ) {
        self.notify::<lsp::notification::DidChangeTextDocument>(document.change(version, changes))
            .await;
    }

    /// Save one editor document.
    pub(super) async fn save(&mut self, document: &TestDocument, source: Option<&str>) {
        self.notify::<lsp::notification::DidSaveTextDocument>(document.save(source))
            .await;
    }

    /// Close one editor document.
    pub(super) async fn close(&mut self, document: &TestDocument) {
        self.notify::<lsp::notification::DidCloseTextDocument>(document.close())
            .await;
    }

    /// Send one typed client request.
    pub(super) async fn request<R>(&mut self, params: R::Params) -> jsonrpc::Result<R::Result>
    where
        R: lsp::request::Request,
    {
        self.start_request::<R>(params).await.wait().await
    }

    /// Require one exact typed client request result.
    pub(super) async fn assert_request<R>(
        &mut self,
        params: R::Params,
        expected: jsonrpc::Result<R::Result>,
    ) where
        R: lsp::request::Request,
        R::Result: Debug + PartialEq,
    {
        let result = self.request::<R>(params).await;

        assert_eq!(result, expected);
    }

    /// Start one typed client request without waiting for its result.
    pub(super) async fn start_request<R>(&mut self, params: R::Params) -> PendingRequest<R>
    where
        R: lsp::request::Request,
    {
        let id = Id::Number(self.next_request_id);
        self.next_request_id += 1;
        let request = jsonrpc::Request::build(R::METHOD)
            .params(to_value(params).unwrap())
            .id(id.clone())
            .finish();
        let call = self.service.ready().await.unwrap().call(request);
        let call = tokio::spawn(call);

        PendingRequest {
            id,
            call,
            request: PhantomData,
        }
    }

    /// Send one typed client notification.
    pub(super) async fn notify<N>(&mut self, params: N::Params)
    where
        N: lsp::notification::Notification,
    {
        self.start_notification::<N>(params).await.wait().await;
    }

    /// Start one typed client notification without waiting for its handler.
    pub(super) async fn start_notification<N>(&mut self, params: N::Params) -> PendingNotification
    where
        N: lsp::notification::Notification,
    {
        let request = jsonrpc::Request::build(N::METHOD)
            .params(to_value(params).unwrap())
            .finish();
        let call = self.service.ready().await.unwrap().call(request);
        let call = tokio::spawn(call);

        PendingNotification { call }
    }

    /// Receive one typed server notification.
    pub(super) async fn receive_notification<N>(&mut self) -> N::Params
    where
        N: lsp::notification::Notification,
    {
        let message = self.receive().await;
        let (method, id, params) = message.into_parts();
        assert_eq!(id, None);
        assert_eq!(method, N::METHOD);
        let params = params.unwrap_or_else(|| panic!("{} omitted its parameters", N::METHOD));

        from_value(params).unwrap()
    }

    /// Require one exact typed server notification.
    pub(super) async fn assert_notification<N>(&mut self, expected: N::Params)
    where
        N: lsp::notification::Notification,
        N::Params: Debug + PartialEq,
    {
        let params = self.receive_notification::<N>().await;

        assert_eq!(params, expected);
    }

    /// Require the server to have sent no further protocol message.
    pub(super) fn assert_no_message(&mut self) {
        match self.socket.next().now_or_never() {
            None => {}
            Some(Some(message)) => panic!("unexpected server message: {message:?}"),
            Some(None) => panic!("client socket closed"),
        }
    }

    /// Receive one typed server request.
    pub(super) async fn receive_request<R>(&mut self) -> (Id, R::Params)
    where
        R: lsp::request::Request,
    {
        let message = self.receive().await;
        let (method, id, params) = message.into_parts();
        assert_eq!(method, R::METHOD);
        let id = id.unwrap_or_else(|| panic!("{} was sent as a notification", R::METHOD));
        let params = params.unwrap_or_else(|| panic!("{} omitted its parameters", R::METHOD));
        let params = from_value(params).unwrap();

        (id, params)
    }

    /// Respond to one typed server request.
    pub(super) async fn respond<R>(&mut self, id: Id, result: jsonrpc::Result<R::Result>)
    where
        R: lsp::request::Request,
    {
        let result = result.map(|result| to_value(result).unwrap());
        let response = jsonrpc::Response::from_parts(id, result);

        self.socket.send(response).await.unwrap();
    }

    /// Return the test workspace root.
    pub(super) fn root(&self) -> &Path {
        self.file_system.root()
    }

    /// Return the test workspace root URI.
    pub(super) fn root_uri(&self) -> lsp::Uri {
        uri(self.root())
    }

    /// Build one document identity from a scoped workspace path.
    fn document(&self, path: impl AsRef<Path>) -> TestDocument {
        let path = self.file_system.path_for(path);
        let uri = uri(&path);

        TestDocument { path, uri }
    }

    /// Receive one server initiated protocol message.
    async fn receive(&mut self) -> jsonrpc::Request {
        tokio::time::timeout(CLIENT_MESSAGE_TIMEOUT, self.socket.next())
            .await
            .unwrap()
            .unwrap()
    }
}

/// One workspace document used by an LSP test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestDocument {
    /// Absolute filesystem path.
    path: PathBuf,
    /// LSP document URI.
    uri: lsp::Uri,
}

impl TestDocument {
    /// Return the absolute filesystem path.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Return the LSP document URI.
    pub(super) fn uri(&self) -> &lsp::Uri {
        &self.uri
    }

    /// Build an LSP document identifier.
    pub(super) fn identifier(&self) -> lsp::TextDocumentIdentifier {
        lsp::TextDocumentIdentifier {
            uri: self.uri.clone(),
        }
    }

    /// Build parameters that open this document.
    pub(super) fn open(&self, version: i32, source: &str) -> lsp::DidOpenTextDocumentParams {
        lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem {
                uri: self.uri.clone(),
                language_id: "destack".to_string(),
                version,
                text: source.to_string(),
            },
        }
    }

    /// Build parameters that change this document.
    pub(super) fn change(
        &self,
        version: i32,
        changes: impl IntoIterator<Item = lsp::TextDocumentContentChangeEvent>,
    ) -> lsp::DidChangeTextDocumentParams {
        lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier {
                uri: self.uri.clone(),
                version,
            },
            content_changes: changes.into_iter().collect(),
        }
    }

    /// Build parameters that save this document.
    pub(super) fn save(&self, source: Option<&str>) -> lsp::DidSaveTextDocumentParams {
        lsp::DidSaveTextDocumentParams {
            text_document: self.identifier(),
            text: source.map(str::to_string),
        }
    }

    /// Build parameters that close this document.
    pub(super) fn close(&self) -> lsp::DidCloseTextDocumentParams {
        lsp::DidCloseTextDocumentParams {
            text_document: self.identifier(),
        }
    }

    /// Build LSP position parameters for this document.
    pub(super) fn position(&self, position: lsp::Position) -> lsp::TextDocumentPositionParams {
        lsp::TextDocumentPositionParams {
            text_document: self.identifier(),
            position,
        }
    }

    /// Build hover parameters for this document.
    pub(super) fn hover(&self, position: lsp::Position) -> lsp::HoverParams {
        lsp::HoverParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        }
    }

    /// Build semantic token parameters for this document.
    pub(super) fn semantic_tokens(&self) -> lsp::SemanticTokensParams {
        lsp::SemanticTokensParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        }
    }
}

/// One typed client request being evaluated by the server.
pub(super) struct PendingRequest<R>
where
    R: lsp::request::Request,
{
    /// Client request identity.
    id: Id,
    /// Running server call.
    call: ServerCall,
    /// LSP request type.
    request: PhantomData<R>,
}

impl<R> PendingRequest<R>
where
    R: lsp::request::Request,
{
    /// Wait for the typed server result.
    pub(super) async fn wait(self) -> jsonrpc::Result<R::Result> {
        let response = self.call.await.unwrap().unwrap().unwrap_or_else(|| {
            panic!("{} returned no response", R::METHOD);
        });
        let (id, result) = response.into_parts();
        assert_eq!(id, self.id);

        result.map(|result| from_value(result).unwrap())
    }
}

/// One client notification being handled by the server.
pub(super) struct PendingNotification {
    /// Running server call.
    call: ServerCall,
}

impl PendingNotification {
    /// Wait until the server finishes handling the notification.
    pub(super) async fn wait(self) {
        let response = self.call.await.unwrap().unwrap();

        assert_eq!(response, None);
    }
}

/// Running in-process language server call.
type ServerCall = tokio::task::JoinHandle<Result<Option<jsonrpc::Response>, ExitedError>>;

/// Build one LSP source position.
pub(super) const fn position(line: u32, character: u32) -> lsp::Position {
    lsp::Position { line, character }
}

/// Build one LSP source range.
pub(super) const fn range(
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
) -> lsp::Range {
    lsp::Range {
        start: position(start_line, start_character),
        end: position(end_line, end_character),
    }
}

/// Replace one LSP source range.
pub(super) fn replace(
    range: lsp::Range,
    text: impl Into<String>,
) -> lsp::TextDocumentContentChangeEvent {
    lsp::TextDocumentContentChangeEvent {
        range: Some(range),
        range_length: None,
        text: text.into(),
    }
}

/// Replace one complete LSP document.
pub(super) fn replace_document(text: impl Into<String>) -> lsp::TextDocumentContentChangeEvent {
    lsp::TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: text.into(),
    }
}

/// Build Markdown protocol contents.
pub(super) fn markdown(value: impl Into<String>) -> lsp::MarkupContent {
    lsp::MarkupContent {
        kind: lsp::MarkupKind::Markdown,
        value: value.into(),
    }
}

/// Build one LSP URI from a filesystem path.
fn uri(path: impl AsRef<Path>) -> lsp::Uri {
    crate::query::DocumentUri::path(path).unwrap()
}
