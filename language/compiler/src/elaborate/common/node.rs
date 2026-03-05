use destack_dir as dir;
use dir::{
    Argument, BindingAnchor, Block, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    Declarator, Expression, LocalNodeId, LocalNodeIdAny, LocalSymbolId, Mutability, Name, NodeType,
    Path, Pattern,
};

use super::ElaborateState;
use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when one node is active in the local symbol table.
    pub(crate) fn is_active_in_state(
        &self,
        state: &ElaborateState<'_>,
        node_id: LocalNodeIdAny,
    ) -> bool {
        self.is_node_active(state.tree, state.symbols, node_id)
    }

    /// Recompute one block expression type in local state.
    pub(crate) fn reinfer_block_type_in_state(
        &self,
        state: &mut ElaborateState<'_>,
        block_id: LocalNodeId<Block>,
    ) -> ElaborateResult<()> {
        self.reinfer_block_type(block_id, state.tree, state.types, state.ctx.module_id)
    }

    /// Insert a statement wrapper expression around another expression.
    pub(crate) fn insert_statement_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        statement: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // create one statement wrapper expression
        let statement_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let statement_id = state
            .tree
            .insert(statement_id, Expression::Statement { statement });

        // statement expressions are always void typed
        self.set_void_expression_type(state.types, state.ctx.module_id, statement_id);
        statement_id
    }

    /// Clone an expression node and copy node-level analysis metadata.
    pub(crate) fn clone_expression_with_analysis(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        expression: &Expression,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // clone the expression node in the requested scope
        let cloned_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let cloned_id = state.tree.insert(cloned_id, expression.clone());

        // copy inferred type and resolution metadata
        state.types.copy_node_analysis(
            origin_id.into_global_any(state.ctx.module_id),
            cloned_id.into_global_any(state.ctx.module_id),
        );

        cloned_id
    }

    /// Clone call argument nodes and copy node-level analysis metadata.
    pub(crate) fn clone_arguments_with_analysis(
        &self,
        state: &mut ElaborateState<'_>,
        origin_expression_id: LocalNodeId<Expression>,
        argument_ids: &[LocalNodeId<Argument>],
        scope: dir::LocalScope,
    ) -> Vec<LocalNodeId<Argument>> {
        let mut cloned_argument_ids = Vec::with_capacity(argument_ids.len());
        for argument_id in argument_ids {
            // clone each argument node in the current expression scope
            let argument = state.tree.get(*argument_id).clone();
            let cloned_argument_id = state.tree.reserve_from(
                NodeType::Argument,
                origin_expression_id.into_any(),
                scope,
                None,
            );
            let cloned_argument_id = state.tree.insert(cloned_argument_id, argument);

            // copy inferred analysis for the cloned node
            state.types.copy_node_analysis(
                argument_id.into_global_any(state.ctx.module_id),
                cloned_argument_id.into_global_any(state.ctx.module_id),
            );
            cloned_argument_ids.push(cloned_argument_id);
        }

        cloned_argument_ids
    }

    /// Replace an expression with a statement-wrapped explicit return.
    pub(crate) fn replace_expression_with_statement_return(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) {
        // clone the original value expression so the return can own it
        let expression = state.tree.get(expression_id).clone();
        let original_expression_id =
            self.clone_expression_with_analysis(state, expression_id, &expression, scope);

        // build the explicit return node
        let return_id =
            state
                .tree
                .reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
        let return_expression_id: LocalNodeId<Expression> = state.tree.insert(
            return_id,
            Expression::Return {
                value: Some(original_expression_id),
            },
        );
        self.set_never_expression_type(state.types, state.ctx.module_id, return_expression_id);

        // replace the original value expression with a statement wrapper
        let statement = Expression::Statement {
            statement: return_expression_id,
        };
        state.tree.replace(expression_id, statement);
        self.set_void_expression_type(state.types, state.ctx.module_id, expression_id);
    }

    /// Insert a local reference expression for one value symbol.
    pub(crate) fn insert_local_reference_expression_for_symbol(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeIdAny,
        scope: dir::LocalScope,
        name: dir::StringId,
        target_symbol: dir::GlobalSymbolId,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // resolve the symbol value type
        let value_type_id = self.value_type_id_or_error(
            state.ctx.module_id,
            target_symbol,
            origin_id,
            state.types,
        )?;

        // insert the local reference expression
        let reference_id = state
            .tree
            .reserve_from(NodeType::Expression, origin_id, scope, None);
        let reference_id: LocalNodeId<Expression> = state.tree.insert(
            reference_id,
            Expression::LocalReference {
                path: Path::from(&[name][..]),
                static_arguments: None,
                target_symbol,
            },
        );

        // annotate the reference type
        self.set_expression_type(
            state.types,
            state.ctx.module_id,
            reference_id,
            value_type_id,
        );

        Ok(reference_id)
    }

    /// Insert a single local let binding expression.
    pub(crate) fn insert_single_binding_let_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        scope: dir::LocalScope,
        name: dir::StringId,
        symbol: LocalSymbolId,
        pattern_mutability: Option<Mutability>,
        let_mutability: Mutability,
        value: Option<LocalNodeId<Expression>>,
    ) -> LocalNodeId<Expression> {
        // create the binding pattern
        let pattern_id =
            state
                .tree
                .reserve_from(NodeType::Pattern, origin_id.into_any(), scope, None);
        let pattern_id: LocalNodeId<Pattern> = state.tree.insert(
            pattern_id,
            Pattern::Binding {
                mutability: pattern_mutability,
                name,
                pattern: None,
                symbol,
            },
        );

        // create the declarator
        let declarator_id =
            state
                .tree
                .reserve_from(NodeType::Declarator, origin_id.into_any(), scope, None);
        let declarator_id: LocalNodeId<Declarator> = state.tree.insert(
            declarator_id,
            Declarator {
                pattern: pattern_id,
                ty: None,
                value,
            },
        );

        // create the descriptor and let expression
        let descriptor = DeclarationDescriptor {
            kind: DeclarationKind::Definition,
            abstraction: DeclarationAbstraction::Concrete,
            anchor: BindingAnchor::Instance,
            name: Some(Name::Identifier(name)),
            export: None,
            symbol,
        };
        let let_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let let_id: LocalNodeId<Expression> = state.tree.insert(
            let_id,
            Expression::Let {
                descriptor,
                mutability: let_mutability,
                declarators: vec![declarator_id],
            },
        );
        self.set_void_expression_type(state.types, state.ctx.module_id, let_id);

        let_id
    }
}
