// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{BatchEdit, FileContentId, Span};

/// One concrete source label in a diagnostic.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DiagnosticLabel {
    content: FileContentId,
    span: Span,
    message: Option<String>,
}

#[wasm_bindgen]
impl DiagnosticLabel {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(content: FileContentId, span: Span, message: Option<String>) -> Self {
        Self {
            content,
            span,
            message,
        }
    }

    /// Exact file content containing the span.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> FileContentId {
        self.content.clone()
    }

    /// Concrete source span.
    #[wasm_bindgen(getter, js_name = "span")]
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    /// Optional label shown on the span.
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn message(&self) -> Option<String> {
        self.message.clone()
    }
}

impl DiagnosticLabel {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticLabel) -> Self {
        Self {
            content: FileContentId::from_bridge(value.content),
            span: Span::from_bridge(value.span),
            message: value.message,
        }
    }
}

/// Extra context for understanding a diagnostic.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DiagnosticNote {
    message: String,
}

#[wasm_bindgen]
impl DiagnosticNote {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(message: String) -> Self {
        Self { message }
    }

    /// Note message.
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl DiagnosticNote {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticNote) -> Self {
        Self {
            message: value.message,
        }
    }
}

/// Guidance for fixing or avoiding a diagnostic.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DiagnosticHelp {
    message: String,
}

#[wasm_bindgen]
impl DiagnosticHelp {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(message: String) -> Self {
        Self { message }
    }

    /// Help message.
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl DiagnosticHelp {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticHelp) -> Self {
        Self {
            message: value.message,
        }
    }
}

/// One suggested source change for a diagnostic.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct DiagnosticSuggestion {
    edits: BatchEdit,
    labels: Vec<DiagnosticLabel>,
    message: String,
    applicability: String,
}

#[wasm_bindgen]
impl DiagnosticSuggestion {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        edits: BatchEdit,
        labels: Vec<DiagnosticLabel>,
        message: String,
        applicability: String,
    ) -> Self {
        Self {
            edits,
            labels,
            message,
            applicability,
        }
    }

    /// Exact source edits for machine application.
    #[wasm_bindgen(getter, js_name = "edits")]
    pub fn edits(&self) -> BatchEdit {
        self.edits.clone()
    }

    /// Source labels to show with the suggestion.
    #[wasm_bindgen(getter, js_name = "labels")]
    pub fn labels(&self) -> Vec<DiagnosticLabel> {
        self.labels.clone()
    }

    /// Suggestion message.
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Suggestion applicability.
    #[wasm_bindgen(getter, js_name = "applicability")]
    pub fn applicability(&self) -> String {
        self.applicability.clone()
    }
}

impl DiagnosticSuggestion {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticSuggestion) -> Self {
        Self {
            edits: BatchEdit::from_bridge(value.edits),
            labels: value
                .labels
                .into_iter()
                .map(|item| DiagnosticLabel::from_bridge(item))
                .collect(),
            message: value.message,
            applicability: applicability_label(value.applicability),
        }
    }
}

/// One final renderable diagnostic.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Diagnostic {
    code: String,
    severity: String,
    message: String,
    primary: DiagnosticLabel,
    labels: Vec<DiagnosticLabel>,
    notes: Vec<DiagnosticNote>,
    helps: Vec<DiagnosticHelp>,
    suggestions: Vec<DiagnosticSuggestion>,
    tags: Vec<String>,
}

#[wasm_bindgen]
impl Diagnostic {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(
        code: String,
        severity: String,
        message: String,
        primary: DiagnosticLabel,
        labels: Vec<DiagnosticLabel>,
        notes: Vec<DiagnosticNote>,
        helps: Vec<DiagnosticHelp>,
        suggestions: Vec<DiagnosticSuggestion>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            code,
            severity,
            message,
            primary,
            labels,
            notes,
            helps,
            suggestions,
            tags,
        }
    }

    /// Stable diagnostic code.
    #[wasm_bindgen(getter, js_name = "code")]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    /// Diagnostic severity.
    #[wasm_bindgen(getter, js_name = "severity")]
    pub fn severity(&self) -> String {
        self.severity.clone()
    }

    /// Diagnostic message.
    #[wasm_bindgen(getter, js_name = "message")]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Main source label.
    #[wasm_bindgen(getter, js_name = "primary")]
    pub fn primary(&self) -> DiagnosticLabel {
        self.primary.clone()
    }

    /// Additional source labels.
    #[wasm_bindgen(getter, js_name = "labels")]
    pub fn labels(&self) -> Vec<DiagnosticLabel> {
        self.labels.clone()
    }

    /// Extra context.
    #[wasm_bindgen(getter, js_name = "notes")]
    pub fn notes(&self) -> Vec<DiagnosticNote> {
        self.notes.clone()
    }

    /// Fixing or avoidance guidance.
    #[wasm_bindgen(getter, js_name = "helps")]
    pub fn helps(&self) -> Vec<DiagnosticHelp> {
        self.helps.clone()
    }

    /// Suggested source changes.
    #[wasm_bindgen(getter, js_name = "suggestions")]
    pub fn suggestions(&self) -> Vec<DiagnosticSuggestion> {
        self.suggestions.clone()
    }

    /// Extra semantic tags.
    #[wasm_bindgen(getter, js_name = "tags")]
    pub fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

impl Diagnostic {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::Diagnostic) -> Self {
        Self {
            code: value.code,
            severity: diagnostic_severity_label(value.severity),
            message: value.message,
            primary: DiagnosticLabel::from_bridge(value.primary),
            labels: value
                .labels
                .into_iter()
                .map(|item| DiagnosticLabel::from_bridge(item))
                .collect(),
            notes: value
                .notes
                .into_iter()
                .map(|item| DiagnosticNote::from_bridge(item))
                .collect(),
            helps: value
                .helps
                .into_iter()
                .map(|item| DiagnosticHelp::from_bridge(item))
                .collect(),
            suggestions: value
                .suggestions
                .into_iter()
                .map(|item| DiagnosticSuggestion::from_bridge(item))
                .collect(),
            tags: value
                .tags
                .into_iter()
                .map(|item| diagnostic_tag_label(item))
                .collect(),
        }
    }
}

/// Return one target enum label.
fn diagnostic_severity_label(value: bridge::DiagnosticSeverity) -> String {
    let label = match value {
        bridge::DiagnosticSeverity::Note => "note",
        bridge::DiagnosticSeverity::Warning => "warning",
        bridge::DiagnosticSeverity::Error => "error",
    };
    label.to_string()
}

/// Return one target enum label.
fn diagnostic_tag_label(value: bridge::DiagnosticTag) -> String {
    let label = match value {
        bridge::DiagnosticTag::Unnecessary => "unnecessary",
        bridge::DiagnosticTag::Deprecated => "deprecated",
    };
    label.to_string()
}

/// Return one target enum label.
fn applicability_label(value: bridge::Applicability) -> String {
    let label = match value {
        bridge::Applicability::Automatic => "automatic",
        bridge::Applicability::Unsafe => "unsafe",
        bridge::Applicability::Dangerous => "dangerous",
    };
    label.to_string()
}
