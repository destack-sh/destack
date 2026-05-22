use destack_dir as dir;
use smallvec::SmallVec;

use super::{Predicate, VariableId};

/// Dynamic flow state while visiting one control-flow region.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FlowState {
    /// Predicate that guards reachability.
    guard: Predicate,
    /// Branch-local symbol refinements.
    refinements: SmallVec<[Refinement; 4]>,
}

impl Default for FlowState {
    /// Create reachable flow with no refinements.
    fn default() -> Self {
        Self::reachable()
    }
}

impl FlowState {
    /// Create reachable flow with no refinements.
    pub(in crate::check) fn reachable() -> Self {
        Self {
            guard: Predicate::Always,
            refinements: SmallVec::new(),
        }
    }

    /// Return the current reachability guard.
    pub(in crate::check) fn guard(&self) -> &Predicate {
        &self.guard
    }

    /// Set the current reachability guard.
    pub(in crate::check) fn set_guard(&mut self, guard: Predicate) {
        self.guard = guard;
    }

    /// Add one branch-local refinement.
    pub(in crate::check) fn push_refinement(&mut self, refinement: Refinement) {
        self.refinements.push(refinement);
    }

    /// Return refinements for one symbol.
    pub(in crate::check) fn refinements(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = &Refinement> {
        self.refinements
            .iter()
            .filter(move |refinement| refinement.symbol == symbol)
    }
}

/// Branch-local refinement for one symbol.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Refinement {
    /// The refined symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The refinement predicate.
    pub(in crate::check) predicate: Predicate,
    /// The refined value variable.
    pub(in crate::check) value: VariableId,
}
