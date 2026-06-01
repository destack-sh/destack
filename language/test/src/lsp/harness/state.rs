use std::collections::{HashMap, VecDeque};

use destack_lsp_server::jsonrpc::Response;
use destack_lsp_types as lsp;

use crate::lsp::{LspDriver, LspFixture, Marker, Range};

/// One mutable applied-LSP test state.
#[derive(Debug)]
pub struct LspTestState {
    /// The parsed fixture that owns files, markers, and ranges.
    pub fixture: LspFixture,
    /// The lower-level in-process LSP driver.
    pub driver: LspDriver,
    /// The editor-facing state for active documents, selection, and caret position.
    pub editor: EditorState,
    /// The buffered protocol notifications observed by the harness.
    pub notifications: NotificationBuffer,
    /// The tracked in-flight requests and cancellation policy.
    pub requests: RequestTracker,
    /// The mutable client-side formatting configuration.
    pub formatting: FormattingState,
}

/// One editor caret and selection snapshot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EditorState {
    /// The active file path when one document is focused.
    pub active_file_path: Option<String>,
    /// The current caret byte offset in the active file.
    pub caret_offset: usize,
    /// The current selection start byte offset when a selection exists.
    pub selection_start: Option<usize>,
    /// The current selection end byte offset when a selection exists.
    pub selection_end: Option<usize>,
    /// Open documents keyed by fixture-relative file path.
    pub open_documents: HashMap<String, OpenDocumentState>,
}

/// One open document overlay tracked by the client harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenDocumentState {
    /// The fixture-relative file path.
    pub file_path: String,
    /// The current client-visible text.
    pub text: String,
    /// The current LSP document version.
    pub version: i32,
}

/// One tracked asynchronous request in the applied harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequest {
    /// The protocol request id.
    pub id: i64,
    /// The protocol method name.
    pub method: String,
    /// The work-done progress token when the request carries one.
    pub work_done_token: Option<lsp::ProgressToken>,
}

/// One buffered notification stream snapshot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NotificationBuffer {
    /// Buffered notification method names in arrival order.
    pub methods: VecDeque<String>,
}

/// One tracked request and cancellation state snapshot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RequestTracker {
    /// Pending protocol requests keyed by request id.
    pub pending: HashMap<i64, PendingRequest>,
    /// The pending automatic cancellation policy.
    pub cancellation: CancellationState,
}

/// One fixture text span in byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    /// The byte start offset.
    pub start: usize,
    /// The byte length.
    pub length: usize,
}

/// One pending cancellation policy for the next started requests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CancellationState {
    /// The remaining request-start checkpoints before automatic cancellation.
    pub requests_until_cancel: Option<usize>,
}

/// One mutable formatting configuration snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct FormattingState {
    /// The current client-side formatting options.
    pub options: lsp::FormattingOptions,
    /// Whether format requests should currently apply edits.
    pub is_enabled: bool,
}

