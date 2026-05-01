use std::hash::Hash;

use destack_source::{
    Diagnostic, DiagnosticHelp, DiagnosticLabel, DiagnosticNote, DiagnosticSuggestion,
};

use crate::{DiagnosticError, DiagnosticSite, ProviderContext, ToDiagnostic};

/// One pending secondary diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticBuilderLabel {
    /// The provider-side source site.
    pub site: DiagnosticSite,
    /// The label message.
    pub message: String,
}

impl DiagnosticBuilderLabel {
    /// Create one pending diagnostic label.
    pub fn new(site: impl Into<DiagnosticSite>, message: impl Into<String>) -> Self {
        Self {
            site: site.into(),
            message: message.into(),
        }
    }
}

/// Builder for decorating one provider diagnostic before finalization.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticBuilder<T> {
    /// The provider diagnostic value.
    pub diagnostic: T,
    /// Secondary source labels to add.
    pub labels: Vec<DiagnosticBuilderLabel>,
    /// Notes to add.
    pub notes: Vec<DiagnosticNote>,
    /// Help messages to add.
    pub helps: Vec<DiagnosticHelp>,
    /// Source edit suggestions to add.
    pub suggestions: Vec<DiagnosticSuggestion>,
}

impl<T> DiagnosticBuilder<T> {
    /// Create a diagnostic builder.
    pub fn new(diagnostic: T) -> Self {
        Self {
            diagnostic,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Return the wrapped provider diagnostic.
    pub fn diagnostic(&self) -> &T {
        &self.diagnostic
    }

    /// Add one secondary source label.
    pub fn label(mut self, site: impl Into<DiagnosticSite>, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticBuilderLabel::new(site, message));

        self
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

    /// Return the wrapped provider diagnostic.
    pub fn into_inner(self) -> T {
        self.diagnostic
    }
}

impl<T> From<T> for DiagnosticBuilder<T> {
    /// Create a diagnostic builder from one provider diagnostic.
    fn from(diagnostic: T) -> Self {
        Self::new(diagnostic)
    }
}

impl<R, T> ToDiagnostic<R> for DiagnosticBuilder<T>
where
    R: Copy + Eq + Hash,
    T: ToDiagnostic<R>,
{
    /// Convert the decorated provider diagnostic into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError> {
        let mut diagnostic = self.diagnostic.to_diagnostic(context)?;

        for label in &self.labels {
            let anchor = context.anchor(&label.site)?;
            let span = context.resolve_diagnostic_anchor(&anchor)?.ok_or_else(|| {
                DiagnosticError::UnresolvedLabel {
                    anchor: anchor.clone(),
                }
            })?;
            diagnostic = diagnostic.label(DiagnosticLabel::message(span, label.message.clone()));
        }

        for note in &self.notes {
            diagnostic = diagnostic.note(note.clone());
        }

        for help in &self.helps {
            diagnostic = diagnostic.help(help.clone());
        }

        for suggestion in &self.suggestions {
            diagnostic = diagnostic.suggestion(suggestion.clone());
        }

        Ok(diagnostic)
    }
}
