use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    BodyState, Expectation, NewtypeInstance, NewtypeOverload, OperatorExpressionResult, Origin,
    ProtocolCall, Scope, SignatureInstance,
};

/// The callable identity one selection decides for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Callee {
    /// A declared callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// A builtin binary operator.
    Operator(dir::BinaryOperator),
    /// A newtype construction under one backing selection rule.
    Newtype(dir::GlobalSymbolId, NewtypeOverload),
}

/// The decided question one callee answers for closed operands in one scope.
pub(in crate::check) type SelectionKey =
    (Callee, Option<dir::GlobalTypeId>, dir::TypeListId, Scope);

impl BodyState<'_, '_> {
    /// Derive the key one selection decides under, unless an operand is open.
    pub(in crate::check) fn derive_selection_key(
        &mut self,
        origin: Origin,
        callee: Callee,
        expectation: Option<Expectation>,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<SelectionKey>> {
        // an open operand leaves the question undecidable
        let expected = expectation.map(|expectation| expectation.target);
        for operand in expected.iter().chain(operands) {
            if self.type_flags(*operand)?.has_variable() {
                return Ok(None);
            }
        }
        let operands = self.intern_type_ids(operands)?;

        Ok(Some((
            callee,
            expected,
            operands,
            self.assuming_scope(origin)?,
        )))
    }
}

/// One callable selected and instantiated for closed operand types.
#[derive(Debug, Clone)]
pub(in crate::check) enum Selection {
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
