use destack_dir::{Expression, LocalNodeId};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Reify overloaded operators into resolved method calls.
    pub(super) fn reify_operator_expression(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        Ok(())
    }
}
