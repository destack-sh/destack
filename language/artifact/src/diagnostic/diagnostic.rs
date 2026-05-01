use std::any::Any;
use std::hash::Hash;

use destack_source::{Diagnostic, DiagnosticSeverity};

use crate::{DiagnosticContext, DiagnosticError, DiagnosticSite};

/// Typed value that can become one final diagnostic.
pub trait IntoDiagnostic<R>
where
    R: Copy + Eq + Hash,
{
    /// Convert this typed value into one final diagnostic.
    fn into_diagnostic(
        &self,
        context: &dyn DiagnosticContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError>;
}

/// Provider-side diagnostic before finalization.
pub trait DiagnosticDraft<R>: IntoDiagnostic<R> + std::fmt::Debug + Send + Sync + 'static
where
    R: Copy + Eq + Hash,
{
    /// Return this diagnostic as `Any` for policy-specific downcasts.
    fn as_any(&self) -> &dyn Any;

    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str;

    /// Return the default source severity.
    fn severity(&self) -> DiagnosticSeverity;

    /// Return the provider-side diagnostic site.
    fn site(&self) -> Result<DiagnosticSite, DiagnosticError>;

    /// Return whether source directives can control this diagnostic.
    fn is_directive(&self) -> bool;
}

impl<R> IntoDiagnostic<R> for Diagnostic
where
    R: Copy + Eq + Hash,
{
    /// Return this already-resolved source diagnostic.
    fn into_diagnostic(
        &self,
        _context: &dyn DiagnosticContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError> {
        Ok(self.clone())
    }
}

impl<R, T> IntoDiagnostic<R> for Box<T>
where
    R: Copy + Eq + Hash,
    T: IntoDiagnostic<R> + ?Sized,
{
    /// Convert this boxed value into one final diagnostic.
    fn into_diagnostic(
        &self,
        context: &dyn DiagnosticContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError> {
        self.as_ref().into_diagnostic(context)
    }
}

impl<R, T> DiagnosticDraft<R> for Box<T>
where
    R: Copy + Eq + Hash,
    T: DiagnosticDraft<R> + ?Sized,
{
    /// Return this diagnostic as `Any` for policy-specific downcasts.
    fn as_any(&self) -> &dyn Any {
        self.as_ref().as_any()
    }

    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str {
        self.as_ref().code()
    }

    /// Return the default source severity.
    fn severity(&self) -> DiagnosticSeverity {
        self.as_ref().severity()
    }

    /// Return the provider-side diagnostic site.
    fn site(&self) -> Result<DiagnosticSite, DiagnosticError> {
        self.as_ref().site()
    }

    /// Return whether source directives can control this diagnostic.
    fn is_directive(&self) -> bool {
        self.as_ref().is_directive()
    }
}
