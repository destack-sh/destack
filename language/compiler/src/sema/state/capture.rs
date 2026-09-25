use tspp_dir as dir;

/// One capture directive and its decorator source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct CaptureAnnotation {
    /// The decorator source.
    pub(in crate::sema) source: dir::GlobalNodeId<dir::Decorator>,
    /// The selected capture directive.
    pub(in crate::sema) directive: dir::CaptureDirective,
}

/// Captures discovered for one walked function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct Capture {
    /// The function symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// Outer symbols read by this function.
    pub(in crate::sema) symbols: Vec<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::sema) receiver: Option<ReceiverBinding>,
    /// The optional capture annotation.
    pub(in crate::sema) annotation: Option<CaptureAnnotation>,
}

/// Receiver type visible in one lexical context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct Receiver {
    /// The declaration that introduces contextual `this`, when any.
    pub(in crate::sema) declaration: Option<dir::GlobalSymbolId>,
    /// The receiver declaration's default ownership, when fixed.
    pub(in crate::sema) ownership: Option<dir::Ownership>,
    /// The receiver type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The superclass receiver type, when the owner extends one.
    pub(in crate::sema) super_ty: Option<dir::GlobalTypeId>,
}

/// Receiver binding visible in one lexical context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct ReceiverBinding {
    /// The receiver value symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The receiver type context.
    pub(in crate::sema) receiver: Receiver,
}
