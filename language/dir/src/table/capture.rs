use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StringId};

/// Capture side table keyed by function symbols.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CaptureTable {
    /// Capture for each function symbol.
    pub capture_by_function: IndexMap<GlobalSymbolId, Capture>,
}

impl CaptureTable {
    /// Create an empty capture table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store capture for a function symbol.
    pub fn set_capture(&mut self, symbol: GlobalSymbolId, capture: Capture) {
        self.capture_by_function.insert(symbol, capture);
    }

    /// Store capture directive for a function symbol.
    pub fn set_capture_directive(&mut self, symbol: GlobalSymbolId, directive: CaptureDirective) {
        let capture = self.capture_by_function.entry(symbol).or_default();
        capture.directive = Some(directive);
    }

    /// Get capture directive for a function symbol.
    pub fn capture_directive(&self, symbol: GlobalSymbolId) -> Option<&CaptureDirective> {
        self.capture_by_function
            .get(&symbol)
            .and_then(|capture| capture.directive.as_ref())
    }

    /// Get capture for a function symbol.
    pub fn capture(&self, symbol: GlobalSymbolId) -> Option<&Capture> {
        self.capture_by_function.get(&symbol)
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

/// Captures for a function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capture {
    /// The resolved captures in discovery order.
    pub captures: Vec<CapturedBinding>,
    /// The captured `this` binding.
    pub this: Option<CapturedBinding>,
    /// The capture directive applied to this function.
    pub directive: Option<CaptureDirective>,
}
