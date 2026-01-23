use destack_dir::{
    Declarator, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalSymbolId, NodeTree,
    NodeType, Pattern, SymbolTable,
};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Resolve a direct binding declarator for a symbol.
    pub(crate) fn direct_binding_declarator_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<LocalNodeId<Declarator>> {
        // only local bindings can use local declaration data
        if symbol.module_id != module.id {
            return None;
        }

        // read the primary declaration for the symbol
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;

        // ensure the declaration is local to the module
        if primary_declaration.module_id != module.id {
            return None;
        }

        // require a direct binding for the primary declaration
        if !self.primary_declaration_is_direct_binding(
            primary_declaration.local_id,
            symbol.local_id,
            tree,
        ) {
            return None;
        }

        // walk up to find the declarator containing the binding
        if let Some(declarator_id) =
            self.declarator_parent_for_node(primary_declaration.local_id, tree)
        {
            return Some(declarator_id);
        }

        // scan let or using expressions for a matching direct binding
        self.direct_binding_declarator_in_expression(
            primary_declaration.local_id,
            symbol.local_id,
            tree,
        )
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
