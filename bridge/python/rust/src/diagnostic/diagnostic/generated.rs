// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{BatchEdit, FileContentId, Span};

/// Diagnostic severity crossing bridge boundaries.
#[pyclass(
    name = "DiagnosticSeverity",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct DiagnosticSeverity {
    pub(crate) value: bridge::DiagnosticSeverity,
}

#[pymethods]
impl DiagnosticSeverity {
    /// Informative message.
    #[staticmethod]
    pub fn note() -> Self {
        Self {
            value: bridge::DiagnosticSeverity::Note,
        }
    }

    /// Non-critical issue.
    #[staticmethod]
    pub fn warning() -> Self {
        Self {
            value: bridge::DiagnosticSeverity::Warning,
        }
    }

    /// Critical issue.
    #[staticmethod]
    pub fn error() -> Self {
        Self {
            value: bridge::DiagnosticSeverity::Error,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::DiagnosticSeverity::Note => "note",
            bridge::DiagnosticSeverity::Warning => "warning",
            bridge::DiagnosticSeverity::Error => "error",
        }
    }
}

impl DiagnosticSeverity {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticSeverity {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticSeverity) -> Self {
        Self { value }
    }
}

/// Extra semantic diagnostic tag crossing bridge boundaries.
#[pyclass(name = "DiagnosticTag", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DiagnosticTag {
    pub(crate) value: bridge::DiagnosticTag,
}

#[pymethods]
impl DiagnosticTag {
    /// Unused or unnecessary source.
    #[staticmethod]
    pub fn unnecessary() -> Self {
        Self {
            value: bridge::DiagnosticTag::Unnecessary,
        }
    }

    /// Deprecated source.
    #[staticmethod]
    pub fn deprecated() -> Self {
        Self {
            value: bridge::DiagnosticTag::Deprecated,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::DiagnosticTag::Unnecessary => "unnecessary",
            bridge::DiagnosticTag::Deprecated => "deprecated",
        }
    }
}

impl DiagnosticTag {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticTag {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticTag) -> Self {
        Self { value }
    }
}

/// Whether a suggestion can be applied automatically.
#[pyclass(name = "Applicability", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Applicability {
    pub(crate) value: bridge::Applicability,
}

#[pymethods]
impl Applicability {
    /// Machine-applicable suggestion.
    #[staticmethod]
    pub fn automatic() -> Self {
        Self {
            value: bridge::Applicability::Automatic,
        }
    }

    /// Machine-applicable suggestion that may change behavior.
    #[staticmethod]
    pub fn r#unsafe() -> Self {
        Self {
            value: bridge::Applicability::Unsafe,
        }
    }

    /// Maybe incorrect suggestion.
    #[staticmethod]
    pub fn dangerous() -> Self {
        Self {
            value: bridge::Applicability::Dangerous,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::Applicability::Automatic => "automatic",
            bridge::Applicability::Unsafe => "unsafe",
            bridge::Applicability::Dangerous => "dangerous",
        }
    }
}

impl Applicability {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Applicability {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Applicability) -> Self {
        Self { value }
    }
}

/// One concrete source label in a diagnostic.
#[pyclass(name = "DiagnosticLabel", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub(crate) value: bridge::DiagnosticLabel,
}

#[pymethods]
impl DiagnosticLabel {
    /// Create one value.
    #[new]
    pub fn new(content: FileContentId, span: Span, message: Option<String>) -> Self {
        Self {
            value: bridge::DiagnosticLabel {
                content: content.into_bridge(),
                span: span.into_bridge(),
                message,
            },
        }
    }

    /// Exact file content containing the span.
    #[getter]
    pub fn content(&self) -> FileContentId {
        FileContentId::from_bridge(self.value.content.clone())
    }

    /// Concrete source span.
    #[getter]
    pub fn span(&self) -> Span {
        Span::from_bridge(self.value.span.clone())
    }

    /// Optional label shown on the span.
    #[getter]
    pub fn message(&self) -> Option<String> {
        self.value.message.clone()
    }
}

#[allow(dead_code)]
impl DiagnosticLabel {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticLabel {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticLabel) -> Self {
        Self { value }
    }
}

/// Extra context for understanding a diagnostic.
#[pyclass(name = "DiagnosticNote", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DiagnosticNote {
    pub(crate) value: bridge::DiagnosticNote,
}

#[pymethods]
impl DiagnosticNote {
    /// Create one value.
    #[new]
    pub fn new(message: String) -> Self {
        Self {
            value: bridge::DiagnosticNote { message },
        }
    }

    /// Note message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }
}

#[allow(dead_code)]
impl DiagnosticNote {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticNote {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticNote) -> Self {
        Self { value }
    }
}

/// Guidance for fixing or avoiding a diagnostic.
#[pyclass(name = "DiagnosticHelp", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DiagnosticHelp {
    pub(crate) value: bridge::DiagnosticHelp,
}

#[pymethods]
impl DiagnosticHelp {
    /// Create one value.
    #[new]
    pub fn new(message: String) -> Self {
        Self {
            value: bridge::DiagnosticHelp { message },
        }
    }

