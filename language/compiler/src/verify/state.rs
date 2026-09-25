use std::mem;
use std::sync::Arc;

use destack_artifact::{DiagnosticBuilder, MirLowered};
use destack_core::{FxIndexSet, StringPool};
use destack_mir::{
    CallTable, DispatchTable, DropTable, EffectTable, FormatOptions, Formatter, Function,
    FunctionBehavior, FunctionCache, FunctionId, LocalNodeId, LocalNodeIdAny, ResolutionTable,
    RetentionTable, TargetLayout, Tree, TypeId, WitnessTable,
};

use crate::verify::{FunctionChecker, VerifyError};
use crate::{CompilerError, DiagnosticAnchor};

/// State for one MIR verification.
pub(crate) struct VerifyState<'a> {
    /// The MIR tree being verified.
    pub(in crate::verify) tree: &'a Tree,
    /// The strings the tree names its symbols through.
    strings: &'a StringPool,
    /// MIR drop definitions.
    pub(in crate::verify) drops: &'a DropTable,
    /// Target ABI layout.
    target: TargetLayout,

    /// Function and call effect table.
    pub(in crate::verify) effects: Arc<EffectTable>,

    /// Verified ownership retention.
    retention: RetentionTable,
    /// Accumulated errors.
    errors: Vec<DiagnosticBuilder<VerifyError>>,
    /// The MIR invariant violations found, each a defect of the stage producing the MIR.
    pub(in crate::verify) invalid_mir: Vec<String>,
}

impl<'a> VerifyState<'a> {
    /// Create verification state for one lowered MIR module.
    pub(crate) fn new(lowered: &'a MirLowered, strings: &'a StringPool) -> Self {
        Self::over(
            &lowered.tree,
            strings,
            &lowered.drops,
            &lowered.dispatch,
            &lowered.effects,
            Some(&lowered.witnesses),
            lowered.target,
        )
    }

    /// Create verification state over one MIR tree and its tables.
    pub(crate) fn over(
        tree: &'a Tree,
        strings: &'a StringPool,
        drops: &'a DropTable,
        dispatch: &DispatchTable,
        effects: &EffectTable,
        witnesses: Option<&WitnessTable>,
        target: TargetLayout,
    ) -> Self {
        let resolution = ResolutionTable::analyse(dispatch, witnesses, tree);
        let calls = CallTable::analyse(&resolution, tree);
        let effects = Arc::new(EffectTable::analyse(&resolution, &calls, effects, tree));

        Self {
            tree,
            strings,
            drops,
            target,
            effects,
            retention: RetentionTable::default(),
            errors: Vec::new(),
            invalid_mir: Vec::new(),
        }
    }

    /// Format one MIR type inside one function for a diagnostic.
    pub(in crate::verify) fn format_type(&self, function: FunctionId, ty: TypeId) -> String {
        Formatter::new(
            self.tree,
            self.target,
            self.strings,
            FormatOptions::default(),
        )
        .format_type_in(function, ty)
        .unwrap_or_else(|error| unreachable!("a constructed MIR type failed to format: {error:?}"))
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

        // check each function holding the MIR invariants its analyses assume
        for &id in functions {
            if !self.validate(id) {
                continue;
            }
            let mut analyses = FunctionCache::with_target_layout(self.target);
            let retention = FunctionChecker::new(id, tree, self, &mut analyses).check();
            self.retention.extend(retention);
        }

        self.retention.sort();

        // check drop hooks for forbidden effects
        self.check_drop_effects();
    }

    /// Check every authored drop hook for forbidden effects.
    fn check_drop_effects(&mut self) {
        let hooks: FxIndexSet<_> = self.drops.hooks().map(|(_, function)| function).collect();
        let effects = self.effects.clone();

        // check each hook whose body this module analyzed
        for function in hooks {
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

    /// Record one MIR invariant violation inside one function.
    pub(crate) fn reject_mir(&mut self, function: FunctionId, message: String) {
        let name = self.strings.get(self.tree.get(function).name);
        self.invalid_mir.push(format!("{message} in '{name}'"));
    }

    /// Take the MIR invariant violations as one internal error, if any.
    pub(crate) fn take_invalid_mir(&mut self) -> Option<CompilerError> {
        if self.invalid_mir.is_empty() {
            return None;
        }
        let violations = mem::take(&mut self.invalid_mir).join("; ");

        Some(CompilerError::Internal {
            message: format!("invalid MIR: {violations}"),
        })
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
