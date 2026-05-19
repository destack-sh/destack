use destack_dir as dir;

use super::{InferId, StaticInferId};

/// Runtime flow state while visiting one control-flow region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowState {
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

impl FlowState {
    /// Create reachable flow with no narrowings.
    pub fn reachable() -> Self {
        Self {
            reachability: Reachability::Always,
            narrowings: Vec::new(),
        }
    }

    /// Create unreachable flow.
    pub fn unreachable() -> Self {
        Self {
            reachability: Reachability::Never,
            narrowings: Vec::new(),
        }
    }

    /// Create flow guarded by a static condition.
    pub fn reachable_when(condition: StaticInferId) -> Self {
        Self {
            reachability: Reachability::Static(condition),
            narrowings: Vec::new(),
        }
    }

    /// Return the current reachability.
    pub fn reachability(&self) -> Reachability {
        self.reachability
    }

    /// Set the current reachability.
    pub fn set_reachability(&mut self, reachability: Reachability) {
        self.reachability = reachability;
    }

    /// Return the current narrowed type for one symbol.
    pub fn narrowed_type(&self, symbol: dir::GlobalSymbolId) -> Option<InferId> {
        self.narrowings
            .iter()
            .rev()
            .find_map(|narrowing| (narrowing.symbol == symbol).then_some(narrowing.ty))
    }

    /// Add one branch-local narrowing.
    pub fn push_narrowing(&mut self, narrowing: Narrowing) {
        self.narrowings.push(narrowing);
    }

    /// Return a copy with one branch-local narrowing added.
    pub fn with_narrowing(mut self, narrowing: Narrowing) -> Self {
        self.push_narrowing(narrowing);

        self
    }
}

/// Compile-time reachability for a flow region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reachability {
    /// Region is always reachable.
    Always,
    /// Region is never reachable.
    Never,
    /// Region is reachable when a static condition is true.
    Static(StaticInferId),
}

/// Runtime type narrowing for one symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Narrowing {
    /// The narrowed symbol.
    pub symbol: dir::GlobalSymbolId,
    /// The narrowed type.
    pub ty: InferId,
}
