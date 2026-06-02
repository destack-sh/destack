use destack_dir as dir;

use crate::check::{CheckState, TypeOperand};

/// Resolved contextual receiver selected by check.
///
/// Examples:
/// ```ds
/// this
/// super
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverResolution {
    /// The receiver expression node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver syntax kind.
    pub(in crate::check) kind: dir::ReceiverKind,
    /// The declaration that introduces the receiver.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The receiver type variable.
    pub(in crate::check) ty: TypeOperand,
}

impl CheckState<'_> {
    /// Select one receiver resolution.
    pub(in crate::check) fn select_receiver(&mut self, receiver: ReceiverResolution) {
        match self.inference.select_receiver(receiver) {
            Ok(()) => {}
            Err(crate::CompilerError::Internal { message }) => self.record_internal_error(message),
            Err(error) => self.record_internal_error(format!("{error:?}")),
        }
    }
}
