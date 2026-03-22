use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::lsp::fixture::parse::parse_mdtest_case;
use crate::mdtest::MdTestCase;

/// One parsed applied-LSP fixture case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspFixture {
    /// The source fixture path on disk.
    pub path: PathBuf,
    /// The virtual files declared by this fixture.
    pub files: Vec<LspSourceFile>,
    /// The step-indexed workspace snapshots declared by stepped source blocks.
    pub step_snapshots: BTreeMap<usize, LspStepSnapshot>,
    /// The named markers declared across all files.
    pub markers: BTreeMap<String, Marker>,
    /// The ranges declared across all files.
    pub ranges: Vec<Range>,
    /// The explicit ordered step assertions and actions keyed by step index.
    pub step_cases: BTreeMap<usize, Vec<LspStepCase>>,
    /// The capability expectations declared by fixture metadata.
    pub expectations: LspExpectations,
}

/// One grouped expectation payload for a fixture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspExpectations {
    /// The hover expectations declared by fixture metadata.
    pub hover: HoverExpectations,
    /// The signature help expectations declared by fixture metadata.
    pub signature_help: SignatureHelpExpectations,
    /// The symbol expectations declared by fixture metadata.
    pub symbols: SymbolExpectations,
    /// The completion expectations declared by fixture metadata.
    pub completion: CompletionExpectations,
    /// The document-link expectations declared by fixture metadata.
    pub document_links: DocumentLinkExpectations,
    /// The folding-range expectations declared by fixture metadata.
    pub folding_ranges: FoldingRangeExpectations,
    /// The inlay-hint expectations declared by fixture metadata.
    pub inlay_hints: InlayHintExpectations,
    /// The code-lens expectations declared by fixture metadata.
    pub code_lenses: CodeLensExpectations,
    /// The formatting expectations declared by fixture metadata.
    pub formatting: FormattingExpectations,
    /// The code action expectations declared by fixture metadata.
    pub code_actions: CodeActionExpectations,
    /// The semantic token expectations declared by fixture metadata.
    pub semantic_tokens: SemanticTokenExpectations,
    /// The hierarchy expectations declared by fixture metadata.
    pub hierarchy: HierarchyExpectations,
    /// The diagnostic expectations declared by fixture metadata.
    pub diagnostics: DiagnosticExpectations,
    /// The edit expectations declared by fixture metadata.
    pub edits: EditExpectations,
    /// The workspace command expectations declared by fixture metadata.
    pub commands: CommandExpectations,
}

/// One stepped workspace snapshot for a fixture step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspStepSnapshot {
    /// The fixture step index.
    pub index: usize,
    /// The virtual files declared at this step.
    pub files: Vec<LspSourceFile>,
    /// The files removed from the workspace at this step.
    pub removed_file_paths: Vec<String>,
    /// The named markers declared at this step.
    pub markers: BTreeMap<String, Marker>,
    /// The ranges declared at this step.
    pub ranges: Vec<Range>,
}

/// One fully resolved workspace snapshot for a fixture step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedLspStepSnapshot {
    /// The fixture step index.
    pub index: usize,
    /// The full virtual file set visible at this step.
    pub files: Vec<LspSourceFile>,
    /// The full marker set visible at this step.
    pub markers: BTreeMap<String, Marker>,
    /// The full range set visible at this step.
    pub ranges: Vec<Range>,
}

