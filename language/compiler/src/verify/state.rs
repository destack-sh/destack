use destack_artifact::{DiagnosticBuilder, DiagnosticLike, MirLowered, MirVerified};
use destack_core::StringPool;
use destack_mir as mir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::DiagnosticAnchor;
use crate::verify::VerifyError;

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
    /// The MIR tree being verified.
    pub(in crate::verify) tree: mir::Tree,
    /// Target ABI layout.
    pub(in crate::verify) target_layout: mir::TargetLayout,
    /// Canonical MIR type table.
    pub(in crate::verify) types: mir::TypeTable,
    /// Canonical MIR layout table.
    pub(in crate::verify) layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub(in crate::verify) dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub(in crate::verify) drops: mir::DropTable,
    /// Explicit MIR memory access table.
    pub(in crate::verify) memory: mir::MemoryTable,
    /// Function and call effect table.
    pub(in crate::verify) effects: mir::EffectTable,
    /// Static profile counter table.
    pub(in crate::verify) profile_table: mir::ProfileTable,
    /// Strings needed by generated MIR names.
    pub(in crate::verify) strings: &'a StringPool,
    /// Accumulated errors.
    errors: Vec<DiagnosticBuilder<VerifyError>>,
}

impl std::fmt::Debug for VerifyState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyState")
            .field("module", &self.module)
            .field("profile", &self.profile)
            .field("target", &self.target)
            .field("tree", &"mir::Tree")
            .field("errors", &self.errors.len())
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
        lowered: MirLowered,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            module,
            profile,
            target,
            context,
            tree: lowered.tree,
            target_layout: lowered.target,
            types: lowered.types,
            layouts: lowered.layouts,
            dispatch: lowered.dispatch,
            drops: lowered.drops,
            memory: lowered.memory,
            effects: lowered.effects,
            profile_table: lowered.profile,
            strings,
            errors: Vec::new(),
        }
    }

    /// Finish verified MIR.
    pub(crate) fn finish(self) -> MirVerified {
        MirVerified {
            tree: self.tree,
            target: self.target_layout,
            types: self.types,
            layouts: self.layouts,
            dispatch: self.dispatch,
            drops: self.drops,
            memory: self.memory,
            effects: self.effects,
            profile: self.profile_table,
        }
    }

    /// Create a source anchor for one MIR node.
    pub(crate) fn anchor(&self, tree: &mir::Tree, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        if let Some(span) = tree.source_span_by_id(node.id) {
            DiagnosticAnchor::Span(span)
        } else {
            DiagnosticAnchor::Module(self.module)
        }
    }

    /// Emit one verification error.
    pub(crate) fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        self.errors.push(error.into());
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
