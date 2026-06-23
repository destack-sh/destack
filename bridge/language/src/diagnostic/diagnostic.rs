use crate::{ContentId, PatchSet, SourceIdParseError, Span, bridge};

/// Diagnostic severity crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    /// Informative message.
    Note,
    /// Non-critical issue.
    Warning,
    /// Critical issue.
    Error,
}

/// Extra semantic diagnostic tag crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticTag {
    /// Unused or unnecessary source.
    Unnecessary,
    /// Deprecated source.
    Deprecated,
}

/// Whether a suggestion can be applied automatically.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Applicability {
    /// Machine-applicable suggestion.
    Automatic,
    /// Machine-applicable suggestion that may change behavior.
    Unsafe,
    /// Maybe incorrect suggestion.
    Dangerous,
}

/// One concrete source label in a diagnostic.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticLabel {
    /// Exact content containing the span.
    pub content: ContentId,
    /// Concrete source span.
    pub span: Span,
    /// Optional label shown on the span.
    pub message: Option<String>,
}

/// Extra context for understanding a diagnostic.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticNote {
    /// Note message.
    pub message: String,
}

/// Guidance for fixing or avoiding a diagnostic.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticHelp {
    /// Help message.
    pub message: String,
}

/// One suggested source change for a diagnostic.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticSuggestion {
    /// Exact source patches for machine application.
    pub patches: PatchSet,
    /// Source labels to show with the suggestion.
    pub labels: Vec<DiagnosticLabel>,
    /// Suggestion message.
    pub message: String,
    /// Suggestion applicability.
    pub applicability: Applicability,
}

/// One final renderable diagnostic.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Diagnostic message.
    pub message: String,
    /// Main source label.
    pub primary: DiagnosticLabel,
    /// Additional source labels.
    pub labels: Vec<DiagnosticLabel>,
    /// Extra context.
    pub notes: Vec<DiagnosticNote>,
    /// Fixing or avoidance guidance.
    pub helps: Vec<DiagnosticHelp>,
    /// Suggested source changes.
    pub suggestions: Vec<DiagnosticSuggestion>,
    /// Extra semantic tags.
    pub tags: Vec<DiagnosticTag>,
}

impl DiagnosticSeverity {
    /// Convert one source diagnostic severity into one bridge severity.
    pub fn from_source(severity: destack_source::DiagnosticSeverity) -> Self {
        match severity {
            destack_source::DiagnosticSeverity::Note => Self::Note,
            destack_source::DiagnosticSeverity::Warning => Self::Warning,
            destack_source::DiagnosticSeverity::Error => Self::Error,
        }
    }

    /// Convert this bridge severity into one source diagnostic severity.
    pub fn into_source(self) -> destack_source::DiagnosticSeverity {
        match self {
            Self::Note => destack_source::DiagnosticSeverity::Note,
            Self::Warning => destack_source::DiagnosticSeverity::Warning,
            Self::Error => destack_source::DiagnosticSeverity::Error,
        }
    }
}

impl DiagnosticTag {
    /// Convert one source diagnostic tag into one bridge tag.
    pub fn from_source(tag: destack_source::DiagnosticTag) -> Self {
        match tag {
            destack_source::DiagnosticTag::Unnecessary => Self::Unnecessary,
            destack_source::DiagnosticTag::Deprecated => Self::Deprecated,
        }
    }

    /// Convert this bridge tag into one source diagnostic tag.
    pub fn into_source(self) -> destack_source::DiagnosticTag {
        match self {
            Self::Unnecessary => destack_source::DiagnosticTag::Unnecessary,
            Self::Deprecated => destack_source::DiagnosticTag::Deprecated,
        }
    }
}

impl Applicability {
    /// Convert one source applicability into one bridge applicability.
    pub fn from_source(applicability: destack_source::Applicability) -> Self {
        match applicability {
            destack_source::Applicability::Automatic => Self::Automatic,
            destack_source::Applicability::Unsafe => Self::Unsafe,
            destack_source::Applicability::Dangerous => Self::Dangerous,
        }
    }

    /// Convert this bridge applicability into one source applicability.
    pub fn into_source(self) -> destack_source::Applicability {
        match self {
            Self::Automatic => destack_source::Applicability::Automatic,
            Self::Unsafe => destack_source::Applicability::Unsafe,
            Self::Dangerous => destack_source::Applicability::Dangerous,
        }
    }
}

impl DiagnosticLabel {
    /// Convert one source diagnostic label into one bridge label.
    pub fn from_source(label: destack_source::DiagnosticLabel) -> Self {
        Self {
            content: label.content.into(),
            span: label.span.into(),
            message: label.message,
        }
    }

    /// Convert this bridge label into one source diagnostic label.
    pub fn into_source(self) -> Result<destack_source::DiagnosticLabel, SourceIdParseError> {
        Ok(destack_source::DiagnosticLabel {
            content: self.content.into_source()?,
            span: self.span.into_source()?,
            message: self.message,
        })
    }
}

impl DiagnosticSuggestion {
    /// Convert one source diagnostic suggestion into one bridge suggestion.
    pub fn from_source(suggestion: destack_source::DiagnosticSuggestion) -> Self {
        Self {
            patches: suggestion.patches.into(),
            labels: suggestion
                .labels
                .into_iter()
                .map(DiagnosticLabel::from_source)
                .collect(),
            message: suggestion.message,
            applicability: Applicability::from_source(suggestion.applicability),
        }
    }