/// One explicit step-owned action or assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspStepCase {
    /// Open one file from disk before later step assertions run.
    OpenFile { file_path: String },
    /// Focus one named marker before later step assertions run.
    GoToMarker { marker_name: String },
    /// Select the span between two named markers before later step assertions run.
    SelectMarkers {
        start_marker_name: String,
        end_marker_name: String,
    },
    /// Select the full contents of one file before later step assertions run.
    SelectAll { file_path: String },
    /// Move the caret to the beginning of the active file.
    GoToBof,
    /// Move the caret right by a codepoint count.
    MoveRight { count: usize },
    /// Replace the current selection before later step assertions run.
    ReplaceSelection { text: String },
    /// Paste text at the caret before later step assertions run.
    Paste { text: String },
    /// Delete codepoints to the left of the caret before later step assertions run.
    Backspace { count: usize },
    /// Delete codepoints at the caret before later step assertions run.
    DeleteAtCaret { count: usize },
    /// Delete one zero-based line before later step assertions run.
    DeleteLine { index: usize },
    /// Delete one inclusive zero-based line range before later step assertions run.
    DeleteLineRange {
        start_index: usize,
        end_index_inclusive: usize,
    },
    /// Replace one zero-based line before later step assertions run.
    ReplaceLine { index: usize, text: String },
    /// Open one file with explicit overlay text before step assertions run.
    OpenText { file_path: String, text: String },
    /// Execute one workspace command before step assertions run.
    ExecuteCommand { command: String },
    /// Save one open file before step assertions run.
    SaveFile { file_path: String },
    /// Close one open file before step assertions run.
    CloseFile { file_path: String },
    /// Verify exact definition locations for one marker at this step.
    Definition {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact references for one marker at this step.
    References {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact quick info for one marker at this step.
    QuickInfo { marker_name: String },
    /// Verify exact indentation at one marker at this step.
    Indentation {
        marker_name: String,
        number_of_spaces: usize,
    },
    /// Verify that signature help stays absent at one marker.
    NoSignatureHelp { marker_name: String },
    /// Verify that trigger-character signature help appears at one marker.
    SignatureHelpTrigger {
        marker_name: String,
        trigger_character: String,
    },
    /// Verify that trigger-reason signature help stays absent at one marker.
    NoSignatureHelpForTriggerReason { marker_name: String },
    /// Verify exact document diagnostics for one file at this step.
    DocumentDiagnostic {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact workspace diagnostics at this step.
    WorkspaceDiagnostic { snapshot_text: String },
    /// Verify exact current file contents for one file at this step.
    CurrentFile { file_path: String, text: String },
    /// Verify exact code actions for one marker at this step.
    CodeAction {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact completion items for one marker at this step.
    Completion {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact resolved completion data for one marker at this step.
    CompletionResolve {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact document links for one file at this step.
    DocumentLink {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact document symbols for one file at this step.
    DocumentSymbols {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact workspace symbols for one query at this step.
    WorkspaceSymbols {
        query: String,
        snapshot_text: String,
    },
    /// Verify exact folding ranges for one file at this step.
    FoldingRange {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact inlay hints for one file at this step.
    InlayHint {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact code lenses for one file at this step.
    CodeLens {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact semantic tokens for one file at this step.
    SemanticTokens {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact semantic-token delta application for one file at this step.
    SemanticTokensDelta {
        file_path: String,
        snapshot_text: String,
    },
    /// Verify exact incoming call hierarchy edges for one marker at this step.
    CallHierarchyIncoming {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact outgoing call hierarchy edges for one marker at this step.
    CallHierarchyOutgoing {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact type hierarchy supertypes for one marker at this step.
    TypeHierarchySupertypes {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify exact type hierarchy subtypes for one marker at this step.
    TypeHierarchySubtypes {
        marker_name: String,
        snapshot_text: String,
    },
    /// Verify that one file has no diagnostics at this step.
    NoErrors { file_path: String },
    /// Format one full file and verify its resulting text.
    FormatDocument { file_path: String },
    /// Format one selected span and verify its resulting text.
    FormatSelection {
        start_marker_name: String,
        end_marker_name: String,
    },
    /// Apply on-type formatting at one marker and verify its resulting text.
    OnTypeFormatting { marker_name: String },
    /// Disable fixture-scoped formatting support.
    DisableFormatting,
    /// Enable fixture-scoped formatting support.
    EnableFormatting,
    /// Set one fixture-scoped formatting option.
    SetFormatOption { name: String, value: String },
    /// Verify the current formatting option state.
    VerifyFormatOptions { snapshot_text: String },
    /// Cancel one references request by request id.
    CancelReferencesRequest { marker_name: String },
    /// Cancel one references request by work-done progress token.
    CancelReferencesProgress { marker_name: String },
    /// Cancel one references request through the auto policy path.
    CancelReferencesRequestByPolicy { marker_name: String },
    /// Cancel one references progress path through the auto policy path.
    CancelReferencesProgressByPolicy { marker_name: String },
}

/// One hover expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HoverExpectations {
    /// The expected hover signature when one fixture declares it.
    pub signature: Option<String>,
    /// The expected hover documentation when one fixture declares it.
    pub documentation: Option<String>,
}

/// One signature-help expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureHelpExpectations {
    /// The expected signature help label when one fixture declares it.
    pub label: Option<String>,
    /// The expected active parameter when one fixture declares it.
    pub active_parameter: Option<usize>,
}

/// One symbol expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolExpectations {
    /// The expected document symbols declared by marker and kind.
    pub document: Vec<ExpectedDocumentSymbol>,
    /// The expected workspace symbol query when one fixture declares it.
    pub workspace_query: Option<String>,
    /// The expected workspace symbols declared by marker and kind.
    pub workspace: Vec<ExpectedWorkspaceSymbol>,
}

/// One completion expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionExpectations {
    /// The expected completion items declared by label and kind.
    pub items: Vec<ExpectedCompletionItem>,
    /// The expected resolved completion label when one fixture declares it.
    pub resolve_label: Option<String>,
    /// The expected resolved completion documentation when one fixture declares it.
    pub resolve_documentation: Option<String>,
}

/// One document-link expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentLinkExpectations {
    /// The expected exact document-link snapshot.
    pub snapshot_text: Option<String>,
}

/// One folding-range expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldingRangeExpectations {
    /// The expected exact folding-range snapshot.
    pub snapshot_text: Option<String>,
}

/// One inlay-hint expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlayHintExpectations {
    /// The expected exact inlay-hint snapshot.
    pub snapshot_text: Option<String>,
}

/// One code-lens expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeLensExpectations {
    /// The expected exact code-lens snapshot.
    pub snapshot_text: Option<String>,
}

/// One formatting expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormattingExpectations {
    /// The expected whole-document formatting snapshot.
    pub document_expected_text: Option<String>,
    /// The expected range-formatting snapshot.
    pub range_expected_text: Option<String>,
    /// The expected current-file snapshot after formatting flows.
    pub current_file_text: Option<String>,
}

/// One code-action expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeActionExpectations {
    /// The expected code actions declared by title, kind, and preference.
    pub actions: Vec<ExpectedCodeAction>,
    /// The expected current-file snapshot after applying the first code action.
    pub expected_text: Option<String>,
    /// Whether lazy code action resolve support should be enabled.
    pub enable_resolve_support: bool,
}

/// One semantic-token expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokenExpectations {
    /// The expected semantic-token snapshot.
    pub full_expected_text: Option<String>,
    /// The expected semantic-token range snapshot.
    pub range_expected_text: Option<String>,
    /// The changed source text used before requesting semantic-token delta.
    pub delta_source_text: Option<String>,
    /// The expected semantic-token snapshot after delta application.
    pub delta_expected_text: Option<String>,
}

/// One hierarchy expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HierarchyExpectations {
    /// The expected incoming call-hierarchy snapshot.
    pub call_incoming_text: Option<String>,
    /// The expected outgoing call-hierarchy snapshot.
    pub call_outgoing_text: Option<String>,
    /// The expected type-hierarchy supertypes snapshot.
    pub type_supertypes_text: Option<String>,
    /// The expected type-hierarchy subtypes snapshot.
    pub type_subtypes_text: Option<String>,
}

/// One diagnostic expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticExpectations {
    /// The expected full workspace-diagnostic snapshot.
    pub workspace_text: Option<String>,
    /// The expected partial workspace-diagnostic snapshot.
    pub workspace_partial_text: Option<String>,
    /// The expected selection-range snapshot.
    pub selection_range_text: Option<String>,
}

/// One edit expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditExpectations {
    /// The expected current-file snapshot after one edit flow.
    pub current_file_text: Option<String>,
}

