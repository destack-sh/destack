use std::mem;
use std::sync::Arc;

use destack_artifact::{DiagnosticBuilder, MirLowered};
use destack_mir::{
    AccessTable, AnalysisCache, AnalysisOptions, DropTable, EffectTable, Function, FunctionCache,
    LocalNodeIdAny, ResolutionTable, RetentionTable, TargetLayout, Tree,
};
use destack_source::ModuleId;

use crate::DiagnosticAnchor;
use crate::verify::{BorrowChecker, DropChecker, MoveChecker, VerifyError};

/// State for one MIR verification.
pub(crate) struct VerifyState<'a> {
    /// The module being verified.
    module: ModuleId,
    /// The MIR tree being verified.
    pub(in crate::verify) tree: &'a Tree,
    /// MIR drop definitions.
    pub(in crate::verify) drops: &'a DropTable,
    /// Explicit MIR memory accesses.
    pub(in crate::verify) accesses: &'a AccessTable,
    /// Target ABI layout.
    target: TargetLayout,

    /// Function and call effect table.
    pub(in crate::verify) effects: Arc<EffectTable>,
    /// Static callsite resolutions.
    pub(in crate::verify) resolution: Arc<ResolutionTable>,

    /// Verified ownership retention.
    retention: RetentionTable,
    /// Accumulated errors.
    errors: Vec<DiagnosticBuilder<VerifyError>>,
}

impl<'a> VerifyState<'a> {
    /// Create verification state for one lowered MIR module.
    pub(crate) fn new(module: ModuleId, lowered: &'a MirLowered) -> Self {
        let mut analyses = AnalysisCache::new();
        let resolution = analyses.resolution(&lowered.tree, &lowered.dispatch);
        let effects = analyses.effect(
            &lowered.tree,
            &lowered.accesses,
            &lowered.effects,
            &lowered.dispatch,
        );

        Self {
            module,
            tree: &lowered.tree,
            drops: &lowered.drops,
            accesses: &lowered.accesses,
            target: lowered.target,
            effects,
            resolution,
            retention: RetentionTable::default(),
            errors: Vec::new(),
        }
    }

    /// Verify the current MIR tree.
    pub(crate) fn verify(&mut self) {
        let tree = self.tree;

        // verify every defined function
        for (_, function) in tree.iter_nodes::<Function>() {
            if !function.is_defined() {
                continue;
            }

            let options = AnalysisOptions::new(self.target);
            let mut analyses = FunctionCache::with_options(options);

            // check moves before borrow legality
            MoveChecker::new(function, tree, self, &mut analyses).check();

            // check borrows and retain ownership roots
            let retention = BorrowChecker::new(function, tree, self, &mut analyses).check();
            self.retention.extend(retention);
        }

        self.retention.sort();

        // check drop hooks for forbidden effects
        DropChecker::new(self).check();
    }

    /// Create a source anchor for one MIR node.
    pub(crate) fn anchor(&self, node: LocalNodeIdAny) -> DiagnosticAnchor {
        if let Some(span) = self.tree.source_span_by_id(node.id) {
            DiagnosticAnchor::Span(span)
        } else {
            DiagnosticAnchor::Module(self.module)
        }
    }

    /// Emit one verification error.
    pub(crate) fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        self.errors.push(error.into());
    }

    /// Take verified ownership retention.
    pub(crate) fn take_retention(&mut self) -> RetentionTable {
        mem::take(&mut self.retention)
    }

    /// Take accumulated errors.
    pub(crate) fn take_errors(&mut self) -> Vec<DiagnosticBuilder<VerifyError>> {
        mem::take(&mut self.errors)
    }
}
