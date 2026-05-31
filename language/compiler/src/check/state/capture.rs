use destack_dir as dir;

use crate::check::TypeOperand;

/// Captures discovered for one walked function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Capture {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// Outer symbols read by this function.
    pub(in crate::check) symbols: Vec<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check) receiver: Option<ReceiverCapture>,
    /// The explicit capture directive.
    pub(in crate::check) directive: Option<dir::CaptureDirective>,
}

/// Receiver captured by one walked function body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverCapture {
    /// The receiver symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The nominal owner that supplies contextual `this`, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type.
    pub(in crate::check) ty: TypeOperand,
}