/// One command expectation block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandExpectations {
    /// The commands that should execute before semantic assertions run.
    pub execute: Vec<String>,
}

/// One virtual file declared inside a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSourceFile {
    /// The fixture-relative file path.
    pub path: String,
    /// The source text after marker and range delimiters are stripped.
    pub text: String,
}

/// One named marker location in a virtual file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// The marker name.
    pub name: String,
    /// The file path that owns the marker.
    pub file_path: String,
    /// The byte offset in the stripped file text.
    pub offset: usize,
    /// The zero-based line index.
    pub line: usize,
    /// The zero-based character index.
    pub character: usize,
}

/// One parsed source range in a virtual file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// The file path that owns the range.
    pub file_path: String,
    /// The byte start offset in the stripped file text.
    pub start_offset: usize,
    /// The byte end offset in the stripped file text.
    pub end_offset: usize,
    /// The zero-based start line index.
    pub start_line: usize,
    /// The zero-based start character index.
    pub start_character: usize,
    /// The zero-based end line index.
    pub end_line: usize,
    /// The zero-based end character index.
    pub end_character: usize,
    /// The stripped text covered by this range.
    pub text: String,
}

/// One expected document symbol declared by fixture metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDocumentSymbol {
    /// The marker that identifies the symbol selection range.
    pub marker_name: String,
    /// The expected symbol kind name.
    pub kind: String,
}

