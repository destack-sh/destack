use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    BodyState, Expectation, NewtypeInstance, NewtypeOverload, OperatorExpressionResult,
    ProtocolCall, SignatureInstance,
};

/// The callable identity one selection decides for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Callee {
    /// A declared callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// A builtin binary operator.
    Operator(dir::BinaryOperator),
    /// A newtype construction under one backing selection rule.
    Newtype(dir::GlobalSymbolId, NewtypeOverload),
}

/// The decided question one callee answers for closed operands.
pub(in crate::sema) type SelectionKey = (Callee, Option<dir::GlobalTypeId>, dir::TypeListId);

impl BodyState<'_, '_> {
    /// Derive the key one selection decides under, unless an operand is open.
    pub(in crate::sema) fn derive_selection_key(
        &mut self,
        callee: Callee,
        expectation: Option<Expectation>,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<SelectionKey>> {
        // open, parameter, and This operands leave the question undecidable
        let expected = expectation.map(|expectation| expectation.target);
        for operand in expected.iter().chain(operands) {
            let flags = self.type_flags(*operand)?;
            if flags.has_variable() || flags.has_parameter() || flags.has_this() {
                return Ok(None);
            }
        }
        let operands = self.intern_type_ids(operands)?;

        Ok(Some((callee, expected, operands)))
    }
}

/// One callable selected and instantiated for closed operand types.
#[derive(Debug, Clone)]
pub(in crate::sema) enum Selection {
    /// The selected declaration with its instantiated signature.
    Callable(SignatureInstance),
    /// The selected newtype backing without per-site coercions.
    Newtype(NewtypeInstance),
    /// The selected operator protocol call.
    Protocol(ProtocolCall, OperatorExpressionResult),
    /// A builtin binary operator application.
    Builtin {
        /// The applied result type.
        result: dir::GlobalTypeId,
        /// The coerced operand types.
        operands: [dir::GlobalTypeId; 2],
    },
    /// No candidate applies to these operands.
    Rejected,
}
