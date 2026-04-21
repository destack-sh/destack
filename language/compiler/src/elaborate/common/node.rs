use destack_dir as dir;
use dir::{
    Ambientness, Block, Declarator, Expression, LocalNodeId, LocalNodeIdAny, LocalSymbolId,
    Mutability, NodeType, Path, Pattern,
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

    /// Return one expression for effect-position insertion.
    pub(crate) fn insert_effect_expression(
        &self,
        state: &mut ElaborateState<'_>,
        _origin_id: LocalNodeId<Expression>,
        expression_id: LocalNodeId<Expression>,
        _scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        let _ = state;

        expression_id
    }

    /// Clone an expression node and copy node-level analysis metadata.
    pub(crate) fn clone_expression_with_analysis(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        expression: &Expression,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // FUGU #Performance #Architecture: slice 3 still clones whole expression nodes
        // during elaborate rewrites, slice 4 should replace this with finer-grained builders

        // clone the expression node in the requested scope
        let cloned_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let cloned_id = state.tree.insert_as_owner(cloned_id, expression.clone());

        // copy inferred type and resolution metadata
        state.types.copy_node_analysis(
            origin_id.into_global_any(state.ctx.module_id),
            cloned_id.into_global_any(state.ctx.module_id),
        );

        cloned_id
    }

    /// Replace one expression with an explicit return expression.
    pub(crate) fn replace_expression_with_explicit_return(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) {
        // clone the original expression into a new value node
        let original_expression = state.tree.get(expression_id).clone();
        let value_id =
            self.clone_expression_with_analysis(state, expression_id, &original_expression, scope);

        // replace the original expression with an explicit return
        state.tree.replace(
            expression_id,
            Expression::Return {
                value: Some(value_id),
            },
        );
        self.set_never_expression_type(state.types, state.ctx.module_id, expression_id);
    }

    /// Insert one assignment pattern that targets one expression.
    pub(crate) fn insert_expression_assign_pattern(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeIdAny,
        scope: dir::LocalScope,
        parent_id: LocalNodeIdAny,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<dir::AssignPattern> {
        let assign_pattern_id = state.tree.reserve_from(
            NodeType::AssignPattern,
            origin_id,
            scope,
            Some(parent_id),
            Some(dir::ProvenanceReason::Elaborated),
        );

        state.tree.insert_as_owner(
            assign_pattern_id,
            dir::AssignPattern::Expression {
                value: expression_id,
            },
        )
    }

    /// Replace one expression with an explicit assignment expression.
    pub(crate) fn replace_expression_with_explicit_assignment(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        target: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) {
        // clone the original expression into a new value node
        let original_expression = state.tree.get(expression_id).clone();
        let value_id =
            self.clone_expression_with_analysis(state, expression_id, &original_expression, scope);
        let left = self.insert_expression_assign_pattern(
            state,
            expression_id.into_any(),
            scope,
            expression_id.into_any(),
            target,
        );

        // replace the original expression with an explicit assignment
        state.tree.replace(
            expression_id,
            Expression::Assign {
                left,
                right: value_id,
            },
        );
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
        let reference_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id,
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let reference_id: LocalNodeId<Expression> = state.tree.insert_as_owner(
            reference_id,
            Expression::LocalReference {
                path: Path::from(&[name][..]),
                generic_arguments: Vec::new(),
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
        let pattern_id = state.tree.reserve_from(
            NodeType::Pattern,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let pattern_id: LocalNodeId<Pattern> = state.tree.insert_as_owner(
            pattern_id,
            Pattern::Binding {
                mutability: pattern_mutability,
                name,
                pattern: None,
                symbol,
            },
        );

        // create the declarator
        let declarator_id = state.tree.reserve_from(
            NodeType::Declarator,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let declarator_id: LocalNodeId<Declarator> = state.tree.insert_as_owner(
            declarator_id,
            Declarator {
                pattern: pattern_id,
                ty: None,
                value,
            },
        );

        // create the let expression
        let let_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Elaborated),
        );
        let let_id: LocalNodeId<Expression> = state.tree.insert_as_owner(
            let_id,
            Expression::Let {
                export: None,
                ambient: Ambientness::Concrete,
                mutability: let_mutability,
                declarators: vec![declarator_id],
            },
        );
        self.set_void_expression_type(state.types, state.ctx.module_id, let_id);

        let_id
    }
}
