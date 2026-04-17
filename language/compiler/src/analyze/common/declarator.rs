use destack_dir::{
    Declarator, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalSymbolId, NodeTree,
    NodeType, Pattern, TypeExpression,
};

use super::TreeSymbolView;
use crate::Compiler;

impl Compiler {
    /// Resolve a direct binding declarator for a symbol.
    pub(crate) fn direct_binding_declarator_for_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<LocalNodeId<Declarator>> {
        // only local bindings can use local declaration data
        if symbol.module_id != ctx.module.id {
            return None;
        }

        // collect primary and secondary declarations for the symbol
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        let mut declaration_ids = Vec::new();
        if let Some(primary) = symbol_entry.primary_declaration {
            declaration_ids.push(primary);
        }
        if let Some(secondaries) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondaries.iter().copied());
        }

        // find the first direct binding declarator
        for declaration_id in declaration_ids {
            if declaration_id.module_id != ctx.module.id {
                continue;
            }

            if !self.primary_declaration_is_direct_binding(
                declaration_id.local_id,
                symbol.local_id,
                ctx.tree,
            ) {
                continue;
            }

            if let Some(declarator_id) =
                self.declarator_parent_for_node(declaration_id.local_id, ctx.tree)
            {
                return Some(declarator_id);
            }

            if let Some(declarator_id) = self.direct_binding_declarator_in_expression(
                declaration_id.local_id,
                symbol.local_id,
                ctx.tree,
            ) {
                return Some(declarator_id);
            }
        }

        None
    }

    /// Return true when a declarator initializer is a const assertion.
    pub(crate) fn declarator_is_const_assertion(
        &self,
        declarator_id: LocalNodeId<Declarator>,
        tree: &NodeTree,
    ) -> bool {
        let declarator = tree.get(declarator_id);
        let Some(value_id) = declarator.value else {
            return false;
        };

        self.expression_is_const_assertion(value_id, tree)
    }

    /// Return true when an expression is a const assertion.
    pub(crate) fn expression_is_const_assertion(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);

        // `as const`
        let Expression::As { target_type, .. } = tree.get(expression_id) else {
            return false;
        };

        let target_type = self.unwrap_parenthesized_type_expression(*target_type, tree);

        matches!(tree.get(target_type), TypeExpression::Const)
    }

    /// Return true when the primary declaration is a direct binding.
    fn primary_declaration_is_direct_binding(
        &self,
        declaration_id: LocalNodeIdAny,
        symbol: LocalSymbolId,
        tree: &NodeTree,
    ) -> bool {
        // accept direct binding patterns with no destructuring
        if declaration_id.ty == NodeType::Pattern {
            let pattern_id = declaration_id.into_typed::<Pattern>();
            let Pattern::Binding {
                symbol: binding_symbol,
                pattern,
                ..
            } = tree.get(pattern_id)
            else {
                return false;
            };

            return *binding_symbol == symbol && pattern.is_none();
        }

        // reject pattern fields because they are not primary bindings
        if declaration_id.ty == NodeType::PatternField {
            return false;
        }

        true
    }

    /// Walk up the tree to find an enclosing declarator.
    fn declarator_parent_for_node(
        &self,
        node_id: LocalNodeIdAny,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<Declarator>> {
        // climb parents until a declarator is found
        let mut current = node_id;
        loop {
            if current.ty == NodeType::Declarator {
                return Some(current.into_typed());
            }
            let parent = tree.get_parent(current.id)?;
            current = parent;
        }
    }

    /// Find a direct binding declarator inside a let or using expression.
    fn direct_binding_declarator_in_expression(
        &self,
        declaration_id: LocalNodeIdAny,
        symbol: LocalSymbolId,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<Declarator>> {
        // only expressions can contain declarator lists
        if declaration_id.ty != NodeType::Expression {
            return None;
        }

        // select declarator lists from let and using expressions
        let expression_id = declaration_id.into_typed::<Expression>();
        let declarators = match tree.get(expression_id) {
            Expression::Let { declarators, .. } => declarators.as_slice(),
            Expression::Using { declarators, .. } => declarators.as_slice(),
            _ => return None,
        };

        // find a matching direct binding declarator
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            let Pattern::Binding {
                symbol: binding_symbol,
                pattern,
                ..
            } = tree.get(declarator.pattern)
            else {
                continue;
            };

            if *binding_symbol == symbol && pattern.is_none() {
                return Some(*declarator_id);
            }
        }

        None
    }
}
