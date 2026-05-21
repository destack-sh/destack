use destack_dir as dir;

use super::{StaticInferId, TypeInferId};

/// Runtime flow state while visiting one control-flow region.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct FlowState {
    /// Whether this flow state can still execute.
    reachability: Reachability,
    /// Branch-local symbol narrowings.
    narrowings: Vec<Narrowing>,
}

impl Default for FlowState {
    /// Create reachable flow with no narrowings.
    fn default() -> Self {
        Self::reachable()
    }
}

#[allow(dead_code)]
impl FlowState {
    /// Create reachable flow with no narrowings.
    pub(in crate::check) fn reachable() -> Self {
        Self {
            reachability: Reachability::Always,
            narrowings: Vec::new(),
        }
    }

    /// Create unreachable flow.
    pub(in crate::check) fn unreachable() -> Self {
        Self {
            reachability: Reachability::Never,
            narrowings: Vec::new(),
        }
    }

    /// Create flow guarded by a static condition.
    pub(in crate::check) fn reachable_when(condition: StaticInferId) -> Self {
        Self {
            reachability: Reachability::Static(condition),
            narrowings: Vec::new(),
        }
    }

    /// Return the current reachability.
    pub(in crate::check) fn reachability(&self) -> Reachability {
        self.reachability
    }

    /// Set the current reachability.
    pub(in crate::check) fn set_reachability(&mut self, reachability: Reachability) {
        self.reachability = reachability;
    }

    /// Return the current narrowed type for one symbol.
    pub(in crate::check) fn narrowed_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<TypeInferId> {
        self.narrowings
            .iter()
            .rev()
            .find_map(|narrowing| (narrowing.symbol == symbol).then_some(narrowing.ty))
    }

    /// Add one branch-local narrowing.
    pub(in crate::check) fn push_narrowing(&mut self, narrowing: Narrowing) {
        self.narrowings.push(narrowing);
    }

    /// Return a copy with one branch-local narrowing added.
    pub(in crate::check) fn with_narrowing(mut self, narrowing: Narrowing) -> Self {
        self.push_narrowing(narrowing);

        self
    }
}

/// Compile-time reachability for a flow region.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Reachability {
    /// Region is always reachable.
    Always,
    /// Region is never reachable.
    Never,
    /// Region is reachable when a static condition is true.
    Static(StaticInferId),
}

/// Runtime type narrowing for one symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Narrowing {
    /// The narrowed symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The narrowed type.
    pub(in crate::check) ty: TypeInferId,
}