    /// Help message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }
}

#[allow(dead_code)]
impl DiagnosticHelp {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticHelp {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticHelp) -> Self {
        Self { value }
    }
}

/// One suggested source change for a diagnostic.
#[pyclass(
    name = "DiagnosticSuggestion",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct DiagnosticSuggestion {
    pub(crate) value: bridge::DiagnosticSuggestion,
}

#[pymethods]
impl DiagnosticSuggestion {
    /// Create one value.
    #[new]
    pub fn new(
        edits: BatchEdit,
        labels: Vec<DiagnosticLabel>,
        message: String,
        applicability: Applicability,
    ) -> Self {
        Self {
            value: bridge::DiagnosticSuggestion {
                edits: edits.into_bridge(),
                labels: labels.into_iter().map(|item| item.into_bridge()).collect(),
                message,
                applicability: applicability.into_bridge(),
            },
        }
    }

    /// Exact source edits for machine application.
    #[getter]
    pub fn edits(&self) -> BatchEdit {
        BatchEdit::from_bridge(self.value.edits.clone())
    }

    /// Source labels to show with the suggestion.
    #[getter]
    pub fn labels(&self) -> Vec<DiagnosticLabel> {
        self.value
            .labels
            .clone()
            .into_iter()
            .map(|item| DiagnosticLabel::from_bridge(item))
            .collect()
    }

    /// Suggestion message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }

    /// Suggestion applicability.
    #[getter]
    pub fn applicability(&self) -> Applicability {
        Applicability::from_bridge(self.value.applicability.clone())
    }
}

#[allow(dead_code)]
impl DiagnosticSuggestion {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::DiagnosticSuggestion {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticSuggestion) -> Self {
        Self { value }
    }
}

/// One final renderable diagnostic.
#[pyclass(name = "Diagnostic", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub(crate) value: bridge::Diagnostic,
}

#[pymethods]
impl Diagnostic {
    /// Create one value.
    #[new]
    pub fn new(
        code: String,
        severity: DiagnosticSeverity,
        message: String,
        primary: DiagnosticLabel,
        labels: Vec<DiagnosticLabel>,
        notes: Vec<DiagnosticNote>,
        helps: Vec<DiagnosticHelp>,
        suggestions: Vec<DiagnosticSuggestion>,
        tags: Vec<DiagnosticTag>,
    ) -> Self {
        Self {
            value: bridge::Diagnostic {
                code,
                severity: severity.into_bridge(),
                message,
                primary: primary.into_bridge(),
                labels: labels.into_iter().map(|item| item.into_bridge()).collect(),
                notes: notes.into_iter().map(|item| item.into_bridge()).collect(),
                helps: helps.into_iter().map(|item| item.into_bridge()).collect(),
                suggestions: suggestions
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
                tags: tags.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Stable diagnostic code.
    #[getter]
    pub fn code(&self) -> String {
        self.value.code.clone()
    }

    /// Diagnostic severity.
    #[getter]
    pub fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_bridge(self.value.severity.clone())
    }

    /// Diagnostic message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }

    /// Main source label.
    #[getter]
    pub fn primary(&self) -> DiagnosticLabel {
        DiagnosticLabel::from_bridge(self.value.primary.clone())
    }

    /// Additional source labels.
    #[getter]
    pub fn labels(&self) -> Vec<DiagnosticLabel> {
        self.value
            .labels
            .clone()
            .into_iter()
            .map(|item| DiagnosticLabel::from_bridge(item))
            .collect()
    }

    /// Extra context.
    #[getter]
    pub fn notes(&self) -> Vec<DiagnosticNote> {
        self.value
            .notes
            .clone()
            .into_iter()
            .map(|item| DiagnosticNote::from_bridge(item))
            .collect()
    }

    /// Fixing or avoidance guidance.
    #[getter]
    pub fn helps(&self) -> Vec<DiagnosticHelp> {
        self.value
            .helps
            .clone()
            .into_iter()
            .map(|item| DiagnosticHelp::from_bridge(item))
            .collect()
    }

    /// Suggested source changes.
    #[getter]
    pub fn suggestions(&self) -> Vec<DiagnosticSuggestion> {
        self.value
            .suggestions
            .clone()
            .into_iter()
            .map(|item| DiagnosticSuggestion::from_bridge(item))
            .collect()
    }

    /// Extra semantic tags.
    #[getter]
    pub fn tags(&self) -> Vec<DiagnosticTag> {
        self.value
            .tags
            .clone()
            .into_iter()
            .map(|item| DiagnosticTag::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl Diagnostic {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Diagnostic) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DiagnosticSeverity>()?;
    module.add_class::<DiagnosticTag>()?;
    module.add_class::<Applicability>()?;
    module.add_class::<DiagnosticLabel>()?;
    module.add_class::<DiagnosticNote>()?;
    module.add_class::<DiagnosticHelp>()?;
    module.add_class::<DiagnosticSuggestion>()?;
    module.add_class::<Diagnostic>()?;
    Ok(())
}