impl LspTestState {
    /// Create one test state and materialize the fixture workspace.
    pub fn from_fixture(prefix: &str, fixture: &LspFixture) -> Result<Self, String> {
        // bootstrap the driver before any editor state is populated
        let driver = LspDriver::from_fixture(prefix, fixture)?;

        Ok(Self {
            fixture: fixture.clone(),
            driver,
            editor: EditorState {
                active_file_path: None,
                caret_offset: 0,
                selection_start: None,
                selection_end: None,
                open_documents: HashMap::new(),
            },
            notifications: NotificationBuffer::default(),
            requests: RequestTracker {
                pending: HashMap::new(),
                cancellation: CancellationState {
                    requests_until_cancel: None,
                },
            },
            formatting: FormattingState {
                options: lsp::FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    properties: HashMap::new(),
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                },
                is_enabled: true,
            },
        })
    }

    /// Return the materialized workspace root.
    pub fn workspace_root(&self) -> &std::path::Path {
        self.driver.workspace_root()
    }

    /// Return the current active file path.
    pub fn active_file_path(&self) -> Option<&str> {
        self.editor.active_file_path.as_deref()
    }

    /// Return whether one file currently has an open editor overlay.
    pub fn is_file_open(&self, file_path: &str) -> bool {
        self.editor.open_documents.contains_key(file_path)
    }

    /// Return one named marker from the loaded fixture.
    pub fn marker(&self, marker_name: &str) -> Option<&Marker> {
        self.fixture.marker(marker_name)
    }

    /// Return all marker names from the loaded fixture in source order.
    pub fn marker_names(&self) -> Vec<String> {
        self.fixture.markers.keys().cloned().collect()
    }

    /// Return all named markers from the loaded fixture in source order.
    pub fn markers(&self) -> Vec<&Marker> {
        self.fixture.markers.values().collect()
    }

    /// Return all parsed ranges from the loaded fixture in source order.
    pub fn ranges(&self) -> &[Range] {
        &self.fixture.ranges
    }

    /// Return parsed text spans for one file, or every file when no file is provided.
    pub fn spans(&self, file_path: Option<&str>) -> Vec<TextSpan> {
        self.ranges_in_file(file_path)
            .into_iter()
            .map(|range| TextSpan {
                start: range.start_offset,
                length: range.end_offset - range.start_offset,
            })
            .collect()
    }

    /// Return all parsed ranges for one file, or every range when no file is provided.
    pub fn ranges_in_file(&self, file_path: Option<&str>) -> Vec<&Range> {
        self.fixture
            .ranges
            .iter()
            .filter(|range| file_path.is_none_or(|file_path| range.file_path == file_path))
            .collect()
    }

    /// Return one parsed range with matching text from the loaded fixture.
    pub fn range_by_text(&self, text: &str) -> Option<&Range> {
        self.fixture.range_by_text(text)
    }

    /// Return all parsed ranges grouped by stripped text.
    pub fn ranges_by_text(&self) -> HashMap<String, Vec<&Range>> {
        let mut ranges = HashMap::<String, Vec<&Range>>::new();

        // group ranges exactly by stripped text for marker based lookup
        for range in &self.fixture.ranges {
            ranges.entry(range.text.clone()).or_default().push(range);
        }

        ranges
    }

    /// Open every file declared by the fixture.
    pub fn open_fixture_files(&mut self) -> Result<(), String> {
        // open files in fixture order so diagnostics arrive deterministically
        let file_paths = self
            .fixture
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        for file_path in file_paths {
            self.open_file(&file_path)?;
        }

        Ok(())
    }

    /// Open one file with explicit overlay text and make it the active document.
    pub fn open_file_with_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        // require a real reopen so open-text steps model one actual didOpen path
        if self.editor.open_documents.contains_key(file_path) {
            return Err(format!(
                "cannot open overlay text for already open file {file_path}"
            ));
        }

        let version = 1;

        self.driver.open_file_with_text(file_path, text, version)?;
        self.editor.open_documents.insert(
            file_path.to_string(),
            OpenDocumentState {
                file_path: file_path.to_string(),
                text: text.to_string(),
                version,
            },
        );
        self.editor.active_file_path = Some(file_path.to_string());
        self.editor.caret_offset = 0;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Focus one already-open file without changing its text.
    pub fn go_to_file(&mut self, file_path: &str) -> Result<(), String> {
        self.focus_offset(file_path, 0)
    }

    /// Focus the caret at one named marker.
    pub fn go_to_marker(&mut self, marker_name: &str) -> Result<(), String> {
        let marker = self
            .marker(marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{marker_name}*/ marker"))?;

        self.focus_offset(&marker.file_path, marker.offset)
    }

    /// Visit each named marker in source order.
    pub fn go_to_each_marker<F>(&mut self, mut visit: F) -> Result<(), String>
    where
        F: FnMut(&mut Self, &Marker, usize) -> Result<(), String>,
    {
        let markers = self.markers().into_iter().cloned().collect::<Vec<_>>();

        // re-focus each marker before invoking the caller-provided action
        for (index, marker) in markers.iter().enumerate() {
            self.go_to_marker(&marker.name)?;
            visit(self, marker, index)?;
        }

        Ok(())
    }

    /// Visit a named marker subset in the provided order.
    pub fn go_to_each_named_marker<F>(
        &mut self,
        marker_names: &[&str],
        mut visit: F,
    ) -> Result<(), String>
    where
        F: FnMut(&mut Self, &Marker, usize) -> Result<(), String>,
    {
        let markers = marker_names
            .iter()
            .map(|marker_name| {
                self.marker(marker_name)
                    .cloned()
                    .ok_or_else(|| format!("fixture is missing /*{marker_name}*/ marker"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // re-focus each requested marker before invoking the caller-provided action
        for (index, marker) in markers.iter().enumerate() {
            self.go_to_marker(&marker.name)?;
            visit(self, marker, index)?;
        }

        Ok(())
    }

    /// Focus the caret at the start of one parsed range by stripped text.
    pub fn go_to_range_start(&mut self, text: &str) -> Result<(), String> {
        let range = self
            .range_by_text(text)
            .cloned()
            .ok_or_else(|| format!("fixture is missing range for text {text:?}"))?;

        self.editor.active_file_path = Some(range.file_path);
        self.editor.caret_offset = range.start_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Focus the caret at the end of one parsed range by stripped text.
    pub fn go_to_range_end(&mut self, text: &str) -> Result<(), String> {
        let range = self
            .range_by_text(text)
            .cloned()
            .ok_or_else(|| format!("fixture is missing range for text {text:?}"))?;

        self.editor.active_file_path = Some(range.file_path);
        self.editor.caret_offset = range.end_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Visit each parsed range in source order.
    pub fn go_to_each_range<F>(&mut self, mut visit: F) -> Result<(), String>
    where
        F: FnMut(&mut Self, &Range, usize) -> Result<(), String>,
    {
        let ranges = self.ranges().to_vec();

        // re-focus each range start before invoking the caller-provided action
        for (index, range) in ranges.iter().enumerate() {
            self.focus_offset(&range.file_path, range.start_offset)?;
            visit(self, range, index)?;
        }

        Ok(())
    }

    /// Focus the caret at the beginning of the active file.
    pub fn go_to_bof(&mut self) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.focus_offset(&file_path, 0)
    }

    /// Focus the caret at the end of the active file.
    pub fn go_to_eof(&mut self) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;

        self.focus_offset(&file_path, document.text.len())
    }

    /// Focus one file and place the caret at a byte offset.
    pub fn go_to_position(&mut self, offset: usize, file_path: Option<&str>) -> Result<(), String> {
        let file_path = file_path
            .map(str::to_string)
            .or_else(|| self.editor.active_file_path.clone())
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.go_to_offset(&file_path, offset)
    }

    /// Focus one file and place the caret at a byte offset.
    pub fn go_to_offset(&mut self, file_path: &str, offset: usize) -> Result<(), String> {
        self.focus_offset(file_path, offset)
    }

    /// Select text between two named markers in the same file.
    pub fn select(&mut self, start_marker_name: &str, end_marker_name: &str) -> Result<(), String> {
        self.select_markers(start_marker_name, end_marker_name)
    }

    /// Select text between two named markers in the same file.
    pub fn select_markers(
        &mut self,
        start_marker_name: &str,
        end_marker_name: &str,
    ) -> Result<(), String> {
        let start_marker = self
            .marker(start_marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{start_marker_name}*/ marker"))?;
        let end_marker = self
            .marker(end_marker_name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{end_marker_name}*/ marker"))?;
        if start_marker.file_path != end_marker.file_path {
            return Err(format!(
                "cannot select across files: {} and {}",
                start_marker.file_path, end_marker.file_path
            ));
        }

        self.select_offsets(
            &start_marker.file_path,
            start_marker.offset,
            end_marker.offset,
        )
    }

    /// Select text between two byte offsets in one file.
    pub fn select_offsets_in_file(
        &mut self,
        file_path: &str,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<(), String> {
        self.select_offsets(file_path, start_offset, end_offset)
    }

    /// Select one parsed range.
    pub fn select_range(&mut self, range: &Range) -> Result<(), String> {
        self.select_offsets(&range.file_path, range.start_offset, range.end_offset)
    }

    /// Select one parsed fixture range by stripped text.
    pub fn select_range_by_text(&mut self, text: &str) -> Result<(), String> {
        let range = self
            .range_by_text(text)
            .cloned()
            .ok_or_else(|| format!("fixture is missing range for text {text:?}"))?;

        self.select_offsets(&range.file_path, range.start_offset, range.end_offset)
    }

    /// Select the full contents of one open file.
    pub fn select_all_in_file(&mut self, file_path: &str) -> Result<(), String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot select unopened file {file_path}"))?;

        self.select_offsets(file_path, 0, document.text.len())
    }

    /// Return the current active file path and protocol position.
    pub fn current_position(&self) -> Result<(String, lsp::Position), String> {
        // require one active file before issuing position-based requests
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let position = position_for_offset(&document.text, self.editor.caret_offset)?;

        Ok((file_path, position))
    }

    /// Request goto definition at the current caret.
    pub fn request_definition(&mut self) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.goto_definition(&file_path, position)
    }

    /// Request goto declaration at the current caret.
    pub fn request_declaration(&mut self) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.goto_declaration(&file_path, position)
    }

    /// Request goto type definition at the current caret.
    pub fn request_type_definition(
        &mut self,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.goto_type_definition(&file_path, position)
    }

    /// Request goto implementation at the current caret.
    pub fn request_implementation(
        &mut self,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.goto_implementation(&file_path, position)
    }

    /// Request references at the current caret.
    pub fn request_references(
        &mut self,
        include_declaration: bool,
    ) -> Result<Option<Vec<lsp::Location>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver
            .find_references(&file_path, position, include_declaration)
    }

    /// Request hover at the current caret.
    pub fn request_hover(&mut self) -> Result<Option<lsp::Hover>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.hover(&file_path, position)
    }

    /// Request document highlights at the current caret.
    pub fn request_document_highlights(
        &mut self,
    ) -> Result<Option<Vec<lsp::DocumentHighlight>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.document_highlights(&file_path, position)
    }

    /// Request signature help at the current caret.
    pub fn request_signature_help(&mut self) -> Result<Option<lsp::SignatureHelp>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.signature_help(&file_path, position, None)
    }

    /// Request signature help at the current caret with explicit trigger context.
    pub fn request_signature_help_with_context(
        &mut self,
        context: lsp::SignatureHelpContext,
    ) -> Result<Option<lsp::SignatureHelp>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver
            .signature_help(&file_path, position, Some(context))
    }

    /// Request document symbols for the current active file.
    pub fn request_document_symbols(
        &mut self,
    ) -> Result<Option<lsp::DocumentSymbolResponse>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver.document_symbols(&file_path)
    }

    /// Request selection ranges at the current caret.
    pub fn request_selection_ranges(&mut self) -> Result<Option<Vec<lsp::SelectionRange>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.selection_ranges(&file_path, vec![position])
    }

    /// Start selection ranges with partial progress at the current caret.
    pub fn start_selection_ranges_with_partial_progress(
        &mut self,
        partial_result_token: lsp::ProgressToken,
    ) -> Result<i64, String> {
        let (file_path, position) = self.current_position()?;
        let request_id = self.driver.start_request(
            "textDocument/selectionRange",
            lsp::SelectionRangeParams {
                text_document: lsp::TextDocumentIdentifier::new(self.driver.uri_for(&file_path)),
                positions: vec![position],
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: Some(partial_result_token),
                },
            },
        )?;

        self.requests.pending.insert(
            request_id,
            PendingRequest {
                id: request_id,
                method: "textDocument/selectionRange".to_string(),
                work_done_token: None,
            },
        );

        Ok(request_id)
    }

    /// Request call hierarchy prepare items at the current caret.
    pub fn request_prepare_call_hierarchy(
        &mut self,
    ) -> Result<Option<Vec<lsp::CallHierarchyItem>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.prepare_call_hierarchy(&file_path, position)
    }

    /// Request incoming call hierarchy edges for one prepared item.
    pub fn request_call_hierarchy_incoming(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Result<Option<Vec<lsp::CallHierarchyIncomingCall>>, String> {
        self.driver.call_hierarchy_incoming(item)
    }

    /// Request outgoing call hierarchy edges for one prepared item.
    pub fn request_call_hierarchy_outgoing(
        &mut self,
        item: lsp::CallHierarchyItem,
    ) -> Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>, String> {
        self.driver.call_hierarchy_outgoing(item)
    }

    /// Request type hierarchy prepare items at the current caret.
    pub fn request_prepare_type_hierarchy(
        &mut self,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.prepare_type_hierarchy(&file_path, position)
    }

    /// Request type hierarchy supertypes for one prepared item.
    pub fn request_type_hierarchy_supertypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        self.driver.type_hierarchy_supertypes(item)
    }

    /// Request type hierarchy subtypes for one prepared item.
    pub fn request_type_hierarchy_subtypes(
        &mut self,
        item: lsp::TypeHierarchyItem,
    ) -> Result<Option<Vec<lsp::TypeHierarchyItem>>, String> {
        self.driver.type_hierarchy_subtypes(item)
    }

    /// Request workspace symbols for one search query.
    pub fn request_workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>, String>
    {
        self.driver.workspace_symbols(query)
    }

    /// Request completion items at the current caret.
    pub fn request_completion(&mut self) -> Result<Option<lsp::CompletionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.completion(&file_path, position)
    }

    /// Resolve one previously returned completion item.
    pub fn resolve_completion(
        &mut self,
        item: lsp::CompletionItem,
    ) -> Result<Option<lsp::CompletionItem>, String> {
        self.driver.completion_resolve(item)
    }

    /// Request folding ranges for the current active file.
    pub fn request_folding_ranges(&mut self) -> Result<Option<Vec<lsp::FoldingRange>>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver.folding_ranges(&file_path)
    }

    /// Request document links for the current active file.
    pub fn request_document_links(&mut self) -> Result<Option<Vec<lsp::DocumentLink>>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver.document_links(&file_path)
    }

    /// Request whole-document formatting edits for the current active file.
    pub fn request_document_formatting(&mut self) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver
            .document_formatting(&file_path, self.formatting.options.clone())
    }

    /// Request range formatting edits for one parsed fixture range.
    pub fn request_range_formatting_for_range(
        &mut self,
        range: &Range,
    ) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        self.driver.range_formatting(
            &range.file_path,
            lsp::Range {
                start: lsp::Position {
                    line: range.start_line as u32,
                    character: range.start_character as u32,
                },
                end: lsp::Position {
                    line: range.end_line as u32,
                    character: range.end_character as u32,
                },
            },
            self.formatting.options.clone(),
        )
    }

    /// Request on-type formatting edits at the current caret.
    pub fn request_on_type_formatting(
        &mut self,
        ch: &str,
    ) -> Result<Option<Vec<lsp::TextEdit>>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver
            .on_type_formatting(&file_path, position, ch, self.formatting.options.clone())
    }

    /// Request inlay hints for the full active file range.
    pub fn request_inlay_hints(&mut self) -> Result<Option<Vec<lsp::InlayHint>>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let end_position = position_for_offset(&document.text, document.text.len())?;
        let range = lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: end_position,
        };

        self.driver.inlay_hints(&file_path, range)
    }

    /// Return a copy of the current client-side formatting options.
    pub fn formatting_options(&self) -> lsp::FormattingOptions {
        self.formatting.options.clone()
    }

    /// Replace the full client-side formatting options.
    pub fn set_formatting_options(&mut self, options: lsp::FormattingOptions) {
        self.formatting.options = options;
    }

    /// Focus one open file at the provided byte offset and clear any selection.
    fn focus_offset(&mut self, file_path: &str, offset: usize) -> Result<(), String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot focus unopened file {file_path}"))?;
        if offset > document.text.len() {
            return Err(format!(
                "caret offset {offset} exceeds document length {}",
                document.text.len()
            ));
        }

        self.editor.active_file_path = Some(file_path.to_string());
        self.editor.caret_offset = offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Select one byte range in one open file and place the caret at the end.
    fn select_offsets(
        &mut self,
        file_path: &str,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<(), String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot select unopened file {file_path}"))?;
        if start_offset > end_offset {
            return Err(format!(
                "selection start {start_offset} exceeds end {end_offset}"
            ));
        }
        if end_offset > document.text.len() {
            return Err(format!(
                "selection end {end_offset} exceeds document length {}",
                document.text.len()
            ));
        }

        self.editor.active_file_path = Some(file_path.to_string());
        self.editor.caret_offset = end_offset;
        self.editor.selection_start = Some(start_offset);
        self.editor.selection_end = Some(end_offset);

        Ok(())
    }

    /// Request code actions for one zero-width range at the current caret.
    pub fn request_code_actions(&mut self) -> Result<Option<lsp::CodeActionResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.code_actions(
            &file_path,
            lsp::Range {
                start: position,
                end: position,
            },
        )
    }

    /// Request full semantic tokens for the current active file.
    pub fn request_semantic_tokens_full(
        &mut self,
    ) -> Result<Option<lsp::SemanticTokensResult>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver.semantic_tokens_full(&file_path)
    }

    /// Request semantic tokens delta for the current active file.
    pub fn request_semantic_tokens_full_delta(
        &mut self,
        previous_result_id: String,
    ) -> Result<Option<lsp::SemanticTokensFullDeltaResult>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver
            .semantic_tokens_full_delta(&file_path, previous_result_id)
    }

    /// Request semantic tokens for one parsed fixture range.
    pub fn request_semantic_tokens_range_for_range(
        &mut self,
        range: &Range,
    ) -> Result<Option<lsp::SemanticTokensRangeResult>, String> {
        self.driver.semantic_tokens_range(
            &range.file_path,
            lsp::Range {
                start: lsp::Position {
                    line: range.start_line as u32,
                    character: range.start_character as u32,
                },
                end: lsp::Position {
                    line: range.end_line as u32,
                    character: range.end_character as u32,
                },
            },
        )
    }

    /// Resolve one previously returned code action.
    pub fn resolve_code_action(
        &mut self,
        action: lsp::CodeAction,
    ) -> Result<Option<lsp::CodeAction>, String> {
        self.driver.code_action_resolve(action)
    }

    /// Request code lenses for the current active file.
    pub fn request_code_lenses(&mut self) -> Result<Option<Vec<lsp::CodeLens>>, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.driver.code_lenses(&file_path)
    }

    /// Resolve one previously returned code lens.
    pub fn resolve_code_lens(
        &mut self,
        lens: lsp::CodeLens,
    ) -> Result<Option<lsp::CodeLens>, String> {
        self.driver.code_lens_resolve(lens)
    }

    /// Request rename edits at the current caret.
    pub fn request_rename(&mut self, new_name: &str) -> Result<Option<lsp::WorkspaceEdit>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.rename(&file_path, position, new_name)
    }

    /// Request prepare rename at the current caret.
    pub fn request_prepare_rename(&mut self) -> Result<Option<lsp::PrepareRenameResponse>, String> {
        let (file_path, position) = self.current_position()?;

        self.driver.prepare_rename(&file_path, position)
    }

    /// Request document diagnostics for one fixture-relative file path.
    pub fn request_document_diagnostics(
        &mut self,
        file_path: &str,
    ) -> Result<lsp::DocumentDiagnosticReportResult, String> {
        self.driver.document_diagnostic_for_file(file_path)
    }

    /// Request full workspace diagnostics for the current workspace.
    pub fn request_workspace_diagnostics(
        &mut self,
    ) -> Result<lsp::WorkspaceDiagnosticReportResult, String> {
        self.driver.workspace_diagnostic()
    }

    /// Execute one workspace command and wait until queued mutations become idle.
    pub fn execute_command(&mut self, command: &str) -> Result<Option<lsp::LSPAny>, String> {
        self.driver.execute_command(command)
    }

    /// Wait until queued server mutations become idle.
    pub fn wait_for_mutation_idle(&mut self) {
        self.driver.wait_for_mutation_idle();
    }

    /// Return normalized semantic diagnostics for one file, or the active file when none is provided.
    pub fn semantic_diagnostics(
        &mut self,
        file_path: Option<&str>,
    ) -> Result<Vec<crate::lsp::NormalizedDiagnostic>, String> {
        let file_path = file_path
            .map(str::to_string)
            .or_else(|| self.editor.active_file_path.clone())
            .ok_or_else(|| "no active file is focused".to_string())?;
        let report = self.request_document_diagnostics(&file_path)?;
        let lsp::DocumentDiagnosticReportResult::Report(report) = report else {
            return Err("expected document diagnostic report".to_string());
        };
        let lsp::DocumentDiagnosticReport::Full(full) = report else {
            return Err("expected full document diagnostics".to_string());
        };

        Ok(crate::lsp::normalize_diagnostics(
            &file_path,
            &full.full_document_diagnostic_report.items,
        ))
    }

    /// Return the indentation width of the current line in leading spaces.
    pub fn current_line_indentation(&self) -> Result<usize, String> {
        let line = self.current_line_content()?;

        Ok(line
            .chars()
            .take_while(|character| *character == ' ')
            .count())
    }

    /// Return the indentation width at one file offset in leading spaces.
    pub fn indentation_at_position(&self, file_path: &str, offset: usize) -> Result<usize, String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot inspect unopened file {file_path}"))?;
        let clamped_offset = offset.min(document.text.len());
        let line_start = document
            .text
            .get(..clamped_offset)
            .ok_or_else(|| format!("offset {clamped_offset} is not a char boundary"))?
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let line = document
            .text
            .get(line_start..)
            .and_then(|suffix| suffix.lines().next())
            .ok_or_else(|| "failed to read line at indentation position".to_string())?;

        Ok(line
            .chars()
            .take_while(|character| *character == ' ')
            .count())
    }

    /// Start references with explicit work-done progress tracking.
    pub fn start_references_with_progress(
        &mut self,
        include_declaration: bool,
        work_done_token: lsp::ProgressToken,
    ) -> Result<i64, String> {
        let (file_path, position) = self.current_position()?;
        let request_id = self.driver.start_request(
            "textDocument/references",
            lsp::ReferenceParams {
                text_document_position: lsp::TextDocumentPositionParams {
                    text_document: lsp::TextDocumentIdentifier::new(
                        self.driver.uri_for(&file_path),
                    ),
                    position,
                },
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: Some(work_done_token.clone()),
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: None,
                },
                context: lsp::ReferenceContext {
                    include_declaration,
                },
            },
        )?;

        self.requests.pending.insert(
            request_id,
            PendingRequest {
                id: request_id,
                method: "textDocument/references".to_string(),
                work_done_token: Some(work_done_token.clone()),
            },
        );
        self.apply_cancellation_policy(request_id, Some(work_done_token));

        Ok(request_id)
    }

    /// Start references without work-done progress tracking.
    pub fn start_references_request(&mut self, include_declaration: bool) -> Result<i64, String> {
        let (file_path, position) = self.current_position()?;
        let request_id = self.driver.start_request(
            "textDocument/references",
            lsp::ReferenceParams {
                text_document_position: lsp::TextDocumentPositionParams {
                    text_document: lsp::TextDocumentIdentifier::new(
                        self.driver.uri_for(&file_path),
                    ),
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
        )?;

        self.requests.pending.insert(
            request_id,
            PendingRequest {
                id: request_id,
                method: "textDocument/references".to_string(),
                work_done_token: None,
            },
        );
        self.apply_cancellation_policy(request_id, None);

        Ok(request_id)
    }

    /// Start workspace diagnostics with explicit progress tokens.
    pub fn start_workspace_diagnostics_with_partial_progress(
        &mut self,
        partial_result_token: lsp::ProgressToken,
        work_done_token: Option<lsp::ProgressToken>,
    ) -> Result<i64, String> {
        let request_id = self.driver.start_request(
            "workspace/diagnostic",
            lsp::WorkspaceDiagnosticParams {
                identifier: None,
                previous_result_ids: Vec::new(),
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: work_done_token.clone(),
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: Some(partial_result_token),
                },
            },
        )?;

        self.requests.pending.insert(
            request_id,
            PendingRequest {
                id: request_id,
                method: "workspace/diagnostic".to_string(),
                work_done_token: work_done_token.clone(),
            },
        );
        self.apply_cancellation_policy(request_id, work_done_token);

        Ok(request_id)
    }

    /// Reset the automatic cancellation policy.
    pub fn reset_cancelled(&mut self) {
        self.requests.cancellation.requests_until_cancel = None;
    }

    /// Set the automatic cancellation policy in request-start checkpoints.
    pub fn set_cancelled(&mut self, number_of_calls: usize) {
        self.requests.cancellation.requests_until_cancel = Some(number_of_calls);
    }

    /// Await one previously started request response.
    pub fn await_request(&mut self, request_id: i64) -> Result<Response, String> {
        if self.requests.pending.remove(&request_id).is_none() {
            return Err(format!("request id {request_id} is not pending"));
        }

        self.driver.await_request(request_id)
    }

    /// Send protocol request cancellation for one pending request.
    pub fn cancel_request(&mut self, request_id: i64) {
        self.driver.cancel_request(request_id);
    }

    /// Send work-done progress cancellation for one token.
    pub fn cancel_work_done_progress(&mut self, token: lsp::ProgressToken) {
        self.driver.cancel_work_done_progress(token);
    }

    /// Apply the pending cancellation policy to one newly started request.
    fn apply_cancellation_policy(
        &mut self,
        request_id: i64,
        work_done_token: Option<lsp::ProgressToken>,
    ) {
        let Some(requests_until_cancel) = self.requests.cancellation.requests_until_cancel else {
            return;
        };

        // cancel immediately when the requested checkpoint count is exhausted
        if requests_until_cancel == 0 {
            if let Some(work_done_token) = work_done_token {
                self.cancel_work_done_progress(work_done_token);
            } else {
                self.cancel_request(request_id);
            }

            return;
        }

        // otherwise burn one checkpoint and leave the request running
        self.requests.cancellation.requests_until_cancel = Some(requests_until_cancel - 1);
    }

    /// Wait for the next partial progress payload on one token.
    pub fn wait_for_partial_progress(
        &mut self,
        token: &lsp::ProgressToken,
    ) -> Result<serde_json::Value, String> {
        self.driver.next_partial_progress(token)
    }

    /// Wait for one specific work-done progress kind on one token.
    pub fn wait_for_work_done_progress_kind(
        &mut self,
        token: &lsp::ProgressToken,
        expected_kind: &str,
    ) -> Result<(), String> {
        self.driver
            .wait_for_work_done_progress_kind(token, expected_kind)
    }

    /// Wait for the next pushed diagnostics notification for one file.
    pub fn wait_for_diagnostics_report(
        &mut self,
        file_path: &str,
    ) -> lsp::PublishDiagnosticsParams {
        self.driver.next_diagnostics_for(file_path)
    }

    /// Wait for pushed diagnostics to reach one expected document version.
    pub fn wait_for_diagnostics_version(
        &mut self,
        file_path: &str,
        expected_version: i32,
    ) -> Result<lsp::PublishDiagnosticsParams, String> {
        for _ in 0..8 {
            let diagnostics = self.wait_for_diagnostics_report(file_path);
            if diagnostics.version == Some(expected_version) {
                return Ok(diagnostics);
            }

            if diagnostics
                .version
                .is_some_and(|version| version > expected_version)
            {
                return Err(format!(
                    "diagnostics for {file_path} skipped expected version {expected_version}, got {:?}",
                    diagnostics.version
                ));
            }
        }

        Err(format!(
            "did not observe diagnostics version {expected_version} for {file_path}"
        ))
    }

    /// Wait for the next pushed diagnostics notification for one file.
    pub fn wait_for_diagnostics(&mut self, file_path: &str) -> Vec<lsp::Diagnostic> {
        let diagnostics = self.wait_for_diagnostics_report(file_path);

        diagnostics.diagnostics
    }
}

/// Convert one byte offset inside UTF-8 text into an LSP line and character.
fn position_for_offset(text: &str, offset: usize) -> Result<lsp::Position, String> {
    // reject offsets beyond the current document text
    if offset > text.len() {
        return Err(format!(
            "caret offset {offset} exceeds document length {}",
            text.len()
        ));
    }

    // count line and character positions up to the byte boundary
    let prefix = text
        .get(..offset)
        .ok_or_else(|| format!("caret offset {offset} does not align to a char boundary"))?;
    let mut line = 0_u32;
    let mut character = 0_u32;
    for ch in prefix.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
            continue;
        }

        character += 1;
    }

    Ok(lsp::Position { line, character })
}
