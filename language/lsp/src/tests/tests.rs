use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::Path;
use std::time::Duration;

use destack_lsp_server::jsonrpc::{self, Id};
use destack_lsp_server::{ClientSocket, ExitedError, LspService};
use destack_lsp_types as lsp;
use destack_source::TemporaryPhysicalFileSystem;
use futures::{FutureExt, SinkExt, StreamExt};
use serde_json::{Value, from_value, to_value};
use tower::{Service, ServiceExt};

use crate::DestackLanguageServer;
use crate::query::ToLspUri;

/// The single-target workspace manifest test fixtures share.
pub(super) const MANIFEST: &str = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "include": ["src/**/*.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;

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

/// The displayed text from one completion item.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CompletionDisplay<'a> {
    /// The completion label.
    pub(super) label: &'a str,
    /// The text displayed after the label.
    pub(super) label_detail: Option<&'a str>,
    /// The secondary label description.
    pub(super) description: Option<&'a str>,
    /// The expanded completion detail.
    pub(super) detail: Option<&'a str>,
    /// The rich completion documentation.
    pub(super) documentation: Option<&'a str>,
}

impl<'a> From<&'a lsp::CompletionItem> for CompletionDisplay<'a> {
    /// Read every displayed field from one LSP completion item.
    fn from(item: &'a lsp::CompletionItem) -> Self {
        let label_details = item.label_details.as_ref();
        let documentation = match item.documentation.as_ref() {
            Some(lsp::Documentation::MarkupContent(markup)) => {
                assert_eq!(markup.kind, lsp::MarkupKind::Markdown);

                Some(markup.value.as_str())
            }
            Some(lsp::Documentation::String(documentation)) => {
                panic!("completion documentation is plain text: {documentation}");
            }
            None => None,
        };

        Self {
            label: &item.label,
            label_detail: label_details.and_then(|details| details.detail.as_deref()),
            description: label_details.and_then(|details| details.description.as_deref()),
            detail: item.detail.as_deref(),
            documentation,
        }
    }
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
        let root = self.root().to_path_buf();

        self.initialize_workspace(&root, capabilities, initialization_options)
            .await
    }

    /// Initialize the server against one existing workspace root.
    #[allow(deprecated)]
    pub(super) async fn initialize_workspace(
        &mut self,
        root: &Path,
        capabilities: lsp::ClientCapabilities,
        initialization_options: Option<Value>,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        let params = lsp::InitializeParams {
            capabilities,
            initialization_options,
            root_uri: Some(uri(root)),
            ..lsp::InitializeParams::default()
        };

        self.request::<lsp::request::Initialize>(params).await
    }

    /// Notify the server that client initialization is complete.
    pub(super) async fn initialized(&mut self) {
        self.notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await;
    }

    /// Open one single-target workspace and its entry document.
    ///
    /// Writes the default manifest and every source file, initializes the
    /// server, opens the entry, and asserts it publishes no diagnostics.
    pub(super) async fn open_workspace(
        name: &str,
        files: &[(&str, &str)],
        entry: &str,
    ) -> (Self, TestDocument) {
        let mut server = Self::new(name);
        server.write("destack.json", MANIFEST);

        // write every source and remember the entry document
        let mut opened = None;
        for (path, source) in files {
            let document = server.write(path, source);
            if path == &entry {
                opened = Some((document, *source));
            }
        }
        let (document, source) = opened.expect("workspace entry should be written");

        // initialize and open the entry without diagnostics
        server
            .initialize(lsp::ClientCapabilities::default(), None)
            .await
            .unwrap();
        server.initialized().await;
        server.open(&document, 1, source).await;
        server
            .assert_notification::<lsp::notification::PublishDiagnostics>(
                lsp::PublishDiagnosticsParams {
                    uri: document.uri().clone(),
                    diagnostics: Vec::new(),
                    version: Some(1),
                },
            )
            .await;

        (server, document)
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

    /// Type text into one document as successive character revisions.
    pub(super) async fn type_text(
        &mut self,
        document: &TestDocument,
        mut version: i32,
        mut position: lsp::Position,
        text: &str,
    ) -> (i32, lsp::Position) {
        for character in text.chars() {
            version += 1;
            self.change(
                document,
                version,
                [replace(
                    lsp::Range::new(position, position),
                    character.to_string(),
                )],
            )
            .await;

            // advance the UTF-16 editor position
            if character == '\n' {
                position.line += 1;
                position.character = 0;
            } else {
                position.character += character.len_utf16() as u32;
            }
        }

        (version, position)
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

    /// Return completion items for one document position.
    pub(super) async fn complete(
        &mut self,
        document: &TestDocument,
        position: lsp::Position,
    ) -> Vec<lsp::CompletionItem> {
        let response = self
            .request::<lsp::request::Completion>(document.completion(position))
            .await
            .unwrap();

        match response {
            Some(lsp::CompletionResponse::Array(items)) => items,
            Some(lsp::CompletionResponse::List(list)) => list.items,
            None => Vec::new(),
        }
    }

    /// Require one completion and its compact and expanded details.
    pub(super) async fn assert_completion(
        &mut self,
        document: &TestDocument,
        position: lsp::Position,
        expected: CompletionDisplay<'_>,
    ) {
        let items = self.complete(document, position).await;
        let matching = items
            .iter()
            .filter(|item| item.label == expected.label)
            .collect::<Vec<_>>();
        let labels = items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "expected one {:?} completion, found labels {labels:?}",
            expected.label,
        );
        // compare every displayed detail
        let actual = CompletionDisplay::from(matching[0]);
        assert_eq!(actual, expected);
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

    /// Require the user-facing diagnostics published for one document revision.
    pub(super) async fn assert_diagnostics(
        &mut self,
        document: &TestDocument,
        version: i32,
        expected: Vec<lsp::Diagnostic>,
    ) {
        let mut published = self
            .receive_notification::<lsp::notification::PublishDiagnostics>()
            .await;
        assert_eq!(published.uri, document.uri);
        assert_eq!(published.version, Some(version));

        // omit the private payload used to resolve later code actions
        for diagnostic in &mut published.diagnostics {
            diagnostic.data = None;
        }

        assert_eq!(published.diagnostics, expected);
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

        TestDocument::from(path.as_path())
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
    /// LSP document URI.
    uri: lsp::Uri,
}

impl From<&Path> for TestDocument {
    /// Build one test document from its absolute filesystem path.
    fn from(path: &Path) -> Self {
        Self { uri: uri(path) }
    }
}

impl TestDocument {
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

    /// Build goto implementation parameters for this document.
    pub(super) fn implementations(
        &self,
        position: lsp::Position,
    ) -> lsp::request::GotoImplementationParams {
        lsp::request::GotoImplementationParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        }
    }

    /// Build completion parameters for this document.
    pub(super) fn completion(&self, position: lsp::Position) -> lsp::CompletionParams {
        lsp::CompletionParams {
            text_document_position: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
            context: None,
        }
    }

    /// Build rename parameters for this document.
    pub(super) fn rename(&self, position: lsp::Position, new_name: &str) -> lsp::RenameParams {
        lsp::RenameParams {
            text_document_position: self.position(position),
            new_name: new_name.to_string(),
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

    /// Build outline parameters for this document.
    pub(super) fn outline(&self) -> lsp::DocumentSymbolParams {
        lsp::DocumentSymbolParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        }
    }

    /// Build inlay hint parameters for one document range.
    pub(super) fn inlay_hints(&self, range: lsp::Range) -> lsp::InlayHintParams {
        lsp::InlayHintParams {
            text_document: self.identifier(),
            range,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        }
    }

    /// Build document link parameters for this document.
    pub(super) fn links(&self) -> lsp::DocumentLinkParams {
        lsp::DocumentLinkParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        }
    }

    /// Build code action parameters for this document.
    pub(super) fn code_actions(
        &self,
        range: lsp::Range,
        context: lsp::CodeActionContext,
    ) -> lsp::CodeActionParams {
        lsp::CodeActionParams {
            text_document: self.identifier(),
            range,
            context,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        }
    }

    /// Build code lens parameters for this document.
    pub(super) fn code_lenses(&self) -> lsp::CodeLensParams {
        lsp::CodeLensParams {
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

    /// Wait for one result superseded only by a newer document revision.
    pub(super) async fn wait_or_content_modified(self) {
        match self.wait().await {
            Ok(_) => {}
            Err(error) if error.code == jsonrpc::ErrorCode::ContentModified => {}
            Err(error) => panic!("{} failed: {error}", R::METHOD),
        }
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
    path.as_ref().to_lsp_uri().unwrap()
}
