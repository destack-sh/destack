use std::collections::HashMap;

use destack_dir::{GlobalSymbolId, LocalTypeId};

/// TypeContext holds contextual, flow-sensitive information during type analysis.
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Type narrowings currently in scope.
    /// For example, after `if (typeof x === "number")`, `x` maps to `number`.
    pub narrowings: HashMap<GlobalSymbolId, LocalTypeId>,
    /// Whether we're in an unreachable code region (after `return`, `throw`, etc.).
    pub is_unreachable: bool,
}

impl TypeContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            narrowings: HashMap::new(),
            is_unreachable: false,
        }
    }

    /// Fork the context for a code region (e.g., then/else of an if).
    /// Returns a new context that inherits current narrowings.
    pub fn fork(&self) -> Self {
        Self {
            narrowings: self.narrowings.clone(),
            is_unreachable: self.is_unreachable,
        }
    }

    /// Add a narrowing for a symbol.
    pub fn narrow(&mut self, symbol: GlobalSymbolId, ty: LocalTypeId) {
        self.narrowings.insert(symbol, ty);
    }

    /// Get the narrowed type for a symbol, if any.
    pub fn get_narrowed(&self, symbol: GlobalSymbolId) -> Option<LocalTypeId> {
        self.narrowings.get(&symbol).copied()
    }

    /// Mark this context as unreachable.
    pub fn mark_unreachable(&mut self) {
        self.is_unreachable = true;
    }

    /// Merge two "branched" context together.
    pub fn merge(&mut self, other: &TypeContext) {
        // if neither is unreachable
        if self.is_unreachable && other.is_unreachable {
            // both unreachable, stay unreachable
        }
        // if one is unreachable, keep the other's narrowings
        // if both branches are unreachable, result is unreachable
        else if self.is_unreachable {
            // self is unreachable, take other's state
            self.narrowings = other.narrowings.clone();
            self.is_unreachable = false;
        }
        // other is unreachable
        else if other.is_unreachable {
            // keep self's state
        }
        // both reachable -> keep only narrowings that exist in both
        else {
            self.narrowings
                .retain(|k, v| other.narrowings.get(k) == Some(v));
        }
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}
