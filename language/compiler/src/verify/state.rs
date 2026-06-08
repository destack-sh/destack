use destack_artifact::{DiagnosticBuilder, DiagnosticLike};
use destack_mir as mir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::DiagnosticAnchor;
use crate::verify::VerifyError;

/// Borrow obligation with a source anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BorrowObligationRecord {
    /// The borrow obligation.
    pub obligation: mir::BorrowObligation,
    /// The suspension point that needs the proof.
    pub anchor: DiagnosticAnchor,
}

/// State for one verify phase provider run.
pub(crate) struct VerifyState<'a> {
    /// The module being verified.
    pub(in crate::verify) module: ModuleId,
    /// The profile being verified.
    pub(in crate::verify) profile: ProfileId,
    /// The target being verified.
    pub(in crate::verify) target: TargetId,
    /// The provider attempt that receives diagnostics.
    pub(in crate::verify) context: &'a dyn ProviderContext,
    /// Accumulated errors.
    errors: Vec<DiagnosticBuilder<VerifyError>>,
    /// Accumulated borrow obligations.
    borrow_obligations: Vec<BorrowObligationRecord>,
}

impl std::fmt::Debug for VerifyState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyState")
            .field("module", &self.module)
            .field("profile", &self.profile)
            .field("target", &self.target)
            .field("errors", &self.errors.len())
            .field("borrow_obligations", &self.borrow_obligations.len())
            .finish()
    }
}

impl<'a> VerifyState<'a> {
    /// Create verify provider state.
    pub(in crate::verify) fn new(
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &'a dyn ProviderContext,
    ) -> Self {
        Self {
            module,
            profile,
            target,
            context,
            errors: Vec::new(),
            borrow_obligations: Vec::new(),
        }
    }

    /// Create a source anchor for one MIR node.
    pub(crate) fn anchor(&self, tree: &mir::Tree, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        if let Some(span) = tree.get_span_by_id(node.id) {
            DiagnosticAnchor::Span(span)
        } else {
            DiagnosticAnchor::Module(self.module)
        }
    }

    /// Emit one verification error.
    pub(crate) fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        self.errors.push(error.into());
    }

    /// Record one borrow obligation.
    pub(crate) fn require_borrow(&mut self, obligation: BorrowObligationRecord) {
        if !self.borrow_obligations.contains(&obligation) {
            self.borrow_obligations.push(obligation);
        }
    }

    /// Return true when any errors were emitted.
    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Return accumulated errors.
    #[cfg(test)]
    pub(crate) fn errors(&self) -> &[DiagnosticBuilder<VerifyError>] {
        &self.errors
    }

    /// Return accumulated borrow obligations.
    #[cfg(test)]
    pub(crate) fn borrow_obligations(&self) -> &[BorrowObligationRecord] {
        &self.borrow_obligations
    }

    /// Drain all diagnostics.
    pub(crate) fn take_diagnostics(&mut self) -> Vec<Box<dyn DiagnosticLike>> {
        let mut diagnostics: Vec<Box<dyn DiagnosticLike>> = Vec::with_capacity(self.errors.len());

        diagnostics.extend(
            self.errors
                .drain(..)
                .map(|diagnostic| Box::new(diagnostic) as Box<dyn DiagnosticLike>),
        );

        diagnostics
    }
}
