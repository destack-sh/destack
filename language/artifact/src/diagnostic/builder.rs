use tspp_dir::{GlobalSymbolId, ReferenceTarget};
use tspp_source::{
    Diagnostic, DiagnosticHelp, DiagnosticNote, DiagnosticSuggestion, DiagnosticTag,
};

use crate::{
    DeferredDiagnosticLabel, DiagnosticAnchor, DiagnosticContext, DiagnosticError,
    DiagnosticRecord, ToDiagnostic,
};

/// One secondary diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SecondaryLabel {
    /// The source anchor.
    anchor: DiagnosticAnchor,
    /// The label message.
    message: String,
}

impl SecondaryLabel {
    /// Create one secondary source label.
    fn new(anchor: impl Into<DiagnosticAnchor>, message: impl Into<String>) -> Self {
        Self {
            anchor: anchor.into(),
            message: message.into(),
        }
    }
}

/// Additional diagnostic fields attached to one provider diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticBuilder<T> {
    /// The provider diagnostic value.
    diagnostic: T,
    /// The optional primary label message.
    primary: Option<String>,
    /// Secondary source labels to add.
    labels: Vec<SecondaryLabel>,
    /// Diagnostic labels to resolve when the record is read.
    deferred_labels: Vec<DeferredDiagnosticLabel>,
    /// Notes to add.
    notes: Vec<DiagnosticNote>,
    /// Help messages to add.
    helps: Vec<DiagnosticHelp>,
    /// Source edit suggestions to add.
    suggestions: Vec<DiagnosticSuggestion>,
    /// Diagnostic tags to add.
    tags: Vec<DiagnosticTag>,
}

impl<T> DiagnosticBuilder<T> {
    /// Create a diagnostic builder.
    pub fn new(diagnostic: T) -> Self {
        Self {
            diagnostic,
            primary: None,
            labels: Vec::new(),
            deferred_labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            suggestions: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Return the wrapped provider diagnostic.
    pub fn diagnostic(&self) -> &T {
        &self.diagnostic
    }

    /// Return the wrapped provider diagnostic mutably.
    pub fn diagnostic_mut(&mut self) -> &mut T {
        &mut self.diagnostic
    }

    /// Set the primary label message.
    pub fn primary(mut self, message: impl Into<String>) -> Self {
        self.primary = Some(message.into());

        self
    }

    /// Add one secondary source label.
    pub fn label(
        mut self,
        anchor: impl Into<DiagnosticAnchor>,
        message: impl Into<String>,
    ) -> Self {
        self.labels.push(SecondaryLabel::new(anchor, message));
        self
    }

    /// Add one deferred declaration label.
    pub fn declaration(mut self, declaration: GlobalSymbolId, message: impl Into<String>) -> Self {
        self.deferred_labels.push(DeferredDiagnosticLabel {
            anchor: declaration,
            message: message.into(),
        });

        self
    }

    /// Add one resolved reference target as a secondary label.
    pub fn reference(self, target: ReferenceTarget, message: impl Into<String>) -> Self {
        match target {
            ReferenceTarget::Symbol(declaration) => self.declaration(declaration, message),
            ReferenceTarget::Namespace(module) => self.label(module, message),
        }
    }

    /// Add one note.
    pub fn note(mut self, note: impl Into<DiagnosticNote>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add one help message.
    pub fn help(mut self, help: impl Into<DiagnosticHelp>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Add one source edit suggestion.
    pub fn suggestion(mut self, suggestion: DiagnosticSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add one diagnostic tag.
    pub fn tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);

        self
    }

    /// Return the wrapped provider diagnostic.
    pub fn into_inner(self) -> T {
        self.diagnostic
    }
}

impl<T> DiagnosticBuilder<T>
where
    T: ToDiagnostic,
{
    /// Resolve immediate fields into one source diagnostic.
    fn resolve(&self, context: &dyn DiagnosticContext) -> Result<Diagnostic, DiagnosticError> {
        // base diagnostic
        let mut diagnostic = self.diagnostic.to_diagnostic(context)?;
        if let Some(message) = &self.primary {
            diagnostic.primary.message = Some(message.clone());
        }

        // secondary labels
        for label in &self.labels {
            let label = context.label(&label.anchor, Some(label.message.clone()))?;
            diagnostic = diagnostic.label(label);
        }

        // notes
        for note in &self.notes {
            diagnostic = diagnostic.note(note.clone());
        }

        // help messages
        for help in &self.helps {
            diagnostic = diagnostic.help(help.clone());
        }

        // suggestions
        for suggestion in &self.suggestions {
            diagnostic = diagnostic.suggestion(suggestion.clone());
        }

        // tags
        for tag in &self.tags {
            diagnostic = diagnostic.tag(*tag);
        }

        Ok(diagnostic)
    }

    /// Convert the provider diagnostic into a stored record.
    pub fn to_record(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<DiagnosticRecord, DiagnosticError> {
        let diagnostic = self.resolve(context)?;

        Ok(DiagnosticRecord {
            diagnostic,
            deferred_labels: self.deferred_labels.clone(),
        })
    }
}

impl<T> From<T> for DiagnosticBuilder<T> {
    /// Create a diagnostic builder from one provider diagnostic.
    fn from(diagnostic: T) -> Self {
        Self::new(diagnostic)
    }
}

impl<T> ToDiagnostic for DiagnosticBuilder<T>
where
    T: ToDiagnostic,
{
    /// Convert the provider diagnostic and its attached fields into a source diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        // require deferred labels to pass through a stored diagnostic record
        if !self.deferred_labels.is_empty() {
            return Err(DiagnosticError::InvalidDiagnostic {
                message: "declaration labels require a stored diagnostic record".to_string(),
            });
        }

        self.resolve(context)
    }
}
