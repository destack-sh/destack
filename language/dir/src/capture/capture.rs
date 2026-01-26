use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, StringId};

/// The capture kind for a closure binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureKind {
    /// Capture the binding value in the environment.
    ByValue,
    /// Capture a reference to the binding storage.
    ByReference,
    /// Capture the binding by move into the environment.
    ByMove,
}

/// The default capture policy for a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapturePolicy {
    /// Use const by value and let by reference.
    Default,
    /// Capture all bindings by value.
    ByValue,
    /// Capture all bindings by reference.
    ByReference,
    /// Capture all bindings by move.
    ByMove,
}

impl CapturePolicy {
    /// Resolve the default capture kind for a policy.
    pub fn default_kind(self) -> CaptureKind {
        match self {
            CapturePolicy::Default => CaptureKind::ByReference,
            CapturePolicy::ByValue => CaptureKind::ByValue,
            CapturePolicy::ByReference => CaptureKind::ByReference,
            CapturePolicy::ByMove => CaptureKind::ByMove,
        }
    }
}

/// A capture rule keyed by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRule {
    /// The binding name to override.
    pub name: StringId,
    /// The capture kind to use for this binding.
    pub kind: CaptureKind,
}

/// The capture directive for a closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureDirective {
    /// The default capture policy.
    pub policy: CapturePolicy,
    /// Per binding rules by name.
    pub rules: Vec<CaptureRule>,
}

impl Default for CaptureDirective {
    fn default() -> Self {
        Self {
            policy: CapturePolicy::Default,
            rules: Vec::new(),
        }
    }
}

impl CaptureDirective {
    /// Return an override capture kind for the given name.
    pub fn override_for_name(&self, name: StringId) -> Option<CaptureKind> {
        self.rules
            .iter()
            .find(|rule_entry| rule_entry.name == name)
            .map(|rule_entry| rule_entry.kind)
    }
}

/// A single captured binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedBinding {
    /// The captured symbol.
    pub symbol: GlobalSymbolId,
    /// The capture kind for the symbol.
    pub kind: CaptureKind,
}

/// Capture set for a function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSet {
    /// The resolved captures in discovery order.
    pub captures: Vec<CapturedBinding>,
    /// The symbol bound to `this` when captured.
    pub this_symbol: Option<GlobalSymbolId>,
    /// The capture directive applied to this function.
    pub directive: CaptureDirective,
}
