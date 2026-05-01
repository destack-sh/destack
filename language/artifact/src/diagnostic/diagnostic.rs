use std::any::Any;
use std::hash::Hash;

use destack_source::{Diagnostic, DiagnosticSeverity};

use crate::{DiagnosticAnchor, DiagnosticError, ProviderContext};

/// Provider-side typed value that can become one final diagnostic.
pub trait ToDiagnostic<R>
where
    R: Copy + Eq + Hash,
{
    /// Convert this typed provider diagnostic into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError>;
}

/// Provider-side typed diagnostic metadata.
pub trait ProviderDiagnostic<R>: ToDiagnostic<R> + std::fmt::Debug + Send + Sync + 'static
where
    R: Copy + Eq + Hash,
{
    /// Return this diagnostic as `Any` for policy-specific downcasts.
    fn as_any(&self) -> &dyn Any;

    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str;

    /// Return the default source severity.
    fn severity(&self) -> DiagnosticSeverity;

    /// Return the provider-side source anchor.
    fn anchor(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<DiagnosticAnchor, DiagnosticError>;

    /// Return whether source directives can control this diagnostic.
    fn is_directive(&self) -> bool;
}

impl<R> ToDiagnostic<R> for Diagnostic
where
    R: Copy + Eq + Hash,
{
    /// Return this already-resolved source diagnostic.
    fn to_diagnostic(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError> {
        Ok(self.clone())
    }
}

impl<R, T> ToDiagnostic<R> for Box<T>
where
    R: Copy + Eq + Hash,
    T: ToDiagnostic<R> + ?Sized,
{
    /// Convert this boxed provider diagnostic into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<Diagnostic, DiagnosticError> {
        self.as_ref().to_diagnostic(context)
    }
}

impl<R, T> ProviderDiagnostic<R> for Box<T>
where
    R: Copy + Eq + Hash,
    T: ProviderDiagnostic<R> + ?Sized,
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

    /// Return the provider-side source anchor.
    fn anchor(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<DiagnosticAnchor, DiagnosticError> {
        self.as_ref().anchor(context)
    }

    /// Return whether source directives can control this diagnostic.
    fn is_directive(&self) -> bool {
        self.as_ref().is_directive()
    }
}
