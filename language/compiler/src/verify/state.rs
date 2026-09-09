use std::mem;
use std::sync::Arc;

use destack_artifact::{DiagnosticBuilder, MirLowered};
use destack_core::FxIndexSet;
use destack_mir::{
    AccessTable, AnalysisCache, AnalysisOptions, DispatchTable, DropTable, EffectTable, Function,
    FunctionBehavior, FunctionCache, FunctionId, LocalNodeId, LocalNodeIdAny, ResolutionTable,
    RetentionTable, SafepointTable, TargetLayout, Tree,
};

use crate::DiagnosticAnchor;
use crate::verify::{FunctionChecker, VerifyError};

/// State for one MIR verification.
pub(crate) struct VerifyState<'a> {
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
    /// The safepoints of every verified function.
    safepoints: SafepointTable,
    /// Accumulated errors.
    errors: Vec<DiagnosticBuilder<VerifyError>>,
}

impl<'a> VerifyState<'a> {
    /// Create verification state for one lowered MIR module.
    pub(crate) fn new(lowered: &'a MirLowered) -> Self {
        Self::over(
            &lowered.tree,
            &lowered.drops,
            &lowered.accesses,
            &lowered.dispatch,
            &lowered.effects,
            lowered.target,
        )
    }

    /// Create verification state over one MIR tree and its tables.
    pub(crate) fn over(
        tree: &'a Tree,
        drops: &'a DropTable,
        accesses: &'a AccessTable,
        dispatch: &DispatchTable,
        effects: &EffectTable,
        target: TargetLayout,
    ) -> Self {
        let mut analyses = AnalysisCache::new();
        let resolution = analyses.resolution(tree, dispatch);
        let effects = analyses.effect(tree, accesses, effects, dispatch);

        Self {
            tree,
            drops,
            accesses,
            target,
            effects,
            resolution,
            retention: RetentionTable::default(),
            safepoints: SafepointTable::default(),
            errors: Vec::new(),
        }
    }

    /// Verify every defined function of the tree.
    pub(crate) fn verify(&mut self) {
        let functions: Vec<_> = self
            .tree
            .iter_nodes::<Function>()
            .filter(|(_, function)| function.is_defined())
            .map(|(id, _)| id)
            .collect();

        self.verify_functions(&functions);
    }

    /// Verify the given functions.
    pub(crate) fn verify_functions(&mut self, functions: &[FunctionId]) {
        let tree = self.tree;

        // verify each function
        for id in functions {
            let function = tree.get(*id);

            let options = AnalysisOptions::new(self.target);
            let mut analyses = FunctionCache::with_options(options);
            let verdict = FunctionChecker::new(function, tree, self, &mut analyses).check();
            self.retention.extend(verdict.retention);
            self.safepoints.extend(verdict.safepoints);
        }

        self.retention.sort();
        self.safepoints.sort();

        // check drop hooks for forbidden effects
        self.check_drop_effects();
    }

    /// Check every authored drop hook for forbidden effects.
    fn check_drop_effects(&mut self) {
        // collect each hook once across its registered storages
        let hooks: FxIndexSet<_> = self.drops.hooks().map(|(_, function)| function).collect();
        let effects = self.effects.clone();

        for function in hooks {
            // check each hook where its body was analyzed, an imported hook checking in its module
            let Some(effect) = effects.function(function) else {
                continue;
            };
            let behavior = effect.behavior.clone();

            self.check_drop_hook(function, &behavior);
        }
    }

    /// Check one drop hook's closed behavior.
    fn check_drop_hook(&mut self, function: LocalNodeId<Function>, behavior: &FunctionBehavior) {
        if !behavior.park.may_park() {
            return;
        }

        let anchor = self.anchor(function.into_any());

        self.emit_error(VerifyError::DropEffect { anchor });
    }

    /// Create a source anchor for one MIR node.
    pub(crate) fn anchor(&self, node: LocalNodeIdAny) -> DiagnosticAnchor {
        let Some(span) = self.tree.source_span_by_id(node.id) else {
            unreachable!("verified MIR node {node:?} has no source span");
        };

        DiagnosticAnchor::Span(span)
    }

    /// Emit one verification error.
    pub(crate) fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        self.errors.push(error.into());
    }

    /// Take verified ownership retention.
    pub(crate) fn take_retention(&mut self) -> RetentionTable {
        mem::take(&mut self.retention)
    }

    /// Take the safepoints of every verified function.
    pub(crate) fn take_safepoints(&mut self) -> SafepointTable {
        mem::take(&mut self.safepoints)
    }

    /// Take accumulated errors.
    pub(crate) fn take_errors(&mut self) -> Vec<DiagnosticBuilder<VerifyError>> {
        mem::take(&mut self.errors)
    }
}