    /// Convert this bridge suggestion into one source diagnostic suggestion.
    pub fn into_source(self) -> Result<destack_source::DiagnosticSuggestion, SourceIdParseError> {
        let labels = self
            .labels
            .into_iter()
            .map(DiagnosticLabel::into_source)
            .collect::<Result<Vec<_>, _>>()?;
        let mut suggestion = destack_source::DiagnosticSuggestion::new(
            self.message,
            self.patches.into_source()?,
            self.applicability.into_source(),
        );
        suggestion.labels = labels;

        Ok(suggestion)
    }
}

impl Diagnostic {
    /// Convert one source diagnostic into one bridge diagnostic.
    pub fn from_source(diagnostic: destack_source::Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: DiagnosticSeverity::from_source(diagnostic.severity),
            message: diagnostic.message,
            primary: diagnostic.primary.into(),
            labels: diagnostic
                .labels
                .into_iter()
                .map(DiagnosticLabel::from_source)
                .collect(),
            notes: diagnostic
                .notes
                .into_iter()
                .map(|note| DiagnosticNote {
                    message: note.message,
                })
                .collect(),
            helps: diagnostic
                .helps
                .into_iter()
                .map(|help| DiagnosticHelp {
                    message: help.message,
                })
                .collect(),
            suggestions: diagnostic
                .suggestions
                .into_iter()
                .map(DiagnosticSuggestion::from_source)
                .collect(),
            tags: diagnostic
                .tags
                .into_iter()
                .map(DiagnosticTag::from_source)
                .collect(),
        }
    }
}

impl From<destack_source::DiagnosticSeverity> for DiagnosticSeverity {
    /// Convert one source diagnostic severity into one bridge severity.
    fn from(severity: destack_source::DiagnosticSeverity) -> Self {
        Self::from_source(severity)
    }
}

impl From<DiagnosticSeverity> for destack_source::DiagnosticSeverity {
    /// Convert one bridge severity into one source diagnostic severity.
    fn from(severity: DiagnosticSeverity) -> Self {
        severity.into_source()
    }
}

impl From<destack_source::DiagnosticTag> for DiagnosticTag {
    /// Convert one source diagnostic tag into one bridge tag.
    fn from(tag: destack_source::DiagnosticTag) -> Self {
        Self::from_source(tag)
    }
}

impl From<DiagnosticTag> for destack_source::DiagnosticTag {
    /// Convert one bridge tag into one source diagnostic tag.
    fn from(tag: DiagnosticTag) -> Self {
        tag.into_source()
    }
}

impl From<destack_source::Applicability> for Applicability {
    /// Convert one source applicability into one bridge applicability.
    fn from(applicability: destack_source::Applicability) -> Self {
        Self::from_source(applicability)
    }
}

impl From<Applicability> for destack_source::Applicability {
    /// Convert one bridge applicability into one source applicability.
    fn from(applicability: Applicability) -> Self {
        applicability.into_source()
    }
}

impl From<destack_source::DiagnosticLabel> for DiagnosticLabel {
    /// Convert one source diagnostic label into one bridge label.
    fn from(label: destack_source::DiagnosticLabel) -> Self {
        Self::from_source(label)
    }
}

impl TryFrom<DiagnosticLabel> for destack_source::DiagnosticLabel {
    type Error = SourceIdParseError;

    /// Convert one bridge label into one source diagnostic label.
    fn try_from(label: DiagnosticLabel) -> Result<Self, Self::Error> {
        label.into_source()
    }
}

impl From<destack_source::DiagnosticNote> for DiagnosticNote {
    /// Convert one source diagnostic note into one bridge note.
    fn from(note: destack_source::DiagnosticNote) -> Self {
        Self {
            message: note.message,
        }
    }
}

impl From<DiagnosticNote> for destack_source::DiagnosticNote {
    /// Convert one bridge diagnostic note into one source note.
    fn from(note: DiagnosticNote) -> Self {
        destack_source::DiagnosticNote::new(note.message)
    }
}

impl From<destack_source::DiagnosticHelp> for DiagnosticHelp {
    /// Convert one source diagnostic help into one bridge help.
    fn from(help: destack_source::DiagnosticHelp) -> Self {
        Self {
            message: help.message,
        }
    }
}

impl From<DiagnosticHelp> for destack_source::DiagnosticHelp {
    /// Convert one bridge diagnostic help into one source help.
    fn from(help: DiagnosticHelp) -> Self {
        destack_source::DiagnosticHelp::new(help.message)
    }
}

impl From<destack_source::DiagnosticSuggestion> for DiagnosticSuggestion {
    /// Convert one source diagnostic suggestion into one bridge suggestion.
    fn from(suggestion: destack_source::DiagnosticSuggestion) -> Self {
        Self::from_source(suggestion)
    }
}

impl TryFrom<DiagnosticSuggestion> for destack_source::DiagnosticSuggestion {
    type Error = SourceIdParseError;

    /// Convert one bridge suggestion into one source diagnostic suggestion.
    fn try_from(suggestion: DiagnosticSuggestion) -> Result<Self, Self::Error> {
        suggestion.into_source()
    }
}

impl From<destack_source::Diagnostic> for Diagnostic {
    /// Convert one source diagnostic into one bridge diagnostic.
    fn from(diagnostic: destack_source::Diagnostic) -> Self {
        Self::from_source(diagnostic)
    }
}
