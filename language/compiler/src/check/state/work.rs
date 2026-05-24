use destack_dir as dir;

use crate::CheckError;

use super::{CheckImportState, CheckVariableState, Constraint, FlowState, Obligation};

/// Check work state for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckWorkState {
    /// Checked dependency ids imported into this module.
    pub(in crate::check) imports: CheckImportState,
    /// Variable graph built and solved by check.
    pub(in crate::check) variables: CheckVariableState,
    /// Flow state while walking this module.
    pub(in crate::check) flow: FlowState,
    /// Function captures collected while walking.
    pub(in crate::check) captures: Vec<Capture>,
    /// Constraints produced by walking DIR.
    pub(in crate::check) constraints: Vec<Constraint>,
    /// Obligations produced by walking DIR.
    pub(in crate::check) obligations: Vec<Obligation>,

    /// Recoverable diagnostics collected while checking.
    pub(in crate::check) diagnostics: Vec<CheckError>,
}

impl CheckWorkState {
    /// Create empty check work state.
    pub(in crate::check) fn new() -> Self {
        Self {
            imports: CheckImportState::new(),
            variables: CheckVariableState::new(),
            flow: FlowState::default(),
            captures: Vec::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

/// Captures discovered for one walked function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Capture {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// Outer symbols read by this function.
    pub(in crate::check) symbols: Vec<dir::GlobalSymbolId>,
    /// Whether this function reads an outer `this`.
    pub(in crate::check) captures_this: bool,
}
