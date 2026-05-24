use destack_dir as dir;

use crate::CheckError;

use super::{CheckVariableState, Constraint, FlowState, Obligation, VariableId};

/// Check work state for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckWorkState {
    /// Variable graph built and solved by check.
    pub(in crate::check) variables: CheckVariableState,
    /// Flow state while walking this module.
    pub(in crate::check) flow: FlowState,
    /// Receiver resolutions collected while walking.
    pub(in crate::check) receivers: Vec<ReceiverResolution>,
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
            variables: CheckVariableState::new(),
            flow: FlowState::default(),
            receivers: Vec::new(),
            captures: Vec::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl CheckWorkState {
    /// Record one receiver resolution whose type still needs commit.
    pub(in crate::check) fn record_receiver_resolution(&mut self, receiver: ReceiverResolution) {
        self.receivers.push(receiver);
    }
}

/// Receiver resolution collected while walking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverResolution {
    /// The receiver expression node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver syntax kind.
    pub(in crate::check) kind: dir::ReceiverKind,
    /// The declaration that introduces the receiver, when known.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type variable.
    pub(in crate::check) ty: VariableId,
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