/// One expected workspace symbol declared by fixture metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWorkspaceSymbol {
    /// The marker that identifies the symbol selection range.
    pub marker_name: String,
    /// The expected symbol kind name.
    pub kind: String,
    /// The expected container name when one exists.
    pub container_name: Option<String>,
}

/// One expected completion item declared by fixture metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedCompletionItem {
    /// The expected completion label.
    pub label: String,
    /// The expected completion kind name.
    pub kind: String,
}

/// One expected code action declared by fixture metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedCodeAction {
    /// The expected action title.
    pub title: String,
    /// The expected action kind name.
    pub kind: String,
    /// Whether the action is preferred.
    pub is_preferred: bool,
    /// Whether the action should already carry an edit payload.
    pub has_edit: bool,
}

/// One fixture parsing failure with path and line context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureParseError {
    /// The fixture path that failed to parse.
    pub path: PathBuf,
    /// The one-based fixture line number.
    pub line: usize,
    /// The human-readable failure message.
    pub message: String,
}

impl LspFixture {
    /// Parse one markdown LSP case into the native harness model.
    pub fn from_mdtest(path: &Path, test: &MdTestCase) -> Result<Self, FixtureParseError> {
        parse_mdtest_case(path, test)
    }

    /// Return one fixture file by relative path.
    pub fn file(&self, file_path: &str) -> Option<&LspSourceFile> {
        self.files.iter().find(|file| file.path == file_path)
    }

    /// Return one named marker by name.
    pub fn marker(&self, marker_name: &str) -> Option<&Marker> {
        self.markers.get(marker_name)
    }

    /// Return one parsed range with matching stripped text.
    pub fn range_by_text(&self, text: &str) -> Option<&Range> {
        self.ranges.iter().find(|range| range.text == text)
    }

    /// Return one step snapshot by index.
    pub fn step_snapshot(&self, index: usize) -> Option<&LspStepSnapshot> {
        self.step_snapshots.get(&index)
    }

    /// Return one fully resolved step snapshot with prior file state carried forward.
    pub fn resolved_step_snapshot(&self, index: usize) -> Option<ResolvedLspStepSnapshot> {
        let step_indices = self.step_indices();
        if !step_indices.is_empty() && !step_indices.contains(&index) {
            return None;
        }

        let mut files_by_path = self
            .files
            .iter()
            .cloned()
            .map(|file| (file.path.clone(), file))
            .collect::<BTreeMap<_, _>>();
        let mut markers_by_name = self.markers.clone();
        let mut ranges_by_file = BTreeMap::<String, Vec<Range>>::new();

        // base ranges
        for range in &self.ranges {
            ranges_by_file
                .entry(range.file_path.clone())
                .or_default()
                .push(range.clone());
        }

        // carry forward each changed file snapshot up to the requested step
        for step in 0..=index {
            let Some(snapshot) = self.step_snapshot(step) else {
                continue;
            };

            overlay_step_snapshot(
                snapshot,
                &mut files_by_path,
                &mut markers_by_name,
                &mut ranges_by_file,
            );
        }

        let mut files = files_by_path.into_values().collect::<Vec<_>>();
        let mut ranges = ranges_by_file.into_values().flatten().collect::<Vec<_>>();

        // compare resolved snapshots in stable declaration order
        files.sort_by(|left, right| left.path.cmp(&right.path));

        // compare resolved ranges in stable source order
        ranges.sort_by(|left, right| {
            (
                left.file_path.as_str(),
                left.start_offset,
                left.end_offset,
                left.text.as_str(),
            )
                .cmp(&(
                    right.file_path.as_str(),
                    right.start_offset,
                    right.end_offset,
                    right.text.as_str(),
                ))
        });

        Some(ResolvedLspStepSnapshot {
            index,
            files,
            markers: markers_by_name,
            ranges,
        })
    }

    /// Return the ordered step indices declared by stepped files or step-owned blocks.
    pub fn step_indices(&self) -> Vec<usize> {
        let mut indices = self.step_snapshots.keys().copied().collect::<Vec<_>>();

        for index in self.step_cases.keys().copied() {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }

        indices.sort_unstable();
        indices
    }

    /// Return whether this fixture declares an explicit multi-step sequence.
    pub fn has_step_sequence(&self) -> bool {
        self.step_indices().iter().any(|index| *index > 0) || !self.step_cases.is_empty()
    }

