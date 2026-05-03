use destack_dir as dir;
use dir::{Expression, LocalNodeId, NodeType, ScalarLiteral};

use crate::Compiler;
use crate::elaborate::ElaborateState;

impl Compiler {
    /// Insert a boolean scalar literal expression.
    pub(crate) fn insert_boolean_literal_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value: bool,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // insert a boolean scalar literal expression
        let literal_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let expression_id = state.tree.insert_as_owner(
            literal_id,
            Expression::ScalarLiteral {
                value: ScalarLiteral::Boolean(value),
            },
        );

        // annotate the literal with boolean type
        self.set_scalar_literal_type(
            state.types,
            state.tree.module_id,
            expression_id,
            ScalarLiteral::Boolean(value),
        );

        expression_id
    }
}
