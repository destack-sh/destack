use destack_dir as dir;
use dir::{Asynchrony, Block, Expression, LocalNodeId, Mutability, NodeType};

use crate::elaborate::ElaborateState;
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
        let mut new_leading_expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut modified = false;

        for &expression_id in &block.leading_expressions {
            let binding_expression_id = match state.tree.get(expression_id) {
                Expression::Let { .. } | Expression::LetElse { .. } | Expression::Using { .. } => {
                    expression_id
                }
                _ => {
                    new_leading_expressions.push(expression_id);
                    continue;
                }
            };

            let (export, ambient, binding_kind, declarators) =
                match state.tree.get(binding_expression_id).clone() {
                    Expression::Let {
                        export,
                        ambient,
                        mutability,
                        declarators,
                    } => (
                        export,
                        ambient,
                        BindingKind::Let { mutability },
                        declarators,
                    ),
                    Expression::Using {
                        asynchrony,
                        export,
                        ambient,
                        declarators,
                    } => (
                        export,
                        ambient,
                        BindingKind::Using { asynchrony },
                        declarators,
                    ),
                    _ => {
                        new_leading_expressions.push(expression_id);
                        continue;
                    }
                };

            // only split if there are multiple declarators
            if declarators.len() <= 1 {
                new_leading_expressions.push(expression_id);
                continue;
            }

            modified = true;
            let scope = state.tree.get_scope(binding_expression_id);

            // this original multi-declarator binding is removed from the block
            state.tree.mark_inactive(binding_expression_id.into_any());

            // create individual binding for each declarator
            for declarator_id in declarators {
                let new_let_id = state.tree.reserve_from(
                    NodeType::Expression,
                    binding_expression_id.into_any(),
                    scope,
                    None,
                    Some(dir::ProvenanceReason::Elaborated),
                );

                let new_binding = match binding_kind {
                    BindingKind::Let { mutability } => Expression::Let {
                        export,
                        ambient,
                        mutability,
                        declarators: vec![declarator_id],
                    },
                    BindingKind::Using { asynchrony } => Expression::Using {
                        asynchrony,
                        export,
                        ambient,
                        declarators: vec![declarator_id],
                    },
                };
                let new_let: LocalNodeId<Expression> =
                    state.tree.insert_as_owner(new_let_id, new_binding);
                self.set_void_expression_type(state.types, state.types.module_id, new_let);
                new_leading_expressions.push(new_let);
            }
        }

        // update block if modified
        if modified {
            let new_block = Block {
                leading_expressions: new_leading_expressions,
                ..block
            };
            state.tree.replace(block_id, new_block);
        }

        Ok(())
    }
}
