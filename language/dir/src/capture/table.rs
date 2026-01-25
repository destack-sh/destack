use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{CaptureDirective, CaptureSet, GlobalSymbolId};

/// Capture side table keyed by function symbols.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CaptureTable {
    /// Capture sets for each function symbol.
    pub captures_by_function: IndexMap<GlobalSymbolId, CaptureSet>,
    /// Locals captured by reference for each owner function symbol.
    pub reference_locals_by_owner: IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
}

impl CaptureTable {
    /// Create an empty capture table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store capture set for a function symbol.
    pub fn set_capture_set(&mut self, symbol: GlobalSymbolId, capture_set: CaptureSet) {
        self.captures_by_function.insert(symbol, capture_set);
    }

    /// Store capture directive for a function symbol.
    pub fn set_capture_directive(&mut self, symbol: GlobalSymbolId, directive: CaptureDirective) {
        self.captures_by_function
            .entry(symbol)
            .and_modify(|info| info.directive = directive.clone())
            .or_insert_with(|| CaptureSet {
                captures: Vec::new(),
                directive,
            });
    }

    /// Get capture directive for a function symbol.
    pub fn capture_directive(&self, symbol: GlobalSymbolId) -> Option<&CaptureDirective> {
        self.captures_by_function
            .get(&symbol)
            .map(|info| &info.directive)
    }

    /// Get capture set for a function symbol.
    pub fn capture_set(&self, symbol: GlobalSymbolId) -> Option<&CaptureSet> {
        self.captures_by_function.get(&symbol)
    }

    /// Store by-reference locals for an owner function symbol.
    pub fn set_reference_locals(&mut self, symbol: GlobalSymbolId, locals: Vec<GlobalSymbolId>) {
        self.reference_locals_by_owner.insert(symbol, locals);
    }

    /// Get by-reference locals for an owner function symbol.
    pub fn reference_locals(&self, symbol: GlobalSymbolId) -> Option<&[GlobalSymbolId]> {
        self.reference_locals_by_owner
            .get(&symbol)
            .map(|locals| locals.as_slice())
    }
}
