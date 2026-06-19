// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{BatchEdit, ContentId, Span};

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
    /// Exact content containing the span.
    #[getter]
    pub fn content(&self) -> ContentId {
        ContentId::from_bridge(self.value.content.clone())
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
    /// Note message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }
}

#[allow(dead_code)]
impl DiagnosticNote {
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
    /// Help message.
    #[getter]
    pub fn message(&self) -> String {
        self.value.message.clone()
    }
}

#[allow(dead_code)]
impl DiagnosticHelp {
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
            .map(DiagnosticLabel::from_bridge)
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
        Applicability::from_bridge(self.value.applicability)
    }
}

#[allow(dead_code)]
impl DiagnosticSuggestion {
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
    /// Stable diagnostic code.
    #[getter]
    pub fn code(&self) -> String {
        self.value.code.clone()
    }

    /// Diagnostic severity.
    #[getter]
    pub fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_bridge(self.value.severity)
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
            .map(DiagnosticLabel::from_bridge)
            .collect()
    }

    /// Extra context.
    #[getter]
    pub fn notes(&self) -> Vec<DiagnosticNote> {
        self.value
            .notes
            .clone()
            .into_iter()
            .map(DiagnosticNote::from_bridge)
            .collect()
    }

    /// Fixing or avoidance guidance.
    #[getter]
    pub fn helps(&self) -> Vec<DiagnosticHelp> {
        self.value
            .helps
            .clone()
            .into_iter()
            .map(DiagnosticHelp::from_bridge)
            .collect()
    }

    /// Suggested source changes.
    #[getter]
    pub fn suggestions(&self) -> Vec<DiagnosticSuggestion> {
        self.value
            .suggestions
            .clone()
            .into_iter()
            .map(DiagnosticSuggestion::from_bridge)
            .collect()
    }

    /// Extra semantic tags.
    #[getter]
    pub fn tags(&self) -> Vec<DiagnosticTag> {
        self.value
            .tags
            .clone()
            .into_iter()
            .map(DiagnosticTag::from_bridge)
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
