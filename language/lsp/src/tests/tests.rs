use std::collections::VecDeque;
use std::env;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use futures::executor::block_on;
use futures::{FutureExt, SinkExt, StreamExt};
use serde_json::{Value, from_value, to_value};
use tower::{Service, ServiceExt};
use tspp_artifact::{ArtifactKey, ArtifactOutcome, BuildId};
use tspp_lsp_server::jsonrpc::{self, Id};
use tspp_lsp_server::{ExitedError, LspService, ResponseSink};
use tspp_lsp_types as lsp;
use tspp_lsp_types::notification::Notification;
use tspp_repository::{
    DestackLayoutOverride, Environment, Execution, Host, Repository, RevisionPin, Settings,
};
use tspp_session::{ArtifactPriority, Executor, Session};
use tspp_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem, Uri};

use crate::TsppLanguageServer;
use crate::query::ToLspUri;
use crate::server::ProjectId;

/// The single-target workspace manifest test fixtures share.
pub(super) const MANIFEST: &str = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "include": ["src/**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// Maximum time to wait for one server initiated message.
const CLIENT_MESSAGE_TIMEOUT: Duration = Duration::from_secs(15);
/// Maximum time to wait for one server call.
const SERVER_CALL_TIMEOUT: Duration = Duration::from_secs(15);
/// Environment variable enabling timing output.
const TIMINGS_ENV: &str = "TSPP_TIMINGS";
/// Environment variable selecting the shared artifact worker count.
const WORKERS_ENV: &str = "TSPP_TEST_WORKERS";

/// Library prerequisites retained for all protocol fixtures in this process.
static LIBRARY: OnceLock<TestLibrary> = OnceLock::new();

/// Package configuration used by LSP integration tests.
const DESTACK_JSON: &str = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "entry": ["main.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// Checked library artifacts shared by isolated protocol sessions.
struct TestLibrary {
    /// Session retaining the shared host and executor.
    session: Session,
    /// Source revision retaining the checked library artifacts.
    revision: RevisionPin,
    /// Physical sources retained for the suite.
    _files: TemporaryPhysicalFileSystem,
}

impl TestLibrary {
    /// Return the checked library shared by this process.
    fn shared() -> &'static Self {
        LIBRARY.get_or_init(Self::new)
    }

    /// Check the embedded library under the profiles used by protocol fixtures.
    fn new() -> Self {
        // import the ordinary fixture package and embedded library
        let files = TemporaryPhysicalFileSystem::new_with_prefix("lsp-library");
        files.write_text_or_error("destack.json", DESTACK_JSON);
        files.write_text_or_error("main.tspp", "export const value = 1;\n");
        let host = Host::new(
            BuildId::test(),
            Environment::capture_process(),
            Arc::new(PhysicalFileSystem::new()),
        );
        let (repository, revision) = Repository::open(
            files.root().to_path_buf(),
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .unwrap();
        let repository = Arc::new(repository);
        let revision = repository.pin(revision).unwrap();

        // use the same artifact worker configuration as query fixtures
        let worker_count = match env::var(WORKERS_ENV) {
            Ok(value) => value.parse::<usize>().unwrap_or_else(|error| {
                panic!("invalid {WORKERS_ENV} value '{value}': {error}");
            }),
            Err(env::VarError::NotPresent) => Executor::default_worker_count(),
            Err(env::VarError::NotUnicode(_)) => panic!("{WORKERS_ENV} is not valid UTF-8"),
        };
        let executor = Executor::new(Execution::Threaded, worker_count).unwrap();
        let session = Session::new(repository.clone(), executor).unwrap();

        // retain one checked result per library module and selected profile
        let mut profiles = Vec::new();
        for package in repository.package_ids(revision.revision()).unwrap() {
            if let Some((target, _)) = repository
                .package_default_target(revision.revision(), package)
                .unwrap()
            {
                profiles.push(
                    repository
                        .profile_for_target(revision.revision(), target)
                        .unwrap()
                        .id(),
                );
            }
        }
        profiles.sort_unstable();
        profiles.dedup();
        let modules = repository.builtin_module_ids(revision.revision()).unwrap();
        let mut artifacts = Vec::new();
        for profile in profiles {
            artifacts.extend(
                modules
                    .iter()
                    .map(|module| ArtifactKey::dir_checked(*module, profile)),
            );
        }
        let run = session.provide(
            revision.revision(),
            &artifacts,
            ArtifactPriority::Foreground,
        );
        block_on(run.wait()).unwrap();
        for key in artifacts {
            assert_eq!(
                repository
                    .current_artifact_outcome(revision.revision(), &key)
                    .unwrap(),
                Some(ArtifactOutcome::Ok)
            );
        }

        Self {
            session,
            revision,
            _files: files,
        }
    }
}

/// One isolated language server and physical workspace.
pub(super) struct TestServer {
    /// In-process LSP service.
    service: LspService<TsppLanguageServer>,

    /// Server initiated messages consumed by the simulated client.
    requests: UnboundedReceiver<jsonrpc::Request>,
    /// Client responses routed back to the server.
    responses: ResponseSink,

    /// Non-trace messages retained while draining timing output.
    messages: VecDeque<jsonrpc::Request>,
    /// Next client request identity.
    next_request_id: i64,

    /// Temporary workspace storage retained through server shutdown.
    file_system: TemporaryPhysicalFileSystem,
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

/// One typed request issued by the simulated client.
pub(super) struct TestRequest<R>
where
    R: lsp::request::Request,
{
    /// Request parameters.
    params: R::Params,
    /// LSP request type.
    request: PhantomData<R>,
}

impl<R> TestRequest<R>
where
    R: lsp::request::Request,
{
    /// Create one typed client request.
    fn new(params: R::Params) -> Self {
        Self {
            params,
            request: PhantomData,
        }
    }
}

impl TestServer {
    /// Create one configured language server.
    pub(super) fn new(name: &str) -> Self {
        let file_system = TemporaryPhysicalFileSystem::new_with_prefix(name);
        file_system.write_text_or_error("destack.json", DESTACK_JSON);

        Self::start(file_system)
    }

    /// Create one language server rooted at an editor folder without a manifest.
    pub(super) fn new_editor_folder(name: &str) -> Self {
        let file_system = TemporaryPhysicalFileSystem::new_with_prefix(name);

        Self::start(file_system)
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
            trace: Self::trace_value(),
            ..lsp::InitializeParams::default()
        };

        self.request(TestRequest::<lsp::request::Initialize>::new(params))
            .await
    }

    /// Initialize the server against exact editor workspace folders.
    #[allow(deprecated)]
    pub(super) async fn initialize_folders(
        &mut self,
        folders: &[&Path],
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        let workspace_folders = folders
            .iter()
            .map(|folder| lsp::WorkspaceFolder {
                uri: uri(folder),
                name: folder.display().to_string(),
            })
            .collect();
        let params = lsp::InitializeParams {
            capabilities: lsp::ClientCapabilities::default(),
            workspace_folders: Some(workspace_folders),
            trace: Self::trace_value(),
            ..lsp::InitializeParams::default()
        };

        self.request(TestRequest::<lsp::request::Initialize>::new(params))
            .await
    }

    /// Notify the server that client initialization is complete.
    pub(super) async fn initialized(&mut self) {
        self.notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await;
    }

    /// Initialize completion capabilities for one simulated client.
    pub(super) async fn initialize_completion(
        &mut self,
        resolve_properties: &[&str],
        supports_label_details: bool,
    ) {
        let capabilities = lsp::ClientCapabilities {
            text_document: Some(lsp::TextDocumentClientCapabilities {
                completion: Some(lsp::CompletionClientCapabilities {
                    completion_item: Some(lsp::CompletionItemCapability {
                        label_details_support: Some(supports_label_details),
                        resolve_support: Some(lsp::CompletionItemCapabilityResolveSupport {
                            properties: resolve_properties
                                .iter()
                                .map(|property| property.to_string())
                                .collect(),
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.initialize(capabilities, None).await.unwrap();
        self.initialized().await;
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
        server.assert_diagnostics(&document, 1, Vec::new()).await;

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
    pub(super) async fn request<R>(&mut self, request: TestRequest<R>) -> jsonrpc::Result<R::Result>
    where
        R: lsp::request::Request,
    {
        let result = self.start_request(request).await.wait().await;
        self.drain_output();

        result
    }

    // completion

    /// Return completion items for one document position.
    pub(super) async fn complete(
        &mut self,
        request: TestRequest<lsp::request::Completion>,
    ) -> Vec<lsp::CompletionItem> {
        let response = self.request(request).await.unwrap();

        match response {
            Some(lsp::CompletionResponse::Array(items)) => items,
            Some(lsp::CompletionResponse::List(list)) => list.items,
            None => Vec::new(),
        }
    }

    /// Return completion labels for one document position.
    pub(super) async fn completion_labels(
        &mut self,
        request: TestRequest<lsp::request::Completion>,
    ) -> Vec<String> {
        self.complete(request)
            .await
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    /// Select one exact completion item by label.
    pub(super) async fn select_completion(
        &mut self,
        request: TestRequest<lsp::request::Completion>,
        label: &str,
    ) -> lsp::CompletionItem {
        let items = self.complete(request).await;
        let labels = items
            .iter()
            .map(|item| item.label.clone())
            .collect::<Vec<_>>();
        let matching = items
            .into_iter()
            .filter(|item| item.label == label)
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "expected one {label:?} completion, found labels {labels:?}",
        );

        matching.into_iter().next().unwrap()
    }

    /// Resolve one completion item selected from a completion list.
    pub(super) async fn resolve_completion(
        &mut self,
        item: lsp::CompletionItem,
    ) -> jsonrpc::Result<lsp::CompletionItem> {
        self.request(TestRequest::<lsp::request::ResolveCompletionItem>::new(
            item,
        ))
        .await
    }

    /// Return one virtual document.
    pub(super) async fn virtual_document(
        &mut self,
        uri: lsp::Uri,
    ) -> lsp::TextDocumentContentResult {
        self.request(
            TestRequest::<lsp::request::TextDocumentContentRequest>::new(
                lsp::TextDocumentContentParams { uri },
            ),
        )
        .await
        .unwrap()
    }

    /// Search every open workspace for symbols matching one query.
    pub(super) fn workspace_symbols(
        &self,
        query: &str,
    ) -> TestRequest<lsp::request::WorkspaceSymbolRequest> {
        TestRequest::new(lsp::WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Request diagnostics for every open workspace.
    pub(super) fn workspace_diagnostics(
        &self,
        previous_result_ids: Vec<lsp::PreviousResultId>,
    ) -> TestRequest<lsp::request::WorkspaceDiagnosticRequest> {
        TestRequest::new(lsp::WorkspaceDiagnosticParams {
            identifier: None,
            previous_result_ids,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Reload every workspace owned by this server.
    pub(super) fn reload(&self) -> TestRequest<lsp::request::ExecuteCommand> {
        TestRequest::new(lsp::ExecuteCommandParams {
            command: "tspp.reload".to_string(),
            arguments: Vec::new(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Resolve the edit carried by one deferred code action.
    pub(super) fn resolve_code_action(
        &self,
        action: lsp::CodeAction,
    ) -> TestRequest<lsp::request::CodeActionResolveRequest> {
        TestRequest::new(action)
    }

    /// Request import edits for one workspace file rename.
    pub(super) fn rename_file(
        &self,
        old_document: &TestDocument,
        new_document: &TestDocument,
    ) -> TestRequest<lsp::request::WillRenameFiles> {
        TestRequest::new(lsp::RenameFilesParams {
            files: vec![lsp::FileRename {
                old_uri: old_document.uri.to_string(),
                new_uri: new_document.uri.to_string(),
            }],
        })
    }

    // hierarchy

    /// Require exactly one response item.
    pub(super) async fn request_one<R, T>(&mut self, request: TestRequest<R>) -> T
    where
        R: lsp::request::Request<Result = Option<Vec<T>>>,
        T: Debug,
    {
        let items = self.request(request).await.unwrap().unwrap();
        let [item]: [T; 1] = items.try_into().expect("expected one response item");

        item
    }

    /// Return incoming calls for one prepared hierarchy item.
    pub(super) async fn incoming_calls(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Option<Vec<lsp::CallHierarchyIncomingCall>> {
        self.request(
            TestRequest::<lsp::request::CallHierarchyIncomingCalls>::new(
                lsp::CallHierarchyIncomingCallsParams {
                    item,
                    work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                    partial_result_params: lsp::PartialResultParams::default(),
                },
            ),
        )
        .await
        .unwrap()
    }

    /// Return outgoing calls for one prepared hierarchy item.
    pub(super) async fn outgoing_calls(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Option<Vec<lsp::CallHierarchyOutgoingCall>> {
        self.request(
            TestRequest::<lsp::request::CallHierarchyOutgoingCalls>::new(
                lsp::CallHierarchyOutgoingCallsParams {
                    item,
                    work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                    partial_result_params: lsp::PartialResultParams::default(),
                },
            ),
        )
        .await
        .unwrap()
    }

    /// Return supertypes for one prepared hierarchy item.
    pub(super) async fn supertypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Option<Vec<lsp::TypeHierarchyItem>> {
        self.request(TestRequest::<lsp::request::TypeHierarchySupertypes>::new(
            lsp::TypeHierarchySupertypesParams {
                item,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
        .await
        .unwrap()
    }

    /// Return subtypes for one prepared hierarchy item.
    pub(super) async fn subtypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Option<Vec<lsp::TypeHierarchyItem>> {
        self.request(TestRequest::<lsp::request::TypeHierarchySubtypes>::new(
            lsp::TypeHierarchySubtypesParams {
                item,
                work_done_progress_params: lsp::WorkDoneProgressParams::default(),
                partial_result_params: lsp::PartialResultParams::default(),
            },
        ))
        .await
        .unwrap()
    }

    /// Require one completion and its compact and expanded details.
    pub(super) async fn assert_completion(
        &mut self,
        request: TestRequest<lsp::request::Completion>,
        expected: CompletionDisplay<'_>,
    ) {
        let item = self.select_completion(request, expected.label).await;

        // resolve and compare every displayed detail
        let item = if item.data.is_some() {
            self.resolve_completion(item).await.unwrap()
        } else {
            item
        };
        let actual = CompletionDisplay::from(&item);
        assert_eq!(actual, expected);
    }

    /// Require one exact typed client request result.
    pub(super) async fn assert_request<R>(
        &mut self,
        request: TestRequest<R>,
        expected: jsonrpc::Result<R::Result>,
    ) where
        R: lsp::request::Request,
        R::Result: Debug + PartialEq,
    {
        let result = self.request(request).await;

        assert_eq!(result, expected);
    }

    /// Start one typed client request without waiting for its result.
    pub(super) async fn start_request<R>(&mut self, request: TestRequest<R>) -> PendingRequest<R>
    where
        R: lsp::request::Request,
    {
        let TestRequest { params, .. } = request;
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
        self.drain_output();
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

        PendingNotification {
            method: N::METHOD,
            call,
        }
    }

    /// Receive diagnostics for one exact document revision.
    pub(super) async fn receive_diagnostics(
        &mut self,
        document: &TestDocument,
        version: impl Into<Option<i32>>,
    ) -> lsp::PublishDiagnosticsParams {
        let version = version.into();
        let mut unmatched = VecDeque::new();

        // select the requested document revision
        loop {
            let message = self.receive().await;
            if message.method() != lsp::notification::PublishDiagnostics::METHOD {
                unmatched.push_back(message);

                continue;
            }

            // decode one diagnostic publication
            let (_, id, params) = message.clone().into_parts();
            assert_eq!(id, None);
            let params = params.unwrap_or_else(|| {
                panic!(
                    "{} omitted its parameters",
                    lsp::notification::PublishDiagnostics::METHOD
                )
            });
            let published = from_value::<lsp::PublishDiagnosticsParams>(params).unwrap();
            if published.uri != document.uri || published.version != version {
                unmatched.push_back(message);

                continue;
            }

            // restore unrelated protocol messages
            unmatched.append(&mut self.messages);
            self.messages = unmatched;

            return published;
        }
    }

    /// Require the user-facing diagnostics published for one document revision.
    pub(super) async fn assert_diagnostics(
        &mut self,
        document: &TestDocument,
        version: impl Into<Option<i32>>,
        expected: Vec<lsp::Diagnostic>,
    ) {
        let mut published = self.receive_diagnostics(document, version).await;

        // omit the private payload used to resolve later code actions
        for diagnostic in &mut published.diagnostics {
            diagnostic.data = None;
        }

        assert_eq!(published.diagnostics, expected);
    }

    /// Require the current diagnostics returned for one document.
    pub(super) async fn assert_document_diagnostics(
        &mut self,
        document: &TestDocument,
        expected: Vec<lsp::Diagnostic>,
    ) {
        let response = self.request(document.diagnostics()).await.unwrap();
        let lsp::DocumentDiagnosticReportResult::Report(lsp::DocumentDiagnosticReport::Full(
            report,
        )) = response
        else {
            panic!("document diagnostics did not return a full report");
        };
        let mut diagnostics = report.full_document_diagnostic_report.items;

        // omit the private payload used to resolve later code actions
        for diagnostic in &mut diagnostics {
            diagnostic.data = None;
        }

        assert_eq!(diagnostics, expected);
    }

    /// Require the server to have sent no further protocol message.
    pub(super) fn assert_no_message(&mut self) {
        if let Some(message) = self.messages.pop_front() {
            panic!("unexpected server message: {message:?}");
        }

        loop {
            match self.requests.next().now_or_never() {
                None => return,
                Some(Some(message)) if self.consume_output(&message) => {}
                Some(Some(message)) => panic!("unexpected server message: {message:?}"),
                Some(None) => panic!("client socket closed"),
            }
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

        self.responses.send(response).await.unwrap();
    }

    /// Return the test workspace root.
    pub(super) fn root(&self) -> &Path {
        self.file_system.root()
    }

    /// Return the test workspace root URI.
    pub(super) fn root_uri(&self) -> lsp::Uri {
        uri(self.root())
    }

    /// Build one project-qualified builtin URI.
    pub(super) fn builtin_uri(&self, uri: &str) -> lsp::Uri {
        self.builtin_uri_at(self.root(), uri)
    }

    /// Build one project-qualified builtin URI for an exact root.
    pub(super) fn builtin_uri_at(&self, root: &Path, uri: &str) -> lsp::Uri {
        let project = ProjectId::from_root(root);
        let source = Uri::from_string(uri);

        project.qualify(&source).unwrap()
    }

    /// Build one document identity from a scoped workspace path.
    pub(super) fn document(&self, path: impl AsRef<Path>) -> TestDocument {
        let path = self.file_system.path_for(path);

        TestDocument::from(path.as_path())
    }

    /// Receive one server initiated protocol message.
    async fn receive(&mut self) -> jsonrpc::Request {
        loop {
            if let Some(message) = self.messages.pop_front() {
                return message;
            }

            let message = tokio::time::timeout(CLIENT_MESSAGE_TIMEOUT, self.requests.next())
                .await
                .unwrap()
                .unwrap();
            if !self.consume_output(&message) {
                return message;
            }
        }
    }

    /// Drain completed trace output without consuming protocol messages.
    fn drain_output(&mut self) {
        loop {
            let Some(message) = self.requests.next().now_or_never().flatten() else {
                return;
            };
            if !self.consume_output(&message) {
                self.messages.push_back(message);
            }
        }
    }

    /// Consume one server log or trace record.
    fn consume_output(&self, message: &jsonrpc::Request) -> bool {
        if Self::is_log_record(message) {
            return true;
        }
        if message.method() != lsp::notification::LogTrace::METHOD {
            return false;
        }

        let params = message
            .params()
            .cloned()
            .unwrap_or_else(|| panic!("{} omitted its parameters", message.method()));
        let params = from_value::<lsp::LogTraceParams>(params).unwrap();
        if params.message.starts_with("event=") && !params.message.starts_with("event=lsp.") {
            println!("\n{}", params.message);
            if let Some(report) = params.verbose {
                println!("{report}");
            }
        }

        true
    }

    /// Return the fixture trace level selected by the process environment.
    fn trace_value() -> Option<lsp::TraceValue> {
        env::var_os(TIMINGS_ENV)
            .is_some_and(|value| !value.is_empty() && value != "0")
            .then_some(lsp::TraceValue::Messages)
    }

    /// Return whether one message is a structured informational log record.
    fn is_log_record(message: &jsonrpc::Request) -> bool {
        if message.method() != lsp::notification::LogMessage::METHOD {
            return false;
        }
        let params = message
            .params()
            .cloned()
            .unwrap_or_else(|| panic!("{} omitted its parameters", message.method()));
        let params = from_value::<lsp::LogMessageParams>(params).unwrap();

        params.typ == lsp::MessageType::INFO && params.message.starts_with("event=")
    }

    /// Start one server and continuously receive its client messages.
    fn start(file_system: TemporaryPhysicalFileSystem) -> Self {
        // create isolated process capabilities
        let mut environment = Environment::capture_process();
        environment.cwd = Some(file_system.root().to_path_buf());
        let library = TestLibrary::shared();
        let host = library
            .revision
            .repository()
            .host()
            .clone()
            .with_environment(environment);
        let executor = library.session.executor();

        // create one server over the isolated host
        let (service, socket) = LspService::new(move |client| {
            TsppLanguageServer::new(client, host.clone(), executor.clone())
        });
        let (mut requests, responses) = socket.split();
        let (sender, received) = unbounded();

        // consume the bounded protocol queue independently from server calls
        drop(tokio::spawn(async move {
            while let Some(request) = requests.next().await {
                if sender.unbounded_send(request).is_err() {
                    return;
                }
            }
        }));

        Self {
            service,
            requests: received,
            responses,
            messages: VecDeque::new(),
            next_request_id: 1,
            file_system,
        }
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

impl From<lsp::Uri> for TestDocument {
    /// Build one test document from its exact LSP URI.
    fn from(uri: lsp::Uri) -> Self {
        Self { uri }
    }
}

impl TestDocument {
    /// Return the LSP document URI.
    pub(super) fn uri(&self) -> &lsp::Uri {
        &self.uri
    }

    /// Build a location in this document.
    pub(super) fn location(&self, range: lsp::Range) -> lsp::Location {
        lsp::Location {
            uri: self.uri.clone(),
            range,
        }
    }

    /// Link an origin selection to a declaration in this document.
    pub(super) fn link(
        &self,
        origin: lsp::Range,
        range: lsp::Range,
        selection: lsp::Range,
    ) -> lsp::LocationLink {
        lsp::LocationLink {
            origin_selection_range: Some(origin),
            target_uri: self.uri.clone(),
            target_range: range,
            target_selection_range: selection,
        }
    }

    /// Build one exact error diagnostic for this document.
    pub(super) fn error(&self, range: lsp::Range, code: &str, message: &str) -> lsp::Diagnostic {
        lsp::Diagnostic {
            range,
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            code: Some(lsp::NumberOrString::String(code.to_string())),
            source: Some("tspp".to_string()),
            message: message.to_string(),
            ..lsp::Diagnostic::default()
        }
    }

    /// Build one exact related location in this document.
    pub(super) fn related(
        &self,
        range: lsp::Range,
        message: &str,
    ) -> lsp::DiagnosticRelatedInformation {
        lsp::DiagnosticRelatedInformation {
            location: self.location(range),
            message: message.to_string(),
        }
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
                language_id: "tspp".to_string(),
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

    /// Build a hover request for this document.
    pub(super) fn hover(&self, position: lsp::Position) -> TestRequest<lsp::request::HoverRequest> {
        TestRequest::new(lsp::HoverParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a definition request for this document.
    pub(super) fn definition(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::GotoDefinition> {
        TestRequest::new(self.goto(position))
    }

    /// Build a declaration request for this document.
    pub(super) fn declaration(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::GotoDeclaration> {
        TestRequest::new(self.goto(position))
    }

    /// Build a type definition request for this document.
    pub(super) fn type_definition(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::GotoTypeDefinition> {
        TestRequest::new(self.goto(position))
    }

    /// Build an implementation request for this document.
    pub(super) fn implementations(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::GotoImplementation> {
        TestRequest::new(self.goto(position))
    }

    /// Build a reference request for this document.
    pub(super) fn references(
        &self,
        position: lsp::Position,
        include_declaration: bool,
    ) -> TestRequest<lsp::request::References> {
        TestRequest::new(lsp::ReferenceParams {
            text_document_position: self.position(position),
            context: lsp::ReferenceContext {
                include_declaration,
            },
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a document highlight request for this document.
    pub(super) fn highlights(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::DocumentHighlightRequest> {
        TestRequest::new(lsp::DocumentHighlightParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a call hierarchy preparation request for this document.
    pub(super) fn call_hierarchy(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::CallHierarchyPrepare> {
        TestRequest::new(lsp::CallHierarchyPrepareParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a type hierarchy preparation request for this document.
    pub(super) fn type_hierarchy(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::TypeHierarchyPrepare> {
        TestRequest::new(lsp::TypeHierarchyPrepareParams {
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a completion request for this document.
    pub(super) fn completion(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::Completion> {
        TestRequest::new(lsp::CompletionParams {
            text_document_position: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
            context: None,
        })
    }

    /// Build a signature help request for this document.
    pub(super) fn signature_help(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::SignatureHelpRequest> {
        TestRequest::new(lsp::SignatureHelpParams {
            context: None,
            text_document_position_params: self.position(position),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a rename request for this document.
    pub(super) fn rename(
        &self,
        position: lsp::Position,
        new_name: &str,
    ) -> TestRequest<lsp::request::Rename> {
        TestRequest::new(lsp::RenameParams {
            text_document_position: self.position(position),
            new_name: new_name.to_string(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a rename preparation request for this document.
    pub(super) fn prepare_rename(
        &self,
        position: lsp::Position,
    ) -> TestRequest<lsp::request::PrepareRenameRequest> {
        TestRequest::new(self.position(position))
    }

    /// Build a whole document formatting request.
    pub(super) fn format(&self) -> TestRequest<lsp::request::Formatting> {
        TestRequest::new(lsp::DocumentFormattingParams {
            text_document: self.identifier(),
            options: lsp::FormattingOptions::default(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a range formatting request.
    pub(super) fn format_range(
        &self,
        range: lsp::Range,
    ) -> TestRequest<lsp::request::RangeFormatting> {
        TestRequest::new(lsp::DocumentRangeFormattingParams {
            text_document: self.identifier(),
            range,
            options: lsp::FormattingOptions::default(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build an on type formatting request.
    pub(super) fn format_on_type(
        &self,
        position: lsp::Position,
        character: &str,
    ) -> TestRequest<lsp::request::OnTypeFormatting> {
        TestRequest::new(lsp::DocumentOnTypeFormattingParams {
            text_document_position: self.position(position),
            ch: character.to_string(),
            options: lsp::FormattingOptions::default(),
        })
    }

    /// Build a semantic token request for this document.
    pub(super) fn semantic_tokens(&self) -> TestRequest<lsp::request::SemanticTokensFullRequest> {
        TestRequest::new(lsp::SemanticTokensParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a semantic token request for one document range.
    pub(super) fn semantic_tokens_range(
        &self,
        range: lsp::Range,
    ) -> TestRequest<lsp::request::SemanticTokensRangeRequest> {
        TestRequest::new(lsp::SemanticTokensRangeParams {
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
            text_document: self.identifier(),
            range,
        })
    }

    /// Build an outline request for this document.
    pub(super) fn outline(&self) -> TestRequest<lsp::request::DocumentSymbolRequest> {
        TestRequest::new(lsp::DocumentSymbolParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a folding range request for this document.
    pub(super) fn folding_ranges(&self) -> TestRequest<lsp::request::FoldingRangeRequest> {
        TestRequest::new(lsp::FoldingRangeParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a selection range request for this document.
    pub(super) fn selection_ranges(
        &self,
        positions: impl IntoIterator<Item = lsp::Position>,
    ) -> TestRequest<lsp::request::SelectionRangeRequest> {
        TestRequest::new(lsp::SelectionRangeParams {
            text_document: self.identifier(),
            positions: positions.into_iter().collect(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a current document diagnostic request.
    pub(super) fn diagnostics(&self) -> TestRequest<lsp::request::DocumentDiagnosticRequest> {
        TestRequest::new(lsp::DocumentDiagnosticParams {
            text_document: self.identifier(),
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build an inlay hint request for one document range.
    pub(super) fn inlay_hints(
        &self,
        range: lsp::Range,
    ) -> TestRequest<lsp::request::InlayHintRequest> {
        TestRequest::new(lsp::InlayHintParams {
            text_document: self.identifier(),
            range,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        })
    }

    /// Build a document link request for this document.
    pub(super) fn links(&self) -> TestRequest<lsp::request::DocumentLinkRequest> {
        TestRequest::new(lsp::DocumentLinkParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a code action request for this document.
    pub(super) fn code_actions(
        &self,
        range: lsp::Range,
        context: lsp::CodeActionContext,
    ) -> TestRequest<lsp::request::CodeActionRequest> {
        TestRequest::new(lsp::CodeActionParams {
            text_document: self.identifier(),
            range,
            context,
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build a code lens request for this document.
    pub(super) fn code_lenses(&self) -> TestRequest<lsp::request::CodeLensRequest> {
        TestRequest::new(lsp::CodeLensParams {
            text_document: self.identifier(),
            work_done_progress_params: lsp::WorkDoneProgressParams::default(),
            partial_result_params: lsp::PartialResultParams::default(),
        })
    }

    /// Build goto parameters for this document.
    fn goto(&self, position: lsp::Position) -> lsp::GotoDefinitionParams {
        lsp::GotoDefinitionParams {
            text_document_position_params: self.position(position),
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
        let response = tokio::time::timeout(SERVER_CALL_TIMEOUT, self.call)
            .await
            .unwrap_or_else(|_| panic!("{} timed out", R::METHOD))
            .unwrap()
            .unwrap()
            .unwrap_or_else(|| {
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
    /// Notification method being evaluated.
    method: &'static str,
    /// Running server call.
    call: ServerCall,
}

impl PendingNotification {
    /// Wait until the server finishes handling the notification.
    pub(super) async fn wait(self) {
        let method = self.method;
        let response = tokio::time::timeout(SERVER_CALL_TIMEOUT, self.call)
            .await
            .unwrap_or_else(|_| panic!("{method} timed out"))
            .unwrap()
            .unwrap();

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
