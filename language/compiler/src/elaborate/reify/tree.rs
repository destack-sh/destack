use destack_dir as dir;
use dir::{Expression, LocalNodeId};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Reify a tree literal into constructor calls.
    pub(super) fn reify_tree_expression(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify tree literals
        // <Div>{children}</Div> → createElement(Div, null, children)
        Ok(())
    }
}
