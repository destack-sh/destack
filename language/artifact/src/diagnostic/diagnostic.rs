use std::sync::Arc;

use tspp_source::Diagnostic;

use crate::{DiagnosticContext, DiagnosticError};

/// Typed value that can become one final diagnostic.
pub trait ToDiagnostic {
    /// Convert this typed value into one final diagnostic.
    fn to_diagnostic(&self, context: &dyn DiagnosticContext)
    -> Result<Diagnostic, DiagnosticError>;
}

/// Provider-side diagnostic value accepted by provider contexts.
pub trait DiagnosticLike: ToDiagnostic + std::fmt::Debug + Send + Sync + 'static {}

impl<T> DiagnosticLike for T where T: ToDiagnostic + std::fmt::Debug + Send + Sync + 'static {}

impl ToDiagnostic for Diagnostic {
    /// Return this already-resolved source diagnostic.
    fn to_diagnostic(
        &self,
        _context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        Ok(self.clone())
    }
}

impl<T> ToDiagnostic for Box<T>
where
    T: ToDiagnostic + ?Sized,
{
    /// Convert this boxed value into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        self.as_ref().to_diagnostic(context)
    }
}

impl<T> ToDiagnostic for Arc<T>
where
    T: ToDiagnostic + ?Sized,
{
    /// Convert this shared value into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        self.as_ref().to_diagnostic(context)
    }
}
