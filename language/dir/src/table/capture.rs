use indexmap::IndexMap;
use indexmap::map::Entry;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StringId};

/// Capture side table keyed by function symbols.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CaptureTable {
    /// Capture sets for each function symbol.
    pub captures_by_function: IndexMap<GlobalSymbolId, CaptureSet>,
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
        // preserve capture metadata when it already exists
        match self.captures_by_function.entry(symbol) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().directive = directive;
            }
            Entry::Vacant(entry) => {
                entry.insert(CaptureSet {
                    captures: Vec::new(),
                    reference_locals: Vec::new(),
                    this_symbol: None,
                    directive,
                });
            }
        }
    }

    /// Get capture directive for a function symbol.
    pub fn capture_directive(&self, symbol: GlobalSymbolId) -> Option<&CaptureDirective> {
        self.captures_by_function
            .get(&symbol)
            .map(|capture| &capture.directive)
    }

    /// Get capture set for a function symbol.
    pub fn capture_set(&self, symbol: GlobalSymbolId) -> Option<&CaptureSet> {
        self.captures_by_function.get(&symbol)
    }

    /// Store by-reference locals for an owner function symbol.
    pub fn set_reference_locals(&mut self, symbol: GlobalSymbolId, locals: Vec<GlobalSymbolId>) {
        // preserve capture metadata when it already exists
        match self.captures_by_function.entry(symbol) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().reference_locals = locals;
            }
            Entry::Vacant(entry) => {
                entry.insert(CaptureSet {
                    captures: Vec::new(),
                    reference_locals: locals,
                    this_symbol: None,
                    directive: CaptureDirective::default(),
                });
            }
        }
    }

    /// Get by-reference locals for an owner function symbol.
    pub fn reference_locals(&self, symbol: GlobalSymbolId) -> Option<&[GlobalSymbolId]> {
        self.captures_by_function
            .get(&symbol)
            .map(|capture| capture.reference_locals.as_slice())
    }
}

/// The capture mode for a closure binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    /// Borrow access to the original binding.
    Borrow,
    /// Copy the binding value into the environment.
    Copy,
    /// Move the binding value into the environment.
    Move,
}

/// A capture rule keyed by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRule {
    /// The binding name to override.
    pub name: StringId,
    /// The capture mode to use for this binding.
    pub mode: CaptureMode,
}

/// The capture directive for a closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureDirective {
    /// The default capture mode.
    pub default: CaptureMode,
    /// Per binding rules by name.
    pub rules: Vec<CaptureRule>,
}

impl Default for CaptureDirective {
    fn default() -> Self {
        Self {
            default: CaptureMode::Borrow,
            rules: Vec::new(),
        }
    }
}

impl CaptureDirective {
    /// Return the capture mode for the given binding name.
    pub fn mode_for_name(&self, name: StringId) -> CaptureMode {
        self.rules
            .iter()
            .find(|rule_entry| rule_entry.name == name)
            .map(|rule_entry| rule_entry.mode)
            .unwrap_or(self.default)
    }
}

/// A single captured binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedBinding {
    /// The captured symbol.
    pub symbol: GlobalSymbolId,
    /// The capture mode for the symbol.
    pub mode: CaptureMode,
}

/// Capture set for a function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSet {
    /// The resolved captures in discovery order.
    pub captures: Vec<CapturedBinding>,
    /// Locals captured by reference.
    pub reference_locals: Vec<GlobalSymbolId>,
    /// The symbol bound to `this` when captured.
    pub this_symbol: Option<GlobalSymbolId>,
    /// The capture directive applied to this function.
    pub directive: CaptureDirective,
}
