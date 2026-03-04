use destack_dir as dir;
use dir::{
    Asynchrony, Block, DeclarationDescriptor, Expression, LocalNodeId, Mutability, NodeType,
};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Split multi-declarator let statements into individual let statements.
    ///
    /// ```ds
    /// let a = 1, b = 2;
    /// ```
    /// ->
    /// ```ds
    /// let a = 1;
    /// let b = 2;
    /// ```
    pub(super) fn transform_split_declarators(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        // collect all blocks that need transformation
        let block_ids: Vec<_> = state.tree.iter_node_ids_of_type::<Block>();

        for block_id in block_ids {
            if !self.is_active_in_state(state, block_id.into_any()) {
                continue;
            }
            self.split_declarators_in_block(state, block_id)?;
        }

        Ok(())
    }

    /// Split multi-declarators in a single block.
    fn split_declarators_in_block(
        &self,
        state: &mut ElaborateState<'_>,
        block_id: LocalNodeId<Block>,
    ) -> ElaborateResult<()> {
        #[derive(Clone, Copy)]
        enum BindingKind {
            Let { mutability: Mutability },
            Using { asynchrony: Asynchrony },
        }

        let block = state.tree.get(block_id).clone();
        let mut new_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut modified = false;

        for expr_id in &block.expressions {
            // check if this is a statement wrapping a binding
            let (binding_expr_id, is_statement) = match state.tree.get(*expr_id) {
                Expression::Statement { statement } => (*statement, true),
                Expression::Let { .. } | Expression::Using { .. } => (*expr_id, false),
                _ => {
                    new_expressions.push(*expr_id);
                    continue;
                }
            };

            let (descriptor, binding_kind, declarators) =
                match state.tree.get(binding_expr_id).clone() {
                    Expression::Let {
                        descriptor,
                        mutability,
                        declarators,
                    } => (descriptor, BindingKind::Let { mutability }, declarators),
                    Expression::Using {
                        asynchrony,
                        descriptor,
                        declarators,
                    } => (descriptor, BindingKind::Using { asynchrony }, declarators),
                    _ => {
                        new_expressions.push(*expr_id);
                        continue;
                    }
                };

            // only split if there are multiple declarators
            if declarators.len() <= 1 {
                new_expressions.push(*expr_id);
                continue;
            }

            modified = true;
            let scope = state.tree.get_scope(binding_expr_id);

            // create individual binding for each declarator
            for declarator_id in declarators {
                let new_let_id = state.tree.reserve_from(
                    NodeType::Expression,
                    binding_expr_id.into_any(),
                    scope,
                    None,
                );

                // create a new descriptor for this binding using the declarator symbol
                let declarator = state.tree.get(declarator_id);
                let pattern = state.tree.get(declarator.pattern);
                let new_symbol = pattern.symbol().unwrap_or(descriptor.symbol);
                let new_descriptor = DeclarationDescriptor {
                    symbol: new_symbol,
                    ..descriptor.clone()
                };
                let new_binding = match binding_kind {
                    BindingKind::Let { mutability } => Expression::Let {
                        descriptor: new_descriptor,
                        mutability,
                        declarators: vec![declarator_id],
                    },
                    BindingKind::Using { asynchrony } => Expression::Using {
                        asynchrony,
                        descriptor: new_descriptor,
                        declarators: vec![declarator_id],
                    },
                };
                let new_let: LocalNodeId<Expression> = state.tree.insert(new_let_id, new_binding);
                self.set_void_expression_type(state.types, state.types.module_id, new_let);

                // wrap in statement if original was wrapped
                let final_expr = if is_statement {
                    self.insert_statement_expression(state, binding_expr_id, new_let, scope)
                } else {
                    new_let
                };

                new_expressions.push(final_expr);
            }
        }

        // update block if modified
        if modified {
            let new_block = Block {
                expressions: new_expressions,
                ..block
            };
            state.tree.replace(block_id, new_block);
        }

        Ok(())
    }
}
