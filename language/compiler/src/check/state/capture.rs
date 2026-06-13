use destack_dir as dir;

/// Captures discovered for one walked function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Capture {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// Outer symbols read by this function.
    pub(in crate::check) symbols: Vec<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check) receiver: Option<ReceiverBinding>,
    /// The explicit capture directive.
    pub(in crate::check) directive: Option<dir::CaptureDirective>,
}

/// Receiver type visible in one lexical context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct Receiver {
    /// The nominal owner that supplies contextual `this`, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The superclass receiver type, when the owner extends one.
    pub(in crate::check) super_ty: Option<dir::GlobalTypeId>,
}

/// Receiver binding visible in one lexical context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverBinding {
    /// The receiver value symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The receiver type context.
    pub(in crate::check) receiver: Receiver,
}