    /// Validate that explicit step indices form one contiguous sequence from zero.
    pub fn validate_step_sequence(&self) -> Result<(), String> {
        let step_indices = self.step_indices();
        if step_indices.is_empty() {
            return Ok(());
        }

        // stepped fixtures always start from the base workspace state
        if step_indices.first().copied() != Some(0) {
            return Err("stepped fixtures must declare a base [0] state".to_string());
        }

        // reject sparse or out-of-order step numbering loudly
        for (expected_index, actual_index) in step_indices.iter().copied().enumerate() {
            if actual_index != expected_index {
                return Err(format!(
                    "stepped fixtures must use contiguous step indices: expected [{expected_index}] before [{actual_index}]"
                ));
            }
        }

        Ok(())
    }

    /// Return whether this fixture declares any declarative runnable cases.
    pub fn has_declarative_cases(&self) -> bool {
        self.expectations.has_runnable_cases()
    }

    /// Return the step-owned cases for one index.
    pub fn step_cases(&self, index: usize) -> &[LspStepCase] {
        self.step_cases
            .get(&index)
            .map(|cases| cases.as_slice())
            .unwrap_or(&[])
    }

    /// Return the first file path in declaration order.
    pub fn first_file_path(&self) -> Result<String, String> {
        self.files
            .first()
            .map(|file| file.path.clone())
            .ok_or_else(|| "fixture is missing a file".to_string())
    }
}

/// Overlay one partial step snapshot onto one resolved workspace state.
fn overlay_step_snapshot(
    snapshot: &LspStepSnapshot,
    files_by_path: &mut BTreeMap<String, LspSourceFile>,
    markers_by_name: &mut BTreeMap<String, Marker>,
    ranges_by_file: &mut BTreeMap<String, Vec<Range>>,
) {
    let changed_file_paths = snapshot
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    // replace the changed file texts
    for file in &snapshot.files {
        files_by_path.insert(file.path.clone(), file.clone());
    }

    // drop deleted files from the resolved workspace state
    for file_path in &snapshot.removed_file_paths {
        files_by_path.remove(file_path);
    }

    // drop stale markers for changed files before inserting the new set
    markers_by_name.retain(|_, marker| !changed_file_paths.contains(&marker.file_path));

    // drop markers for deleted files before later steps run
    markers_by_name.retain(|_, marker| !snapshot.removed_file_paths.contains(&marker.file_path));

    for marker in snapshot.markers.values() {
        markers_by_name.insert(marker.name.clone(), marker.clone());
    }

    // drop stale ranges for changed files before inserting the new set
    for file_path in &changed_file_paths {
        ranges_by_file.remove(file_path);
    }

    // drop ranges for deleted files before later steps run
    for file_path in &snapshot.removed_file_paths {
        ranges_by_file.remove(file_path);
    }

    for range in &snapshot.ranges {
        ranges_by_file
            .entry(range.file_path.clone())
            .or_default()
            .push(range.clone());
    }
}

impl fmt::Display for FixtureParseError {
    /// Format one fixture parse failure with path and line context.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.path.display(),
            self.line,
            self.message
        )
    }
}

impl std::error::Error for FixtureParseError {}

impl LspExpectations {
    /// Return whether these expectations include runnable declarative cases.
    pub fn has_runnable_cases(&self) -> bool {
        !self.symbols.document.is_empty()
            || self.symbols.workspace_query.is_some()
            || !self.symbols.workspace.is_empty()
            || !self.completion.items.is_empty()
            || self.completion.resolve_label.is_some()
            || self.document_links.snapshot_text.is_some()
            || self.folding_ranges.snapshot_text.is_some()
            || self.inlay_hints.snapshot_text.is_some()
            || self.code_lenses.snapshot_text.is_some()
            || self.formatting.document_expected_text.is_some()
            || self.formatting.range_expected_text.is_some()
            || self.formatting.current_file_text.is_some()
            || !self.code_actions.actions.is_empty()
            || self.code_actions.expected_text.is_some()
            || self.semantic_tokens.full_expected_text.is_some()
            || self.semantic_tokens.range_expected_text.is_some()
            || self.semantic_tokens.delta_source_text.is_some()
            || self.semantic_tokens.delta_expected_text.is_some()
            || self.hierarchy.call_incoming_text.is_some()
            || self.hierarchy.call_outgoing_text.is_some()
            || self.hierarchy.type_supertypes_text.is_some()
            || self.hierarchy.type_subtypes_text.is_some()
            || self.diagnostics.workspace_text.is_some()
            || self.diagnostics.workspace_partial_text.is_some()
            || self.diagnostics.selection_range_text.is_some()
            || self.edits.current_file_text.is_some()
            || !self.commands.execute.is_empty()
    }
}
