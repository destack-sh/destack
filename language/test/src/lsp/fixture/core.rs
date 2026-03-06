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
    /// The named markers declared across all files.
    pub markers: BTreeMap<String, Marker>,
    /// The ranges declared across all files.
    pub ranges: Vec<Range>,
    /// The bespoke scenario behaviors declared by fixture metadata.
    pub scenarios: Vec<LspScenario>,
    /// The capability expectations declared by fixture metadata.
    pub expectations: LspExpectations,
    /// The fixture-relative open-text overrides keyed by target file path.
    pub open_text_overrides: BTreeMap<String, String>,
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

/// One bespoke applied-LSP scenario declared by fixture metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LspScenario {
    /// The each-marker cross-module definition walk.
    DefinitionEachMarkerCrossModule,
    /// The each-marker same-file definition walk.
    DefinitionEachMarkerSameFile,
    /// The each-range cross-module references walk.
    ReferencesEachRangeCrossModule,
    /// The quick-info existence check.
    QuickInfoExists,
    /// The indentation assertions on the active line.
    IndentationCurrentLine,
    /// The grouped quick-info assertions.
    QuickInfos,
    /// The negative signature-help check.
    NoSignatureHelp,
    /// The trigger-character signature-help check.
    SignatureHelpTriggerCharacter,
    /// The trigger-reason negative signature-help check.
    NoSignatureHelpForTriggerReason,
    /// The initial open-overlay document diagnostic check.
    DocumentDiagnosticOpenOverlay,
    /// The marker-window diagnostic helper check.
    DiagnosticMarkerWindows,
    /// The overlay change document diagnostic check.
    DocumentDiagnosticChangeOverlay,
    /// The save-persisted document diagnostic check.
    DocumentDiagnosticSavePersistsOverlay,
    /// The close-reverted document diagnostic check.
    DocumentDiagnosticCloseRevertsOverlay,
    /// The clean workspace no-diagnostic check.
    NoErrorsCleanWorkspace,
    /// The request-id references cancellation check.
    ReferencesRequestCancel,
    /// The progress-token references cancellation check.
    ReferencesProgressCancel,
    /// The auto-policy request-id cancellation check.
    ReferencesRequestCancelledByPolicy,
    /// The auto-policy progress-token cancellation check.
    ReferencesProgressCancelledByPolicy,
    /// The multi-file edit responsiveness check.
    MultifileEditResponsiveness,
    /// The end-to-end edit roundtrip check.
    EditRoundtrip,
    /// The select-all replace check.
    EditSelectAllReplace,
    /// The replace and insert-lines edit check.
    EditReplaceAndInsertLines,
    /// The select-range replace check.
    EditSelectRangeReplace,
    /// The paste, delete, and bof edit check.
    EditPasteDeleteAndBof,
    /// The delete-line edit check.
    EditDeleteLine,
    /// The delete-line-range edit check.
    EditDeleteLineRange,
    /// The replace-line edit check.
    EditReplaceLine,
    /// The no-op document formatting check.
    DocumentFormattingChangesNothing,
    /// The marker-driven selection formatting check.
    FormatSelectionMarkers,
    /// The on-type formatting check.
    OnTypeFormattingBrace,
    /// The formatting option roundtrip check.
    FormatOptionRoundtrip,
    /// The format toggle roundtrip check.
    FormatDisableEnableRoundtrip,
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

    /// Return whether one bespoke scenario is declared by this fixture.
    pub fn has_scenario(&self, scenario: LspScenario) -> bool {
        self.scenarios.contains(&scenario)
    }

    /// Return the first file path in declaration order.
    pub fn first_file_path(&self) -> Result<String, String> {
        self.files
            .first()
            .map(|file| file.path.clone())
            .ok_or_else(|| "fixture is missing a file".to_string())
    }
}

impl LspScenario {
    /// Parse one scenario name from fixture metadata.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "definition-each-marker-cross-module" => Some(Self::DefinitionEachMarkerCrossModule),
            "definition-each-marker-same-file" => Some(Self::DefinitionEachMarkerSameFile),
            "references-each-range-cross-module" => Some(Self::ReferencesEachRangeCrossModule),
            "quick-info-exists" => Some(Self::QuickInfoExists),
            "indentation-current-line" => Some(Self::IndentationCurrentLine),
            "quick-infos" => Some(Self::QuickInfos),
            "no-signature-help" => Some(Self::NoSignatureHelp),
            "signature-help-trigger-character" => Some(Self::SignatureHelpTriggerCharacter),
            "no-signature-help-for-trigger-reason" => Some(Self::NoSignatureHelpForTriggerReason),
            "document-diagnostic-open-overlay" => Some(Self::DocumentDiagnosticOpenOverlay),
            "diagnostic-marker-windows" => Some(Self::DiagnosticMarkerWindows),
            "document-diagnostic-change-overlay" => Some(Self::DocumentDiagnosticChangeOverlay),
            "document-diagnostic-save-persists-overlay" => {
                Some(Self::DocumentDiagnosticSavePersistsOverlay)
            }
            "document-diagnostic-close-reverts-overlay" => {
                Some(Self::DocumentDiagnosticCloseRevertsOverlay)
            }
            "no-errors-clean-workspace" => Some(Self::NoErrorsCleanWorkspace),
            "references-request-cancel" => Some(Self::ReferencesRequestCancel),
            "references-progress-cancel" => Some(Self::ReferencesProgressCancel),
            "references-request-cancelled-by-policy" => {
                Some(Self::ReferencesRequestCancelledByPolicy)
            }
            "references-progress-cancelled-by-policy" => {
                Some(Self::ReferencesProgressCancelledByPolicy)
            }
            "multifile-edit-responsiveness" => Some(Self::MultifileEditResponsiveness),
            "edit-roundtrip" => Some(Self::EditRoundtrip),
            "edit-select-all-replace" => Some(Self::EditSelectAllReplace),
            "edit-replace-and-insert-lines" => Some(Self::EditReplaceAndInsertLines),
            "edit-select-range-replace" => Some(Self::EditSelectRangeReplace),
            "edit-paste-delete-and-bof" => Some(Self::EditPasteDeleteAndBof),
            "edit-delete-line" => Some(Self::EditDeleteLine),
            "edit-delete-line-range" => Some(Self::EditDeleteLineRange),
            "edit-replace-line" => Some(Self::EditReplaceLine),
            "document-formatting-changes-nothing" => Some(Self::DocumentFormattingChangesNothing),
            "format-selection-markers" => Some(Self::FormatSelectionMarkers),
            "on-type-formatting-brace" => Some(Self::OnTypeFormattingBrace),
            "format-option-roundtrip" => Some(Self::FormatOptionRoundtrip),
            "format-disable-enable-roundtrip" => Some(Self::FormatDisableEnableRoundtrip),
            _ => None,
        }
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
